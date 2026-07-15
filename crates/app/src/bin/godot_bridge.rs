//! Local read-only view bridge for the Godot companion.

#[path = "../runtime.rs"]
mod runtime;

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use threadnations_analysis::{activity_points, read_jsonl, ActivityRecord, ActivitySourceKind};
use threadnations_common::{NationId, TileCoord};
use threadnations_persistence::{load, save, HistoricalImportState, SavedWorld, WindowState};
use threadnations_simulation::{Nation, World};

use runtime::{random_world_seed, world_database_path, Config, INITIAL_NATIONS};

const SNAPSHOT_PATH: &str = ".tmp/godot-world.json";
const REQUEST_PATH: &str = ".tmp/godot-view-request.json";

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    View { center: TileCoord, radius: i32 },
}

#[derive(Serialize)]
struct WorldView {
    version: u8,
    tick: u64,
    year: u32,
    activity_total: u64,
    center: TileCoord,
    tiles: Vec<TileView>,
    nations: Vec<NationView>,
    buildings: Vec<BuildingView>,
    paths: Vec<PathView>,
    workers: Vec<WorkerView>,
    wildlife: Vec<WildlifeView>,
    conflicts: Vec<ConflictView>,
    history: Vec<HistoryView>,
}

#[derive(Serialize)]
struct TileView {
    x: i32,
    y: i32,
    terrain: String,
    biome: String,
    owner: Option<u64>,
    wall: bool,
    feature: Option<&'static str>,
}

#[derive(Serialize)]
struct NationView {
    id: u64,
    name: String,
    government: String,
    ruler: String,
    capital: TileCoord,
    population: u64,
    military: u32,
    food: u32,
    materials: u32,
    wealth: u32,
    stability: u8,
    research: u32,
    territory: usize,
}

#[derive(Serialize)]
struct BuildingView {
    nation: u64,
    kind: String,
    location: TileCoord,
}

#[derive(Serialize)]
struct PathView {
    nation: u64,
    trade: bool,
    tiles: Vec<TileCoord>,
}

#[derive(Serialize)]
struct WorkerView {
    id: u64,
    nation: u64,
    role: String,
    location: TileCoord,
}

#[derive(Serialize)]
struct WildlifeView {
    id: u64,
    kind: String,
    location: TileCoord,
    heading: u8,
}

#[derive(Serialize)]
struct ConflictView {
    attacker: u64,
    defender: u64,
}

#[derive(Serialize)]
struct HistoryView {
    year: u32,
    summary: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load().map_err(std::io::Error::other)?;
    let database = world_database_path();
    let (mut world, mut activity_offset, historical_import) =
        load_or_create(&mut config, &database)?;
    prepare_world(&mut world);

    let (mut view_center, mut view_radius) = initial_view(&world);
    write_snapshot(&world, view_center, view_radius)?;
    println!("ThreadNations Godot bridge writing {SNAPSHOT_PATH}");

    let mut last_tick = Instant::now();
    let mut last_ingest = Instant::now();
    let mut last_save = Instant::now();
    let mut last_snapshot = Instant::now();
    loop {
        advance(
            &mut world,
            &config,
            &mut activity_offset,
            &mut last_tick,
            &mut last_ingest,
        );
        if last_save.elapsed() >= Duration::from_secs(config.autosave_seconds.max(5)) {
            persist(&database, &world, activity_offset, &historical_import)?;
            last_save = Instant::now();
        }
        if last_snapshot.elapsed() >= Duration::from_millis(250) {
            if let Some(Request::View { center, radius }) = read_view_request() {
                view_center = center;
                view_radius = radius;
            }
            if let Err(error) = write_snapshot(&world, view_center, view_radius) {
                eprintln!("Godot bridge snapshot failed: {error}");
            }
            last_snapshot = Instant::now();
        }
        thread::sleep(Duration::from_millis(16));
    }
}

