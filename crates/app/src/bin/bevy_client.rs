//! GPU-rendered `ThreadNations` client. The simulation remains independent from Bevy.

// Bevy systems accept ECS wrappers by value, while its 2D API deliberately
// uses f32 coordinates. The simulation remains integer-based.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value
)]

#[path = "../runtime.rs"]
mod runtime;

use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::WindowResolution,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use image::GenericImageView;
use threadnations_common::{ChunkCoord, NationId, TileCoord, CHUNK_SIZE};
use threadnations_persistence::{load, save, HistoricalImportState, SavedWorld, WindowState};
use threadnations_simulation::{Building, BuildingKind, Nation, WildAnimalKind, WorkerRole, World};
use threadnations_worldgen::{Biome, TerrainType, Tile};

use runtime::{random_world_seed, world_database_path, Config, INITIAL_NATIONS};

const TILE_SIZE: f32 = 16.0;
const CHUNK_RADIUS: i32 = 2;

#[derive(Component)]
struct WorldCamera;

#[derive(Resource)]
struct SimulationHost {
    world: World,
    config: Config,
    database: std::path::PathBuf,
    activity_offset: u64,
    historical_import: HistoricalImportState,
    tick_timer: Timer,
    save_timer: Timer,
}

#[derive(Resource, Default)]
struct RenderedChunks {
    center: Option<ChunkCoord>,
    tick: u64,
    borders: bool,
    entities: BTreeMap<ChunkCoord, Vec<Entity>>,
}

#[derive(Resource, Default)]
struct SelectedNation(Option<NationId>);

#[derive(Resource, Default)]
struct ShowBorders(bool);

#[derive(Resource, Default, Eq, PartialEq)]
enum InspectorTab {
    #[default]
    Nation,
    History,
}

#[derive(Clone)]
struct Sheet {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    frames: usize,
}

impl Sheet {
    fn sprite(&self, frame: usize, size: Vec2) -> Sprite {
        Sprite {
            image: self.image.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: self.layout.clone(),
                index: frame % self.frames,
            }),
            custom_size: Some(size),
            ..default()
        }
    }
}

#[derive(Resource)]
struct SpriteAssets {
    grass: Sheet,
    shore: Sheet,
    trees: Sheet,
    pine_trees: Sheet,
    coconut_trees: Sheet,
    cactus: Sheet,
    rocks: Sheet,
    wheat: Sheet,
    farm: Sheet,
    palettes: [BuildingPalette; 5],
    walls: Sheet,
    worker: Sheet,
    soldier: Sheet,
    boar: Sheet,
    fish: Sheet,
}

struct BuildingPalette {
    keep: Sheet,
    houses: Sheet,
    huts: Sheet,
    workshop: Sheet,
    market: Sheet,
    chapel: Sheet,
    barracks: Sheet,
    well: Sheet,
    resources: Sheet,
}

fn main() {
    let host = load_host();
    let window_width = host.config.window_width;
    let window_height = host.config.window_height;
    let borderless = host.config.borderless;
    App::new()
        .insert_resource(ClearColor(Color::srgb_u8(20, 24, 20)))
        .insert_resource(host)
        .init_resource::<RenderedChunks>()
        .init_resource::<SelectedNation>()
        .init_resource::<ShowBorders>()
        .init_resource::<InspectorTab>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ThreadNations".into(),
                resolution: WindowResolution::new(
                    window_width.round() as u32,
                    window_height.round() as u32,
                ),
                resizable: true,
                decorations: !borderless,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, (load_sprite_assets, setup_camera).chain())
        .add_systems(
            Update,
            (
                advance_simulation,
                camera_controls,
                focus_selected_nation,
                refresh_visible_chunks,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, inspector_ui)
        .run();
}

