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
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::WindowResolution,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use threadnations_common::{ChunkCoord, NationId, TileCoord, CHUNK_SIZE};
use threadnations_persistence::{load, save, HistoricalImportState, SavedWorld, WindowState};
use threadnations_simulation::{Nation, World};
use threadnations_worldgen::TerrainType;

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
    entities: BTreeMap<ChunkCoord, Vec<Entity>>,
}

#[derive(Resource, Default)]
struct SelectedNation(Option<NationId>);

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
        .add_systems(Startup, setup_camera)
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
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    let (mut transform, mut projection) = camera.into_inner();
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
    mut rendered: ResMut<RenderedChunks>,
) {
    let tile = TileCoord::new(
        (camera.translation.x / TILE_SIZE).floor() as i32,
        (-camera.translation.y / TILE_SIZE).floor() as i32,
    );
    let center = tile.chunk();
    if rendered.center == Some(center) && rendered.tick == host.world.tick.0 {
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
        .filter(|chunk| !desired.contains(chunk) || rendered.tick != host.world.tick.0)
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
            entry.insert(spawn_chunk(&mut commands, &host.world, chunk));
        }
    }
    rendered.center = Some(center);
    rendered.tick = host.world.tick.0;
}

fn spawn_chunk(commands: &mut Commands, world: &World, chunk: ChunkCoord) -> Vec<Entity> {
    // ponytail: sprites batch well for the current 5x5 visible chunk window; replace this with
    // per-chunk meshes before raising CHUNK_RADIUS or adding dense per-tile effects.
    let mut entities = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
    for tile in world.generator().generate_chunk(chunk).tiles {
        let coord = tile.coord;
        entities.push(
            commands
                .spawn((
                    Sprite::from_color(
                        tile_color(tile.terrain, world.owner_at(coord)),
                        Vec2::splat(TILE_SIZE + 0.2),
                    ),
                    Transform::from_xyz(
                        coord.x as f32 * TILE_SIZE,
                        -coord.y as f32 * TILE_SIZE,
                        0.0,
                    ),
                ))
                .id(),
        );
    }
    for path in &world.paths {
        for coord in std::iter::once(path.from)
            .chain(path.tiles.iter().copied())
            .chain(std::iter::once(path.to))
            .filter(|coord| coord.chunk() == chunk)
        {
            entities.push(
                commands
                    .spawn((
                        Sprite::from_color(
                            Color::srgb_u8(125, 91, 53),
                            Vec2::splat(TILE_SIZE * 0.48),
                        ),
                        Transform::from_xyz(
                            coord.x as f32 * TILE_SIZE,
                            -coord.y as f32 * TILE_SIZE,
                            1.0,
                        ),
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
        entities.push(
            commands
                .spawn((
                    Sprite::from_color(
                        nation_color(building.nation),
                        Vec2::splat(TILE_SIZE * 0.74),
                    ),
                    Transform::from_xyz(
                        building.location.x as f32 * TILE_SIZE,
                        -building.location.y as f32 * TILE_SIZE,
                        2.0,
                    ),
                ))
                .id(),
        );
    }
    entities
}

fn inspector_ui(
    mut contexts: EguiContexts,
    host: Res<SimulationHost>,
    mut selected: ResMut<SelectedNation>,
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
            ui.label("Middle drag to pan · Wheel to zoom");
        });
    });
    egui::Panel::right("nation-card")
        .resizable(true)
        .default_size(320.0)
        .size_range(260.0..=480.0)
        .show(&mut viewport_ui, |ui| {
            ui.heading("Nations");
            egui::ScrollArea::vertical().show(ui, |ui| {
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
                    nation_inspector(ui, nation);
                }
            });
        });
    Ok(())
}

fn nation_inspector(ui: &mut egui::Ui, nation: &Nation) {
    ui.heading(&nation.name);
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

fn tile_color(terrain: TerrainType, owner: Option<NationId>) -> Color {
    if let Some(owner) = owner {
        return nation_color(owner).with_alpha(0.78);
    }
    match terrain {
        TerrainType::Water => Color::srgb_u8(44, 78, 75),
        TerrainType::Ocean | TerrainType::Lake | TerrainType::River => Color::srgb_u8(77, 186, 184),
        TerrainType::Beach => Color::srgb_u8(234, 213, 157),
        TerrainType::Plains => Color::srgb_u8(83, 101, 57),
        TerrainType::Grassland => Color::srgb_u8(106, 157, 70),
        TerrainType::Prairie => Color::srgb_u8(185, 213, 87),
        TerrainType::Forest => Color::srgb_u8(46, 76, 49),
        TerrainType::Hills => Color::srgb_u8(103, 91, 65),
        TerrainType::Mountains => Color::srgb_u8(91, 85, 78),
        TerrainType::Desert => Color::srgb_u8(144, 124, 81),
        TerrainType::Wetlands => Color::srgb_u8(71, 98, 76),
    }
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
    fn terrain_colors_are_stable() {
        assert_eq!(
            tile_color(TerrainType::Water, None),
            Color::srgb_u8(44, 78, 75)
        );
    }
}
