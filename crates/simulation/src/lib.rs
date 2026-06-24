//! Autonomous simulation engine foundation for ThreadNations.

use std::collections::BTreeMap;

use threadnations_analysis::{classify_activity, focus_for_topic, usage_pressure, ActivitySource, NationFocus};
use threadnations_common::{
    HistoryEventId, IdAllocator, NationId, SettlementId, TileCoord, WorldTick,
};
use threadnations_worldgen::{MapChunk, ResourceKind, Tile, WorldGenerator};

/// Nation resource inventory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceInventory {
    resources: BTreeMap<ResourceKind, u32>,
}

impl ResourceInventory {
    /// Adds resources to the inventory.
    pub fn add(&mut self, resource: ResourceKind, amount: u32) {
        let entry = self.resources.entry(resource).or_default();
        *entry = entry.saturating_add(amount);
    }

    /// Gets the current amount for a resource.
    #[must_use]
    pub fn amount(&self, resource: ResourceKind) -> u32 {
        self.resources.get(&resource).copied().unwrap_or_default()
    }

    /// Returns all tracked resources in deterministic order.
    pub fn entries(&self) -> impl Iterator<Item = (ResourceKind, u32)> + '_ {
        self.resources.iter().map(|(resource, amount)| (*resource, *amount))
    }
}

/// Aggregated population state for an MVP-scale nation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationCohort {
    /// Citizen population.
    pub citizens: u64,
    /// Military population.
    pub military: u64,
    /// Workforce population.
    pub workforce: u64,
    /// Abstract education index from 0 to 100.
    pub education: u8,
    /// Public health index from 0 to 100.
    pub health: u8,
    /// Happiness index from 0 to 100.
    pub happiness: u8,
    /// Crime pressure from 0 to 100.
    pub crime: u8,
    /// Immigration pressure.
    pub immigration: u32,
    /// Emigration pressure.
    pub emigration: u32,
}

impl PopulationCohort {
    /// Creates a starting population from bootstrap pressure.
    #[must_use]
    pub fn bootstrap(population_pull: u32) -> Self {
        let citizens = 800 + u64::from(population_pull) * 25;
        Self {
            citizens,
            military: citizens / 100,
            workforce: citizens / 2,
            education: 25,
            health: 55,
            happiness: 50,
            crime: 5,
            immigration: population_pull,
            emigration: 0,
        }
    }

    /// Applies population growth from activity and living conditions.
    pub fn grow(&mut self, population_pull: u32, food_surplus: bool, water_access: bool) {
        let base_growth = if food_surplus && water_access { 18 } else { 5 };
        let growth = u64::from(base_growth + population_pull.min(200));
        self.citizens = self.citizens.saturating_add(growth);
        self.workforce = self.citizens / 2;
        self.military = (self.citizens / 100).max(self.military);
        self.immigration = self.immigration.saturating_add(population_pull / 2);
    }
}

/// Government type for a nation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GovernmentType {
    /// Tribal council.
    TribalCouncil,
    /// Monarchy.
    Monarchy,
    /// Theocracy.
    Theocracy,
    /// Dictatorship.
    Dictatorship,
    /// Military junta.
    MilitaryJunta,
    /// Republic.
    Republic,
    /// Parliamentary democracy.
    ParliamentaryDemocracy,
    /// Corporate state.
    CorporateState,
    /// Technocracy.
    Technocracy,
    /// Socialist state.
    SocialistState,
    /// Federal union.
    FederalUnion,
    /// Failed state.
    FailedState,
}

/// Technology age for uneven national progression.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TechnologyAge {
    /// Stone age.
    Stone,
    /// Bronze age.
    Bronze,
    /// Iron age.
    Iron,
    /// Classical age.
    Classical,
    /// Medieval age.
    Medieval,
    /// Early industrial age.
    EarlyIndustrial,
    /// Industrial age.
    Industrial,
    /// Modern age.
    Modern,
    /// Atomic age.
    Atomic,
    /// Information age.
    Information,
    /// Space age.
    Space,
    /// Post-space age.
    PostSpace,
}

/// Settlement stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementStage {
    /// Camp.
    Camp,
    /// Village.
    Village,
    /// Town.
    Town,
    /// City.
    City,
    /// Metropolis.
    Metropolis,
    /// Mega-region.
    MegaRegion,
}