fn load_host() -> SimulationHost {
    let mut config =
        Config::load().unwrap_or_else(|error| panic!("could not load config: {error}"));
    let database = world_database_path();
    let saved = load(&database).ok().flatten();
    let (mut world, activity_offset, historical_import) = saved.map_or_else(
        || {
            config.world_seed = random_world_seed();
            let _ = config.save();
            (
                World::new_demo(config.world_seed, INITIAL_NATIONS),
                0,
                HistoricalImportState::default(),
            )
        },
        |saved| (saved.world, saved.activity_offset, saved.historical_import),
    );
    world.relocate_water_capitals();
    world.normalize_territories();
    world.ensure_lords();
    world.refresh_settlement_layout();
    world.refresh_wildlife();
    SimulationHost {
        world,
        tick_timer: Timer::from_seconds(
            config.calendar_step_ms as f32 / 1_000.0,
            TimerMode::Repeating,
        ),
        save_timer: Timer::from_seconds(
            config.autosave_seconds.max(5) as f32,
            TimerMode::Repeating,
        ),
        config,
        database,
        activity_offset,
        historical_import,
    }
}

#[allow(clippy::too_many_lines)] // Static embedded-asset manifest; splitting it only obscures parity.
fn load_sprite_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    macro_rules! sheet {
        ($path:expr, $cell:expr) => {
            load_sheet(&mut images, &mut layouts, include_bytes!($path), $cell)
        };
    }
    macro_rules! palette {
        (
            $keep:literal, $houses:literal, $huts:literal, $workshop:literal,
            $market:literal, $chapel:literal, $barracks:literal, $resources:literal
        ) => {
            BuildingPalette {
                keep: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $keep),
                    32
                ),
                houses: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $houses),
                    16
                ),
                huts: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $huts),
                    16
                ),
                workshop: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $workshop),
                    16
                ),
                market: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $market),
                    16
                ),
                chapel: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $chapel),
                    16
                ),
                barracks: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $barracks),
                    16
                ),
                well: sheet!(
                    "../../../../assets/MiniWorldSprites/Miscellaneous/Well.png",
                    16
                ),
                resources: sheet!(
                    concat!("../../../../assets/MiniWorldSprites/Buildings/", $resources),
                    16
                ),
            }
        };
    }
    commands.insert_resource(SpriteAssets {
        grass: sheet!("../../../../assets/MiniWorldSprites/Ground/Grass.png", 16),
        shore: sheet!("../../../../assets/MiniWorldSprites/Ground/Shore.png", 16),
        trees: sheet!("../../../../assets/MiniWorldSprites/Nature/Trees.png", 16),
        pine_trees: sheet!("../../../../assets/MiniWorldSprites/Nature/PineTrees.png", 16),
        coconut_trees: sheet!(
            "../../../../assets/MiniWorldSprites/Nature/CoconutTrees.png",
            16
        ),
        cactus: sheet!("../../../../assets/MiniWorldSprites/Nature/Cactus.png", 16),
        rocks: sheet!("../../../../assets/MiniWorldSprites/Nature/Rocks.png", 16),
        wheat: sheet!("../../../../assets/MiniWorldSprites/Nature/Wheatfield.png", 16),
        farm: sheet!(
            "../../../../assets/MiniWorldSprites/Buildings/Enemy/Orc/Farms.png",
            16
        ),
        palettes: [
            palette!(
                "Wood/Keep.png", "Wood/Houses.png", "Wood/Huts.png", "Wood/Workshops.png",
                "Wood/Market.png", "Wood/Chapels.png", "Wood/Barracks.png", "Wood/Resources.png"
            ),
            palette!(
                "Cyan/CyanKeep.png", "Cyan/CyanHouses.png", "Cyan/CyanHuts.png",
                "Cyan/CyanWorkshops.png", "Cyan/CyanMarket.png", "Cyan/CyanChapels.png",
                "Cyan/CyanBarracks.png", "Cyan/CyanResources.png"
            ),
            palette!(
                "Lime/LimeKeep.png", "Lime/LimeHouses.png", "Lime/LimeHuts.png",
                "Lime/LimeWorkshops.png", "Lime/LimeMarket.png", "Lime/LimeChapels.png",
                "Lime/LimeBarracks.png", "Lime/LimeResources.png"
            ),
            palette!(
                "Purple/PurpleKeep.png", "Purple/PurpleHouses.png", "Purple/PurpleHuts.png",
                "Purple/PurpleWorkshops.png", "Purple/PurpleMarket.png", "Purple/PurpleChapels.png",
                "Purple/PurpleBarracks.png", "Purple/PurpleResources.png"
            ),
            palette!(
                "Red/RedKeep.png", "Red/RedHouses.png", "Red/RedHuts.png", "Red/RedWorkshops.png",
                "Red/RedMarket.png", "Red/RedChapels.png", "Red/RedBarracks.png", "Red/RedResources.png"
            ),
        ],
        walls: sheet!(
            "../../../../assets/MiniWorldSprites/Buildings/Enemy/Orc/Walls.png",
            16
        ),
        worker: sheet!(
            "../../../../assets/MiniWorldSprites/Characters/Workers/CyanWorker/FarmerCyan.png",
            16
        ),
        soldier: sheet!(
            "../../../../assets/MiniWorldSprites/Characters/Soldiers/Melee/CyanMelee/SwordsmanCyan.png",
            16
        ),
        boar: sheet!("../../../../assets/MiniWorldSprites/Animals/Boar.png", 16),
        fish: sheet!(
            "../../../../assets/MiniWorldSprites/Animals/MarineAnimals.png",
            16
        ),
    });
}

