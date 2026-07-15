//! Inspectable desktop map for `ThreadNations`. Navigation never changes simulation state.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod runtime;

use std::{
    collections::{HashMap, HashSet},
    env,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Stroke, TextureHandle, TextureOptions, Vec2,
};
use threadnations_analysis::{
    activity_points, import_codex_session_metadata, read_jsonl, ActivityRecord, ActivitySourceKind,
};
use threadnations_common::{NationId, TileCoord};
use threadnations_persistence::{load, save, HistoricalImportState, SavedWorld, WindowState};
use threadnations_simulation::{
    BuildingKind, Conflict, EventKind, Nation, Settlement, TradeRoute, WildAnimalKind, WorkerRole,
    World,
};
use threadnations_worldgen::{Biome, TerrainType};

use runtime::{random_world_seed, world_database_path, Config, INITIAL_NATIONS};

struct ThreadNationsApp {
    config: Config,
    world: World,
    database: PathBuf,
    activity_offset: u64,
    historical_import: HistoricalImportState,
    selected_nation: Option<NationId>,
    inspector_tab: InspectorTab,
    nations_open: bool,
    show_borders: bool,
    map: MapView,
    import_error: Option<String>,
    sprites: Option<SpriteAtlas>,
    sprite_error: Option<String>,
    options_open: bool,
    options_message: Option<String>,
    reset_receiver: Option<Receiver<ResetUpdate>>,
    reset_progress: Option<(u64, u64)>,
    history_import_receiver: Option<Receiver<HistoryImportUpdate>>,
    history_import_progress: Option<(u64, u64)>,
    backup_receiver: Option<Receiver<BackupUpdate>>,
    last_tick: Instant,
    last_ingest: Instant,
    last_save: Instant,
}