/// A settlement owned by a nation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settlement {
    /// Stable settlement ID.
    pub id: SettlementId,
    /// Settlement name.
    pub name: String,
    /// Settlement stage.
    pub stage: SettlementStage,
    /// Settlement location.
    pub location: TileCoord,
    /// Settlement population.
    pub population: u64,
}

/// Autonomous nation state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nation {
    /// Stable nation ID.
    pub id: NationId,
    /// Activity source that founded this nation.
    pub source_id: threadnations_common::ActivitySourceId,
    /// Generated nation name.
    pub name: String,
    /// Core national focus.
    pub focus: NationFocus,
    /// Government type.
    pub government: GovernmentType,
    /// Population cohort.
    pub population: PopulationCohort,
    /// Current technology age.
    pub technology_age: TechnologyAge,
    /// Claimed tile territory.
    pub territory: Vec<TileCoord>,
    /// Settlements.
    pub settlements: Vec<Settlement>,
    /// Resource inventory.
    pub inventory: ResourceInventory,
    /// Abstract military strength score.
    pub military_strength: u32,
    /// Research progress points.
    pub research_progress: u32,
    /// Culture progress points.
    pub culture_progress: u32,
}

impl Nation {
    /// Bootstraps a nation from an activity source and spawn tile.
    #[must_use]
    pub fn from_activity(
        id: NationId,
        settlement_id: SettlementId,
        source: &ActivitySource,
        spawn_tile: &Tile,
    ) -> Self {
        let topic = classify_activity(source);
        let focus = focus_for_topic(topic);
        let pressure = usage_pressure(source);
        let population = PopulationCohort::bootstrap(pressure.population_pull);
        let government = government_for_focus(focus);
        let mut inventory = ResourceInventory::default();

        for deposit in &spawn_tile.resources {
            inventory.add(deposit.kind, u32::from(deposit.abundance));
        }
        inventory.add(ResourceKind::FoodCrops, pressure.production / 4);
        inventory.add(ResourceKind::FreshWater, 25);

        let name = generate_nation_name(source, focus);
        let settlement = Settlement {
            id: settlement_id,
            name: format!("{} Hearth", name_core(&name)),
            stage: SettlementStage::Camp,
            location: spawn_tile.coord,
            population: population.citizens,
        };

        Self {
            id,
            source_id: source.id,
            name,
            focus,
            government,
            population,
            technology_age: TechnologyAge::Stone,
            territory: vec![spawn_tile.coord],
            settlements: vec![settlement],
            inventory,
            military_strength: 10 + pressure.production / 20,
            research_progress: pressure.research,
            culture_progress: pressure.culture,
        }
    }

    /// Returns true if this nation owns a tile.
    #[must_use]
    pub fn owns_tile(&self, coord: TileCoord) -> bool {
        self.territory.contains(&coord)
    }
}

/// Returns a government tendency from focus.
#[must_use]
pub const fn government_for_focus(focus: NationFocus) -> GovernmentType {
    match focus {
        NationFocus::TechnologyAndEngineering | NationFocus::ScientificResearch => {
            GovernmentType::Technocracy
        }
        NationFocus::CulturalProduction | NationFocus::ReligiousAndArtisticCulture => {
            GovernmentType::Theocracy
        }
        NationFocus::ImportExportEconomy => GovernmentType::Republic,
        NationFocus::BureaucracyAndLaw => GovernmentType::ParliamentaryDemocracy,
        NationFocus::SurvivalistProduction => GovernmentType::TribalCouncil,
        NationFocus::MilitaryIndustry => GovernmentType::MilitaryJunta,
    }
}

/// Category of historical event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryEventKind {
    /// A nation formed from an activity source.
    NationFounded,
    /// A nation grew due to activity or autonomous pressure.
    NationGrowth,
    /// A border expanded into unclaimed land.
    BorderExpanded,
}

/// A readable event in world history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEvent {
    /// Stable event ID.
    pub id: HistoryEventId,
    /// Tick when the event occurred.
    pub tick: WorldTick,
    /// Event kind.
    pub kind: HistoryEventKind,
    /// Optional nation attached to this event.
    pub nation_id: Option<NationId>,
    /// Human-readable summary.
    pub summary: String,
}

/// Persistent world state owned by the simulation crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    /// World seed.
    pub seed: u64,
    /// Current simulation tick.
    pub tick: WorldTick,
    /// Generated chunks currently held in memory.
    pub chunks: Vec<MapChunk>,
    /// Autonomous nations.
    pub nations: Vec<Nation>,
    /// History log.
    pub history: Vec<HistoryEvent>,
    generator: WorldGenerator,
    nation_ids: IdAllocator,
    settlement_ids: IdAllocator,
    history_ids: IdAllocator,
}