fn load_sheet(
    images: &mut Assets<Image>,
    layouts: &mut Assets<TextureAtlasLayout>,
    bytes: &[u8],
    cell_size: u32,
) -> Sheet {
    let source =
        image::load_from_memory(bytes).expect("bundled MiniWorld sprite must be valid PNG");
    let (width, height) = source.dimensions();
    let mut image = Image::from_dynamic(source, true, RenderAssetUsages::default());
    image.sampler = ImageSampler::nearest();
    let columns = width / cell_size;
    let rows = height / cell_size;
    Sheet {
        image: images.add(image),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(cell_size),
            columns,
            rows,
            None,
            None,
        )),
        frames: (columns * rows) as usize,
    }
}

fn setup_camera(mut commands: Commands, host: Res<SimulationHost>) {
    let origin = capital(
        &host.world,
        host.world.nations.first().map(|nation| nation.id),
    )
    .map_or(TileCoord::default(), |capital| capital.location);
    commands.spawn((
        Camera2d,
        WorldCamera,
        Transform::from_xyz(
            origin.x as f32 * TILE_SIZE,
            -origin.y as f32 * TILE_SIZE,
            0.0,
        ),
    ));
}

fn advance_simulation(time: Res<Time>, mut host: ResMut<SimulationHost>) {
    host.tick_timer.tick(time.delta());
    if host.tick_timer.just_finished() {
        let activity = u32::from(
            host.config.synthetic_activity
                && host
                    .world
                    .tick
                    .0
                    .is_multiple_of(host.config.synthetic_interval_ticks.max(1)),
        ) * 10;
        host.world.advance(activity);
    }
    host.save_timer.tick(time.delta());
    if host.save_timer.just_finished() {
        let state = SavedWorld {
            world: host.world.clone(),
            activity_offset: host.activity_offset,
            window: WindowState::default(),
            historical_import: host.historical_import.clone(),
        };
        let _ = save(&host.database, &state);
    }
}

fn camera_controls(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    host: Res<SimulationHost>,
    camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    let (mut transform, mut projection) = camera.into_inner();
    if keys.just_pressed(KeyCode::Home) {
        if let Some(capital) = capital(
            &host.world,
            host.world.nations.first().map(|nation| nation.id),
        ) {
            transform.translation =
                world_position(capital.location).extend(transform.translation.z);
        }
    }
    if mouse_buttons.pressed(MouseButton::Middle) {
        let scale = match &*projection {
            Projection::Orthographic(projection) => projection.scale,
            _ => 1.0,
        };
        transform.translation.x -= mouse_motion.delta.x * scale;
        transform.translation.y += mouse_motion.delta.y * scale;
    }
    if let Projection::Orthographic(projection) = &mut *projection {
        projection.scale =
            (projection.scale * (1.0 - mouse_scroll.delta.y * 0.08)).clamp(0.35, 3.0);
    }
}

fn focus_selected_nation(
    selected: Res<SelectedNation>,
    host: Res<SimulationHost>,
    mut camera: Single<&mut Transform, With<WorldCamera>>,
) {
    if !selected.is_changed() {
        return;
    }
    if let Some(capital) = capital(&host.world, selected.0) {
        camera.translation.x = capital.location.x as f32 * TILE_SIZE;
        camera.translation.y = -capital.location.y as f32 * TILE_SIZE;
    }
}