#[derive(Clone, Copy, Debug)]
struct MapView {
    center: Vec2,
    home: Vec2,
    tile_size: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectorTab {
    Nation,
    History,
}

struct SpriteAtlas {
    grass: TextureHandle,
    shore: TextureHandle,
    trees: TextureHandle,
    pine_trees: TextureHandle,
    coconut_trees: TextureHandle,
    cactus: TextureHandle,
    rocks: TextureHandle,
    wheat: TextureHandle,
    farm: TextureHandle,
    palettes: [BuildingPalette; 5],
    walls: TextureHandle,
    worker: TextureHandle,
    soldier: TextureHandle,
    boar: TextureHandle,
    fish: TextureHandle,
}

struct BuildingPalette {
    keep: TextureHandle,
    houses: TextureHandle,
    huts: TextureHandle,
}

impl SpriteAtlas {
    fn load(ctx: &egui::Context) -> Result<Self, String> {
        Ok(Self {
            grass: load_texture(ctx, "ground-grass", include_bytes!("../../../assets/MiniWorldSprites/Ground/Grass.png"))?,
            shore: load_texture(ctx, "ground-shore", include_bytes!("../../../assets/MiniWorldSprites/Ground/Shore.png"))?,
            trees: load_texture(ctx, "nature-trees", include_bytes!("../../../assets/MiniWorldSprites/Nature/Trees.png"))?,
            pine_trees: load_texture(ctx, "nature-pines", include_bytes!("../../../assets/MiniWorldSprites/Nature/PineTrees.png"))?,
            coconut_trees: load_texture(ctx, "nature-coconuts", include_bytes!("../../../assets/MiniWorldSprites/Nature/CoconutTrees.png"))?,
            cactus: load_texture(ctx, "nature-cactus", include_bytes!("../../../assets/MiniWorldSprites/Nature/Cactus.png"))?,
            rocks: load_texture(ctx, "nature-rocks", include_bytes!("../../../assets/MiniWorldSprites/Nature/Rocks.png"))?,
            wheat: load_texture(ctx, "nature-wheat", include_bytes!("../../../assets/MiniWorldSprites/Nature/Wheatfield.png"))?,
            farm: load_texture(ctx, "building-farm", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Enemy/Orc/Farms.png"))?,
            palettes: [
                BuildingPalette {
                    keep: load_texture(ctx, "building-wood-keep", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Wood/Keep.png"))?,
                    houses: load_texture(ctx, "building-wood-houses", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Wood/Houses.png"))?,
                    huts: load_texture(ctx, "building-wood-huts", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Wood/Huts.png"))?,
                },
                BuildingPalette {
                    keep: load_texture(ctx, "building-cyan-keep", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Cyan/CyanKeep.png"))?,
                    houses: load_texture(ctx, "building-cyan-houses", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Cyan/CyanHouses.png"))?,
                    huts: load_texture(ctx, "building-cyan-huts", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Cyan/CyanHuts.png"))?,
                },
                BuildingPalette {
                    keep: load_texture(ctx, "building-lime-keep", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Lime/LimeKeep.png"))?,
                    houses: load_texture(ctx, "building-lime-houses", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Lime/LimeHouses.png"))?,
                    huts: load_texture(ctx, "building-lime-huts", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Lime/LimeHuts.png"))?,
                },
                BuildingPalette {
                    keep: load_texture(ctx, "building-purple-keep", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Purple/PurpleKeep.png"))?,
                    houses: load_texture(ctx, "building-purple-houses", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Purple/PurpleHouses.png"))?,
                    huts: load_texture(ctx, "building-purple-huts", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Purple/PurpleHuts.png"))?,
                },
                BuildingPalette {
                    keep: load_texture(ctx, "building-red-keep", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Red/RedKeep.png"))?,
                    houses: load_texture(ctx, "building-red-houses", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Red/RedHouses.png"))?,
                    huts: load_texture(ctx, "building-red-huts", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Red/RedHuts.png"))?,
                },
            ],
            walls: load_texture(ctx, "building-orc-walls", include_bytes!("../../../assets/MiniWorldSprites/Buildings/Enemy/Orc/Walls.png"))?,
            worker: load_texture(ctx, "unit-worker", include_bytes!("../../../assets/MiniWorldSprites/Characters/Workers/CyanWorker/FarmerCyan.png"))?,
            soldier: load_texture(ctx, "unit-soldier", include_bytes!("../../../assets/MiniWorldSprites/Characters/Soldiers/Melee/CyanMelee/SwordsmanCyan.png"))?,
            boar: load_texture(ctx, "animal-boar", include_bytes!("../../../assets/MiniWorldSprites/Animals/Boar.png"))?,
            fish: load_texture(ctx, "animal-fish", include_bytes!("../../../assets/MiniWorldSprites/Animals/MarineAnimals.png"))?,
        })
    }
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Result<TextureHandle, String> {
    let image = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    let rgba = image.to_rgba8();
    let size = [
        usize::try_from(rgba.width()).map_err(|error| error.to_string())?,
        usize::try_from(rgba.height()).map_err(|error| error.to_string())?,
    ];
    Ok(ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
        TextureOptions::NEAREST,
    ))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
impl MapView {
    const MIN_TILE_SIZE: f32 = 12.0;
    const MAX_TILE_SIZE: f32 = 42.0;

    fn for_world(world: &World) -> Self {
        let center = world
            .nations
            .first()
            .and_then(|nation| nation.territory.first())
            .copied()
            .unwrap_or_default();
        Self {
            center: Vec2::new(center.x as f32, center.y as f32),
            home: Vec2::new(center.x as f32, center.y as f32),
            tile_size: 16.0,
        }
    }

    fn screen_position(self, rect: Rect, coord: TileCoord) -> Pos2 {
        rect.center() + (Vec2::new(coord.x as f32, coord.y as f32) - self.center) * self.tile_size
    }

    fn tile_rect(self, rect: Rect, coord: TileCoord) -> Rect {
        let min = rect.center()
            + (Vec2::new(coord.x as f32 - 0.5, coord.y as f32 - 0.5) - self.center)
                * self.tile_size;
        let max = rect.center()
            + (Vec2::new(coord.x as f32 + 0.5, coord.y as f32 + 0.5) - self.center)
                * self.tile_size;
        Rect::from_min_max(min.round(), max.round())
    }

    fn tile_at(self, rect: Rect, pointer: Pos2) -> TileCoord {
        let world = self.center + (pointer - rect.center()) / self.tile_size;
        TileCoord::new(world.x.floor() as i32, world.y.floor() as i32)
    }

    fn pan(&mut self, pixels: Vec2) {
        self.center -= pixels / self.tile_size;
    }

    fn zoom_at(&mut self, rect: Rect, pointer: Pos2, scroll: f32) {
        let before = self.center + (pointer - rect.center()) / self.tile_size;
        self.tile_size = (self.tile_size * (scroll / 520.0).exp())
            .clamp(Self::MIN_TILE_SIZE, Self::MAX_TILE_SIZE);
        self.center = before - (pointer - rect.center()) / self.tile_size;
    }

    fn focus(&mut self, coord: TileCoord) {
        self.center = Vec2::new(coord.x as f32, coord.y as f32);
    }
}

impl ThreadNationsApp {
    fn new(mut config: Config) -> Self {
        let database = world_database_path();
        let (mut world, activity_offset, historical_import) =
            if let Some(saved) = load(&database).ok().flatten() {
                (saved.world, saved.activity_offset, saved.historical_import)
            } else {
                config.world_seed = random_world_seed();
                let _ = config.save();
                (
                    World::new_demo(config.world_seed, INITIAL_NATIONS),
                    0,
                    HistoricalImportState::default(),
                )
            };
        world.relocate_water_capitals();
        world.normalize_territories();
        world.ensure_lords();
        world.refresh_settlement_layout();
        world.refresh_wildlife();
        let map = MapView::for_world(&world);
        Self {
            config,
            world,
            database,
            activity_offset,
            historical_import,
            selected_nation: None,
            inspector_tab: InspectorTab::Nation,
            nations_open: false,
            show_borders: false,
            map,
            import_error: None,
            sprites: None,
            sprite_error: None,
            options_open: false,
            options_message: None,
            reset_receiver: None,
            reset_progress: None,
            history_import_receiver: None,
            history_import_progress: None,
            backup_receiver: None,
            last_tick: Instant::now(),
            last_ingest: Instant::now(),
            last_save: Instant::now(),
        }
    }

    fn ingest_activity(&mut self) -> u32 {
        if self.last_ingest.elapsed() < Duration::from_secs(1) {
            return 0;
        }
        self.last_ingest = Instant::now();
        let Ok(read) = read_jsonl(Path::new(&self.config.jsonl_inbox), self.activity_offset) else {
            return 0;
        };
        if read.next_offset > self.activity_offset {
            self.activity_offset = read.next_offset;
        }
        read.records.iter().map(activity_points).sum()
    }

    fn ensure_sprites(&mut self, ctx: &egui::Context) {
        if self.sprites.is_none() && self.sprite_error.is_none() {
            match SpriteAtlas::load(ctx) {
                Ok(sprites) => self.sprites = Some(sprites),
                Err(error) => self.sprite_error = Some(error),
            }
        }
    }

    fn reset_world(&mut self) {
        if self.operation_in_progress() {
            return;
        }
        let seed = random_world_seed();
        let initial_nations = INITIAL_NATIONS;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let world = World::new_demo_with_progress(seed, initial_nations, |completed, total| {
                let _ = sender.send(ResetUpdate::Progress {
                    completed: u64::from(completed),
                    total: u64::from(total),
                });
            });
            let _ = sender.send(ResetUpdate::Completed(seed, Box::new(world)));
        });
        self.reset_receiver = Some(receiver);
        self.reset_progress = Some((0, u64::from(initial_nations.saturating_add(1))));
        self.options_message = Some("Creating a new world…".into());
    }

    fn finish_reset(&mut self, seed: u64, world: World) {
        self.config.world_seed = seed;
        self.world = world;
        self.map = MapView::for_world(&self.world);
        self.selected_nation = None;
        self.activity_offset = 0;
        self.historical_import = HistoricalImportState::default();
        self.options_message = match self.config.save() {
            Ok(()) => {
                self.persist();
                Some("Created a new world seed.".into())
            }
            Err(error) => Some(format!("World reset, but config save failed: {error}")),
        };
    }

    fn poll_reset(&mut self) {
        let result = match self.reset_receiver.as_ref() {
            Some(receiver) => receiver.try_recv(),
            None => return,
        };
        match result {
            Ok(ResetUpdate::Progress { completed, total }) => {
                self.reset_progress = Some((completed, total));
            }
            Ok(ResetUpdate::Completed(seed, world)) => {
                self.reset_receiver = None;
                self.reset_progress = None;
                self.finish_reset(seed, *world);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.reset_receiver = None;
                self.reset_progress = None;
                self.options_message = Some("World reset failed before it could finish.".into());
            }
        }
    }

    fn poll_backup(&mut self) {
        let result = match self.backup_receiver.as_ref() {
            Some(receiver) => receiver.try_recv(),
            None => return,
        };
        match result {
            Ok(BackupUpdate::Completed) => {
                self.backup_receiver = None;
                self.options_message = Some("World backup saved.".into());
            }
            Ok(BackupUpdate::Failed(error)) => {
                self.backup_receiver = None;
                self.options_message = Some(format!("Backup failed: {error}"));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.backup_receiver = None;
                self.options_message = Some("Backup stopped unexpectedly.".into());
            }
        }
    }

    fn backup_world(&mut self) {
        if self.operation_in_progress() {
            return;
        }
        let Some(destination) = rfd::FileDialog::new()
            .set_title("Back up ThreadNations world")
            .add_filter("ThreadNations world", &["sqlite"])
            .set_file_name("threadnations-backup.sqlite")
            .save_file()
        else {
            return;
        };
        let state = self.saved_world();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let update = save(&destination, &state).map_or_else(
                |error| BackupUpdate::Failed(error.to_string()),
                |()| BackupUpdate::Completed,
            );
            let _ = sender.send(update);
        });
        self.backup_receiver = Some(receiver);
        self.options_message = Some("Saving world backup…".into());
    }

    fn restore_world(&mut self) {
        if self.operation_in_progress() {
            return;
        }

        let Some(source) = rfd::FileDialog::new()
            .set_title("Restore ThreadNations world")
            .add_filter("ThreadNations world", &["sqlite"])
            .pick_file()
        else {
            return;
        };
        self.options_message = match load(&source) {
            Ok(Some(saved)) => {
                self.world = saved.world;
                self.world.relocate_water_capitals();
                self.world.normalize_territories();
                self.world.ensure_lords();
                self.world.refresh_settlement_layout();
                self.world.refresh_wildlife();
                self.config.world_seed = self.world.seed;
                self.activity_offset = saved.activity_offset;
                self.historical_import = saved.historical_import;
                self.selected_nation = None;
                self.map = MapView::for_world(&self.world);
                match self.config.save() {
                    Ok(()) => {
                        self.persist();
                        Some("World restored.".into())
                    }
                    Err(error) => Some(format!("World restored, but config save failed: {error}")),
                }
            }
            Ok(None) => Some("That file does not contain a saved world.".into()),
            Err(error) => Some(format!("Restore failed: {error}")),
        };
    }

    fn advance(&mut self) {
        let step = Duration::from_millis(self.config.calendar_step_ms.max(1));
        let due = u32::try_from(
            (self.last_tick.elapsed().as_millis() / step.as_millis())
                .min(u128::from(self.config.max_catchup_ticks)),
        )
        .unwrap_or(self.config.max_catchup_ticks);
        if due == 0 {
            return;
        }
        let inbox_points = self.ingest_activity();
        for tick in 0..due {
            let synthetic = self.config.synthetic_activity
                && self
                    .world
                    .tick
                    .0
                    .is_multiple_of(self.config.synthetic_interval_ticks.max(1));
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
            self.world.advance(points);
        }
        self.last_tick += step * due;
        self.world
            .history
            .truncate(self.config.history_retention_limit.max(1));
    }

    fn saved_world(&self) -> SavedWorld {
        SavedWorld {
            world: self.world.clone(),
            activity_offset: self.activity_offset,
            window: WindowState {
                width: self.config.window_width,
                height: self.config.window_height,
                x: None,
                y: None,
            },
            historical_import: self.historical_import.clone(),
        }
    }

    fn persist(&mut self) {
        let state = self.saved_world();
        if let Err(error) = save(&self.database, &state) {
            eprintln!("ThreadNations autosave failed: {error}");
        }
        self.last_save = Instant::now();
    }

    fn featured_nation(&self) -> Option<&Nation> {
        self.world
            .nations
            .get((self.world.tick.0 / 24) as usize % self.world.nations.len().max(1))
    }

    fn selected_nation(&self) -> Option<&Nation> {
        self.selected_nation.and_then(|id| self.world.nation(id))
    }

    fn select_nation(&mut self, nation_id: NationId) {
        let Some(capital) = self.world.nation(nation_id).and_then(|nation| {
            nation
                .settlements
                .iter()
                .find(|city| city.id == nation.capital)
        }) else {
            return;
        };
        self.map.focus(capital.location);
        self.selected_nation = Some(nation_id);
        self.inspector_tab = InspectorTab::Nation;
    }

    fn has_imported_history(&self) -> bool {
        self.historical_import.imported || self.historical_import.imported_records > 0
    }

    fn operation_in_progress(&self) -> bool {
        self.reset_receiver.is_some()
            || self.history_import_receiver.is_some()
            || self.backup_receiver.is_some()
    }

    fn import_codex_history(&mut self) {
        if self.has_imported_history() || self.operation_in_progress() {
            return;
        }
        let root = codex_sessions_root();
        let mut world = self.world.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || match import_codex_session_metadata(&root) {
            Ok(records) => {
                world.seed_nations_from_import(records.len());
                let total = u64::try_from(records.len()).unwrap_or(u64::MAX);
                let _ = sender.send(HistoryImportUpdate::Progress {
                    completed: 0,
                    total,
                });
                let mut completed = 0_u64;
                for batch in records.chunks(32) {
                    let points = batch.iter().map(activity_points).sum();
                    world.advance(points);
                    completed =
                        completed.saturating_add(u64::try_from(batch.len()).unwrap_or(u64::MAX));
                    let _ = sender.send(HistoryImportUpdate::Progress { completed, total });
                }
                let state = HistoricalImportState {
                    answered: true,
                    imported: true,
                    imported_records: total,
                };
                let _ = sender.send(HistoryImportUpdate::Completed(Box::new(world), state));
            }
            Err(error) => {
                let _ = sender.send(HistoryImportUpdate::Failed(error.to_string()));
            }
        });
        self.history_import_receiver = Some(receiver);
        self.history_import_progress = Some((0, 0));
        self.import_error = None;
        self.options_message = Some("Finding previous history…".into());
    }

    fn poll_history_import(&mut self) {
        let result = match self.history_import_receiver.as_ref() {
            Some(receiver) => receiver.try_recv(),
            None => return,
        };
        match result {
            Ok(HistoryImportUpdate::Progress { completed, total }) => {
                self.history_import_progress = Some((completed, total));
                self.options_message = Some("Importing previous history…".into());
            }
            Ok(HistoryImportUpdate::Completed(world, state)) => {
                self.history_import_receiver = None;
                self.history_import_progress = None;
                self.world = *world;
                self.historical_import = state;
                self.import_error = None;
                self.options_message = Some("Previous history imported.".into());
                self.persist();
            }
            Ok(HistoryImportUpdate::Failed(error)) => {
                self.history_import_receiver = None;
                self.history_import_progress = None;
                self.import_error = Some(error);
                self.options_message = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.history_import_receiver = None;
                self.history_import_progress = None;
                self.import_error = Some("History import stopped unexpectedly.".into());
                self.options_message = None;
            }
        }
    }

    fn show_history_import_progress(&self, ui: &mut egui::Ui) {
        show_progress(
            ui,
            self.history_import_progress,
            "Finding previous sessions…",
            |completed, total| format!("Importing {completed} of {total} records"),
        );
    }

    fn show_reset_progress(&self, ui: &mut egui::Ui) {
        show_progress(
            ui,
            self.reset_progress,
            "Creating a new world…",
            |completed, total| format!("Creating world: {completed} of {total} stages"),
        );
    }

    fn show_backup_progress(&self, ui: &mut egui::Ui) {
        if self.backup_receiver.is_some() {
            ui.add(
                egui::ProgressBar::new(0.5)
                    .animate(true)
                    .text("Saving world backup…"),
            );
        }
    }

    fn show_nations_menu(&mut self, ctx: &egui::Context) {
        if !self.nations_open {
            return;
        }
        let mut selected = None;
        egui::Window::new("Nations")
            .open(&mut self.nations_open)
            .resizable(false)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.collapsing("All nations", |ui| {
                    for nation in &self.world.nations {
                        let capital = nation
                            .settlements
                            .iter()
                            .find(|city| city.id == nation.capital)
                            .map_or("Unknown capital", |city| city.name.as_str());
                        if ui
                            .selectable_label(
                                self.selected_nation == Some(nation.id),
                                format!("{} — {}", nation.name, capital),
                            )
                            .clicked()
                        {
                            selected = Some(nation.id);
                        }
                    }
                });
            });
        if let Some(nation_id) = selected {
            self.select_nation(nation_id);
        }
    }
}

fn show_progress(
    ui: &mut egui::Ui,
    progress: Option<(u64, u64)>,
    pending_text: &str,
    complete_text: impl FnOnce(u64, u64) -> String,
) {
    let Some((completed, total)) = progress else {
        return;
    };
    let progress = f32::from(
        u16::try_from(
            completed
                .saturating_mul(10_000)
                .checked_div(total)
                .unwrap_or_default(),
        )
        .unwrap_or(10_000)
        .min(10_000),
    ) / 10_000.0;
    let text = if total == 0 {
        pending_text.into()
    } else {
        complete_text(completed, total)
    };
    ui.add(
        egui::ProgressBar::new(progress)
            .show_percentage()
            .text(text),
    );
}

enum ResetUpdate {
    Progress { completed: u64, total: u64 },
    Completed(u64, Box<World>),
}

enum HistoryImportUpdate {
    Progress { completed: u64, total: u64 },
    Completed(Box<World>, HistoricalImportState),
    Failed(String),
}

enum BackupUpdate {
    Completed,
    Failed(String),
}

impl eframe::App for ThreadNationsApp {
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.ensure_sprites(ctx);
        self.poll_reset();
        self.poll_history_import();
        self.poll_backup();
        if !self.operation_in_progress() {
            self.advance();
        }
        if !self.operation_in_progress()
            && self.last_save.elapsed() >= Duration::from_secs(self.config.autosave_seconds.max(5))
        {
            self.persist();
        }
        egui::TopBottomPanel::top("summary")
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(28, 29, 25))
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Year {}", self.world.year))
                            .font(FontId::monospace(18.0))
                            .color(Color32::from_rgb(235, 229, 211)),
                    );
                    ui.separator();
                    ui.label(format!("{} nations", self.world.nations.len()));
                    ui.separator();
                    ui.label(format!(
                        "{} people",
                        self.world
                            .nations
                            .iter()
                            .map(|nation| nation.population)
                            .sum::<u64>()
                    ));
                    ui.separator();
                    ui.label(format!("{} trade routes", self.world.trade_routes.len()));
                    ui.separator();
                    ui.label(format!("{} conflicts", self.world.conflicts.len()));
                    ui.separator();
                    ui.label("Drag to pan · Wheel to zoom");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Options").clicked() {
                            self.options_open = true;
                        }
                        if ui.button("Nations").clicked() {
                            self.nations_open = !self.nations_open;
                        }
                        if ui.selectable_label(self.show_borders, "Borders").clicked() {
                            self.show_borders = !self.show_borders;
                        }
                    });
                });
            });
        egui::TopBottomPanel::bottom("ticker")
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(28, 29, 25))
                    .inner_margin(egui::Margin::symmetric(16.0, 9.0)),
            )
            .show(ctx, |ui| {
                let text = self
                    .world
                    .history
                    .first()
                    .map_or("World is settling into its first year.", |event| {
                        event.summary.as_str()
                    });
                ui.label(egui::RichText::new(text).color(Color32::from_rgb(208, 201, 183)));
            });
        egui::SidePanel::right("nation-card")
            .exact_width(250.0)
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(35, 36, 31))
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.inspector_tab,
                                InspectorTab::Nation,
                                "Nation",
                            );
                            ui.selectable_value(
                                &mut self.inspector_tab,
                                InspectorTab::History,
                                "History",
                            );
                        });
                        ui.separator();
                        match self.inspector_tab {
                            InspectorTab::Nation => {
                                let selected = self.selected_nation().cloned();
                                if let Some(nation) = selected {
                                    nation_panel(ui, &self.world, &nation);
                                    ui.add_space(12.0);
                                    if ui.button("Clear selection").clicked() {
                                        self.selected_nation = None;
                                    }
                                } else if let Some(nation) = self.featured_nation() {
                                    ui.label("Select a nation on the map for its full report.");
                                    ui.add_space(12.0);
                                    nation_overview(ui, &self.world, nation);
                                }
                            }
                            InspectorTab::History => history_panel(ui, &self.world),
                        }
                    });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_rgb(20, 24, 20)))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response = ui.interact(
                    rect,
                    ui.id().with("world-map"),
                    egui::Sense::click_and_drag(),
                );
                if response.dragged() {
                    self.map.pan(ui.input(|input| input.pointer.delta()));
                }
                if response.hovered() {
                    let scroll = ui.input(|input| input.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        self.map.zoom_at(
                            rect,
                            response.hover_pos().unwrap_or(rect.center()),
                            scroll,
                        );
                    }
                }
                if response.clicked() {
                    self.selected_nation = response
                        .interact_pointer_pos()
                        .and_then(|pointer| self.world.owner_at(self.map.tile_at(rect, pointer)));
                    if self.selected_nation.is_some() {
                        self.inspector_tab = InspectorTab::Nation;
                    }
                }
                let home_rect = Rect::from_min_size(
                    rect.left_top() + Vec2::new(14.0, 14.0),
                    Vec2::new(58.0, 26.0),
                );
                if ui.put(home_rect, egui::Button::new("Home")).clicked() {
                    self.map.center = self.map.home;
                }
                draw_world(
                    ui.painter(),
                    rect,
                    &self.world,
                    self.map,
                    self.selected_nation,
                    self.show_borders,
                    self.sprites.as_ref(),
                    ctx.input(|input| input.time),
                );
                draw_minimap(
                    ui.painter(),
                    rect,
                    &self.world,
                    self.map,
                    self.selected_nation,
                );
                if let Some(error) = &self.sprite_error {
                    ui.painter().text(
                        rect.left_top() + Vec2::splat(12.0),
                        egui::Align2::LEFT_TOP,
                        format!("Sprite load failed: {error}"),
                        FontId::monospace(12.0),
                        Color32::from_rgb(187, 83, 68),
                    );
                }
            });
        self.show_historical_import_prompt(ctx);
        self.show_options(ctx);
        self.show_nations_menu(ctx);
        ctx.request_repaint_after(Duration::from_millis(if self.map.tile_size <= 16.0 {
            33
        } else {
            16
        }));
    }

    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.persist();
    }
}