impl World {
    /// Creates a new world with no nations.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tick: WorldTick::default(),
            chunks: Vec::new(),
            nations: Vec::new(),
            history: Vec::new(),
            generator: WorldGenerator::new(seed),
            nation_ids: IdAllocator::new(),
            settlement_ids: IdAllocator::new(),
            history_ids: IdAllocator::new(),
        }
    }

    /// Returns true if any nation owns the tile.
    #[must_use]
    pub fn is_claimed(&self, coord: TileCoord) -> bool {
        self.nations.iter().any(|nation| nation.owns_tile(coord))
    }

    /// Spawns a nation from an activity source on viable unclaimed land.
    pub fn spawn_nation_from_activity(
        &mut self,
        source: &ActivitySource,
    ) -> threadnations_common::EngineResult<NationId> {
        let spawn_tile = self.generator.find_spawn_site(|coord| self.is_claimed(coord), 8)?;
        self.ensure_chunk_loaded(spawn_tile.coord);

        let nation_id = NationId::new(self.nation_ids.allocate_raw());
        let settlement_id = SettlementId::new(self.settlement_ids.allocate_raw());
        let nation = Nation::from_activity(nation_id, settlement_id, source, &spawn_tile);
        let nation_name = nation.name.clone();
        self.nations.push(nation);

        self.push_history(
            HistoryEventKind::NationFounded,
            Some(nation_id),
            format!("{nation_name} formed from activity source '{}'.", source.title),
        );

        Ok(nation_id)
    }

    /// Advances autonomous simulation by one tick using the latest activity source set.
    pub fn advance_one_tick(&mut self, activities: &[ActivitySource]) {
        self.tick = self.tick.next();

        for activity in activities {
            if let Some(index) = self
                .nations
                .iter()
                .position(|nation| nation.source_id == activity.id)
            {
                self.apply_activity_to_nation(index, activity);
            }
        }

        self.expand_borders();
    }

    /// Finds a nation by ID.
    #[must_use]
    pub fn nation(&self, id: NationId) -> Option<&Nation> {
        self.nations.iter().find(|nation| nation.id == id)
    }

    fn apply_activity_to_nation(&mut self, index: usize, activity: &ActivitySource) {
        let pressure = usage_pressure(activity);
        let (nation_id, nation_name) = {
            let nation = &mut self.nations[index];
            nation.research_progress = nation.research_progress.saturating_add(pressure.research);
            nation.culture_progress = nation.culture_progress.saturating_add(pressure.culture);
            nation.inventory.add(ResourceKind::FoodCrops, pressure.production / 3);
            nation.inventory.add(ResourceKind::Wood, pressure.labor / 20);
            nation.inventory.add(ResourceKind::Iron, pressure.production / 40);
            nation.military_strength = nation
                .military_strength
                .saturating_add(pressure.production / 100);
            let has_food = nation.inventory.amount(ResourceKind::FoodCrops) > 0;
            let has_water = nation.inventory.amount(ResourceKind::FreshWater) > 0;
            nation.population.grow(pressure.population_pull, has_food, has_water);
            (nation.id, nation.name.clone())
        };

        self.push_history(
            HistoryEventKind::NationGrowth,
            Some(nation_id),
            format!("{nation_name} grew from renewed usage pressure."),
        );
    }

    fn expand_borders(&mut self) {
        let claimed: Vec<TileCoord> = self
            .nations
            .iter()
            .flat_map(|nation| nation.territory.iter().copied())
            .collect();
        let mut expansions = Vec::new();

        for (index, nation) in self.nations.iter().enumerate() {
            if nation.population.citizens < (nation.territory.len() as u64 * 500) {
                continue;
            }

            if let Some(expansion) = first_unclaimed_neighbor(&nation.territory, &claimed) {
                expansions.push((index, expansion));
            }
        }

        for (index, coord) in expansions {
            self.ensure_chunk_loaded(coord);
            let expanded = {
                let nation = &mut self.nations[index];
                if nation.territory.contains(&coord) {
                    None
                } else {
                    nation.territory.push(coord);
                    Some((nation.id, nation.name.clone()))
                }
            };

            if let Some((nation_id, nation_name)) = expanded {
                self.push_history(
                    HistoryEventKind::BorderExpanded,
                    Some(nation_id),
                    format!(
                        "{nation_name} expanded into unclaimed land at {},{}.",
                        coord.x, coord.y
                    ),
                );
            }
        }
    }

    fn ensure_chunk_loaded(&mut self, coord: TileCoord) {
        let chunk_coord = coord.chunk();
        if self.chunks.iter().any(|chunk| chunk.coord == chunk_coord) {
            return;
        }
        self.chunks.push(self.generator.generate_chunk(chunk_coord));
    }

    fn push_history(
        &mut self,
        kind: HistoryEventKind,
        nation_id: Option<NationId>,
        summary: String,
    ) {
        self.history.push(HistoryEvent {
            id: HistoryEventId::new(self.history_ids.allocate_raw()),
            tick: self.tick,
            kind,
            nation_id,
            summary,
        });
    }
}