fn refresh_visible_chunks(
    mut commands: Commands,
    camera: Single<&Transform, With<WorldCamera>>,
    host: Res<SimulationHost>,
    sprites: Res<SpriteAssets>,
    borders: Res<ShowBorders>,
    mut rendered: ResMut<RenderedChunks>,
) {
    let tile = TileCoord::new(
        (camera.translation.x / TILE_SIZE).floor() as i32,
        (-camera.translation.y / TILE_SIZE).floor() as i32,
    );
    let center = tile.chunk();
    if rendered.center == Some(center)
        && rendered.tick == host.world.tick.0
        && rendered.borders == borders.0
    {
        return;
    }
    let desired: BTreeSet<_> = ((center.x - CHUNK_RADIUS)..=(center.x + CHUNK_RADIUS))
        .flat_map(|x| {
            ((center.y - CHUNK_RADIUS)..=(center.y + CHUNK_RADIUS))
                .map(move |y| ChunkCoord::new(x, y))
        })
        .collect();
    for chunk in rendered
        .entities
        .keys()
        .copied()
        .filter(|chunk| {
            !desired.contains(chunk)
                || rendered.tick != host.world.tick.0
                || rendered.borders != borders.0
        })
        .collect::<Vec<_>>()
    {
        if let Some(entities) = rendered.entities.remove(&chunk) {
            for entity in entities {
                commands.entity(entity).despawn();
            }
        }
    }
    for chunk in desired {
        if let std::collections::btree_map::Entry::Vacant(entry) = rendered.entities.entry(chunk) {
            entry.insert(spawn_chunk(
                &mut commands,
                &host.world,
                &sprites,
                chunk,
                borders.0,
            ));
        }
    }
    rendered.center = Some(center);
    rendered.tick = host.world.tick.0;
    rendered.borders = borders.0;
}

fn spawn_chunk(
    commands: &mut Commands,
    world: &World,
    sprites: &SpriteAssets,
    chunk: ChunkCoord,
    show_borders: bool,
) -> Vec<Entity> {
    // ponytail: sprites batch well for the current 5x5 visible chunk window; replace this with
    // per-chunk meshes before raising CHUNK_RADIUS or adding dense per-tile effects.
    let mut entities = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE * 3) as usize);
    for tile in world.generator().generate_chunk(chunk).tiles {
        let coord = tile.coord;
        spawn_tile_sprite(
            commands,
            &mut entities,
            ground_sprite(sprites, tile.terrain),
            coord,
            0.0,
        );
        if let Some((sheet, frame)) = decoration(sprites, world, &tile) {
            spawn_tile_sprite(
                commands,
                &mut entities,
                sheet.sprite(frame, Vec2::splat(TILE_SIZE)),
                coord,
                1.0,
            );
        }
        if world.is_wall_tile(coord) {
            spawn_tile_sprite(
                commands,
                &mut entities,
                sprites.walls.sprite(0, Vec2::splat(TILE_SIZE)),
                coord,
                4.0,
            );
        }
    }
    for path in &world.paths {
        let points: Vec<_> = std::iter::once(path.from)
            .chain(path.tiles.iter().copied())
            .chain(std::iter::once(path.to))
            .collect();
        let road_color = if path.trade_route.is_some() {
            Color::srgb_u8(204, 165, 91)
        } else {
            Color::srgb_u8(157, 127, 76)
        };
        for coord in points
            .iter()
            .copied()
            .filter(|coord| coord.chunk() == chunk)
        {
            if is_water(world.tile(coord).terrain) {
                spawn_tile_sprite(
                    commands,
                    &mut entities,
                    Sprite::from_color(road_color, Vec2::splat(TILE_SIZE)),
                    coord,
                    2.0,
                );
            }
        }
        for segment in points
            .windows(2)
            .filter(|segment| segment[0].chunk() == chunk)
        {
            let start = world_position(segment[0]);
            let end = world_position(segment[1]);
            let delta = (end - start).abs();
            entities.push(
                commands
                    .spawn((
                        Sprite::from_color(
                            road_color,
                            Vec2::new(
                                (delta.x + TILE_SIZE * 0.28).max(3.0),
                                (delta.y + TILE_SIZE * 0.28).max(3.0),
                            ),
                        ),
                        Transform::from_translation((start + end).extend(2.2) * 0.5),
                    ))
                    .id(),
            );
        }
    }
    for building in world
        .buildings
        .iter()
        .filter(|building| building.location.chunk() == chunk)
    {
        let (sprite, position) = building_sprite(sprites, world, building);
        entities.push(
            commands
                .spawn((sprite, Transform::from_translation(position.extend(5.0))))
                .id(),
        );
    }
    spawn_units(commands, &mut entities, world, sprites, chunk);
    spawn_settlement_labels(commands, &mut entities, world, chunk);
    if show_borders {
        spawn_borders(commands, &mut entities, world, chunk);
    }
    entities
}

