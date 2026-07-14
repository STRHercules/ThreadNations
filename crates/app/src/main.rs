//! Inspectable desktop map for `ThreadNations`. Navigation never changes simulation state.

use std::{
    env,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use threadnations_analysis::{
    activity_points, import_codex_session_metadata, read_jsonl, ActivityRecord, ActivitySourceKind,
};
use threadnations_common::{NationId, TileCoord};
use threadnations_persistence::{load, save, HistoricalImportState, SavedWorld, WindowState};
use threadnations_simulation::{Conflict, Nation, Settlement, TradeRoute, World};
use threadnations_worldgen::TerrainType;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Config {
    world_seed: u64,
    calendar_step_ms: u64,
    autosave_seconds: u64,
    initial_nations: u8,
    synthetic_activity: bool,
    synthetic_interval_ticks: u64,
    jsonl_inbox: String,
    window_width: f32,
    window_height: f32,
    borderless: bool,
    always_on_top: bool,
    max_catchup_ticks: u32,
    history_retention_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            world_seed: 0x5448_5245_4144,
            calendar_step_ms: 60_000,
            autosave_seconds: 30,
            initial_nations: 4,
            synthetic_activity: true,
            synthetic_interval_ticks: 4,
            jsonl_inbox: "data/activity-inbox.jsonl".into(),
            window_width: 1_240.0,
            window_height: 760.0,
            borderless: false,
            always_on_top: false,
            max_catchup_ticks: 8,
            history_retention_limit: 240,
        }
    }
}

impl Config {
    fn load() -> Result<Self, String> {
        let path = Path::new("data/config.toml");
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                std::fs::create_dir_all("data").map_err(|error| error.to_string())?;
                std::fs::write(
                    path,
                    toml::to_string_pretty(&config).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                Ok(config)
            }
            Err(error) => Err(error.to_string()),
        }
    }
}

struct ThreadNationsApp {
    config: Config,
    world: World,
    database: PathBuf,
    activity_offset: u64,
    historical_import: HistoricalImportState,
    selected_nation: Option<NationId>,
    map: MapView,
    import_error: Option<String>,
    last_tick: Instant,
    last_ingest: Instant,
    last_save: Instant,
}

#[derive(Clone, Copy, Debug)]
struct MapView {
    center: Vec2,
    tile_size: f32,
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
impl MapView {
    const MIN_TILE_SIZE: f32 = 3.0;
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
            tile_size: 11.0,
        }
    }

    fn screen_position(self, rect: Rect, coord: TileCoord) -> Pos2 {
        rect.center() + (Vec2::new(coord.x as f32, coord.y as f32) - self.center) * self.tile_size
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
        self.tile_size = (self.tile_size * (scroll / 240.0).exp())
            .clamp(Self::MIN_TILE_SIZE, Self::MAX_TILE_SIZE);
        self.center = before - (pointer - rect.center()) / self.tile_size;
    }
}

impl ThreadNationsApp {
    fn new(config: Config) -> Self {
        let database = PathBuf::from("data/threadnations.sqlite");
        let (world, activity_offset, historical_import) =
            load(&database).ok().flatten().map_or_else(
                || {
                    (
                        World::new_demo(config.world_seed, config.initial_nations),
                        0,
                        HistoricalImportState::default(),
                    )
                },
                |saved| (saved.world, saved.activity_offset, saved.historical_import),
            );
        let map = MapView::for_world(&world);
        Self {
            config,
            world,
            database,
            activity_offset,
            historical_import,
            selected_nation: None,
            map,
            import_error: None,
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

    fn persist(&mut self) {
        let state = SavedWorld {
            world: self.world.clone(),
            activity_offset: self.activity_offset,
            window: WindowState {
                width: self.config.window_width,
                height: self.config.window_height,
                x: None,
                y: None,
            },
            historical_import: self.historical_import.clone(),
        };
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

    fn import_codex_history(&mut self) {
        let root = codex_sessions_root();
        match import_codex_session_metadata(&root) {
            Ok(records) => {
                for record in &records {
                    self.world.advance(activity_points(record));
                }
                self.historical_import = HistoricalImportState {
                    answered: true,
                    imported_records: u64::try_from(records.len()).unwrap_or(u64::MAX),
                };
                self.import_error = None;
                self.persist();
            }
            Err(error) => self.import_error = Some(error.to_string()),
        }
    }
}

impl eframe::App for ThreadNationsApp {
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.advance();
        if self.last_save.elapsed() >= Duration::from_secs(self.config.autosave_seconds.max(5)) {
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
                }
                draw_world(
                    ui.painter(),
                    rect,
                    &self.world,
                    self.map,
                    self.selected_nation,
                );
            });
        self.show_historical_import_prompt(ctx);
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.persist();
    }
}