impl ThreadNationsApp {
    fn show_options(&mut self, ctx: &egui::Context) {
        if !self.options_open {
            return;
        }
        let mut open = self.options_open;
        egui::Window::new("Options")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("World saves");
                ui.add_space(6.0);
                let busy = self.operation_in_progress();
                if ui
                    .add_enabled(!busy, egui::Button::new("Reset world (new seed)"))
                    .clicked()
                {
                    self.reset_world();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Back up world…"))
                    .clicked()
                {
                    self.backup_world();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Restore world…"))
                    .clicked()
                {
                    self.restore_world();
                }
                self.show_reset_progress(ui);
                self.show_backup_progress(ui);
                ui.add_space(12.0);
                ui.separator();
                ui.label("Previous activity");
                let imported = self.has_imported_history();
                if ui
                    .add_enabled(
                        !imported && !busy,
                        egui::Button::new("Import previous history"),
                    )
                    .clicked()
                {
                    self.import_codex_history();
                }
                if imported {
                    ui.small("Already imported for this world. Reset to import again.");
                }
                self.show_history_import_progress(ui);
                if let Some(error) = &self.import_error {
                    ui.colored_label(Color32::from_rgb(187, 83, 68), error);
                }
                if let Some(message) = &self.options_message {
                    ui.add_space(8.0);
                    ui.label(message);
                }
            });
        self.options_open = open;
    }