fn spawn_tile_sprite(
    commands: &mut Commands,
    entities: &mut Vec<Entity>,
    sprite: Sprite,
    coord: TileCoord,
    z: f32,
) {
    entities.push(
        commands
            .spawn((
                sprite,
                Transform::from_translation(world_position(coord).extend(z)),
            ))
            .id(),
    );
}

fn world_position(coord: TileCoord) -> Vec2 {
    Vec2::new(coord.x as f32 * TILE_SIZE, -coord.y as f32 * TILE_SIZE)
}

fn ground_sprite(sprites: &SpriteAssets, terrain: TerrainType) -> Sprite {
    let (sheet, frame) = match terrain {
        TerrainType::Water | TerrainType::Ocean | TerrainType::Lake | TerrainType::River => {
            (&sprites.grass, 0)
        }
        TerrainType::Beach | TerrainType::Desert => (&sprites.shore, 0),
        TerrainType::Prairie => (&sprites.grass, 1),
        _ => (&sprites.grass, 2),
    };
    sheet.sprite(frame, Vec2::splat(TILE_SIZE + 0.2))
}

fn decoration<'a>(
    sprites: &'a SpriteAssets,
    world: &World,
    tile: &Tile,
) -> Option<(&'a Sheet, usize)> {
    let variation = sprite_variation(tile.coord);
    match tile.terrain {
        _ if world.is_tree_stump_at(tile.coord)
            && matches!(tile.biome, Biome::Boreal | Biome::Tundra) =>
        {
            Some((&sprites.pine_trees, 0))
        }
        _ if world.is_tree_stump_at(tile.coord) && matches!(tile.biome, Biome::Tropical) => {
            Some((&sprites.coconut_trees, 0))
        }
        _ if world.is_tree_stump_at(tile.coord) => Some((&sprites.trees, 0)),
        _ if world.has_tree_at(tile.coord)
            && matches!(tile.biome, Biome::Boreal | Biome::Tundra) =>
        {
            Some((&sprites.pine_trees, 1))
        }
        _ if world.has_tree_at(tile.coord) && matches!(tile.biome, Biome::Tropical) => {
            Some((&sprites.coconut_trees, 2 + variation % 4))
        }
        _ if world.has_tree_at(tile.coord) => Some((&sprites.trees, 1 + variation % 3)),
        TerrainType::Desert if variation % 100 < 8 => Some((&sprites.cactus, variation % 8)),
        TerrainType::Beach if matches!(tile.biome, Biome::Tropical) && variation % 100 < 18 => {
            Some((&sprites.coconut_trees, 2 + variation % 4))
        }
        _ if world.has_rock_at(tile.coord) => Some((&sprites.rocks, variation % 9)),
        _ => None,
    }
}