fn first_unclaimed_neighbor(territory: &[TileCoord], claimed: &[TileCoord]) -> Option<TileCoord> {
    territory
        .iter()
        .flat_map(|coord| coord.cardinal_neighbors())
        .find(|coord| !claimed.contains(coord))
}

fn generate_nation_name(source: &ActivitySource, focus: NationFocus) -> String {
    let root = source
        .project
        .as_deref()
        .or_else(|| source.title.split_whitespace().next())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Thread");

    let suffix = match focus {
        NationFocus::TechnologyAndEngineering => "Compact",
        NationFocus::CulturalProduction => "Dreamlands",
        NationFocus::ScientificResearch => "Observatory",
        NationFocus::ImportExportEconomy => "Exchange",
        NationFocus::BureaucracyAndLaw => "Chancery",
        NationFocus::SurvivalistProduction => "Hearthlands",
        NationFocus::MilitaryIndustry => "Forgeguard",
        NationFocus::ReligiousAndArtisticCulture => "Sanctum",
    };

    format!("{} {}", clean_name_root(root), suffix)
}

fn clean_name_root(root: &str) -> String {
    let cleaned: String = root
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(18)
        .collect();

    if cleaned.is_empty() {
        "Thread".to_owned()
    } else {
        cleaned
    }
}

fn name_core(name: &str) -> &str {
    name.split_whitespace().next().unwrap_or("Thread")
}

#[cfg(test)]
mod tests {
    use threadnations_analysis::{ActivityMetrics, ActivityProvider};
    use threadnations_common::ActivitySourceId;

    use super::*;

    fn activity(id: u64, title: &str, tokens: u64) -> ActivitySource {
        ActivitySource {
            id: ActivitySourceId::new(id),
            provider: ActivityProvider::Codex,
            title: title.to_owned(),
            project: Some("ThreadNations".to_owned()),
            summary: "Rust engine crate simulation worldgen code".to_owned(),
            metrics: ActivityMetrics {
                tokens,
                duration_minutes: 45,
                code_output_units: 20,
                thread_age_days: 10,
            },
        }
    }

    #[test]
    fn nation_spawns_in_unclaimed_viable_land() {
        let mut world = World::new(12);
        let source = activity(1, "Rust simulation engine", 8_000);

        let nation_id = world.spawn_nation_from_activity(&source).unwrap();
        let nation = world.nation(nation_id).unwrap();

        assert_eq!(world.nations.len(), 1);
        assert_eq!(nation.territory.len(), 1);
        assert!(world
            .history
            .iter()
            .any(|event| event.kind == HistoryEventKind::NationFounded));
    }

    #[test]
    fn nations_grow_from_usage_input() {
        let mut world = World::new(12);
        let source = activity(1, "Rust simulation engine", 8_000);
        let nation_id = world.spawn_nation_from_activity(&source).unwrap();
        let before = world.nation(nation_id).unwrap().population.citizens;

        world.advance_one_tick(&[activity(1, "Rust simulation engine", 20_000)]);

        let after = world.nation(nation_id).unwrap().population.citizens;
        assert!(after > before);
    }

    #[test]
    fn border_expands_when_population_exceeds_capacity() {
        let mut world = World::new(12);
        let source = activity(1, "Rust simulation engine", 40_000);
        let nation_id = world.spawn_nation_from_activity(&source).unwrap();

        world.advance_one_tick(&[source]);

        let nation = world.nation(nation_id).unwrap();
        assert!(nation.territory.len() >= 2);
    }
}