    fn show_historical_import_prompt(&mut self, ctx: &egui::Context) {
        if self.historical_import.answered {
            return;
        }
        egui::Window::new("Import previous activity")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Import local Codex session metadata before this world begins?");
                ui.add_space(8.0);
                ui.label("Reads only file times and sizes. Conversation text, names, prompts, and responses remain unread.");
                if let Some(error) = &self.import_error {
                    ui.colored_label(Color32::from_rgb(187, 83, 68), error);
                }
                self.show_history_import_progress(ui);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.operation_in_progress(),
                            egui::Button::new("Import local history"),
                        )
                        .clicked()
                    {
                        self.import_codex_history();
                    }
                    if ui
                        .add_enabled(
                            !self.operation_in_progress(),
                            egui::Button::new("Start without import"),
                        )
                        .clicked()
                    {
                        self.historical_import.answered = true;
                        self.persist();
                    }
                });
            });
    }
}

fn passive_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(Color32::from_rgb(153, 151, 137)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(value)
        });
    });
    ui.add_space(6.0);
}

fn nation_panel(ui: &mut egui::Ui, world: &World, nation: &Nation) {
    ui.label(
        egui::RichText::new(&nation.name)
            .font(FontId::monospace(20.0))
            .color(nation_color(nation.id.get())),
    );
    ui.add_space(12.0);
    nation_overview(ui, world, nation);
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Population").color(Color32::from_rgb(208, 201, 183)));
    let units = population_units(nation);
    passive_row(ui, "Civilians", units.civilians.to_string());
    passive_row(ui, "Workforce", units.workforce.to_string());
    passive_row(ui, "Educated workers", units.educated.to_string());
    passive_row(ui, "Military", units.military.to_string());
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Jobs").color(Color32::from_rgb(208, 201, 183)));
    for job in &nation.jobs {
        passive_row(ui, job.role.name(), job.workers.to_string());
    }
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Map units").color(Color32::from_rgb(208, 201, 183)));
    ui.label(
        professional_units(nation)
            .iter()
            .map(|profession| profession.name())
            .collect::<Vec<_>>()
            .join(" · "),
    );
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Economy").color(Color32::from_rgb(208, 201, 183)));
    passive_row(ui, "Food stores", nation.food.to_string());
    passive_row(ui, "Materials", nation.materials.to_string());
    passive_row(ui, "Wealth", nation.wealth.to_string());
    passive_row(ui, "Opportunity", nation.opportunity.to_string());
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Relations").color(Color32::from_rgb(208, 201, 183)));
    for relation in nation_relations(world, nation.id) {
        ui.label(relation);
    }
}

fn nation_overview(ui: &mut egui::Ui, world: &World, nation: &Nation) {
    passive_row(ui, "Government", format!("{:?}", nation.government));
    passive_row(
        ui,
        "Ruler",
        format!("{} {}", nation.government.lord_title(), nation.lord_name),
    );
    passive_row(ui, "Population", nation.population.to_string());
    passive_row(ui, "Military strength", nation.military.to_string());
    passive_row(ui, "Research capacity", nation.research.to_string());
    passive_row(ui, "Stability", format!("{}%", nation.stability));
    passive_row(ui, "Territory", format!("{} tiles", nation.territory.len()));
    passive_row(ui, "Settlements", nation.settlements.len().to_string());
    if let Some(capital) = nation
        .settlements
        .iter()
        .find(|city| city.id == nation.capital)
    {
        passive_row(
            ui,
            "Capital",
            format!("{} ({:?})", capital.name, capital.stage),
        );
    }
    passive_row(ui, "Tendency", tendency(world, nation).to_owned());
    passive_row(ui, "Status", status(world, nation));
}