fn building_sprite(sprites: &SpriteAssets, world: &World, building: &Building) -> (Sprite, Vec2) {
    let frame = sprite_variation(building.location);
    let palette = &sprites.palettes[building.nation.get() as usize % sprites.palettes.len()];
    let (sheet, size, position) = match building.kind {
        BuildingKind::Keep => (
            &palette.keep,
            TILE_SIZE * 2.0,
            world_position(building.location) + Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 0.5),
        ),
        BuildingKind::House
            if world
                .nation(building.nation)
                .is_some_and(|nation| nation.population < 2_400) =>
        {
            (&palette.huts, TILE_SIZE, world_position(building.location))
        }
        BuildingKind::House => (
            &palette.houses,
            TILE_SIZE,
            world_position(building.location),
        ),
        BuildingKind::Farm => (&sprites.farm, TILE_SIZE, world_position(building.location)),
        BuildingKind::WheatField => (&sprites.wheat, TILE_SIZE, world_position(building.location)),
        BuildingKind::Workshop => (
            &palette.workshop,
            TILE_SIZE,
            world_position(building.location),
        ),
        BuildingKind::Market => (
            &palette.market,
            TILE_SIZE,
            world_position(building.location),
        ),
        BuildingKind::School | BuildingKind::Temple => (
            &palette.chapel,
            TILE_SIZE,
            world_position(building.location),
        ),
        BuildingKind::Barracks => (
            &palette.barracks,
            TILE_SIZE,
            world_position(building.location),
        ),
        BuildingKind::Clinic => (&palette.well, TILE_SIZE, world_position(building.location)),
        BuildingKind::Mine | BuildingKind::Lumberyard => (
            &palette.resources,
            TILE_SIZE,
            world_position(building.location),
        ),
    };
    let frame = match building.kind {
        BuildingKind::Keep => building.nation.get() as usize,
        BuildingKind::Farm => frame % 5,
        BuildingKind::WheatField => 3,
        _ => frame,
    };
    (sheet.sprite(frame, Vec2::splat(size)), position)
}

fn spawn_units(
    commands: &mut Commands,
    entities: &mut Vec<Entity>,
    world: &World,
    sprites: &SpriteAssets,
    chunk: ChunkCoord,
) {
    for animal in world
        .wildlife
        .iter()
        .filter(|animal| animal.location.chunk() == chunk)
    {
        let frame = if animal.kind == WildAnimalKind::Fish {
            fish_sprite_frame(animal.id.get(), animal.heading)
        } else {
            [2, 0, 3, 1][usize::from(animal.heading) % 4] * 4 + animal.id.get() as usize % 4
        };
        let sheet = if animal.kind == WildAnimalKind::Fish {
            &sprites.fish
        } else {
            &sprites.boar
        };
        spawn_tile_sprite(
            commands,
            entities,
            sheet.sprite(frame, Vec2::splat(TILE_SIZE * 0.75)),
            animal.location,
            6.0,
        );
    }
    for worker in world
        .workers
        .iter()
        .filter(|worker| worker.location.chunk() == chunk)
    {
        let sheet = if worker.role == WorkerRole::Hunter {
            &sprites.soldier
        } else {
            &sprites.worker
        };
        spawn_tile_sprite(
            commands,
            entities,
            sheet.sprite(worker.id.get() as usize % 5, Vec2::splat(TILE_SIZE * 0.75)),
            worker.location,
            6.2,
        );
    }
}

fn spawn_settlement_labels(
    commands: &mut Commands,
    entities: &mut Vec<Entity>,
    world: &World,
    chunk: ChunkCoord,
) {
    for settlement in world
        .nations
        .iter()
        .flat_map(|nation| nation.settlements.iter())
        .filter(|settlement| settlement.location.chunk() == chunk)
    {
        entities.push(
            commands
                .spawn((
                    Text2d::new(&settlement.name),
                    TextFont::from_font_size(12.0),
                    TextColor(Color::srgb_u8(34, 38, 29)),
                    Transform::from_translation(
                        (world_position(settlement.location) + Vec2::new(6.0, 5.0)).extend(7.0),
                    ),
                ))
                .id(),
        );
    }
}