fn load_or_create(
    config: &mut Config,
    database: &Path,
) -> Result<(World, u64, HistoricalImportState), Box<dyn std::error::Error>> {
    if let Some(saved) = load(database)? {
        return Ok((saved.world, saved.activity_offset, saved.historical_import));
    }
    config.world_seed = random_world_seed();
    config.save().map_err(std::io::Error::other)?;
    Ok((
        World::new_demo(config.world_seed, INITIAL_NATIONS),
        0,
        HistoricalImportState::default(),
    ))
}

fn prepare_world(world: &mut World) {
    world.relocate_water_capitals();
    world.normalize_territories();
    world.ensure_lords();
    world.refresh_settlement_layout();
    world.refresh_wildlife();
}

fn advance(
    world: &mut World,
    config: &Config,
    activity_offset: &mut u64,
    last_tick: &mut Instant,
    last_ingest: &mut Instant,
) {
    let step = Duration::from_millis(config.calendar_step_ms.max(1));
    let due = u32::try_from(
        (last_tick.elapsed().as_millis() / step.as_millis())
            .min(u128::from(config.max_catchup_ticks)),
    )
    .unwrap_or(config.max_catchup_ticks);
    if due == 0 {
        return;
    }
    let inbox_points = ingest_activity(config, activity_offset, last_ingest);
    for tick in 0..due {
        let synthetic = config.synthetic_activity
            && world
                .tick
                .0
                .is_multiple_of(config.synthetic_interval_ticks.max(1));
        let points = if tick == 0 { inbox_points } else { 0 }
            + if synthetic {
                activity_points(&ActivityRecord {
                    occurred_at_unix_ms: 0,
                    source: ActivitySourceKind::Synthetic,
                    active_seconds: 180,
                    interaction_count: 2,
                    token_estimate: 2_000,
                    generated_bytes: 4_000,
                })
            } else {
                0
            };
        world.advance(points);
    }
    *last_tick += step * due;
    world
        .history
        .truncate(config.history_retention_limit.max(1));
}

fn ingest_activity(config: &Config, activity_offset: &mut u64, last_ingest: &mut Instant) -> u32 {
    if last_ingest.elapsed() < Duration::from_secs(1) {
        return 0;
    }
    *last_ingest = Instant::now();
    let Ok(read) = read_jsonl(Path::new(&config.jsonl_inbox), *activity_offset) else {
        return 0;
    };
    if read.next_offset > *activity_offset {
        *activity_offset = read.next_offset;
    }
    read.records.iter().map(activity_points).sum()
}

fn persist(
    database: &Path,
    world: &World,
    activity_offset: u64,
    historical_import: &HistoricalImportState,
) -> Result<(), Box<dyn std::error::Error>> {
    save(
        database,
        &SavedWorld {
            world: world.clone(),
            activity_offset,
            window: WindowState::default(),
            historical_import: historical_import.clone(),
        },
    )?;
    Ok(())
}

fn initial_view(world: &World) -> (TileCoord, i32) {
    let center = world
        .nations
        .first()
        .and_then(|nation| {
            nation
                .settlements
                .iter()
                .find(|settlement| settlement.id == nation.capital)
        })
        .map(|settlement| settlement.location)
        .unwrap_or_else(|| TileCoord::new(0, 0));
    (center, 24)
}

fn read_view_request() -> Option<Request> {
    serde_json::from_str(&fs::read_to_string(REQUEST_PATH).ok()?).ok()
}

fn write_snapshot(
    world: &World,
    center: TileCoord,
    radius: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(".tmp")?;
    fs::write(
        SNAPSHOT_PATH,
        serde_json::to_vec(&world_view(world, center, radius))?,
    )?;
    Ok(())
}