fn history_panel(ui: &mut egui::Ui, world: &World) {
    ui.label(egui::RichText::new("World history").color(Color32::from_rgb(208, 201, 183)));
    ui.add_space(8.0);
    let mut border_counts = HashMap::new();
    for event in &world.history {
        if event.kind == EventKind::BorderExpanded {
            if let Some(nation) = event.nation {
                *border_counts.entry(nation).or_insert(0_usize) += 1;
            }
        }
    }
    let mut shown_borders = HashSet::new();
    for event in &world.history {
        if event.kind == EventKind::BorderExpanded {
            let Some(nation_id) = event.nation else {
                continue;
            };
            if !shown_borders.insert(nation_id) {
                continue;
            }
            let nation = world.nation(nation_id);
            let name = nation.map_or("A nation", |nation| nation.name.as_str());
            let maximum_size = nation.map_or(0, |nation| nation.territory.len());
            let count = border_counts.get(&nation_id).copied().unwrap_or_default();
            ui.label(format!(
                "{name} grew its border {count} times, reaching {maximum_size} tiles."
            ));
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            continue;
        }
        let year = 1800_u64.saturating_add(event.tick.0 / 12);
        let nation = event
            .nation
            .and_then(|nation_id| world.nation(nation_id))
            .map_or("World", |nation| nation.name.as_str());
        ui.label(
            egui::RichText::new(format!("{year} · {nation}"))
                .color(Color32::from_rgb(153, 151, 137)),
        );
        ui.label(&event.summary);
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PopulationUnits {
    civilians: u64,
    workforce: u64,
    educated: u64,
    military: u64,
}

fn population_units(nation: &Nation) -> PopulationUnits {
    PopulationUnits {
        civilians: nation.civilians,
        workforce: nation.workforce,
        educated: nation.educated_workforce,
        military: u64::from(nation.military),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Profession {
    Farmer,
    Teacher,
    Engineer,
    Soldier,
    Scientist,
    Historian,
}

impl Profession {
    const fn name(self) -> &'static str {
        match self {
            Self::Farmer => "Farmer",
            Self::Teacher => "Teacher",
            Self::Engineer => "Engineer",
            Self::Soldier => "Soldier",
            Self::Scientist => "Scientist",
            Self::Historian => "Historian",
        }
    }

    const fn marker(self) -> &'static str {
        match self {
            Self::Farmer => "F",
            Self::Teacher => "T",
            Self::Engineer => "E",
            Self::Soldier => "S",
            Self::Scientist => "C",
            Self::Historian => "H",
        }
    }

    const fn color(self) -> Color32 {
        match self {
            Self::Farmer => Color32::from_rgb(153, 164, 92),
            Self::Teacher => Color32::from_rgb(205, 191, 127),
            Self::Engineer => Color32::from_rgb(174, 138, 91),
            Self::Soldier => Color32::from_rgb(179, 81, 65),
            Self::Scientist => Color32::from_rgb(135, 119, 178),
            Self::Historian => Color32::from_rgb(170, 132, 97),
        }
    }
}

fn professional_units(nation: &Nation) -> Vec<Profession> {
    let mut professions = vec![Profession::Farmer];
    if nation.population >= 1_000 {
        professions.push(Profession::Teacher);
    }
    if nation.materials >= 80 {
        professions.push(Profession::Engineer);
    }
    if nation.military > 0 {
        professions.push(Profession::Soldier);
    }
    if nation.research > 0 {
        professions.push(Profession::Scientist);
    }
    if nation.wealth >= 80 {
        professions.push(Profession::Historian);
    }
    professions
}

fn nation_relations(world: &World, nation_id: NationId) -> Vec<String> {
    let mut lines = Vec::new();
    for route in &world.trade_routes {
        let other = if route.from == nation_id {
            route.to
        } else if route.to == nation_id {
            route.from
        } else {
            continue;
        };
        if let Some((nation, city)) = capital(world, other) {
            lines.push(format!("Trade: {} ({})", city.name, nation.name));
        }
    }
    for war in &world.conflicts {
        let other = if war.attacker == nation_id {
            war.defender
        } else if war.defender == nation_id {
            war.attacker
        } else {
            continue;
        };
        if let Some((nation, city)) = capital(world, other) {
            lines.push(format!("Conflict: {} ({})", city.name, nation.name));
        }
    }
    if lines.is_empty() {
        lines.push("No external city relations.".into());
    }
    lines
}

fn capital(
    world: &World,
    nation_id: threadnations_common::NationId,
) -> Option<(&Nation, &Settlement)> {
    let nation = world.nation(nation_id)?;
    nation
        .settlements
        .iter()
        .find(|settlement| settlement.id == nation.capital)
        .map(|city| (nation, city))
}

fn tendency(world: &World, nation: &Nation) -> &'static str {
    let (fertility, minerals) =
        nation
            .territory
            .iter()
            .fold((0, 0), |(fertility, minerals), coord| {
                let tile = world.tile(*coord);
                (
                    fertility + u32::from(tile.is_viable_spawn()),
                    minerals
                        + u32::from(matches!(
                            tile.terrain,
                            TerrainType::Hills | TerrainType::Mountains
                        )),
                )
            });
    match fertility.cmp(&minerals) {
        std::cmp::Ordering::Greater => "food security",
        std::cmp::Ordering::Less => "material extraction",
        std::cmp::Ordering::Equal => "urban development",
    }
}

fn status(world: &World, nation: &Nation) -> String {
    if world
        .conflicts
        .iter()
        .any(|war| war.attacker == nation.id || war.defender == nation.id)
    {
        "in conflict".into()
    } else if world
        .trade_routes
        .iter()
        .any(|route| route.from == nation.id || route.to == nation.id)
    {
        "trading".into()
    } else {
        "independent".into()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
fn draw_world(
    painter: &egui::Painter,
    rect: Rect,
    world: &World,
    map: MapView,
    selected_nation: Option<NationId>,
    show_borders: bool,
    sprites: Option<&SpriteAtlas>,
    visual_seconds: f64,
) {
    let tile_size = map.tile_size;
    let columns = (rect.width() / tile_size).ceil() as i32 + 2;
    let rows = (rect.height() / tile_size).ceil() as i32 + 2;
    let center = TileCoord::new(map.center.x.floor() as i32, map.center.y.floor() as i32);
    let origin = TileCoord::new(center.x - columns / 2, center.y - rows / 2);
    let position = |coord: TileCoord| map.screen_position(rect, coord);
    let generator = world.generator();
    for y in 0..rows {
        for x in 0..columns {
            let coord = TileCoord::new(origin.x + x, origin.y + y);
            let (terrain, biome) = generator.terrain_at(coord);
            let cell = map.tile_rect(rect, coord);
            painter.rect_filled(cell, 0.0, terrain_color(terrain));
            if let Some(sprites) = sprites {
                draw_ground_sprite(painter, cell, terrain, biome, coord, world, sprites);
            }
        }
    }
    if show_borders {
        draw_nation_borders(painter, rect, world, map);
    }
    draw_persistent_paths(painter, &position, world, tile_size);
    if let Some(sprites) = sprites {
        draw_nation_walls(painter, rect, world, map, sprites);
    }
    for route in &world.trade_routes {
        if let Some(sprites) = sprites {
            draw_route_units(
                painter,
                &position,
                world,
                route,
                tile_size,
                sprites,
                visual_seconds,
            );
        } else {
            draw_route(painter, &position, world, route);
        }
    }
    for conflict in &world.conflicts {
        draw_conflict(painter, &position, world, conflict);
    }
    if let Some(sprites) = sprites {
        draw_buildings(painter, rect, &position, world, tile_size, sprites);
        draw_wildlife_and_workers(
            painter,
            rect,
            &position,
            world,
            tile_size,
            sprites,
            visual_seconds,
        );
    }
    for nation in &world.nations {
        for settlement in &nation.settlements {
            let point = position(settlement.location);
            if sprites.is_none() {
                painter.circle_filled(
                    point,
                    if settlement.id == nation.capital {
                        4.5
                    } else {
                        3.0
                    },
                    Color32::from_rgb(236, 224, 186),
                );
            }
            if selected_nation == Some(nation.id) && settlement.id == nation.capital {
                painter.circle_stroke(
                    point,
                    7.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(245, 214, 122)),
                );
            }
            if settlement.id == nation.capital {
                draw_profession_units(
                    painter,
                    point,
                    world,
                    nation,
                    tile_size,
                    selected_nation == Some(nation.id),
                    sprites,
                );
            }
            draw_world_label(
                painter,
                point + Vec2::new(6.0, -5.0),
                egui::Align2::LEFT_BOTTOM,
                &settlement.name,
                FontId::monospace(12.0),
            );
        }
    }
}

fn draw_nation_borders(painter: &egui::Painter, rect: Rect, world: &World, map: MapView) {
    for nation in &world.nations {
        let territory: HashSet<TileCoord> = nation.territory.iter().copied().collect();
        let color = nation_color(nation.id.get());
        let outer = Stroke::new(
            (map.tile_size * 0.19).max(3.0),
            Color32::from_rgb(29, 35, 25),
        );
        let inner = Stroke::new((map.tile_size * 0.10).max(1.5), color);
        for coord in &nation.territory {
            let cell = map.tile_rect(rect, *coord);
            for (neighbor, edge) in [
                (
                    TileCoord::new(coord.x, coord.y - 1),
                    [cell.left_top(), cell.right_top()],
                ),
                (
                    TileCoord::new(coord.x + 1, coord.y),
                    [cell.right_top(), cell.right_bottom()],
                ),
                (
                    TileCoord::new(coord.x, coord.y + 1),
                    [cell.left_bottom(), cell.right_bottom()],
                ),
                (
                    TileCoord::new(coord.x - 1, coord.y),
                    [cell.left_top(), cell.left_bottom()],
                ),
            ] {
                if !territory.contains(&neighbor) {
                    painter.line_segment(edge, outer);
                    painter.line_segment(edge, inner);
                }
            }
        }
    }
}

fn draw_nation_walls(
    painter: &egui::Painter,
    rect: Rect,
    world: &World,
    map: MapView,
    sprites: &SpriteAtlas,
) {
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
        for x in min_x..=max_x {
            for (y, frame) in [
                (min_y, if x == min_x || x == max_x { 1 } else { 3 }),
                (max_y, if x == min_x || x == max_x { 4 } else { 3 }),
            ] {
                let wall_rect = map.tile_rect(rect, TileCoord::new(x, y));
                if world.is_wall_tile(TileCoord::new(x, y)) && wall_rect.intersects(rect) {
                    draw_sprite(painter, &sprites.walls, wall_rect, frame);
                }
            }
        }
        for y in (min_y + 1)..max_y {
            for x in [min_x, max_x] {
                let wall_rect = map.tile_rect(rect, TileCoord::new(x, y));
                if world.is_wall_tile(TileCoord::new(x, y)) && wall_rect.intersects(rect) {
                    draw_sprite(painter, &sprites.walls, wall_rect, 0);
                }
            }
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn draw_profession_units(
    painter: &egui::Painter,
    capital: Pos2,
    world: &World,
    nation: &Nation,
    tile_size: f32,
    selected: bool,
    sprites: Option<&SpriteAtlas>,
) {
    if tile_size < 8.0 {
        return;
    }
    let Some(capital_coord) = nation
        .settlements
        .iter()
        .find(|settlement| settlement.id == nation.capital)
        .map(|settlement| settlement.location)
    else {
        return;
    };
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
        return;
    };
    let mut positions: Vec<TileCoord> = nation
        .territory
        .iter()
        .copied()
        .filter(|coord| {
            coord.x > min_x
                && coord.x < max_x
                && coord.y > min_y
                && coord.y < max_y
                && !world
                    .buildings
                    .iter()
                    .any(|building| building.location == *coord)
        })
        .collect();
    positions.sort_by_key(|coord| sprite_variation(*coord));
    if positions.is_empty() {
        return;
    }
    let radius = tile_size * 0.25;
    for (index, profession) in professional_units(nation).iter().enumerate() {
        let coord = positions[index % positions.len()];
        let point = capital
            + Vec2::new(
                (coord.x - capital_coord.x) as f32 * tile_size,
                (coord.y - capital_coord.y) as f32 * tile_size,
            );
        if let Some(sprites) = sprites {
            let texture = if matches!(profession, Profession::Soldier) {
                &sprites.soldier
            } else {
                &sprites.worker
            };
            let sprite_rect = Rect::from_center_size(point, Vec2::splat(tile_size * 0.8));
            draw_sprite(painter, texture, sprite_rect, sprite_variation(coord) % 5);
        } else {
            painter.circle_filled(point, radius, profession.color());
            painter.text(
                point,
                egui::Align2::CENTER_CENTER,
                profession.marker(),
                FontId::monospace(radius * 1.6),
                Color32::from_rgb(27, 28, 23),
            );
        }
        if selected && tile_size >= 16.0 {
            draw_world_label(
                painter,
                point + Vec2::new(radius + 3.0, 0.0),
                egui::Align2::LEFT_CENTER,
                profession.name(),
                FontId::monospace(10.0),
            );
        }
    }
}

fn draw_world_label(
    painter: &egui::Painter,
    position: Pos2,
    align: egui::Align2,
    text: &str,
    font: FontId,
) {
    for offset in [
        Vec2::new(-1.0, -1.0),
        Vec2::new(1.0, -1.0),
        Vec2::new(-1.0, 1.0),
        Vec2::new(1.0, 1.0),
    ] {
        painter.text(
            position + offset,
            align,
            text,
            font.clone(),
            Color32::from_rgb(29, 35, 25),
        );
    }
    painter.text(
        position,
        align,
        text,
        font,
        Color32::from_rgb(255, 248, 218),
    );
}

fn draw_ground_sprite(
    painter: &egui::Painter,
    cell: Rect,
    terrain: TerrainType,
    biome: Biome,
    coord: TileCoord,
    world: &World,
    sprites: &SpriteAtlas,
) {
    let (texture, frame) = match terrain {
        TerrainType::Water | TerrainType::Ocean | TerrainType::Lake | TerrainType::River => {
            (&sprites.grass, 0)
        }
        TerrainType::Beach | TerrainType::Desert => (&sprites.shore, 0),
        TerrainType::Prairie => (&sprites.grass, 1),
        _ => (&sprites.grass, 2),
    };
    draw_tile_sprite(painter, texture, cell, frame);

    let variation = sprite_variation(coord);
    let decoration = match terrain {
        _ if world.is_tree_stump_at(coord) && matches!(biome, Biome::Boreal | Biome::Tundra) => {
            Some((&sprites.pine_trees, 0))
        }
        _ if world.is_tree_stump_at(coord) && matches!(biome, Biome::Tropical) => {
            Some((&sprites.coconut_trees, 0))
        }
        _ if world.is_tree_stump_at(coord) => Some((&sprites.trees, 0)),
        _ if world.has_tree_at(coord) && matches!(biome, Biome::Boreal | Biome::Tundra) => {
            Some((&sprites.pine_trees, 1))
        }
        _ if world.has_tree_at(coord) && matches!(biome, Biome::Tropical) => {
            Some((&sprites.coconut_trees, 2 + variation % 4))
        }
        _ if world.has_tree_at(coord) => Some((&sprites.trees, 1 + variation % 3)),
        TerrainType::Desert if variation % 100 < 8 => Some((&sprites.cactus, variation % 8)),
        TerrainType::Beach if matches!(biome, Biome::Tropical) && variation % 100 < 18 => {
            Some((&sprites.coconut_trees, 2 + variation % 4))
        }
        _ if world.has_rock_at(coord) => Some((&sprites.rocks, variation % 9)),
        _ => None,
    };
    if let Some((texture, frame)) = decoration {
        draw_sprite(painter, texture, cell, frame);
    }
}

fn draw_tile_sprite(painter: &egui::Painter, texture: &TextureHandle, rect: Rect, frame: usize) {
    draw_sprite(
        painter,
        texture,
        Rect::from_min_size(rect.min, rect.size() + Vec2::splat(1.0)),
        frame,
    );
}

#[allow(clippy::cast_precision_loss)]
fn draw_keep_sprite(painter: &egui::Painter, texture: &TextureHandle, rect: Rect, frame: usize) {
    let [width, height] = texture.size();
    let columns = (width / 32).max(1);
    let rows = (height / 32).max(1);
    let frame = frame % columns.saturating_mul(rows);
    let column = frame % columns;
    let row = frame / columns;
    let uv = Rect::from_min_max(
        Pos2::new(column as f32 / columns as f32, row as f32 / rows as f32),
        Pos2::new(
            (column + 1) as f32 / columns as f32,
            (row + 1) as f32 / rows as f32,
        ),
    );
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}

#[allow(clippy::cast_precision_loss)]
fn draw_sprite(painter: &egui::Painter, texture: &TextureHandle, rect: Rect, frame: usize) {
    let [width, height] = texture.size();
    let columns = (width / 16).max(1);
    let rows = (height / 16).max(1);
    let frame = frame % columns.saturating_mul(rows);
    let column = frame % columns;
    let row = frame / columns;
    let uv = Rect::from_min_max(
        Pos2::new(column as f32 / columns as f32, row as f32 / rows as f32),
        Pos2::new(
            (column + 1) as f32 / columns as f32,
            (row + 1) as f32 / rows as f32,
        ),
    );
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}

fn sprite_variation(coord: TileCoord) -> usize {
    let mut value = i64::from(coord.x)
        .cast_unsigned()
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ i64::from(coord.y)
            .cast_unsigned()
            .wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    usize::try_from(value ^ (value >> 31)).unwrap_or_default()
}

fn draw_persistent_paths(
    painter: &egui::Painter,
    position: &impl Fn(TileCoord) -> Pos2,
    world: &World,
    tile_size: f32,
) {
    for path in &world.paths {
        let mut points = Vec::with_capacity(path.tiles.len() + 2);
        points.push(position(path.from));
        points.extend(path.tiles.iter().copied().map(position));
        points.push(position(path.to));
        let (outer_color, inner_color, outer_width, inner_width) = if path.trade_route.is_some() {
            (
                Color32::from_rgb(71, 54, 35),
                Color32::from_rgb(204, 165, 91),
                0.58,
                0.28,
            )
        } else {
            (
                Color32::from_rgb(96, 76, 48),
                Color32::from_rgb(157, 127, 76),
                0.42,
                0.20,
            )
        };
        painter.add(egui::Shape::line(
            points.clone(),
            Stroke::new((tile_size * outer_width).max(2.0), outer_color),
        ));
        painter.add(egui::Shape::line(
            points,
            Stroke::new((tile_size * inner_width).max(1.0), inner_color),
        ));
    }
}

fn draw_buildings(
    painter: &egui::Painter,
    rect: Rect,
    position: &impl Fn(TileCoord) -> Pos2,
    world: &World,
    tile_size: f32,
    sprites: &SpriteAtlas,
) {
    for building in &world.buildings {
        let point = position(building.location);
        let size = if building.kind == BuildingKind::Keep {
            tile_size * 2.0
        } else {
            tile_size
        };
        let center = if building.kind == BuildingKind::Keep {
            point + Vec2::new(tile_size * 0.5, -tile_size * 0.5)
        } else {
            point
        };
        let sprite_rect = Rect::from_center_size(center, Vec2::splat(size));
        if !sprite_rect.intersects(rect) {
            continue;
        }
        let palette = &sprites.palettes
            [usize::try_from(building.nation.get()).unwrap_or_default() % sprites.palettes.len()];
        match building.kind {
            BuildingKind::Keep => draw_keep_sprite(
                painter,
                &palette.keep,
                sprite_rect,
                usize::try_from(building.nation.get()).unwrap_or_default(),
            ),
            BuildingKind::House => {
                let texture = if world
                    .nation(building.nation)
                    .is_some_and(|nation| nation.population < 2_400)
                {
                    &palette.huts
                } else {
                    &palette.houses
                };
                draw_sprite(
                    painter,
                    texture,
                    sprite_rect,
                    sprite_variation(building.location),
                );
            }
            BuildingKind::Farm => draw_sprite(
                painter,
                &sprites.farm,
                sprite_rect,
                sprite_variation(building.location) % 5,
            ),
            BuildingKind::WheatField => draw_sprite(painter, &sprites.wheat, sprite_rect, 3),
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn draw_wildlife_and_workers(
    painter: &egui::Painter,
    rect: Rect,
    position: &impl Fn(TileCoord) -> Pos2,
    world: &World,
    tile_size: f32,
    sprites: &SpriteAtlas,
    visual_seconds: f64,
) {
    let size = Vec2::splat(tile_size * 0.75);
    for animal in &world.wildlife {
        let point = position(animal.location);
        let sprite_rect = Rect::from_center_size(point, size);
        if sprite_rect.intersects(rect) {
            if animal.kind == WildAnimalKind::Fish {
                draw_sprite(
                    painter,
                    &sprites.fish,
                    sprite_rect,
                    fish_sprite_frame(animal.id.get(), animal.heading, visual_seconds),
                );
            } else {
                let walk_frame = ((visual_seconds * 8.0) as usize)
                    .saturating_add(usize::try_from(animal.id.get()).unwrap_or_default())
                    % 4;
                let direction_row = [2, 0, 3, 1][usize::from(animal.heading) % 4];
                draw_sprite(
                    painter,
                    &sprites.boar,
                    sprite_rect,
                    direction_row * 4 + walk_frame,
                );
            }
        }
    }
    for worker in &world.workers {
        let target = match worker.target {
            Some(
                threadnations_simulation::WorkerTarget::Tree(coord)
                | threadnations_simulation::WorkerTarget::Rock(coord),
            ) => Some(coord),
            Some(threadnations_simulation::WorkerTarget::Animal(id)) => world
                .wildlife
                .iter()
                .find(|animal| animal.id == id)
                .map(|animal| animal.location),
            None => None,
        };
        let direction = target.map_or_else(
            || {
                [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y]
                    [usize::try_from(worker.id.get()).unwrap_or_default() % 4]
            },
            |target| {
                let delta = Vec2::new(
                    (target.x - worker.location.x) as f32,
                    (target.y - worker.location.y) as f32,
                );
                if delta.x.abs() >= delta.y.abs() {
                    Vec2::new(delta.x.signum(), 0.0)
                } else {
                    Vec2::new(0.0, delta.y.signum())
                }
            },
        );
        let stride =
            ((visual_seconds as f32 * 3.0 + worker.id.get() as f32).sin() + 1.0) * tile_size * 0.12;
        let point = position(worker.location) + direction * stride;
        let sprite_rect = Rect::from_center_size(point, size);
        if sprite_rect.intersects(rect) {
            let texture = if worker.role == WorkerRole::Hunter {
                &sprites.soldier
            } else {
                &sprites.worker
            };
            let color = match worker.role {
                WorkerRole::Lumberjack => Color32::from_rgb(126, 180, 92),
                WorkerRole::Stonemason => Color32::from_rgb(151, 177, 204),
                WorkerRole::Hunter => Color32::from_rgb(218, 166, 92),
            };
            draw_sprite(
                painter,
                texture,
                sprite_rect,
                ((visual_seconds * 8.0) as usize
                    + usize::try_from(worker.id.get()).unwrap_or_default())
                    % 5,
            );
            painter.circle_stroke(point, tile_size * 0.33, Stroke::new(1.0_f32, color));
            if tile_size >= 24.0 {
                let label = match worker.role {
                    WorkerRole::Lumberjack => "Lumberjack",
                    WorkerRole::Stonemason => "Stonemason",
                    WorkerRole::Hunter => "Hunter",
                };
                painter.text(
                    point + Vec2::new(0.0, tile_size * 0.48),
                    egui::Align2::CENTER_TOP,
                    label,
                    FontId::monospace(9.0),
                    color,
                );
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn fish_sprite_frame(id: u64, heading: u8, visual_seconds: f64) -> usize {
    let heading = usize::from(heading) % 4;
    let id = usize::try_from(id).unwrap_or_default();
    if id % 4 != 0 {
        return [2, 3, 1, 0][heading] * 5 + 2 + id % 3;
    }
    let frame = (visual_seconds * 4.0) as usize % 2;
    match heading {
        0 => (1 + frame) * 5 + 1,
        1 => 3 * 5 + frame,
        2 => (1 + frame) * 5,
        _ => frame,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn draw_route_units(
    painter: &egui::Painter,
    position: &impl Fn(TileCoord) -> Pos2,
    world: &World,
    route: &TradeRoute,
    tile_size: f32,
    sprites: &SpriteAtlas,
    visual_seconds: f64,
) {
    let Some(path) = world
        .paths
        .iter()
        .find(|path| path.trade_route == Some(route.id))
    else {
        return;
    };
    let points: Vec<Pos2> = std::iter::once(path.from)
        .chain(path.tiles.iter().copied())
        .chain(std::iter::once(path.to))
        .map(position)
        .collect();
    let lengths: Vec<f32> = points
        .windows(2)
        .map(|segment| segment[0].distance(segment[1]))
        .collect();
    let total_length: f32 = lengths.iter().sum();
    if total_length <= f32::EPSILON {
        return;
    }
    for index in 0..2 {
        let progress = (visual_seconds as f32 * 0.055 + index as f32 * 0.5).fract();
        let mut remaining = progress * total_length;
        let mut point = points[0];
        for (segment, length) in points.windows(2).zip(&lengths) {
            if remaining <= *length {
                point = segment[0].lerp(segment[1], remaining / length.max(f32::EPSILON));
                break;
            }
            remaining -= length;
        }
        let size = tile_size * 0.72;
        draw_sprite(
            painter,
            &sprites.worker,
            Rect::from_center_size(point, Vec2::splat(size)),
            ((visual_seconds * 8.0) as usize + index) % 5,
        );
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
fn draw_minimap(
    painter: &egui::Painter,
    map_rect: Rect,
    world: &World,
    map: MapView,
    selected_nation: Option<NationId>,
) {
    const WIDTH: usize = 48;
    const HEIGHT: usize = 32;
    const WORLD_WIDTH: i32 = 144;
    const WORLD_HEIGHT: i32 = 96;
    let minimap = Rect::from_min_size(
        Pos2::new(map_rect.left() + 14.0, map_rect.bottom() - 118.0),
        Vec2::new(180.0, 104.0),
    );
    painter.rect_filled(
        minimap.expand(3.0),
        3.0,
        Color32::from_rgba_premultiplied(15, 18, 15, 220),
    );

    let half_width = (map_rect.width() / map.tile_size / 2.0).ceil() as i32;
    let half_height = (map_rect.height() / map.tile_size / 2.0).ceil() as i32;
    let center_x = map.center.x.floor() as i32;
    let center_y = map.center.y.floor() as i32;
    let min_x = center_x - WORLD_WIDTH / 2;
    let min_y = center_y - WORLD_HEIGHT / 2;
    let span_x = WORLD_WIDTH;
    let span_y = WORLD_HEIGHT;
    let step_x = WORLD_WIDTH / WIDTH as i32;
    let step_y = WORLD_HEIGHT / HEIGHT as i32;
    let cell_size = Vec2::new(
        minimap.width() / WIDTH as f32,
        minimap.height() / HEIGHT as f32,
    );
    let generator = world.generator();
    let owners: HashMap<TileCoord, NationId> = world
        .nations
        .iter()
        .flat_map(|nation| {
            nation
                .territory
                .iter()
                .copied()
                .map(move |coord| (coord, nation.id))
        })
        .collect();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let coord = TileCoord::new(min_x + x as i32 * step_x, min_y + y as i32 * step_y);
            let mut color = terrain_color(generator.terrain_at(coord).0);
            if let Some(owner) = owners.get(&coord) {
                color = nation_color(owner.get());
            }
            let cell = Rect::from_min_size(
                minimap.min + Vec2::new(x as f32 * cell_size.x, y as f32 * cell_size.y),
                cell_size + Vec2::splat(0.2),
            );
            painter.rect_filled(cell, 0.0, color);
        }
    }
    let world_to_minimap = |x: f32, y: f32| {
        Pos2::new(
            minimap.left() + (x - min_x as f32) / span_x as f32 * minimap.width(),
            minimap.top() + (y - min_y as f32) / span_y as f32 * minimap.height(),
        )
    };
    let path_painter = painter.with_clip_rect(minimap);
    for path in &world.paths {
        let points: Vec<Pos2> = std::iter::once(path.from)
            .chain(path.tiles.iter().copied())
            .chain(std::iter::once(path.to))
            .map(|coord| world_to_minimap(coord.x as f32, coord.y as f32))
            .collect();
        if points.len() >= 2 {
            let color = if path.trade_route.is_some() {
                Color32::from_rgb(245, 205, 108)
            } else {
                Color32::from_rgb(148, 114, 67)
            };
            path_painter.add(egui::Shape::line(points, Stroke::new(1.2_f32, color)));
        }
    }
    for nation in &world.nations {
        let Some(capital) = nation
            .settlements
            .iter()
            .find(|settlement| settlement.id == nation.capital)
        else {
            continue;
        };
        let radius = if selected_nation == Some(nation.id) {
            5.0
        } else {
            3.5
        };
        let point = world_to_minimap(capital.location.x as f32, capital.location.y as f32);
        let point = Pos2::new(
            point
                .x
                .clamp(minimap.left() + radius, minimap.right() - radius),
            point
                .y
                .clamp(minimap.top() + radius, minimap.bottom() - radius),
        );
        path_painter.circle_filled(point, radius, nation_color(nation.id.get()));
        path_painter.circle_stroke(
            point,
            radius,
            Stroke::new(1.0_f32, Color32::from_rgb(255, 248, 218)),
        );
    }
    let camera = Rect::from_min_max(
        world_to_minimap(
            map.center.x - half_width as f32,
            map.center.y - half_height as f32,
        ),
        world_to_minimap(
            map.center.x + half_width as f32,
            map.center.y + half_height as f32,
        ),
    );
    painter.rect_stroke(
        camera,
        0.0,
        Stroke::new(1.0_f32, Color32::from_rgb(245, 229, 160)),
    );
    painter.text(
        minimap.left_top() + Vec2::new(4.0, 3.0),
        egui::Align2::LEFT_TOP,
        "MAP",
        FontId::monospace(9.0),
        Color32::from_rgb(235, 229, 211),
    );
}

fn codex_sessions_root() -> PathBuf {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
        .join("sessions")
}

fn draw_route(
    painter: &egui::Painter,
    position: &impl Fn(TileCoord) -> Pos2,
    world: &World,
    route: &TradeRoute,
) {
    let Some(from) = world
        .nation(route.from)
        .and_then(|nation| nation.settlements.first())
    else {
        return;
    };
    let Some(to) = world
        .nation(route.to)
        .and_then(|nation| nation.settlements.first())
    else {
        return;
    };
    painter.line_segment(
        [position(from.location), position(to.location)],
        Stroke::new(1.2_f32, Color32::from_rgb(194, 163, 94)),
    );
}

fn draw_conflict(
    painter: &egui::Painter,
    position: &impl Fn(TileCoord) -> Pos2,
    world: &World,
    conflict: &Conflict,
) {
    let Some(a) = world
        .nation(conflict.attacker)
        .and_then(|nation| nation.settlements.first())
    else {
        return;
    };
    let Some(b) = world
        .nation(conflict.defender)
        .and_then(|nation| nation.settlements.first())
    else {
        return;
    };
    let middle = position(a.location).lerp(position(b.location), 0.5);
    painter.line_segment(
        [middle - Vec2::splat(5.0), middle + Vec2::splat(5.0)],
        Stroke::new(2.0_f32, Color32::from_rgb(187, 83, 68)),
    );
    painter.line_segment(
        [middle + Vec2::new(-5.0, 5.0), middle + Vec2::new(5.0, -5.0)],
        Stroke::new(2.0_f32, Color32::from_rgb(187, 83, 68)),
    );
}

fn terrain_color(terrain: TerrainType) -> Color32 {
    match terrain {
        TerrainType::Water => Color32::from_rgb(44, 78, 75),
        TerrainType::Ocean | TerrainType::Lake | TerrainType::River => {
            Color32::from_rgb(77, 186, 184)
        }
        TerrainType::Beach => Color32::from_rgb(234, 213, 157),
        TerrainType::Plains => Color32::from_rgb(83, 101, 57),
        TerrainType::Grassland => Color32::from_rgb(106, 157, 70),
        TerrainType::Prairie => Color32::from_rgb(185, 213, 87),
        TerrainType::Forest => Color32::from_rgb(46, 76, 49),
        TerrainType::Hills => Color32::from_rgb(103, 91, 65),
        TerrainType::Mountains => Color32::from_rgb(91, 85, 78),
        TerrainType::Desert => Color32::from_rgb(144, 124, 81),
        TerrainType::Wetlands => Color32::from_rgb(71, 98, 76),
    }
}

fn nation_color(id: u64) -> Color32 {
    const COLORS: [Color32; 6] = [
        Color32::from_rgb(198, 111, 70),
        Color32::from_rgb(177, 151, 65),
        Color32::from_rgb(133, 157, 82),
        Color32::from_rgb(150, 105, 133),
        Color32::from_rgb(190, 128, 84),
        Color32::from_rgb(112, 151, 137),
    ];
    let index = usize::try_from(id % u64::try_from(COLORS.len()).unwrap_or(1)).unwrap_or(0);
    COLORS[index]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().map_err(std::io::Error::other)?;
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/Icon.png"))?;
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([config.window_width, config.window_height])
        .with_min_inner_size([720.0, 480.0])
        .with_resizable(true)
        .with_decorations(!config.borderless)
        .with_icon(icon);
    if config.always_on_top {
        viewport = viewport.with_always_on_top();
    }
    eframe::run_native(
        "ThreadNations",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(ThreadNationsApp::new(config)))),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_hovered_world_position_fixed() {
        let mut map = MapView {
            center: Vec2::new(12.0, -8.0),
            home: Vec2::new(12.0, -8.0),
            tile_size: 11.0,
        };
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
        let pointer = Pos2::new(610.0, 225.0);
        let before = map.center + (pointer - rect.center()) / map.tile_size;
        map.zoom_at(rect, pointer, 240.0);
        let after = map.center + (pointer - rect.center()) / map.tile_size;
        assert!((before - after).length() < f32::EPSILON);
    }

    #[test]
    fn population_units_never_exceed_population() {
        let mut world = World::new_demo(1, 1);
        let nation = world.nations.remove(0);
        let units = population_units(&nation);
        assert_eq!(
            units.civilians + units.workforce + units.educated + units.military,
            nation.population
        );
        assert!(units.workforce + units.educated <= nation.population);
    }

    #[test]
    fn professional_markers_are_limited_to_notable_roles() {
        let mut world = World::new_demo(1, 1);
        let nation = world.nations.remove(0);
        let professions = professional_units(&nation);
        assert!(professions.contains(&Profession::Farmer));
        assert!(professions.contains(&Profession::Soldier));
        assert!(professions.len() <= 6);
    }

    #[test]
    fn fish_frames_keep_colored_fish_static_and_turtles_directional() {
        assert_eq!(fish_sprite_frame(1, 0, 0.0), fish_sprite_frame(1, 0, 99.0));
        assert_eq!(fish_sprite_frame(4, 0, 0.0), 6);
        assert_eq!(fish_sprite_frame(4, 0, 0.3), 11);
        assert_eq!(fish_sprite_frame(4, 1, 0.0), 15);
        assert_eq!(fish_sprite_frame(4, 2, 0.3), 10);
        assert_eq!(fish_sprite_frame(4, 3, 0.3), 1);
    }
}