fn spawn_borders(
    commands: &mut Commands,
    entities: &mut Vec<Entity>,
    world: &World,
    chunk: ChunkCoord,
) {
    for nation in &world.nations {
        for coord in nation
            .territory
            .iter()
            .copied()
            .filter(|coord| coord.chunk() == chunk)
        {
            for (neighbor, offset, size) in [
                (
                    TileCoord::new(coord.x, coord.y - 1),
                    Vec2::new(0.0, TILE_SIZE * 0.5),
                    Vec2::new(TILE_SIZE, 2.0),
                ),
                (
                    TileCoord::new(coord.x + 1, coord.y),
                    Vec2::new(TILE_SIZE * 0.5, 0.0),
                    Vec2::new(2.0, TILE_SIZE),
                ),
                (
                    TileCoord::new(coord.x, coord.y + 1),
                    Vec2::new(0.0, -TILE_SIZE * 0.5),
                    Vec2::new(TILE_SIZE, 2.0),
                ),
                (
                    TileCoord::new(coord.x - 1, coord.y),
                    Vec2::new(-TILE_SIZE * 0.5, 0.0),
                    Vec2::new(2.0, TILE_SIZE),
                ),
            ] {
                if nation.territory.contains(&neighbor) {
                    continue;
                }
                let position = world_position(coord) + offset;
                entities.push(
                    commands
                        .spawn((
                            Sprite::from_color(Color::srgb_u8(29, 35, 25), size + Vec2::splat(2.0)),
                            Transform::from_translation(position.extend(3.0)),
                        ))
                        .id(),
                );
                entities.push(
                    commands
                        .spawn((
                            Sprite::from_color(nation_color(nation.id), size),
                            Transform::from_translation(position.extend(3.1)),
                        ))
                        .id(),
                );
            }
        }
    }
}

fn is_water(terrain: TerrainType) -> bool {
    matches!(
        terrain,
        TerrainType::Water | TerrainType::Ocean | TerrainType::Lake | TerrainType::River
    )
}

fn fish_sprite_frame(id: u64, heading: u8) -> usize {
    let heading = usize::from(heading) % 4;
    let id = id as usize;
    if !id.is_multiple_of(4) {
        return [2, 3, 1, 0][heading] * 5 + 2 + id % 3;
    }
    match heading {
        0 => 6,
        1 => 15,
        2 => 5,
        _ => 0,
    }
}

fn sprite_variation(coord: TileCoord) -> usize {
    let mut value = (i64::from(coord.x) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (i64::from(coord.y) as u64).wrapping_mul(0xC2B2_AE3D_27D4_E5B9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    (value ^ (value >> 31)) as usize
}

fn inspector_ui(
    mut contexts: EguiContexts,
    host: Res<SimulationHost>,
    mut selected: ResMut<SelectedNation>,
    mut borders: ResMut<ShowBorders>,
    mut tab: ResMut<InspectorTab>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::top("summary").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(format!("Year {}", host.world.year));
            ui.separator();
            ui.label(format!("{} nations", host.world.nations.len()));
            ui.separator();
            ui.label(format!(
                "{} people",
                host.world
                    .nations
                    .iter()
                    .map(|nation| nation.population)
                    .sum::<u64>()
            ));
            ui.separator();
            ui.label(format!("{} trade routes", host.world.trade_routes.len()));
            ui.separator();
            ui.label(format!("{} conflicts", host.world.conflicts.len()));
            ui.separator();
            ui.label("Middle drag to pan · Wheel to zoom · Home resets view");
            ui.separator();
            ui.checkbox(&mut borders.0, "Borders");
        });
    });
    egui::Panel::right("nation-card")
        .resizable(true)
        .default_size(320.0)
        .size_range(260.0..=480.0)
        .show(&mut viewport_ui, |ui| {
            ui.heading("Nations");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut *tab, InspectorTab::Nation, "Nation");
                ui.selectable_value(&mut *tab, InspectorTab::History, "History");
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if *tab == InspectorTab::Nation {
                    for nation in &host.world.nations {
                        if ui
                            .selectable_label(selected.0 == Some(nation.id), &nation.name)
                            .clicked()
                        {
                            selected.0 = Some(nation.id);
                        }
                    }
                    ui.separator();
                    let nation = selected
                        .0
                        .and_then(|id| host.world.nation(id))
                        .or_else(|| host.world.nations.first());
                    if let Some(nation) = nation {
                        nation_inspector(ui, &host.world, nation);
                    }
                } else {
                    history_inspector(ui, &host.world);
                }
            });
        });
    Ok(())
}