fn world_view(world: &World, center: TileCoord, radius: i32) -> WorldView {
    let radius = radius.clamp(8, 64);
    let bounds = |tile: TileCoord| {
        (center.x - radius..=center.x + radius).contains(&tile.x)
            && (center.y - radius..=center.y + radius).contains(&tile.y)
    };
    let owners: HashMap<TileCoord, NationId> = world
        .nations
        .iter()
        .flat_map(|nation| {
            nation
                .territory
                .iter()
                .copied()
                .filter(|tile| bounds(*tile))
                .map(move |tile| (tile, nation.id))
        })
        .collect();
    let walls = wall_tiles(world, center, radius);
    WorldView {
        version: 1,
        tick: world.tick.0,
        year: world.year,
        activity_total: world.activity_total,
        center,
        tiles: tile_views(world, &owners, &walls, center, radius),
        nations: world.nations.iter().filter_map(nation_view).collect(),
        buildings: world
            .buildings
            .iter()
            .filter(|building| bounds(building.location))
            .map(|building| BuildingView {
                nation: building.nation.get(),
                kind: format!("{:?}", building.kind).to_lowercase(),
                location: building.location,
            })
            .collect(),
        paths: world
            .paths
            .iter()
            .filter(|path| path.tiles.iter().any(|tile| bounds(*tile)))
            .map(|path| PathView {
                nation: path.nation.get(),
                trade: path.trade_route.is_some(),
                tiles: path.tiles.clone(),
            })
            .collect(),
        workers: world
            .workers
            .iter()
            .filter(|worker| bounds(worker.location))
            .map(|worker| WorkerView {
                id: worker.id.get(),
                nation: worker.nation.get(),
                role: format!("{:?}", worker.role).to_lowercase(),
                location: worker.location,
            })
            .collect(),
        wildlife: world
            .wildlife
            .iter()
            .filter(|animal| bounds(animal.location))
            .map(|animal| WildlifeView {
                id: animal.id.get(),
                kind: format!("{:?}", animal.kind).to_lowercase(),
                location: animal.location,
                heading: animal.heading,
            })
            .collect(),
        conflicts: world
            .conflicts
            .iter()
            .map(|conflict| ConflictView {
                attacker: conflict.attacker.get(),
                defender: conflict.defender.get(),
            })
            .collect(),
        history: world
            .history
            .iter()
            .take(12)
            .map(|event| HistoryView {
                year: event.tick.0.try_into().unwrap_or(u32::MAX),
                summary: event.summary.clone(),
            })
            .collect(),
    }
}

fn tile_views(
    world: &World,
    owners: &HashMap<TileCoord, NationId>,
    walls: &HashSet<TileCoord>,
    center: TileCoord,
    radius: i32,
) -> Vec<TileView> {
    let generator = world.generator();
    (center.y - radius..=center.y + radius)
        .flat_map(|y| (center.x - radius..=center.x + radius).map(move |x| TileCoord::new(x, y)))
        .map(|coord| {
            let (terrain, biome) = generator.terrain_at(coord);
            let feature = if world.is_tree_stump_at(coord) {
                Some("stump")
            } else if natural_tree(coord, terrain) {
                Some("tree")
            } else if visible_rock(world, coord, terrain) {
                Some("rock")
            } else {
                None
            };
            TileView {
                x: coord.x,
                y: coord.y,
                terrain: format!("{terrain:?}").to_lowercase(),
                biome: format!("{biome:?}").to_lowercase(),
                owner: owners.get(&coord).map(|id| id.get()),
                wall: walls.contains(&coord),
                feature,
            }
        })
        .collect()
}

