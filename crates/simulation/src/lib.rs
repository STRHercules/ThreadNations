//! Deterministic, input-free civilization simulation.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
};

use serde::{Deserialize, Serialize};
use threadnations_common::{
    BuildingId, ChunkCoord, ConflictId, HistoryEventId, IdAllocator, NationId, PathId,
    SettlementId, TileCoord, TradeRouteId, WildlifeId, WorkerId, WorldTick,
};
use threadnations_worldgen::{ResourceKind, TerrainType, Tile, WorldGenerator};

const HISTORY_LIMIT: usize = 2_000;
const RESERVOIR_CAP: u32 = 20_000;
const MIN_NATION_SPAN: i32 = 7;
const TREE_REGROW_TICKS: u64 = 48;
const ROCK_RESPAWN_TICKS: u64 = 36;
const STARTING_REGIONS: [ChunkCoord; 6] = [
    ChunkCoord::new(0, 0),
    ChunkCoord::new(4, -4),
    ChunkCoord::new(-4, 4),
    ChunkCoord::new(8, 0),
    ChunkCoord::new(0, 8),
    ChunkCoord::new(-8, -8),
];

#[derive(Clone, Copy, Debug, Default)]
struct LocalConditions {
    fertility: u32,
    minerals: u32,
    forest: u32,
    water: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct BuildingCounts {
    houses: u32,
    farms: u32,
    workshops: u32,
    markets: u32,
    schools: u32,
    barracks: u32,
    clinics: u32,
    temples: u32,
    sources: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Government {
    Council,
    Republic,
    Monarchy,
    Directorate,
}

impl Government {
    #[must_use]
    pub const fn lord_title(self) -> &'static str {
        match self {
            Self::Council => "Speaker",
            Self::Republic => "President",
            Self::Monarchy => "King",
            Self::Directorate => "Director",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SettlementStage {
    Camp,
    Village,
    Town,
    City,
    Metropolis,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventKind {
    Founded,
    Growth,
    Famine,
    TradeOpened,
    TradeClosed,
    WarBegan,
    WarEnded,
    BorderExpanded,
    SettlementAdvanced,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TradeGood {
    Food,
    Materials,
}

/// Physical structures are persisted separately from settlements so the map is not a renderer-only
/// approximation of national size.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BuildingKind {
    Keep,
    House,
    Farm,
    WheatField,
    Workshop,
    Market,
    School,
    Barracks,
    Clinic,
    Temple,
    Mine,
    Lumberyard,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Building {
    pub id: BuildingId,
    pub nation: NationId,
    pub kind: BuildingKind,
    pub location: TileCoord,
    pub farm: Option<BuildingId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Path {
    pub id: PathId,
    pub nation: NationId,
    #[serde(default)]
    pub trade_route: Option<TradeRouteId>,
    #[serde(default)]
    pub from: TileCoord,
    #[serde(default)]
    pub to: TileCoord,
    pub tiles: Vec<TileCoord>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WorkerRole {
    #[serde(alias = "Gatherer")]
    Lumberjack,
    Stonemason,
    Hunter,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum JobRole {
    Farmer,
    Rancher,
    Fisher,
    Lumberjack,
    Stonemason,
    Hunter,
    Blacksmith,
    Carpenter,
    Mason,
    Weaver,
    Potter,
    Tanner,
    Herbalist,
    Merchant,
    Innkeeper,
    Builder,
    Teamster,
    Healer,
    Scribe,
    Priest,
    Guard,
    Soldier,
}

impl JobRole {
    const ALL: [Self; 22] = [
        Self::Farmer,
        Self::Rancher,
        Self::Fisher,
        Self::Lumberjack,
        Self::Stonemason,
        Self::Hunter,
        Self::Blacksmith,
        Self::Carpenter,
        Self::Mason,
        Self::Weaver,
        Self::Potter,
        Self::Tanner,
        Self::Herbalist,
        Self::Merchant,
        Self::Innkeeper,
        Self::Builder,
        Self::Teamster,
        Self::Healer,
        Self::Scribe,
        Self::Priest,
        Self::Guard,
        Self::Soldier,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Farmer => "Farmer",
            Self::Rancher => "Rancher",
            Self::Fisher => "Fisher",
            Self::Lumberjack => "Lumberjack",
            Self::Stonemason => "Stonemason",
            Self::Hunter => "Hunter",
            Self::Blacksmith => "Blacksmith",
            Self::Carpenter => "Carpenter",
            Self::Mason => "Mason",
            Self::Weaver => "Weaver",
            Self::Potter => "Potter",
            Self::Tanner => "Tanner",
            Self::Herbalist => "Herbalist",
            Self::Merchant => "Merchant",
            Self::Innkeeper => "Innkeeper",
            Self::Builder => "Builder",
            Self::Teamster => "Teamster",
            Self::Healer => "Healer",
            Self::Scribe => "Scribe",
            Self::Priest => "Priest",
            Self::Guard => "Guard",
            Self::Soldier => "Soldier",
        }
    }

    const fn weight(self) -> u64 {
        match self {
            Self::Farmer => 26,
            Self::Rancher => 6,
            Self::Fisher | Self::Lumberjack | Self::Stonemason => 4,
            Self::Hunter
            | Self::Blacksmith
            | Self::Carpenter
            | Self::Mason
            | Self::Weaver
            | Self::Merchant
            | Self::Guard
            | Self::Soldier => 3,
            Self::Potter
            | Self::Tanner
            | Self::Herbalist
            | Self::Innkeeper
            | Self::Builder
            | Self::Teamster
            | Self::Healer
            | Self::Scribe
            | Self::Priest => 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JobAssignment {
    pub role: JobRole,
    pub workers: u64,
}

/// Consumable goods produced by jobs and drained by households, industry, and public services.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceStores {
    pub wood: u32,
    pub stone: u32,
    pub ore: u32,
    pub tools: u32,
    pub goods: u32,
    pub livestock: u32,
    pub fish: u32,
    pub medicine: u32,
}

/// Per-nation ledger. Values are refreshed each economy tick, so the UI reports causes as
/// well as the current stockpiles.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Economy {
    pub housing: u64,
    pub treasury: u32,
    pub tax_rate: u8,
    pub education: u8,
    pub health: u8,
    pub crime: u8,
    pub food_produced: u32,
    pub food_consumed: u32,
    pub materials_produced: u32,
    pub materials_consumed: u32,
    pub taxes_collected: u32,
    pub resources: ResourceStores,
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            housing: 0,
            treasury: 0,
            tax_rate: 12,
            education: 10,
            health: 55,
            crime: 12,
            food_produced: 0,
            food_consumed: 0,
            materials_produced: 0,
            materials_consumed: 0,
            taxes_collected: 0,
            resources: ResourceStores::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WorkerTarget {
    Tree(TileCoord),
    Rock(TileCoord),
    Animal(WildlifeId),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Worker {
    pub id: WorkerId,
    pub nation: NationId,
    pub role: WorkerRole,
    pub location: TileCoord,
    pub target: Option<WorkerTarget>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WildAnimalKind {
    Deer,
    Fish,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WildAnimal {
    pub id: WildlifeId,
    pub kind: WildAnimalKind,
    pub location: TileCoord,
    #[serde(default)]
    pub heading: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RegrowingResource {
    location: TileCoord,
    renews_at: WorldTick,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settlement {
    pub id: SettlementId,
    pub name: String,
    pub location: TileCoord,
    pub population: u64,
    pub stage: SettlementStage,
    pub infrastructure: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nation {
    pub id: NationId,
    pub name: String,
    #[serde(default)]
    pub lord_name: String,
    pub government: Government,
    pub capital: SettlementId,
    pub territory: Vec<TileCoord>,
    pub settlements: Vec<Settlement>,
    pub population: u64,
    #[serde(default)]
    pub civilians: u64,
    #[serde(default)]
    pub workforce: u64,
    #[serde(default)]
    pub educated_workforce: u64,
    #[serde(default)]
    pub jobs: Vec<JobAssignment>,
    pub food: u32,
    pub materials: u32,
    pub wealth: u32,
    pub stability: u8,
    pub military: u32,
    pub research: u32,
    pub opportunity: u32,
    #[serde(default)]
    pub economy: Economy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TradeRoute {
    pub id: TradeRouteId,
    pub from: NationId,
    pub to: NationId,
    pub good: TradeGood,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Relation {
    pub first: NationId,
    pub second: NationId,
    pub opinion: i8,
    pub trust: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Conflict {
    pub id: ConflictId,
    pub attacker: NationId,
    pub defender: NationId,
    pub started: WorldTick,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HistoryEvent {
    pub id: HistoryEventId,
    pub tick: WorldTick,
    pub kind: EventKind,
    pub nation: Option<NationId>,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct World {
    pub seed: u64,
    pub tick: WorldTick,
    pub year: u32,
    pub activity_reservoir: u32,
    pub activity_total: u64,
    pub generated_chunks: Vec<ChunkCoord>,
    pub nations: Vec<Nation>,
    pub relations: Vec<Relation>,
    pub trade_routes: Vec<TradeRoute>,
    pub conflicts: Vec<Conflict>,
    pub history: Vec<HistoryEvent>,
    #[serde(default)]
    pub buildings: Vec<Building>,
    #[serde(default)]
    pub paths: Vec<Path>,
    #[serde(default)]
    pub workers: Vec<Worker>,
    #[serde(default)]
    pub wildlife: Vec<WildAnimal>,
    #[serde(default)]
    wildlife_layout_version: u8,
    #[serde(default)]
    settlement_layout_version: u8,
    #[serde(default)]
    pub harvested: Vec<TileCoord>,
    #[serde(default)]
    tree_stumps: Vec<RegrowingResource>,
    #[serde(default)]
    depleted_rocks: Vec<RegrowingResource>,
    #[serde(default)]
    mined_rock_sources: Vec<TileCoord>,
    #[serde(default)]
    spawned_rocks: Vec<TileCoord>,
    nation_ids: IdAllocator,
    settlement_ids: IdAllocator,
    trade_ids: IdAllocator,
    conflict_ids: IdAllocator,
    history_ids: IdAllocator,
    #[serde(default = "IdAllocator::new")]
    building_ids: IdAllocator,
    #[serde(default = "IdAllocator::new")]
    path_ids: IdAllocator,
    #[serde(default = "IdAllocator::new")]
    worker_ids: IdAllocator,
    #[serde(default = "IdAllocator::new")]
    wildlife_ids: IdAllocator,
}

impl World {
    #[must_use]
    pub fn new_demo(seed: u64, initial_nations: u8) -> Self {
        Self::new_demo_with_progress(seed, initial_nations, |_, _| {})
    }

    /// Creates a new world and reports completed setup stages for responsive callers.
    #[must_use]
    pub fn new_demo_with_progress<F>(seed: u64, initial_nations: u8, mut progress: F) -> Self
    where
        F: FnMut(u8, u8),
    {
        let initial_nations = initial_nations.clamp(1, 6);
        let total_stages = initial_nations.saturating_add(1);
        progress(0, total_stages);
        let mut world = Self {
            seed,
            tick: WorldTick::default(),
            year: 1800,
            activity_reservoir: 0,
            activity_total: 0,
            generated_chunks: Vec::new(),
            nations: Vec::new(),
            relations: Vec::new(),
            trade_routes: Vec::new(),
            conflicts: Vec::new(),
            history: Vec::new(),
            buildings: Vec::new(),
            paths: Vec::new(),
            workers: Vec::new(),
            wildlife: Vec::new(),
            wildlife_layout_version: 1,
            settlement_layout_version: 1,
            harvested: Vec::new(),
            tree_stumps: Vec::new(),
            depleted_rocks: Vec::new(),
            mined_rock_sources: Vec::new(),
            spawned_rocks: Vec::new(),
            nation_ids: IdAllocator::new(),
            settlement_ids: IdAllocator::new(),
            trade_ids: IdAllocator::new(),
            conflict_ids: IdAllocator::new(),
            history_ids: IdAllocator::new(),
            building_ids: IdAllocator::new(),
            path_ids: IdAllocator::new(),
            worker_ids: IdAllocator::new(),
            wildlife_ids: IdAllocator::new(),
        };
        for index in 0..initial_nations {
            world.found_nation(index);
            progress(index.saturating_add(1), total_stages);
        }
        world.reconcile_world();
        progress(total_stages, total_stages);
        world
    }

    #[must_use]
    pub fn generator(&self) -> WorldGenerator {
        WorldGenerator::new(self.seed)
    }

    #[must_use]
    pub fn tile(&self, coord: TileCoord) -> Tile {
        self.generator().generate_tile(coord)
    }

    #[must_use]
    pub fn owner_at(&self, coord: TileCoord) -> Option<NationId> {
        self.nations
            .iter()
            .find(|nation| nation.territory.contains(&coord))
            .map(|nation| nation.id)
    }

    #[must_use]
    pub fn nation(&self, id: NationId) -> Option<&Nation> {
        self.nations.iter().find(|nation| nation.id == id)
    }

    /// Brings saved worlds up to the current population-based territory scale.
    pub fn normalize_territories(&mut self) {
        self.grow_borders(usize::MAX);
        self.reconcile_world();
    }

    /// Relocates capitals from water generated by a newer world map version.
    pub fn relocate_water_capitals(&mut self) {
        let relocations: Vec<(usize, TileCoord, TileCoord)> = self
            .nations
            .iter()
            .enumerate()
            .filter_map(|(index, nation)| {
                let capital = nation
                    .settlements
                    .iter()
                    .find(|settlement| settlement.id == nation.capital)?;
                is_water(&self.tile(capital.location)).then(|| {
                    self.nearest_viable_unclaimed(capital.location, nation.id)
                        .map(|destination| (index, capital.location, destination))
                })?
            })
            .collect();
        for (index, old, destination) in relocations {
            self.ensure_chunk(destination);
            let nation = &mut self.nations[index];
            if let Some(capital) = nation
                .settlements
                .iter_mut()
                .find(|settlement| settlement.id == nation.capital)
            {
                capital.location = destination;
            }
            nation.territory.retain(|coord| *coord != old);
            if !nation.territory.contains(&destination) {
                nation.territory.push(destination);
            }
        }
    }

    /// Fixed simulation step. Activity is a neutral integer reservoir contribution.
    pub fn advance(&mut self, incoming_activity: u32) {
        self.tick = self.tick.next();
        self.year = 1800_u32.saturating_add(u32::try_from(self.tick.0 / 12).unwrap_or(u32::MAX));
        self.activity_total = self
            .activity_total
            .saturating_add(u64::from(incoming_activity));
        self.activity_reservoir = self
            .activity_reservoir
            .saturating_add(incoming_activity)
            .min(RESERVOIR_CAP);
        let allocations = self.allocate_opportunity();
        self.run_economy(&allocations);
        if self.tick.0.is_multiple_of(12) {
            self.expand_borders();
            self.reconcile_world();
        } else {
            self.rebalance_cohorts();
        }
        self.advance_workers();
        self.move_wildlife();
        self.renew_resources();
        self.apply_conflict_losses();
        self.move_trade();
        if self.tick.0.is_multiple_of(20) {
            self.evaluate_trade();
        }
        if self.tick.0.is_multiple_of(50) {
            self.evaluate_conflict();
        }
        if self.tick.0.is_multiple_of(240) && self.nations.len() < 6 && self.activity_total > 0 {
            self.found_nation(u8::try_from(self.nations.len()).unwrap_or(6));
        }
        self.history.truncate(HISTORY_LIMIT);
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.nations.iter().all(|nation| {
            nation.population > 0
                && nation
                    .settlements
                    .iter()
                    .all(|settlement| settlement.population > 0)
                && nation
                    .settlements
                    .iter()
                    .any(|settlement| settlement.id == nation.capital)
        }) && self
            .trade_routes
            .iter()
            .all(|route| self.nation(route.from).is_some() && self.nation(route.to).is_some())
            && self.conflicts.iter().all(|war| {
                self.nation(war.attacker).is_some() && self.nation(war.defender).is_some()
            })
    }

    #[must_use]
    pub fn has_tree_at(&self, coord: TileCoord) -> bool {
        let terrain = self.tile(coord).terrain;
        !self.is_tree_stump_at(coord) && has_natural_tree(coord, terrain)
    }

    #[must_use]
    pub fn is_tree_stump_at(&self, coord: TileCoord) -> bool {
        self.tree_stumps.iter().any(|stump| stump.location == coord)
    }

    #[must_use]
    pub fn has_rock_at(&self, coord: TileCoord) -> bool {
        if self.harvested.contains(&coord) || self.mined_rock_sources.contains(&coord) {
            return false;
        }
        let chance = match self.tile(coord).terrain {
            TerrainType::Beach => 3,
            TerrainType::Desert => 8,
            TerrainType::Hills => 1,
            TerrainType::Mountains => 5,
            _ => 0,
        };
        self.spawned_rocks.contains(&coord) || feature_roll(coord) % 100 < chance
    }

    fn reconcile_world(&mut self) {
        self.ensure_lords();
        self.rebalance_cohorts();
        self.ensure_wall_gates();
        self.sync_buildings();
        self.sync_trade_paths();
        self.sync_workers();
        self.spawn_wildlife();
    }

    /// Fills ruler names for worlds saved before rulers were introduced.
    pub fn ensure_lords(&mut self) {
        for (index, nation) in self.nations.iter_mut().enumerate() {
            if nation.lord_name.is_empty() {
                nation.lord_name = lord_name(self.seed, index);
            }
        }
    }

    /// Refreshes wildlife after loading a world saved with an older habitat layout.
    pub fn refresh_wildlife(&mut self) {
        self.spawn_wildlife();
    }

    /// Rebuilds structures and roads after loading a world saved with an older layout policy.
    pub fn refresh_settlement_layout(&mut self) {
        self.paths.clear();
        self.sync_buildings();
        self.sync_trade_paths();
    }

    /// Adds a bounded number of independently located nations from imported activity volume.
    pub fn seed_nations_from_import(&mut self, imported_records: usize) {
        let desired = 1 + imported_records / 64;
        while self.nations.len() < desired.min(6) {
            self.found_nation(u8::try_from(self.nations.len()).unwrap_or(6));
        }
        self.reconcile_world();
    }

    fn rebalance_cohorts(&mut self) {
        for nation in &mut self.nations {
            let military = u64::from(nation.military).min(nation.population / 20 + 1);
            let educated = nation.population / 10;
            let workforce = nation.population.saturating_mul(9) / 20;
            nation.military = u32::try_from(military).unwrap_or(u32::MAX);
            nation.educated_workforce = educated;
            nation.workforce = workforce;
            nation.jobs = job_assignments(workforce);
            nation.civilians = nation
                .population
                .saturating_sub(military.saturating_add(educated).saturating_add(workforce));
        }
    }

    #[allow(clippy::too_many_lines)]
    fn sync_buildings(&mut self) {
        self.refresh_legacy_farm_layout();
        let territories: Vec<(NationId, Vec<TileCoord>)> = self
            .nations
            .iter()
            .map(|nation| (nation.id, nation.territory.clone()))
            .collect();
        self.buildings.retain(|building| {
            territories.iter().any(|(nation, territory)| {
                *nation == building.nation && territory.contains(&building.location)
            })
        });
        self.paths.retain(|path| path.trade_route.is_some());
        let nations: Vec<(NationId, TileCoord, u64, LocalConditions)> = self
            .nations
            .iter()
            .filter_map(|nation| {
                nation
                    .settlements
                    .iter()
                    .find(|settlement| settlement.id == nation.capital)
                    .map(|capital| {
                        (
                            nation.id,
                            capital.location,
                            nation.population,
                            self.local_conditions(nation),
                        )
                    })
            })
            .collect();
        for (nation, capital, population, conditions) in nations {
            self.place_keep(nation, self.wall_gate(nation).unwrap_or(capital));
            let houses = usize::try_from((population / 600).clamp(3, 48)).unwrap_or(48);
            self.trim_buildings(nation, BuildingKind::House, houses);
            while self.count_buildings(nation, BuildingKind::House) < houses {
                if !self.ensure_building(nation, BuildingKind::House, capital, None, 16) {
                    break;
                }
            }
            let farms = usize::try_from((population / 700).clamp(2, 8)).unwrap_or(8);
            self.trim_buildings(nation, BuildingKind::Farm, farms);
            let farm_ids: Vec<BuildingId> = self
                .buildings
                .iter()
                .filter(|building| building.nation == nation && building.kind == BuildingKind::Farm)
                .map(|building| building.id)
                .collect();
            for farm in farm_ids {
                if !self.buildings.iter().any(|building| {
                    building.kind == BuildingKind::WheatField && building.farm == Some(farm)
                }) {
                    if let Some(location) = self
                        .buildings
                        .iter()
                        .find(|building| building.id == farm)
                        .and_then(|building| {
                            building
                                .location
                                .cardinal_neighbors()
                                .into_iter()
                                .find(|coord| self.is_bare_grass_for(nation, *coord))
                        })
                    {
                        self.add_building(nation, BuildingKind::WheatField, location, Some(farm));
                    }
                }
            }
            let fields: Vec<BuildingId> = self
                .buildings
                .iter()
                .filter(|building| building.kind == BuildingKind::WheatField)
                .filter_map(|building| building.farm)
                .collect();
            self.buildings.retain(|building| {
                building.kind != BuildingKind::Farm
                    || building.nation != nation
                    || fields.contains(&building.id)
            });
            while self.count_buildings(nation, BuildingKind::Farm) < farms {
                let Some(farm) = self.find_bare_grass(nation, capital, 20, 2, 5) else {
                    break;
                };
                let Some(field) = farm
                    .cardinal_neighbors()
                    .into_iter()
                    .find(|coord| self.is_bare_grass_for(nation, *coord))
                else {
                    break;
                };
                let farm_id = self.add_building(nation, BuildingKind::Farm, farm, None);
                self.add_building(nation, BuildingKind::WheatField, field, Some(farm_id));
            }
            self.sync_building_count(
                nation,
                BuildingKind::Workshop,
                usize::try_from((population / 3_000).clamp(1, 8)).unwrap_or(8),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::Market,
                usize::try_from((population / 2_500).clamp(1, 6)).unwrap_or(6),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::School,
                usize::try_from(population / 6_000).unwrap_or(4).min(4),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::Barracks,
                usize::try_from(population / 5_000).unwrap_or(3).min(3),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::Clinic,
                usize::try_from(population / 8_000).unwrap_or(2).min(2),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::Temple,
                usize::try_from(population / 7_000).unwrap_or(2).min(2),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::Mine,
                usize::from(conditions.minerals >= 8),
                capital,
            );
            self.sync_building_count(
                nation,
                BuildingKind::Lumberyard,
                usize::from(conditions.forest >= 8),
                capital,
            );
            self.connect_buildings(nation);
        }
        let farms: Vec<BuildingId> = self
            .buildings
            .iter()
            .filter(|building| building.kind == BuildingKind::Farm)
            .map(|building| building.id)
            .collect();
        self.buildings.retain(|building| {
            building.kind != BuildingKind::WheatField
                || building.farm.is_some_and(|farm| farms.contains(&farm))
        });
    }

    fn refresh_legacy_farm_layout(&mut self) {
        if self.settlement_layout_version == 0 {
            self.buildings.retain(|building| {
                !matches!(building.kind, BuildingKind::Farm | BuildingKind::WheatField)
            });
            self.settlement_layout_version = 1;
        }
    }

    fn count_buildings(&self, nation: NationId, kind: BuildingKind) -> usize {
        self.buildings
            .iter()
            .filter(|building| building.nation == nation && building.kind == kind)
            .count()
    }

    fn building_counts(&self, nation: NationId) -> BuildingCounts {
        let count = |kind| u32::try_from(self.count_buildings(nation, kind)).unwrap_or(u32::MAX);
        BuildingCounts {
            houses: count(BuildingKind::House),
            farms: count(BuildingKind::Farm),
            workshops: count(BuildingKind::Workshop),
            markets: count(BuildingKind::Market),
            schools: count(BuildingKind::School),
            barracks: count(BuildingKind::Barracks),
            clinics: count(BuildingKind::Clinic),
            temples: count(BuildingKind::Temple),
            sources: count(BuildingKind::Mine).saturating_add(count(BuildingKind::Lumberyard)),
        }
    }

    fn trim_buildings(&mut self, nation: NationId, kind: BuildingKind, desired: usize) {
        let mut excess = self.count_buildings(nation, kind).saturating_sub(desired);
        if excess == 0 {
            return;
        }
        self.buildings.retain(|building| {
            if building.nation == nation && building.kind == kind && excess > 0 {
                excess -= 1;
                false
            } else {
                true
            }
        });
    }

    fn sync_building_count(
        &mut self,
        nation: NationId,
        kind: BuildingKind,
        desired: usize,
        capital: TileCoord,
    ) {
        self.trim_buildings(nation, kind, desired);
        while self.count_buildings(nation, kind) < desired {
            if !self.ensure_building(nation, kind, capital, None, 20) {
                break;
            }
        }
    }

    fn ensure_building(
        &mut self,
        nation: NationId,
        kind: BuildingKind,
        origin: TileCoord,
        farm: Option<BuildingId>,
        radius: i32,
    ) -> bool {
        if self.count_buildings(nation, kind) > 0 && kind == BuildingKind::Keep {
            return false;
        }
        let location = if kind == BuildingKind::Keep && self.is_bare_grass_for(nation, origin) {
            Some(origin)
        } else {
            let minimum_distance = if kind == BuildingKind::Farm { 2 } else { 1 };
            self.find_bare_grass(nation, origin, radius, minimum_distance, 0)
        };
        if let Some(location) = location {
            self.add_building(nation, kind, location, farm);
            true
        } else {
            false
        }
    }

    fn place_keep(&mut self, nation: NationId, location: TileCoord) {
        if let Some(keep) = self
            .buildings
            .iter_mut()
            .find(|building| building.nation == nation && building.kind == BuildingKind::Keep)
        {
            keep.location = location;
        } else {
            self.add_building(nation, BuildingKind::Keep, location, None);
        }
    }

    fn add_building(
        &mut self,
        nation: NationId,
        kind: BuildingKind,
        location: TileCoord,
        farm: Option<BuildingId>,
    ) -> BuildingId {
        let id = BuildingId::new(self.building_ids.allocate_raw());
        self.buildings.push(Building {
            id,
            nation,
            kind,
            location,
            farm,
        });
        id
    }

    fn find_bare_grass(
        &self,
        nation: NationId,
        origin: TileCoord,
        radius: i32,
        minimum_distance: i32,
        minimum_building_gap: i32,
    ) -> Option<TileCoord> {
        let mut best = None;
        for distance in 0..=radius {
            if distance < minimum_distance {
                continue;
            }
            for x in -distance..=distance {
                for y in [-distance, distance] {
                    let coord = TileCoord::new(origin.x + x, origin.y + y);
                    if self.is_bare_grass_for(nation, coord)
                        && self.is_far_from_buildings(nation, coord, minimum_building_gap)
                    {
                        best = Some(better_building_site(best, coord));
                    }
                }
            }
            for y in (-distance + 1)..distance {
                for x in [-distance, distance] {
                    let coord = TileCoord::new(origin.x + x, origin.y + y);
                    if self.is_bare_grass_for(nation, coord)
                        && self.is_far_from_buildings(nation, coord, minimum_building_gap)
                    {
                        best = Some(better_building_site(best, coord));
                    }
                }
            }
        }
        best
    }

    fn is_bare_grass(&self, coord: TileCoord) -> bool {
        let tile = self.tile(coord);
        matches!(
            tile.terrain,
            TerrainType::Plains | TerrainType::Grassland | TerrainType::Prairie
        ) && !self.has_tree_at(coord)
            && !self.is_tree_stump_at(coord)
            && !self.has_rock_at(coord)
            && !self
                .buildings
                .iter()
                .any(|building| building.location == coord)
            && !self.paths.iter().any(|path| path.tiles.contains(&coord))
    }

    fn is_far_from_buildings(&self, nation: NationId, coord: TileCoord, minimum_gap: i32) -> bool {
        minimum_gap == 0
            || self
                .buildings
                .iter()
                .filter(|building| building.nation == nation)
                .all(|building| manhattan(building.location, coord) >= minimum_gap)
    }

    fn is_bare_grass_for(&self, nation: NationId, coord: TileCoord) -> bool {
        self.nation(nation)
            .is_some_and(|state| state.territory.contains(&coord))
            && self.wall_gate(nation) != Some(coord)
            && !self.is_wall_tile(coord)
            && self.is_bare_grass(coord)
    }

    fn connect_buildings(&mut self, nation: NationId) {
        let Some(keep) = self
            .buildings
            .iter()
            .find(|building| building.nation == nation && building.kind == BuildingKind::Keep)
            .map(|building| building.location)
        else {
            return;
        };
        let mut pending: Vec<TileCoord> = self
            .buildings
            .iter()
            .filter(|building| {
                building.nation == nation
                    && !matches!(building.kind, BuildingKind::Keep | BuildingKind::WheatField)
            })
            .map(|building| building.location)
            .collect();
        let mut network = vec![keep];
        while !pending.is_empty() {
            let Some((pending_index, endpoint, from)) = pending
                .iter()
                .enumerate()
                .flat_map(|(index, endpoint)| {
                    network.iter().map(move |from| (index, *endpoint, *from))
                })
                .min_by_key(|(_, endpoint, from)| manhattan(*from, *endpoint))
            else {
                break;
            };
            self.paths.push(Path {
                id: PathId::new(self.path_ids.allocate_raw()),
                nation,
                trade_route: None,
                from,
                to: endpoint,
                tiles: self.path_between(from, endpoint),
            });
            network.push(endpoint);
            pending.swap_remove(pending_index);
        }
    }

    fn sync_workers(&mut self) {
        let capitals: Vec<(NationId, TileCoord)> = self
            .nations
            .iter()
            .filter_map(|nation| {
                nation
                    .settlements
                    .iter()
                    .find(|settlement| settlement.id == nation.capital)
                    .map(|capital| (nation.id, capital.location))
            })
            .collect();
        for &(nation, _) in &capitals {
            for role in [
                WorkerRole::Lumberjack,
                WorkerRole::Stonemason,
                WorkerRole::Hunter,
            ] {
                let desired = 2;
                while self
                    .workers
                    .iter()
                    .filter(|worker| worker.nation == nation && worker.role == role)
                    .count()
                    < desired
                {
                    let id = WorkerId::new(self.worker_ids.allocate_raw());
                    self.workers.push(Worker {
                        id,
                        nation,
                        role,
                        location: self.worker_home(nation, id),
                        target: None,
                    });
                }
            }
        }
        for index in 0..self.workers.len() {
            if self.workers[index].target.is_none() {
                let worker = self.workers[index].clone();
                self.workers[index].target = self.worker_target(&worker);
            }
        }
    }

    fn spawn_wildlife(&mut self) {
        const LAND_PER_CHUNK: usize = 1;
        const FISH_PER_CHUNK: usize = 2;
        if self.wildlife_layout_version == 0 {
            self.wildlife.clear();
            self.wildlife_layout_version = 1;
        }
        let capitals: Vec<_> = self
            .nations
            .iter()
            .filter_map(|nation| nation.settlements.iter().find(|s| s.id == nation.capital))
            .map(|settlement| settlement.location.chunk())
            .collect();
        let mut habitat_chunks = self.generated_chunks.clone();
        for capital in capitals {
            for x in -1..=1 {
                for y in -1..=1 {
                    habitat_chunks.push(ChunkCoord::new(capital.x + x, capital.y + y));
                }
            }
        }
        habitat_chunks.sort();
        habitat_chunks.dedup();
        self.generated_chunks.clone_from(&habitat_chunks);
        let generator = self.generator();
        for chunk in habitat_chunks {
            let mut land = Vec::new();
            let mut water = Vec::new();
            for tile in generator.generate_chunk(chunk).tiles {
                if self.is_bare_grass(tile.coord) && !self.is_wall_tile(tile.coord) {
                    land.push(tile.coord);
                } else if is_water(&tile) && !self.is_wall_tile(tile.coord) {
                    water.push(tile.coord);
                }
            }
            self.populate_wildlife(chunk, WildAnimalKind::Deer, LAND_PER_CHUNK, land);
            self.populate_wildlife(chunk, WildAnimalKind::Fish, FISH_PER_CHUNK, water);
        }
    }

    fn populate_wildlife(
        &mut self,
        chunk: ChunkCoord,
        kind: WildAnimalKind,
        target: usize,
        mut candidates: Vec<TileCoord>,
    ) {
        let existing = self
            .wildlife
            .iter()
            .filter(|animal| animal.kind == kind && animal.location.chunk() == chunk)
            .count();
        let needed = target.saturating_sub(existing);
        candidates.sort_by_key(|coord| feature_roll(*coord) ^ self.seed);
        for location in candidates.into_iter().take(needed) {
            if self
                .wildlife
                .iter()
                .all(|animal| animal.location != location)
            {
                self.wildlife.push(WildAnimal {
                    id: WildlifeId::new(self.wildlife_ids.allocate_raw()),
                    kind,
                    location,
                    heading: 0,
                });
            }
        }
    }

    fn move_wildlife(&mut self) {
        const DIRECTIONS: [TileCoord; 4] = [
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(-1, 0),
            TileCoord::new(0, -1),
        ];
        let mut occupied: Vec<TileCoord> =
            self.wildlife.iter().map(|animal| animal.location).collect();
        let mut headings: Vec<u8> = self.wildlife.iter().map(|animal| animal.heading).collect();
        for (index, animal) in self.wildlife.iter().enumerate() {
            let phase =
                usize::try_from(self.tick.0.wrapping_add(animal.id.get())).unwrap_or_default();
            for (heading, direction) in DIRECTIONS
                .iter()
                .cycle()
                .skip(phase % DIRECTIONS.len())
                .take(DIRECTIONS.len())
                .enumerate()
            {
                let candidate = TileCoord::new(
                    animal.location.x + direction.x,
                    animal.location.y + direction.y,
                );
                let valid_habitat = match animal.kind {
                    WildAnimalKind::Deer => self.is_bare_grass(candidate),
                    WildAnimalKind::Fish => is_water(&self.tile(candidate)),
                } && !self.is_wall_tile(candidate);
                if valid_habitat
                    && occupied
                        .iter()
                        .enumerate()
                        .all(|(other, location)| other == index || *location != candidate)
                {
                    occupied[index] = candidate;
                    headings[index] =
                        u8::try_from((phase + heading) % DIRECTIONS.len()).unwrap_or_default();
                    break;
                }
            }
        }
        for ((animal, location), heading) in self.wildlife.iter_mut().zip(occupied).zip(headings) {
            animal.location = location;
            animal.heading = heading;
        }
    }

    fn advance_workers(&mut self) {
        let mut harvests = Vec::new();
        for index in 0..self.workers.len() {
            let worker = self.workers[index].clone();
            let target = worker.target.or_else(|| self.worker_target(&worker));
            let Some(target) = target else {
                let destination = self.worker_idle_destination(&worker);
                self.workers[index].location = step_toward(worker.location, destination, |coord| {
                    self.is_bare_grass_for(worker.nation, coord)
                });
                continue;
            };
            let target_location = match target {
                WorkerTarget::Tree(coord) | WorkerTarget::Rock(coord) => coord,
                WorkerTarget::Animal(id) => self
                    .wildlife
                    .iter()
                    .find(|animal| animal.id == id)
                    .and_then(|animal| self.hunting_position(&worker, animal))
                    .unwrap_or(worker.location),
            };
            let destination = self.worker_route_destination(&worker, target_location);
            let next = step_toward(worker.location, destination, |coord| {
                self.walkable_for_worker(worker.nation, coord)
            });
            self.workers[index].location = next;
            self.workers[index].target = Some(target);
            let reached = match target {
                WorkerTarget::Tree(_) | WorkerTarget::Rock(_) => next == target_location,
                WorkerTarget::Animal(id) => self
                    .wildlife
                    .iter()
                    .find(|animal| animal.id == id)
                    .is_some_and(|animal| {
                        manhattan(next, animal.location)
                            <= i32::from(animal.kind == WildAnimalKind::Fish)
                    }),
            };
            if reached {
                harvests.push((worker.nation, target));
                self.workers[index].target = None;
            }
        }
        for (nation, target) in harvests {
            match target {
                WorkerTarget::Tree(coord) => {
                    if !self.is_tree_stump_at(coord) {
                        self.tree_stumps.push(RegrowingResource {
                            location: coord,
                            renews_at: WorldTick(self.tick.0.saturating_add(TREE_REGROW_TICKS)),
                        });
                        self.add_materials(nation, 24);
                    }
                }
                WorkerTarget::Rock(coord) => {
                    if self.has_rock_at(coord) {
                        self.mined_rock_sources.push(coord);
                        self.spawned_rocks.retain(|location| *location != coord);
                        self.depleted_rocks.push(RegrowingResource {
                            location: coord,
                            renews_at: WorldTick(self.tick.0.saturating_add(ROCK_RESPAWN_TICKS)),
                        });
                        self.add_materials(nation, 32);
                    }
                }
                WorkerTarget::Animal(id) => {
                    if let Some(index) = self.wildlife.iter().position(|animal| animal.id == id) {
                        self.wildlife.remove(index);
                        if let Some(state) =
                            self.nations.iter_mut().find(|state| state.id == nation)
                        {
                            state.food = state.food.saturating_add(60);
                        }
                    }
                }
            }
        }
    }

    fn worker_target(&self, worker: &Worker) -> Option<WorkerTarget> {
        match worker.role {
            WorkerRole::Lumberjack => nearby_coords(worker.location, 20)
                .into_iter()
                .find(|coord| self.has_tree_at(*coord))
                .map(WorkerTarget::Tree),
            WorkerRole::Stonemason => nearby_coords(worker.location, 20)
                .into_iter()
                .find(|coord| self.has_rock_at(*coord))
                .map(WorkerTarget::Rock),
            WorkerRole::Hunter => self
                .wildlife
                .iter()
                .filter_map(|animal| {
                    self.hunting_position(worker, animal)
                        .map(|location| (animal.id, location))
                })
                .filter(|(_, location)| manhattan(worker.location, *location) <= 20)
                .min_by_key(|(_, location)| manhattan(worker.location, *location))
                .map(|(id, _)| WorkerTarget::Animal(id)),
        }
    }

    fn hunting_position(&self, worker: &Worker, animal: &WildAnimal) -> Option<TileCoord> {
        match animal.kind {
            WildAnimalKind::Deer => Some(animal.location),
            WildAnimalKind::Fish => animal
                .location
                .cardinal_neighbors()
                .into_iter()
                .filter(|coord| self.walkable_for_worker(worker.nation, *coord))
                .min_by_key(|coord| manhattan(worker.location, *coord)),
        }
    }

    fn add_materials(&mut self, nation: NationId, amount: u32) {
        if let Some(state) = self.nations.iter_mut().find(|state| state.id == nation) {
            state.materials = state.materials.saturating_add(amount);
        }
    }

    fn renew_resources(&mut self) {
        self.tree_stumps.retain(|stump| stump.renews_at > self.tick);
        let ready: Vec<_> = self
            .depleted_rocks
            .iter()
            .filter(|rock| rock.renews_at <= self.tick)
            .cloned()
            .collect();
        self.depleted_rocks
            .retain(|rock| rock.renews_at > self.tick);
        for rock in ready {
            if let Some(location) = self.rock_respawn_location(rock.location) {
                self.spawned_rocks.push(location);
            }
        }
    }

    fn rock_respawn_location(&self, origin: TileCoord) -> Option<TileCoord> {
        let mut candidates = nearby_coords(origin, 12);
        candidates.sort_by_key(|coord| feature_roll(*coord) ^ self.tick.0);
        candidates
            .into_iter()
            .find(|coord| !self.is_tree_stump_at(*coord) && self.is_bare_grass(*coord))
    }

    fn worker_home(&self, nation: NationId, worker: WorkerId) -> TileCoord {
        let Some(capital) = self.nation(nation).and_then(|nation| {
            nation
                .settlements
                .iter()
                .find(|settlement| settlement.id == nation.capital)
                .map(|settlement| settlement.location)
        }) else {
            return TileCoord::default();
        };
        let homes: Vec<TileCoord> = nearby_coords(capital, 2)
            .into_iter()
            .filter(|coord| self.is_bare_grass_for(nation, *coord))
            .collect();
        homes
            .get(usize::try_from(worker.get()).unwrap_or_default() % homes.len().max(1))
            .copied()
            .unwrap_or(capital)
    }

    fn worker_idle_destination(&self, worker: &Worker) -> TileCoord {
        let Some(nation) = self.nation(worker.nation) else {
            return worker.location;
        };
        let mut destinations: Vec<_> = nation
            .territory
            .iter()
            .copied()
            .filter(|coord| self.is_bare_grass_for(worker.nation, *coord))
            .collect();
        destinations.sort();
        let phase = self.tick.0 / 6;
        destinations
            .get(
                usize::try_from(phase.wrapping_add(worker.id.get())).unwrap_or_default()
                    % destinations.len().max(1),
            )
            .copied()
            .unwrap_or(worker.location)
    }

    fn worker_route_destination(&self, worker: &Worker, destination: TileCoord) -> TileCoord {
        let Some(gate) = self.wall_gate(worker.nation) else {
            return destination;
        };
        if worker.location == gate {
            return destination;
        }
        let from_inside = self.is_within_walls(worker.nation, worker.location);
        let destination_inside = self.is_within_walls(worker.nation, destination);
        if from_inside == destination_inside {
            destination
        } else {
            gate
        }
    }

    fn walkable_for_worker(&self, nation: NationId, coord: TileCoord) -> bool {
        !is_water(&self.tile(coord))
            && !matches!(self.tile(coord).terrain, TerrainType::Mountains)
            && (!self.is_wall_tile(coord) || self.wall_gate(nation) == Some(coord))
    }

    /// Routes roads around natural obstacles. Water remains traversable so the resulting path
    /// becomes a bridge/path tile instead of leaving an impossible gap in the road network.
    fn path_between(&self, from: TileCoord, to: TileCoord) -> Vec<TileCoord> {
        let distance = manhattan(from, to).max(1);
        // ponytail: bounded A* keeps long trade routes finite; widen this only if worldgen
        // introduces obstacle belts wider than 384 tiles.
        let margin = (distance / 3).clamp(96, 192);
        for margin in [margin, margin.saturating_mul(2)] {
            if let Some(path) = self.path_between_with_margin(from, to, margin) {
                return path;
            }
        }
        Vec::new()
    }

    fn path_between_with_margin(
        &self,
        from: TileCoord,
        to: TileCoord,
        margin: i32,
    ) -> Option<Vec<TileCoord>> {
        let min_x = from.x.min(to.x) - margin;
        let max_x = from.x.max(to.x) + margin;
        let min_y = from.y.min(to.y) - margin;
        let max_y = from.y.max(to.y) + margin;
        let mut frontier = BinaryHeap::new();
        let mut cost = BTreeMap::new();
        let mut previous = BTreeMap::new();
        frontier.push((Reverse(0_u32), Reverse(0_u32), Reverse(from)));
        cost.insert(from, 0_u32);

        while let Some((_, Reverse(current_cost), Reverse(current))) = frontier.pop() {
            if current == to {
                let mut path = Vec::new();
                let mut cursor = to;
                while let Some(parent) = previous.get(&cursor).copied() {
                    if cursor != to {
                        path.push(cursor);
                    }
                    cursor = parent;
                }
                path.reverse();
                return Some(path);
            }
            if cost.get(&current).copied() != Some(current_cost) {
                continue;
            }
            for next in current.cardinal_neighbors() {
                if next.x < min_x || next.x > max_x || next.y < min_y || next.y > max_y {
                    continue;
                }
                let Some(step_cost) = self.path_tile_cost(next, from, to) else {
                    continue;
                };
                let next_cost = current_cost.saturating_add(step_cost);
                if cost.get(&next).is_some_and(|known| *known <= next_cost) {
                    continue;
                }
                cost.insert(next, next_cost);
                previous.insert(next, current);
                frontier.push((
                    Reverse(next_cost.saturating_add(
                        u32::try_from(manhattan(next, to)).unwrap_or(u32::MAX) * 10,
                    )),
                    Reverse(next_cost),
                    Reverse(next),
                ));
            }
        }
        None
    }

    fn path_tile_cost(&self, coord: TileCoord, from: TileCoord, to: TileCoord) -> Option<u32> {
        if coord != from
            && coord != to
            && (self.has_tree_at(coord)
                || self.has_rock_at(coord)
                || self.is_wall_tile(coord)
                || self
                    .buildings
                    .iter()
                    .any(|building| building.location == coord))
        {
            return None;
        }
        Some(if is_water(&self.tile(coord)) { 16 } else { 10 })
    }

    fn is_within_walls(&self, nation: NationId, coord: TileCoord) -> bool {
        self.nation(nation)
            .and_then(wall_bounds)
            .is_some_and(|(min_x, max_x, min_y, max_y)| {
                coord.x > min_x && coord.x < max_x && coord.y > min_y && coord.y < max_y
            })
    }

    fn wall_gate(&self, nation: NationId) -> Option<TileCoord> {
        self.nation(nation)
            .and_then(wall_bounds)
            .map(|(min_x, max_x, _min_y, max_y)| TileCoord::new(min_x.midpoint(max_x), max_y))
    }

    fn ensure_wall_gates(&mut self) {
        let gates: Vec<_> = self
            .nations
            .iter()
            .filter_map(|nation| self.wall_gate(nation.id).map(|gate| (nation.id, gate)))
            .collect();
        for (nation_id, gate) in gates {
            if let Some(nation) = self
                .nations
                .iter_mut()
                .find(|nation| nation.id == nation_id)
            {
                if !nation.territory.contains(&gate) {
                    nation.territory.push(gate);
                }
            }
        }
    }

    #[must_use]
    pub fn is_wall_tile(&self, coord: TileCoord) -> bool {
        self.nations.iter().any(|nation| {
            wall_bounds(nation).is_some_and(|(min_x, max_x, min_y, max_y)| {
                self.wall_gate(nation.id) != Some(coord)
                    && (coord.x == min_x
                        || coord.x == max_x
                        || coord.y == min_y
                        || coord.y == max_y)
                    && coord.x >= min_x
                    && coord.x <= max_x
                    && coord.y >= min_y
                    && coord.y <= max_y
            })
        })
    }

    fn found_nation(&mut self, index: u8) {
        let tile = self.find_spawn_tile();
        let territory = self.starting_territory(tile.coord);
        for coord in &territory {
            self.ensure_chunk(*coord);
        }
        let id = NationId::new(self.nation_ids.allocate_raw());
        let capital = SettlementId::new(self.settlement_ids.allocate_raw());
        let name = nation_name(self.seed, index as usize);
        let government = match self.seed.wrapping_add(u64::from(index)) % 4 {
            0 => Government::Council,
            1 => Government::Republic,
            2 => Government::Monarchy,
            _ => Government::Directorate,
        };
        let settlement = Settlement {
            id: capital,
            name: format!(
                "{} Haven",
                name.split_whitespace().next().unwrap_or("Aster")
            ),
            location: tile.coord,
            population: 1_800,
            stage: SettlementStage::Village,
            infrastructure: 8,
        };
        self.nations.push(Nation {
            id,
            name: name.clone(),
            lord_name: lord_name(self.seed, usize::from(index)),
            government,
            capital,
            territory,
            settlements: vec![settlement],
            population: 1_800,
            civilians: 0,
            workforce: 0,
            educated_workforce: 0,
            food: 700,
            materials: 120,
            wealth: 100,
            stability: 62,
            military: 45,
            research: 0,
            opportunity: 0,
            jobs: Vec::new(),
            economy: Economy::default(),
        });
        self.rebalance_cohorts();
        self.event(
            EventKind::Founded,
            Some(id),
            format!("{name} established a settlement."),
        );
    }

    fn starting_territory(&self, capital: TileCoord) -> Vec<TileCoord> {
        let mut territory = Vec::new();
        for x in -3..=3 {
            for y in -3..=3 {
                let coord = TileCoord::new(capital.x + x, capital.y + y);
                let tile = self.tile(coord);
                if !is_water(&tile) && !matches!(tile.terrain, TerrainType::Mountains) {
                    territory.push(coord);
                }
            }
        }
        territory
    }

    fn find_spawn_tile(&self) -> Tile {
        if (self.seed.wrapping_add(self.tick.0) % 100) < 70 {
            if let Some(tile) = self.spawn_near_trade_path() {
                return tile;
            }
        }
        let generator = self.generator();
        let region = STARTING_REGIONS[self.nations.len() % STARTING_REGIONS.len()];
        let mut radius = 0_i32;
        loop {
            for x in -radius..=radius {
                for y in -radius..=radius {
                    if x.abs().max(y.abs()) != radius {
                        continue;
                    }
                    let chunk =
                        generator.generate_chunk(ChunkCoord::new(region.x + x, region.y + y));
                    if let Some(tile) = chunk.tiles.into_iter().find(|tile| {
                        tile.is_viable_spawn() && is_open_grass(tile) && self.is_distant(tile.coord)
                    }) {
                        return tile;
                    }
                }
            }
            radius = radius.saturating_add(1);
        }
    }

    fn spawn_near_trade_path(&self) -> Option<Tile> {
        let paths: Vec<&Path> = self
            .paths
            .iter()
            .filter(|path| path.trade_route.is_some() && path.tiles.len() >= 192)
            .collect();
        let path = paths.get(self.nations.len() % paths.len().max(1))?;
        for step in 0..path.tiles.len() {
            let index = (path.tiles.len() / 2 + step) % path.tiles.len();
            let origin = path.tiles[index];
            for radius in 0..=12 {
                for x in -radius..=radius {
                    for y in [-radius, radius] {
                        let coord = TileCoord::new(origin.x + x, origin.y + y);
                        let tile = self.tile(coord);
                        if tile.is_viable_spawn() && is_open_grass(&tile) && self.is_distant(coord)
                        {
                            return Some(tile);
                        }
                    }
                }
            }
        }
        None
    }

    fn is_distant(&self, tile: TileCoord) -> bool {
        self.nations.iter().all(|nation| {
            nation
                .territory
                .iter()
                .all(|claimed| manhattan(*claimed, tile) >= 96)
        })
    }

    fn nearest_viable_unclaimed(
        &self,
        origin: TileCoord,
        nation_id: NationId,
    ) -> Option<TileCoord> {
        for radius in 1..=64 {
            for x in -radius..=radius {
                for y in [-radius, radius] {
                    let coord = TileCoord::new(origin.x + x, origin.y + y);
                    if self.tile(coord).is_viable_spawn()
                        && self
                            .nations
                            .iter()
                            .filter(|nation| nation.id != nation_id)
                            .all(|nation| !nation.territory.contains(&coord))
                    {
                        return Some(coord);
                    }
                }
            }
            for y in (-radius + 1)..radius {
                for x in [-radius, radius] {
                    let coord = TileCoord::new(origin.x + x, origin.y + y);
                    if self.tile(coord).is_viable_spawn()
                        && self
                            .nations
                            .iter()
                            .filter(|nation| nation.id != nation_id)
                            .all(|nation| !nation.territory.contains(&coord))
                    {
                        return Some(coord);
                    }
                }
            }
        }
        None
    }

    fn allocate_opportunity(&mut self) -> Vec<u32> {
        let spend = self.activity_reservoir.min(20);
        self.activity_reservoir -= spend;
        let weights: Vec<u32> = self
            .nations
            .iter()
            .map(|nation| {
                u32::try_from(nation.population / 250)
                    .unwrap_or(u32::MAX)
                    .saturating_add(u32::from(100 - nation.stability))
                    .max(1)
            })
            .collect();
        let total = weights.iter().copied().sum::<u32>().max(1);
        weights
            .into_iter()
            .map(|weight| spend.saturating_mul(weight) / total)
            .collect()
    }

    #[allow(clippy::too_many_lines)]
    fn run_economy(&mut self, allocations: &[u32]) {
        let conditions: Vec<LocalConditions> = self
            .nations
            .iter()
            .map(|nation| self.local_conditions(nation))
            .collect();
        let capacities: Vec<BuildingCounts> = self
            .nations
            .iter()
            .map(|nation| self.building_counts(nation.id))
            .collect();
        let mut events = Vec::new();
        for (index, nation) in self.nations.iter_mut().enumerate() {
            let opportunity = allocations.get(index).copied().unwrap_or_default();
            let conditions = conditions[index];
            let BuildingCounts {
                houses,
                farms,
                workshops,
                markets,
                schools,
                barracks,
                clinics,
                temples,
                sources,
            } = capacities[index];
            nation.opportunity = nation.opportunity.saturating_add(opportunity);
            let workers = |role| job_workers(&nation.jobs, role);
            let farmer = workers(JobRole::Farmer);
            let rancher = workers(JobRole::Rancher);
            let fisher = workers(JobRole::Fisher);
            let lumberjack = workers(JobRole::Lumberjack);
            let stonemason = workers(JobRole::Stonemason);
            let hunter = workers(JobRole::Hunter);
            let blacksmith = workers(JobRole::Blacksmith);
            let carpenter = workers(JobRole::Carpenter);
            let mason = workers(JobRole::Mason);
            let weaver = workers(JobRole::Weaver);
            let potter = workers(JobRole::Potter);
            let tanner = workers(JobRole::Tanner);
            let herbalist = workers(JobRole::Herbalist);
            let merchant = workers(JobRole::Merchant);
            let innkeeper = workers(JobRole::Innkeeper);
            let builder = workers(JobRole::Builder);
            let teamster = workers(JobRole::Teamster);
            let healer = workers(JobRole::Healer);
            let scribe = workers(JobRole::Scribe);
            let priest = workers(JobRole::Priest);
            let guard = workers(JobRole::Guard);
            let soldier = workers(JobRole::Soldier);

            let resources = &mut nation.economy.resources;
            resources.wood = resources
                .wood
                .saturating_add(lumberjack / 3 + sources.saturating_mul(conditions.forest / 12));
            resources.stone = resources
                .stone
                .saturating_add(stonemason / 3 + sources.saturating_mul(conditions.minerals / 14));
            resources.ore = resources
                .ore
                .saturating_add(stonemason / 5 + sources.saturating_mul(conditions.minerals / 10));
            resources.livestock = resources.livestock.saturating_add(rancher / 4);
            resources.fish = resources
                .fish
                .saturating_add(fisher / 3 + conditions.water.saturating_mul(fisher / 80));
            resources.medicine = resources
                .medicine
                .saturating_add(herbalist / 3 + healer / 8);

            let crafted = (blacksmith / 5 + carpenter / 6 + mason / 6 + workshops * 4)
                .min((resources.wood + resources.stone + resources.ore) / 2);
            let timber_used = crafted.min(resources.wood);
            resources.wood -= timber_used;
            let mineral_used = crafted.saturating_sub(timber_used);
            let stone_used = mineral_used.min(resources.stone);
            resources.stone -= stone_used;
            resources.ore = resources
                .ore
                .saturating_sub(mineral_used.saturating_sub(stone_used));
            resources.tools = resources.tools.saturating_add(blacksmith / 7 + crafted / 4);
            resources.goods = resources
                .goods
                .saturating_add(weaver / 4 + potter / 4 + tanner / 4 + workshops * 2);

            let produce_food = farms
                .saturating_mul(14)
                .saturating_add(farmer / 7)
                .saturating_add(rancher / 5)
                .saturating_add(fisher / 4)
                .saturating_add(hunter / 9)
                .saturating_add(conditions.fertility.saturating_mul(2))
                .saturating_add(opportunity / 8);
            let consume = u32::try_from(nation.population / 350)
                .unwrap_or(u32::MAX)
                .saturating_add(soldier / 18)
                .saturating_add(12);
            let food_cap = u32::try_from(nation.population / 2)
                .unwrap_or(u32::MAX)
                .saturating_add(300);
            nation.economy.food_produced = produce_food;
            nation.economy.food_consumed = consume;
            nation.food = nation.food.saturating_add(produce_food).min(food_cap);
            if nation.food < consume {
                let loss = (nation.population / 220).max(2);
                nation.population = nation.population.saturating_sub(loss).max(1);
                nation.stability = nation.stability.saturating_sub(3);
                if self.tick.0.is_multiple_of(6) {
                    events.push((
                        EventKind::Famine,
                        nation.id,
                        format!("{} faces a food shortage.", nation.name),
                    ));
                }
            } else {
                nation.food -= consume;
                nation.economy.housing = u64::from(houses).saturating_mul(750);
                if nation.population < nation.economy.housing {
                    nation.population = nation.population.saturating_add(
                        (nation.population / 4_000).max(1) + u64::from(opportunity / 30),
                    );
                } else {
                    nation.stability = nation.stability.saturating_sub(2);
                }
                nation.stability = nation.stability.saturating_add(1).min(100);
            }
            let produced_materials = crafted
                .saturating_add(resources.tools / 20)
                .saturating_add(opportunity / 12);
            let materials_consumed = builder / 5 + nation.materials / 120 + markets;
            let material_cap = u32::try_from(nation.population.saturating_mul(2))
                .unwrap_or(u32::MAX)
                .saturating_add(500);
            nation.materials = nation
                .materials
                .saturating_add(produced_materials)
                .saturating_sub(materials_consumed)
                .min(material_cap);
            nation.economy.materials_produced = produced_materials;
            nation.economy.materials_consumed = materials_consumed;
            let income = merchant / 3
                + innkeeper / 4
                + teamster / 5
                + resources.goods / 24
                + markets * 3
                + opportunity / 6;
            let taxes = income.saturating_mul(u32::from(nation.economy.tax_rate)) / 100;
            nation.economy.taxes_collected = taxes;
            nation.wealth = nation
                .wealth
                .saturating_add(income.saturating_sub(taxes))
                .min(material_cap);
            nation.economy.treasury = nation.economy.treasury.saturating_add(taxes);
            let services = schools * 3 + clinics * 3 + barracks * 2 + temples + guard / 10;
            nation.economy.treasury = nation.economy.treasury.saturating_sub(services);
            nation.economy.education = nation
                .economy
                .education
                .saturating_add(u8::try_from(schools + scribe / 80).unwrap_or(u8::MAX))
                .min(100);
            nation.economy.health = nation
                .economy
                .health
                .saturating_add(u8::try_from(clinics + healer / 90).unwrap_or(u8::MAX))
                .min(100);
            nation.economy.crime = nation
                .economy
                .crime
                .saturating_sub(u8::try_from(guard / 70 + markets).unwrap_or(u8::MAX));
            nation.stability = nation
                .stability
                .saturating_add(u8::try_from(priest / 90 + temples).unwrap_or(u8::MAX))
                .min(100);
            nation.research = nation.research.saturating_add(
                opportunity / 10 + u32::from(nation.economy.education) / 8 + scribe / 12,
            );
            let desired_military = u32::try_from(nation.population / 40)
                .unwrap_or(u32::MAX)
                .min(barracks.saturating_add(1).saturating_mul(80));
            if nation.military < desired_military && nation.food > consume.saturating_mul(3) {
                nation.military = nation.military.saturating_add(1);
            }
            if let Some(capital) = nation
                .settlements
                .iter_mut()
                .find(|settlement| settlement.id == nation.capital)
            {
                capital.population = nation.population;
                capital.infrastructure = capital.infrastructure.saturating_add(
                    u16::try_from(builder / 18 + opportunity / 25).unwrap_or(u16::MAX),
                );
                let next = stage_for(capital.population, capital.infrastructure);
                if next != capital.stage {
                    capital.stage = next;
                    events.push((
                        EventKind::SettlementAdvanced,
                        nation.id,
                        format!("{} grew into a {:?}.", capital.name, next),
                    ));
                }
            }
        }
        for (kind, nation, summary) in events {
            self.event(kind, Some(nation), summary);
        }
    }

    fn local_conditions(&self, nation: &Nation) -> LocalConditions {
        nation
            .territory
            .iter()
            .fold(LocalConditions::default(), |mut conditions, coord| {
                let tile = self.tile(*coord);
                conditions.fertility += u32::from(
                    tile.has_resource(ResourceKind::FertileSoil)
                        || tile.has_resource(ResourceKind::FreshWater),
                );
                conditions.minerals += u32::from(
                    tile.has_resource(ResourceKind::Iron)
                        || tile.has_resource(ResourceKind::Coal)
                        || matches!(tile.terrain, TerrainType::Hills | TerrainType::Mountains),
                );
                conditions.forest += u32::from(matches!(tile.terrain, TerrainType::Forest));
                conditions.water += u32::from(is_water(&tile));
                conditions
            })
    }

    fn move_trade(&mut self) {
        for route in self.trade_routes.clone() {
            let (Some(from), Some(to)) = (
                self.nations
                    .iter()
                    .position(|nation| nation.id == route.from),
                self.nations.iter().position(|nation| nation.id == route.to),
            ) else {
                continue;
            };
            let amount = 8;
            if from < to {
                let (left, right) = self.nations.split_at_mut(to);
                let exporter = &mut left[from];
                let importer = &mut right[0];
                transfer(exporter, importer, route.good, amount);
            } else {
                let (left, right) = self.nations.split_at_mut(from);
                let importer = &mut left[to];
                let exporter = &mut right[0];
                transfer(exporter, importer, route.good, amount);
            }
        }
    }

    fn evaluate_trade(&mut self) {
        for from in 0..self.nations.len() {
            for to in 0..self.nations.len() {
                if from == to
                    || self.at_war(self.nations[from].id, self.nations[to].id)
                    || self.route_exists(self.nations[from].id, self.nations[to].id)
                {
                    continue;
                }
                if self.nations[from].food > self.nations[to].food.saturating_add(40) {
                    let route = TradeRoute {
                        id: TradeRouteId::new(self.trade_ids.allocate_raw()),
                        from: self.nations[from].id,
                        to: self.nations[to].id,
                        good: TradeGood::Food,
                    };
                    self.event(
                        EventKind::TradeOpened,
                        Some(route.from),
                        format!(
                            "{} opened a food route to {}.",
                            self.nations[from].name, self.nations[to].name
                        ),
                    );
                    self.adjust_relation(route.from, route.to, 4);
                    self.establish_trade_path(&route);
                    self.trade_routes.push(route);
                    return;
                }
            }
        }
    }

    fn sync_trade_paths(&mut self) {
        for route in self.trade_routes.clone() {
            self.establish_trade_path(&route);
        }
    }

    fn establish_trade_path(&mut self, route: &TradeRoute) {
        if self
            .paths
            .iter()
            .any(|path| path.trade_route == Some(route.id))
        {
            return;
        }
        let Some(from) = self.wall_gate(route.from).or_else(|| {
            self.nation(route.from).and_then(|nation| {
                nation
                    .settlements
                    .iter()
                    .find(|settlement| settlement.id == nation.capital)
                    .map(|settlement| settlement.location)
            })
        }) else {
            return;
        };
        let Some(to) = self.wall_gate(route.to).or_else(|| {
            self.nation(route.to).and_then(|nation| {
                nation
                    .settlements
                    .iter()
                    .find(|settlement| settlement.id == nation.capital)
                    .map(|settlement| settlement.location)
            })
        }) else {
            return;
        };
        self.paths.push(Path {
            id: PathId::new(self.path_ids.allocate_raw()),
            nation: route.from,
            trade_route: Some(route.id),
            from,
            to,
            tiles: self.path_between(from, to),
        });
    }

    fn evaluate_conflict(&mut self) {
        let expired: Vec<ConflictId> = self
            .conflicts
            .iter()
            .filter(|war| self.tick.0.saturating_sub(war.started.0) >= 40)
            .map(|war| war.id)
            .collect();
        for id in expired {
            if let Some(index) = self.conflicts.iter().position(|war| war.id == id) {
                let war = self.conflicts.remove(index);
                self.event(
                    EventKind::WarEnded,
                    Some(war.attacker),
                    format!(
                        "War between {} and {} ended.",
                        self.nation(war.attacker)
                            .map_or("unknown", |nation| nation.name.as_str()),
                        self.nation(war.defender)
                            .map_or("unknown", |nation| nation.name.as_str())
                    ),
                );
            }
        }
        if self.nations.len() < 2 || !self.conflicts.is_empty() {
            return;
        }
        let nation_count = u64::try_from(self.nations.len()).unwrap_or(1);
        let attacker = usize::try_from((self.tick.0 / 50) % nation_count).unwrap_or(0);
        let defender = (attacker + 1) % self.nations.len();
        if self.nations[attacker].stability < 48
            || self.nations[attacker].food < 100
            || self.tick.0.is_multiple_of(300)
        {
            let war = Conflict {
                id: ConflictId::new(self.conflict_ids.allocate_raw()),
                attacker: self.nations[attacker].id,
                defender: self.nations[defender].id,
                started: self.tick,
            };
            self.trade_routes.retain(|route| {
                route.from != war.attacker
                    && route.to != war.attacker
                    && route.from != war.defender
                    && route.to != war.defender
            });
            self.adjust_relation(war.attacker, war.defender, -25);
            self.event(
                EventKind::WarBegan,
                Some(war.attacker),
                format!(
                    "{} entered conflict with {}.",
                    self.nations[attacker].name, self.nations[defender].name
                ),
            );
            self.conflicts.push(war);
        }
    }

    fn apply_conflict_losses(&mut self) {
        for war in self.conflicts.clone() {
            for nation_id in [war.attacker, war.defender] {
                if let Some(nation) = self
                    .nations
                    .iter_mut()
                    .find(|nation| nation.id == nation_id)
                {
                    nation.population = nation
                        .population
                        .saturating_sub((nation.population / 900).max(1));
                    nation.food = nation.food.saturating_sub(4);
                    nation.wealth = nation.wealth.saturating_sub(2);
                    nation.stability = nation.stability.saturating_sub(1);
                }
            }
        }
    }

    fn expand_borders(&mut self) {
        self.grow_borders(4);
    }

    fn grow_borders(&mut self, rounds: usize) {
        for _ in 0..rounds {
            let claimed: Vec<TileCoord> = self
                .nations
                .iter()
                .flat_map(|nation| nation.territory.iter().copied())
                .collect();
            let expansions: Vec<(usize, TileCoord)> = self
                .nations
                .iter()
                .enumerate()
                .filter_map(|(index, nation)| {
                    let capacity =
                        usize::try_from((nation.population / 350).clamp(81, 196)).unwrap_or(196);
                    let (min_x, max_x, min_y, max_y) = wall_bounds(nation)?;
                    let width = max_x - min_x + 1;
                    let height = max_y - min_y + 1;
                    if (nation.territory.len() >= capacity
                        && width >= MIN_NATION_SPAN
                        && height >= MIN_NATION_SPAN)
                        || nation.stability < 35
                    {
                        return None;
                    }
                    nation
                        .territory
                        .iter()
                        .flat_map(|coord| coord.cardinal_neighbors())
                        .filter(|coord| {
                            !claimed.contains(coord) && is_claimable_land(&self.tile(*coord))
                        })
                        .min_by_key(|coord| {
                            let candidate_width = max_x.max(coord.x) - min_x.min(coord.x) + 1;
                            let candidate_height = max_y.max(coord.y) - min_y.min(coord.y) + 1;
                            (
                                candidate_width.abs_diff(candidate_height),
                                candidate_width.max(candidate_height),
                                feature_roll(*coord),
                            )
                        })
                        .map(|coord| (index, coord))
                })
                .collect();
            if expansions.is_empty() {
                return;
            }
            for (index, coord) in expansions {
                self.ensure_chunk(coord);
                let (id, name) = {
                    let nation = &mut self.nations[index];
                    nation.territory.push(coord);
                    (nation.id, nation.name.clone())
                };
                self.event(
                    EventKind::BorderExpanded,
                    Some(id),
                    format!("{name} expanded its border."),
                );
            }
        }
    }

    fn ensure_chunk(&mut self, coord: TileCoord) {
        let chunk = coord.chunk();
        if !self.generated_chunks.contains(&chunk) {
            self.generated_chunks.push(chunk);
            self.generated_chunks.sort();
        }
    }

    fn route_exists(&self, from: NationId, to: NationId) -> bool {
        self.trade_routes
            .iter()
            .any(|route| route.from == from && route.to == to)
    }
    fn at_war(&self, a: NationId, b: NationId) -> bool {
        self.conflicts.iter().any(|war| {
            (war.attacker == a && war.defender == b) || (war.attacker == b && war.defender == a)
        })
    }
    fn adjust_relation(&mut self, a: NationId, b: NationId, change: i8) {
        let (first, second) = if a < b { (a, b) } else { (b, a) };
        let relation = self
            .relations
            .iter_mut()
            .find(|relation| relation.first == first && relation.second == second);
        if let Some(relation) = relation {
            relation.opinion = relation.opinion.saturating_add(change);
            relation.trust = relation.trust.saturating_add_signed(change.max(0));
        } else {
            self.relations.push(Relation {
                first,
                second,
                opinion: change,
                trust: change.max(0).cast_unsigned(),
            });
        }
    }
    fn event(&mut self, kind: EventKind, nation: Option<NationId>, summary: String) {
        self.history.insert(
            0,
            HistoryEvent {
                id: HistoryEventId::new(self.history_ids.allocate_raw()),
                tick: self.tick,
                kind,
                nation,
                summary,
            },
        );
    }
}

fn job_assignments(workforce: u64) -> Vec<JobAssignment> {
    let total_weight: u64 = JobRole::ALL.iter().map(|role| role.weight()).sum();
    let mut assigned = 0_u64;
    JobRole::ALL
        .iter()
        .enumerate()
        .map(|(index, role)| {
            let workers = if index + 1 == JobRole::ALL.len() {
                workforce.saturating_sub(assigned)
            } else {
                workforce.saturating_mul(role.weight()) / total_weight
            };
            assigned = assigned.saturating_add(workers);
            JobAssignment {
                role: *role,
                workers,
            }
        })
        .collect()
}

fn job_workers(jobs: &[JobAssignment], role: JobRole) -> u32 {
    jobs.iter()
        .find(|job| job.role == role)
        .and_then(|job| u32::try_from(job.workers).ok())
        .unwrap_or_default()
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

fn wall_bounds(nation: &Nation) -> Option<(i32, i32, i32, i32)> {
    nation.territory.iter().fold(None, |bounds, coord| {
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
}

fn has_natural_tree(coord: TileCoord, terrain: TerrainType) -> bool {
    let grove = feature_roll(TileCoord::new(coord.x.div_euclid(3), coord.y.div_euclid(3)));
    let canopy = feature_roll(TileCoord::new(coord.x.div_euclid(5), coord.y.div_euclid(5)));
    let local = feature_roll(coord);
    match terrain {
        TerrainType::Forest => grove % 100 < 88 && canopy % 100 < 88 && local % 100 < 94,
        TerrainType::Prairie => grove % 100 < 18 && local % 100 < 44,
        _ => false,
    }
}

fn nearby_coords(origin: TileCoord, radius: i32) -> Vec<TileCoord> {
    let mut coords = Vec::new();
    for distance in 1..=radius {
        for x in -distance..=distance {
            for y in [-distance, distance] {
                coords.push(TileCoord::new(origin.x + x, origin.y + y));
            }
        }
        for y in (-distance + 1)..distance {
            for x in [-distance, distance] {
                coords.push(TileCoord::new(origin.x + x, origin.y + y));
            }
        }
    }
    coords
}

fn step_toward(
    from: TileCoord,
    target: TileCoord,
    walkable: impl Fn(TileCoord) -> bool,
) -> TileCoord {
    from.cardinal_neighbors()
        .into_iter()
        .filter(|coord| walkable(*coord))
        .min_by_key(|coord| manhattan(*coord, target))
        .unwrap_or(from)
}

fn is_claimable_land(tile: &Tile) -> bool {
    !matches!(
        tile.terrain,
        TerrainType::Water
            | TerrainType::Ocean
            | TerrainType::Lake
            | TerrainType::River
            | TerrainType::Mountains
    )
}

fn is_open_grass(tile: &Tile) -> bool {
    matches!(
        tile.terrain,
        TerrainType::Plains | TerrainType::Grassland | TerrainType::Prairie
    ) && !has_natural_tree(tile.coord, tile.terrain)
}

fn is_water(tile: &Tile) -> bool {
    matches!(
        tile.terrain,
        TerrainType::Water | TerrainType::Ocean | TerrainType::Lake | TerrainType::River
    )
}

fn transfer(exporter: &mut Nation, importer: &mut Nation, good: TradeGood, amount: u32) {
    match good {
        TradeGood::Food => {
            let moved = exporter.food.min(amount);
            exporter.food -= moved;
            importer.food = importer.food.saturating_add(moved);
        }
        TradeGood::Materials => {
            let moved = exporter.materials.min(amount);
            exporter.materials -= moved;
            importer.materials = importer.materials.saturating_add(moved);
        }
    }
    exporter.wealth = exporter.wealth.saturating_add(1);
    importer.wealth = importer.wealth.saturating_add(1);
}

fn stage_for(population: u64, infrastructure: u16) -> SettlementStage {
    match (population, infrastructure) {
        (population, _) if population >= 60_000 => SettlementStage::Metropolis,
        (population, infrastructure) if population >= 15_000 && infrastructure >= 80 => {
            SettlementStage::City
        }
        (population, infrastructure) if population >= 5_000 && infrastructure >= 30 => {
            SettlementStage::Town
        }
        (_, infrastructure) if infrastructure >= 10 => SettlementStage::Village,
        _ => SettlementStage::Camp,
    }
}

fn manhattan(a: TileCoord, b: TileCoord) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn better_building_site(current: Option<TileCoord>, candidate: TileCoord) -> TileCoord {
    let Some(current) = current else {
        return candidate;
    };
    if feature_roll(candidate) > feature_roll(current) {
        candidate
    } else {
        current
    }
}

fn nation_name(seed: u64, index: usize) -> String {
    const START: [&str; 64] = [
        "Alder", "Amber", "Aster", "Ash", "Briar", "Brindle", "Cedar", "Cinder", "Coral", "Crow",
        "Dawn", "Dun", "Eagle", "Elder", "Ember", "Fallow", "Frost", "Gale", "Gild", "Glen",
        "Golden", "Granite", "Harbor", "Hearth", "High", "Hollow", "Iron", "Ivory", "Juniper",
        "Lark", "Laurel", "Lumen", "Marble", "Meadow", "Mist", "Moon", "Morrow", "North", "Oak",
        "Onyx", "Pale", "Pine", "Quartz", "Raven", "Red", "River", "Rose", "Rowan", "Sable",
        "Salt", "Silver", "Sky", "Spring", "Stone", "Summer", "Thorn", "Vale", "Verdant", "West",
        "Whisper", "White", "Wild", "Willow", "Winter",
    ];
    const END: [&str; 62] = [
        "barrow", "bloom", "bridge", "brook", "cairn", "cape", "coast", "court", "crest", "cross",
        "crown", "dale", "deep", "dell", "fen", "field", "ford", "gate", "glade", "grove",
        "harbor", "haven", "heart", "height", "hold", "keep", "land", "march", "mark", "meadow",
        "mere", "moor", "port", "reach", "rest", "ridge", "rise", "rock", "run", "scar", "shore",
        "spire", "stead", "strand", "summit", "thorn", "vale", "veil", "ward", "watch", "water",
        "way", "weald", "well", "wind", "wood", "wold", "wright", "yard", "zenith", "hollow",
        "isle",
    ];
    let shift = usize::try_from(seed % u64::try_from(START.len()).unwrap_or(1)).unwrap_or(0);
    format!(
        "{}{}",
        START[(index + shift) % START.len()],
        END[(index * 11 + shift * 7) % END.len()]
    )
}

#[allow(clippy::too_many_lines)]
fn lord_name(seed: u64, index: usize) -> String {
    const GIVEN: [&str; 80] = [
        "Adrian",
        "Aelia",
        "Alden",
        "Alina",
        "Amara",
        "Ansel",
        "Arden",
        "Ari",
        "Aurelia",
        "Bastian",
        "Beatrix",
        "Benedict",
        "Briar",
        "Calla",
        "Cassian",
        "Celeste",
        "Cora",
        "Darius",
        "Delia",
        "Edric",
        "Elara",
        "Elian",
        "Elowen",
        "Emil",
        "Esme",
        "Evander",
        "Felicity",
        "Finnian",
        "Flora",
        "Galen",
        "Giselle",
        "Hadrian",
        "Helena",
        "Hollis",
        "Ilyra",
        "Iris",
        "Isolde",
        "Jasper",
        "Julian",
        "Junia",
        "Kael",
        "Leander",
        "Liora",
        "Lucan",
        "Lyra",
        "Maren",
        "Maris",
        "Mira",
        "Nadia",
        "Nolan",
        "Oriana",
        "Orin",
        "Peregrine",
        "Petra",
        "Quentin",
        "Rhea",
        "Ronan",
        "Rowan",
        "Sabine",
        "Soren",
        "Sylvia",
        "Talia",
        "Theron",
        "Tobin",
        "Valen",
        "Vera",
        "Willa",
        "Xanthe",
        "Yara",
        "Zara",
        "Aster",
        "Bram",
        "Cyra",
        "Dorian",
        "Eira",
        "Freya",
        "Garrick",
        "Hera",
        "Idris",
        "Jonas",
    ];
    const HOUSE: [&str; 64] = [
        "Amberfell",
        "Ashbourne",
        "Blackmere",
        "Brightwater",
        "Cinderhall",
        "Dawncrest",
        "Duskwood",
        "Elderbrook",
        "Emberlain",
        "Fairwind",
        "Fallowmere",
        "Frostvale",
        "Gildedale",
        "Goldriver",
        "Graymark",
        "Greenward",
        "Hallowell",
        "Harthorne",
        "Highridge",
        "Ironwood",
        "Kestrel",
        "Larkspur",
        "Lightfoot",
        "Longmere",
        "Marrowind",
        "Moonfall",
        "Northwatch",
        "Oakenshield",
        "Ravencrest",
        "Redwyne",
        "Riverstone",
        "Rosewood",
        "Saltmarsh",
        "Seabrook",
        "Silverkeep",
        "Skyreach",
        "Snowmelt",
        "Stonebridge",
        "Summerfield",
        "Thornwall",
        "Valehart",
        "Verdigris",
        "Westmere",
        "Whisperwind",
        "Whiteford",
        "Wilding",
        "Willowmere",
        "Winterborn",
        "Woodward",
        "Wyrmrest",
        "Alderwyn",
        "Briarwood",
        "Crownward",
        "Deepwell",
        "Eagleford",
        "Foxglove",
        "Granitehall",
        "Hearthglen",
        "Ivorygate",
        "Juniper",
        "Laurelhill",
        "Morrowdale",
        "Onyxmere",
        "Pineward",
    ];
    let shift = usize::try_from(seed % u64::try_from(GIVEN.len()).unwrap_or(1)).unwrap_or(0);
    format!(
        "{} {}",
        GIVEN[(index + shift) % GIVEN.len()],
        HOUSE[(index * 13 + shift * 5) % HOUSE.len()]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_and_ticks_are_deterministic() {
        let mut first = World::new_demo(12, 4);
        let mut second = World::new_demo(12, 4);
        for _ in 0..100 {
            first.advance(17);
            second.advance(17);
        }
        assert_eq!(first, second);
    }

    #[test]
    fn world_creation_reports_progress() {
        let mut progress = Vec::new();
        let world = World::new_demo_with_progress(12, 3, |completed, total| {
            progress.push((completed, total));
        });

        assert_eq!(progress.first(), Some(&(0, 4)));
        assert_eq!(progress.last(), Some(&(4, 4)));
        assert_eq!(world.nations.len(), 3);
    }

    #[test]
    fn a_new_world_can_start_with_one_nation() {
        assert_eq!(World::new_demo(12, 1).nations.len(), 1);
    }

    #[test]
    fn a_new_nation_claims_a_small_starting_border() {
        let world = World::new_demo(12, 1);
        assert!(world.nations[0].territory.len() >= 25);
    }

    #[test]
    fn keeps_are_wall_gates() {
        let world = World::new_demo(12, 1);
        let nation = &world.nations[0];
        let gate = world.wall_gate(nation.id).unwrap();
        let (min_x, max_x, _, max_y) = wall_bounds(nation).unwrap();
        assert_eq!(gate, TileCoord::new(min_x.midpoint(max_x), max_y));
        assert!(nation.territory.contains(&gate));
        assert!(world.buildings.iter().any(|building| {
            building.nation == nation.id
                && building.kind == BuildingKind::Keep
                && building.location == gate
        }));
        assert!(!world.is_wall_tile(gate));
    }

    #[test]
    fn nations_never_spawn_on_water() {
        for seed in 0..8 {
            let world = World::new_demo(seed, 6);
            assert!(world.nations.iter().all(|nation| {
                nation
                    .settlements
                    .iter()
                    .find(|settlement| settlement.id == nation.capital)
                    .is_some_and(|capital| !is_water(&world.tile(capital.location)))
            }));
        }
    }

    #[test]
    fn imported_activity_seeds_additional_nations() {
        let mut world = World::new_demo(12, 1);
        world.seed_nations_from_import(128);
        assert_eq!(world.nations.len(), 3);
    }

    #[test]
    fn buildings_and_roads_stay_within_their_nation() {
        let mut world = World::new_demo(12, 1);
        for _ in 0..24 {
            world.advance(0);
        }

        assert!(world.buildings.iter().all(|building| {
            world
                .nation(building.nation)
                .is_some_and(|nation| nation.territory.contains(&building.location))
        }));
        assert!(world.paths.iter().all(|path| {
            world.nation(path.nation).is_some_and(|nation| {
                nation.territory.contains(&path.from) && nation.territory.contains(&path.to)
            })
        }));
    }

    #[test]
    fn paths_avoid_tree_and_rock_tiles_but_allow_water_tiles() {
        let mut world = World::new_demo(12, 1);
        world.refresh_settlement_layout();

        assert!(world.paths.iter().any(|path| !path.tiles.is_empty()));
        assert!(world
            .paths
            .iter()
            .flat_map(|path| &path.tiles)
            .all(|coord| { !world.has_tree_at(*coord) && !world.has_rock_at(*coord) }));
        let water = (-96..=96)
            .flat_map(|x| (-96..=96).map(move |y| TileCoord::new(x, y)))
            .find(|coord| is_water(&world.tile(*coord)))
            .expect("world generation should include water");
        assert_eq!(world.path_tile_cost(water, water, water), Some(16));
    }

    #[test]
    fn jobs_feed_the_economy_ledger_and_public_services() {
        let mut world = World::new_demo(12, 1);
        world.advance(0);
        let economy = &world.nations[0].economy;

        assert!(economy.food_produced > 0);
        assert!(economy.materials_produced > 0);
        assert!(economy.taxes_collected > 0);
        assert!(economy.housing > 0);
        assert!(economy.resources.goods > 0);
        assert!(economy.resources.medicine > 0);
    }

    #[test]
    fn trade_routes_establish_permanent_paths() {
        let mut world = World::new_demo(12, 2);
        world.nations[0].food = 1_000;
        world.nations[1].food = 0;
        world.evaluate_trade();

        let route = world.trade_routes[0].id;
        let path = world
            .paths
            .iter()
            .find(|path| path.trade_route == Some(route))
            .expect("trade route should create a path");
        assert!(!path.tiles.is_empty(), "{path:?}");
        world.trade_routes.clear();
        world.sync_buildings();
        assert!(world
            .paths
            .iter()
            .any(|path| path.trade_route == Some(route)));
    }

    #[test]
    fn wildlife_moves_and_nations_receive_unique_rulers() {
        let mut world = World::new_demo(12, 6);
        let wildlife_before = world.wildlife.clone();
        world.move_wildlife();

        assert_ne!(world.wildlife, wildlife_before);
        assert!(world
            .wildlife
            .iter()
            .any(|animal| animal.kind == WildAnimalKind::Fish));
        assert!(world.wildlife.iter().all(|animal| {
            (match animal.kind {
                WildAnimalKind::Deer => world.is_bare_grass(animal.location),
                WildAnimalKind::Fish => is_water(&world.tile(animal.location)),
            }) && !world.is_wall_tile(animal.location)
        }));
        let mut lords: Vec<&str> = world
            .nations
            .iter()
            .map(|nation| nation.lord_name.as_str())
            .collect();
        lords.sort_unstable();
        lords.dedup();
        assert_eq!(lords.len(), world.nations.len());
    }

    #[test]
    fn idle_workers_choose_walkable_nation_tiles() {
        for seed in 0..8 {
            let world = World::new_demo(seed, 1);
            for worker in &world.workers {
                let destination = world.worker_idle_destination(worker);
                assert!(world.is_bare_grass_for(worker.nation, destination));
            }
        }
    }

    #[test]
    fn world_advances_without_activity() {
        let mut world = World::new_demo(12, 4);
        let before = world.tick;
        world.advance(0);
        assert!(world.tick > before);
    }

    #[test]
    fn territory_normalization_matches_population_scale() {
        let mut world = World::new_demo(12, 4);
        world.normalize_territories();

        assert!(world.nations.iter().all(|nation| {
            let (min_x, max_x, min_y, max_y) = wall_bounds(nation).unwrap();
            nation.territory.len() >= 36
                && max_x - min_x + 1 >= MIN_NATION_SPAN
                && max_y - min_y + 1 >= MIN_NATION_SPAN
        }));
        assert!(world.validate());
    }

    #[test]
    fn water_capital_is_relocated_to_viable_land() {
        let mut world = World::new_demo(12, 4);
        let water = (-256..=256)
            .flat_map(|x| (-256..=256).map(move |y| TileCoord::new(x, y)))
            .find(|coord| is_water(&world.tile(*coord)))
            .unwrap();
        let nation = &mut world.nations[0];
        nation.territory = vec![water];
        nation.settlements[0].location = water;

        world.relocate_water_capitals();

        let capital = world.nations[0].settlements[0].location;
        assert_ne!(capital, water);
        assert!(!is_water(&world.tile(capital)));
    }

    #[test]
    fn settlements_keep_paired_farms_and_persistent_workers() {
        let mut world = World::new_demo(31, 3);
        for _ in 0..48 {
            world.advance(0);
        }
        assert!(world
            .workers
            .iter()
            .any(|worker| worker.role == WorkerRole::Lumberjack));
        assert!(world
            .workers
            .iter()
            .any(|worker| worker.role == WorkerRole::Stonemason));
        assert!(world
            .workers
            .iter()
            .any(|worker| worker.role == WorkerRole::Hunter));
        for farm in world
            .buildings
            .iter()
            .filter(|building| building.kind == BuildingKind::Farm)
        {
            let field = world
                .buildings
                .iter()
                .find(|building| {
                    building.kind == BuildingKind::WheatField && building.farm == Some(farm.id)
                })
                .unwrap();
            assert_eq!(manhattan(farm.location, field.location), 1);
        }
        assert!(!world.paths.is_empty());
    }

    #[test]
    fn nations_assign_every_medieval_job() {
        let world = World::new_demo(31, 1);
        let nation = &world.nations[0];

        assert_eq!(nation.jobs.len(), JobRole::ALL.len());
        assert_eq!(
            nation.jobs.iter().map(|job| job.workers).sum::<u64>(),
            nation.workforce
        );
    }

    #[test]
    fn tree_stumps_regrow_after_their_timer() {
        let mut world = World::new_demo(31, 1);
        let tree = (-128..=128)
            .flat_map(|x| (-128..=128).map(move |y| TileCoord::new(x, y)))
            .find(|coord| world.has_tree_at(*coord))
            .unwrap();
        world.tree_stumps.push(RegrowingResource {
            location: tree,
            renews_at: WorldTick(1),
        });
        assert!(world.is_tree_stump_at(tree));
        assert!(!world.has_tree_at(tree));

        world.tick = WorldTick(1);
        world.renew_resources();

        assert!(!world.is_tree_stump_at(tree));
        assert!(world.has_tree_at(tree));
    }

    #[test]
    fn long_run_remains_valid() {
        let mut world = World::new_demo(12, 4);
        for _ in 0..100 {
            world.advance(8);
        }
        assert!(world.validate());
        assert!(!world.history.is_empty());
    }
}