impl ThreadNationsApp {
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
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Import local history").clicked() {
                        self.import_codex_history();
                    }
                    if ui.button("Start without import").clicked() {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PopulationUnits {
    civilians: u64,
    workforce: u64,
    educated: u64,
    military: u64,
}

fn population_units(nation: &Nation) -> PopulationUnits {
    let military = u64::from(nation.military).min(nation.population);
    let civilians = nation.population.saturating_sub(military);
    let workforce = civilians.saturating_mul(3) / 5;
    let educated = u64::from(nation.research).saturating_mul(20).min(workforce);
    PopulationUnits {
        civilians,
        workforce,
        educated,
        military,
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

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn draw_world(
    painter: &egui::Painter,
    rect: Rect,
    world: &World,
    map: MapView,
    selected_nation: Option<NationId>,
) {
    let tile_size = map.tile_size;
    let columns = (rect.width() / tile_size) as i32;
    let rows = (rect.height() / tile_size) as i32;
    let center = TileCoord::new(map.center.x.floor() as i32, map.center.y.floor() as i32);
    let origin = TileCoord::new(center.x - columns / 2 - 1, center.y - rows / 2 - 1);
    let position = |coord: TileCoord| map.screen_position(rect, coord);
    for y in 0..rows {
        for x in 0..columns {
            let coord = TileCoord::new(origin.x + x, origin.y + y);
            let tile = world.tile(coord);
            let center = position(coord);
            let cell = Rect::from_min_size(
                center - Vec2::splat(tile_size / 2.0),
                Vec2::splat(tile_size + 0.5),
            );
            painter.rect_filled(cell, 0.0, terrain_color(tile.terrain));
            if let Some(owner) = world.owner_at(coord) {
                painter.rect_filled(
                    cell.shrink(1.0),
                    0.0,
                    nation_color(owner.get()).gamma_multiply(0.38),
                );
            }
            if let Some(owner) = world.owner_at(coord) {
                if coord
                    .cardinal_neighbors()
                    .iter()
                    .any(|neighbor| world.owner_at(*neighbor) != Some(owner))
                {
                    painter.rect_stroke(
                        cell.shrink(0.5),
                        0.0,
                        Stroke::new(
                            if selected_nation == Some(owner) {
                                2.0_f32
                            } else {
                                1.0_f32
                            },
                            nation_color(owner.get()),
                        ),
                    );
                }
            }
        }
    }
    for route in &world.trade_routes {
        draw_route(painter, &position, world, route);
    }
    for conflict in &world.conflicts {
        draw_conflict(painter, &position, world, conflict);
    }
    for nation in &world.nations {
        for settlement in &nation.settlements {
            let point = position(settlement.location);
            painter.circle_filled(
                point,
                if settlement.id == nation.capital {
                    4.5
                } else {
                    3.0
                },
                Color32::from_rgb(236, 224, 186),
            );
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
                    nation,
                    tile_size,
                    selected_nation == Some(nation.id),
                );
            }
            painter.text(
                point + Vec2::new(6.0, -5.0),
                egui::Align2::LEFT_BOTTOM,
                &settlement.name,
                FontId::monospace(11.0),
                Color32::from_rgb(224, 218, 201),
            );
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn draw_profession_units(
    painter: &egui::Painter,
    capital: Pos2,
    nation: &Nation,
    tile_size: f32,
    selected: bool,
) {
    if tile_size < 8.0 {
        return;
    }
    let spacing = tile_size.clamp(10.0, 18.0);
    let radius = (tile_size * 0.32).clamp(3.0, 5.0);
    for (index, profession) in professional_units(nation).iter().enumerate() {
        let column = index % 3;
        let row = index / 3;
        let point =
            capital + Vec2::new((column as f32 - 1.0) * spacing, 10.0 + row as f32 * spacing);
        painter.circle_filled(point, radius, profession.color());
        painter.text(
            point,
            egui::Align2::CENTER_CENTER,
            profession.marker(),
            FontId::monospace(radius * 1.6),
            Color32::from_rgb(27, 28, 23),
        );
        if selected && tile_size >= 16.0 {
            painter.text(
                point + Vec2::new(radius + 3.0, 0.0),
                egui::Align2::LEFT_CENTER,
                profession.name(),
                FontId::monospace(10.0),
                Color32::from_rgb(224, 218, 201),
            );
        }
    }
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
        TerrainType::Plains => Color32::from_rgb(83, 101, 57),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_hovered_world_position_fixed() {
        let mut map = MapView {
            center: Vec2::new(12.0, -8.0),
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
        assert_eq!(units.civilians + units.military, nation.population);
        assert!(units.workforce <= units.civilians);
        assert!(units.educated <= units.workforce);
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().map_err(std::io::Error::other)?;
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([config.window_width, config.window_height])
        .with_min_inner_size([720.0, 480.0])
        .with_resizable(true)
        .with_decorations(!config.borderless);
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