fn wall_tiles(world: &World, center: TileCoord, radius: i32) -> HashSet<TileCoord> {
    let mut walls = HashSet::new();
    for nation in &world.nations {
        let Some((min_x, max_x, min_y, max_y)) =
            nation
                .territory
                .iter()
                .fold(None::<(i32, i32, i32, i32)>, |bounds, coord| {
                    Some(match bounds {
                        Some((min_x, max_x, min_y, max_y)) => (
                            min_x.min(coord.x),
                            max_x.max(coord.x),
                            min_y.min(coord.y),
                            max_y.max(coord.y),
                        ),
                        None => (coord.x, coord.x, coord.y, coord.y),
                    })
                })
        else {
            continue;
        };
        let visible_min_x = min_x.max(center.x - radius);
        let visible_max_x = max_x.min(center.x + radius);
        let visible_min_y = min_y.max(center.y - radius);
        let visible_max_y = max_y.min(center.y + radius);
        for x in visible_min_x..=visible_max_x {
            for y in [min_y, max_y] {
                if (center.y - radius..=center.y + radius).contains(&y) {
                    walls.insert(TileCoord::new(x, y));
                }
            }
        }
        for y in visible_min_y..=visible_max_y {
            for x in [min_x, max_x] {
                if (center.x - radius..=center.x + radius).contains(&x) {
                    walls.insert(TileCoord::new(x, y));
                }
            }
        }
    }
    walls
}

fn natural_tree(coord: TileCoord, terrain: threadnations_worldgen::TerrainType) -> bool {
    let grove = feature_roll(TileCoord::new(coord.x.div_euclid(3), coord.y.div_euclid(3)));
    let canopy = feature_roll(TileCoord::new(coord.x.div_euclid(5), coord.y.div_euclid(5)));
    let local = feature_roll(coord);
    match terrain {
        threadnations_worldgen::TerrainType::Forest => {
            grove % 100 < 88 && canopy % 100 < 88 && local % 100 < 94
        }
        threadnations_worldgen::TerrainType::Prairie => grove % 100 < 18 && local % 100 < 44,
        _ => false,
    }
}

fn visible_rock(
    world: &World,
    coord: TileCoord,
    terrain: threadnations_worldgen::TerrainType,
) -> bool {
    if world.harvested.contains(&coord) {
        return false;
    }
    let chance = match terrain {
        threadnations_worldgen::TerrainType::Beach => 3,
        threadnations_worldgen::TerrainType::Desert => 8,
        threadnations_worldgen::TerrainType::Hills => 1,
        threadnations_worldgen::TerrainType::Mountains => 5,
        _ => 0,
    };
    feature_roll(coord) % 100 < chance
}

fn feature_roll(coord: TileCoord) -> u64 {
    let mut value = i64::from(coord.x)
        .cast_unsigned()
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ i64::from(coord.y)
            .cast_unsigned()
            .wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^ (value >> 31)
}

fn nation_view(nation: &Nation) -> Option<NationView> {
    let capital = nation
        .settlements
        .iter()
        .find(|settlement| settlement.id == nation.capital)?;
    Some(NationView {
        id: nation.id.get(),
        name: nation.name.clone(),
        government: format!("{:?}", nation.government),
        ruler: format!("{} {}", nation.government.lord_title(), nation.lord_name),
        capital: capital.location,
        population: nation.population,
        military: nation.military,
        food: nation.food,
        materials: nation.materials,
        wealth: nation.wealth,
        stability: nation.stability,
        research: nation.research,
        territory: nation.territory.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_clamps_the_requested_area() {
        let world = World::new_demo(4, 1);
        let capital = world.nations[0].settlements[0].location;
        let view = world_view(&world, capital, 1_000);
        assert_eq!(view.tiles.len(), 129 * 129);
        assert!(!view.workers.is_empty());
        assert!(view.workers.iter().all(|worker| {
            (worker.location.x - capital.x).abs() <= 64
                && (worker.location.y - capital.y).abs() <= 64
        }));
        assert!(!view.wildlife.is_empty());
        assert!(view.wildlife.iter().all(|animal| {
            (animal.location.x - capital.x).abs() <= 64
                && (animal.location.y - capital.y).abs() <= 64
        }));
        assert!(view.tiles.iter().any(|tile| tile.wall));
    }
}