fn nation_inspector(ui: &mut egui::Ui, world: &World, nation: &Nation) {
    ui.heading(&nation.name);
    ui.label(format!("Government: {:?}", nation.government));
    ui.label(format!(
        "{} {}",
        nation.government.lord_title(),
        nation.lord_name
    ));
    ui.label(format!(
        "Population: {} · Stability: {}%",
        nation.population, nation.stability
    ));
    ui.label(format!(
        "Military: {} · Research: {}",
        nation.military, nation.research
    ));
    ui.label(format!(
        "Territory: {} tiles · Settlements: {}",
        nation.territory.len(),
        nation.settlements.len()
    ));
    if let Some(capital) = capital(world, Some(nation.id)) {
        ui.label(format!("Capital: {} ({:?})", capital.name, capital.stage));
    }
    ui.separator();
    ui.strong("Economy");
    economy_pair(
        ui,
        "Food",
        nation.food,
        "Net",
        format!(
            "+{} / -{}",
            nation.economy.food_produced, nation.economy.food_consumed
        ),
    );
    economy_pair(
        ui,
        "Materials",
        nation.materials,
        "Net",
        format!(
            "+{} / -{}",
            nation.economy.materials_produced, nation.economy.materials_consumed
        ),
    );
    economy_pair(
        ui,
        "Wealth",
        nation.wealth,
        "Treasury",
        nation.economy.treasury,
    );
    economy_pair(
        ui,
        "Tax",
        format!("{}%", nation.economy.tax_rate),
        "Housing",
        format!("{}/{}", nation.population, nation.economy.housing),
    );
    economy_pair(
        ui,
        "Education",
        format!("{}%", nation.economy.education),
        "Health",
        format!("{}%", nation.economy.health),
    );
    economy_pair(
        ui,
        "Crime",
        format!("{}%", nation.economy.crime),
        "Momentum",
        nation.opportunity,
    );
    ui.label(format!(
        "Stores: wood {} · stone {} · ore {} · goods {} · medicine {}",
        nation.economy.resources.wood,
        nation.economy.resources.stone,
        nation.economy.resources.ore,
        nation.economy.resources.goods,
        nation.economy.resources.medicine
    ));
    ui.separator();
    ui.strong("Jobs");
    for job in &nation.jobs {
        ui.label(format!("{}: {}", job.role.name(), job.workers));
    }
}

fn history_inspector(ui: &mut egui::Ui, world: &World) {
    ui.heading("World history");
    for event in world.history.iter().rev().take(64) {
        let year = 1800_u64.saturating_add(event.tick.0 / 12);
        let nation = event
            .nation
            .and_then(|id| world.nation(id))
            .map_or("World", |nation| nation.name.as_str());
        ui.strong(format!("{year} · {nation}"));
        ui.label(&event.summary);
        ui.separator();
    }
}

fn economy_pair(
    ui: &mut egui::Ui,
    left_label: &str,
    left_value: impl std::fmt::Display,
    right_label: &str,
    right_value: impl std::fmt::Display,
) {
    ui.columns(2, |columns| {
        columns[0].label(format!("{left_label}: {left_value}"));
        columns[1].label(format!("{right_label}: {right_value}"));
    });
}

fn capital(world: &World, id: Option<NationId>) -> Option<&threadnations_simulation::Settlement> {
    let nation = world.nation(id?)?;
    nation
        .settlements
        .iter()
        .find(|settlement| settlement.id == nation.capital)
}

fn nation_color(id: NationId) -> Color {
    const COLORS: [(u8, u8, u8); 6] = [
        (198, 111, 70),
        (177, 151, 65),
        (133, 157, 82),
        (150, 105, 133),
        (190, 128, 84),
        (112, 151, 137),
    ];
    let index = usize::try_from(id.get() % COLORS.len() as u64).unwrap_or_default();
    let (red, green, blue) = COLORS[index];
    Color::srgb_u8(red, green, blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_variation_is_stable() {
        assert_eq!(
            sprite_variation(TileCoord::new(7, -3)),
            544_557_929_564_706_855
        );
        assert_ne!(
            sprite_variation(TileCoord::new(7, -3)),
            sprite_variation(TileCoord::new(8, -3))
        );
    }
}
