//! Deterministic, input-free civilization simulation.

use serde::{Deserialize, Serialize};
use threadnations_common::{
    ChunkCoord, ConflictId, HistoryEventId, IdAllocator, NationId, SettlementId, TileCoord,
    TradeRouteId, WorldTick,
};
use threadnations_worldgen::{ResourceKind, TerrainType, Tile, WorldGenerator};

const HISTORY_LIMIT: usize = 240;
const RESERVOIR_CAP: u32 = 20_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Government {
    Council,
    Republic,
    Monarchy,
    Directorate,
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
    pub government: Government,
    pub capital: SettlementId,
    pub territory: Vec<TileCoord>,
    pub settlements: Vec<Settlement>,
    pub population: u64,
    pub food: u32,
    pub materials: u32,
    pub wealth: u32,
    pub stability: u8,
    pub military: u32,
    pub research: u32,
    pub opportunity: u32,
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
    nation_ids: IdAllocator,
    settlement_ids: IdAllocator,
    trade_ids: IdAllocator,
    conflict_ids: IdAllocator,
    history_ids: IdAllocator,
}

impl World {
    #[must_use]
    pub fn new_demo(seed: u64, initial_nations: u8) -> Self {
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
            nation_ids: IdAllocator::new(),
            settlement_ids: IdAllocator::new(),
            trade_ids: IdAllocator::new(),
            conflict_ids: IdAllocator::new(),
            history_ids: IdAllocator::new(),
        };
        for index in 0..initial_nations.clamp(3, 6) {
            world.found_nation(index);
        }
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
        self.apply_conflict_losses();
        self.move_trade();
        if self.tick.0.is_multiple_of(12) {
            self.expand_borders();
        }
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

    fn found_nation(&mut self, index: u8) {
        let tile = self.find_spawn_tile();
        self.ensure_chunk(tile.coord);
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
            government,
            capital,
            territory: vec![tile.coord],
            settlements: vec![settlement],
            population: 1_800,
            food: 700,
            materials: 120,
            wealth: 100,
            stability: 62,
            military: 45,
            research: 0,
            opportunity: 0,
        });
        self.event(
            EventKind::Founded,
            Some(id),
            format!("{name} established a settlement."),
        );
    }

    fn find_spawn_tile(&self) -> Tile {
        let generator = self.generator();
        for radius in 0..=5 {
            for x in -radius..=radius {
                for y in [-radius, radius] {
                    let chunk = generator.generate_chunk(ChunkCoord::new(x, y));
                    if let Some(tile) = chunk
                        .tiles
                        .into_iter()
                        .find(|tile| tile.is_viable_spawn() && self.is_distant(tile.coord))
                    {
                        return tile;
                    }
                }
            }
            for y in (-radius + 1)..radius {
                for x in [-radius, radius] {
                    let chunk = generator.generate_chunk(ChunkCoord::new(x, y));
                    if let Some(tile) = chunk
                        .tiles
                        .into_iter()
                        .find(|tile| tile.is_viable_spawn() && self.is_distant(tile.coord))
                    {
                        return tile;
                    }
                }
            }
        }
        let nation_count = i32::try_from(self.nations.len()).unwrap_or(i32::MAX / 12);
        generator.generate_tile(TileCoord::new(1 + nation_count * 12, 1))
    }

    fn is_distant(&self, tile: TileCoord) -> bool {
        self.nations.iter().all(|nation| {
            nation
                .territory
                .iter()
                .all(|claimed| manhattan(*claimed, tile) >= 12)
        })
    }

    fn allocate_opportunity(&mut self) -> Vec<u32> {
        let spend = self.activity_reservoir.min(100);
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

    fn run_economy(&mut self, allocations: &[u32]) {
        let conditions: Vec<(u32, u32)> = self
            .nations
            .iter()
            .map(|nation| self.local_conditions(nation))
            .collect();
        let mut events = Vec::new();
        for (index, nation) in self.nations.iter_mut().enumerate() {
            let opportunity = allocations.get(index).copied().unwrap_or_default();
            let (fertility, minerals) = conditions[index];
            nation.opportunity = nation.opportunity.saturating_add(opportunity);
            let produce_food =
                6 + fertility * 3 + opportunity / if fertility >= minerals { 4 } else { 12 };
            let consume = u32::try_from(nation.population / 350)
                .unwrap_or(u32::MAX)
                .saturating_add(4);
            nation.food = nation.food.saturating_add(produce_food);
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
                nation.population = nation.population.saturating_add(
                    (nation.population / 1_000).max(1) + u64::from(opportunity / 10),
                );
                nation.stability = nation.stability.saturating_add(1).min(100);
            }
            nation.materials = nation.materials.saturating_add(
                minerals * 2 + opportunity / if minerals > fertility { 3 } else { 12 },
            );
            nation.wealth = nation.wealth.saturating_add((nation.materials / 80).max(1));
            nation.research = nation
                .research
                .saturating_add(opportunity / 3 + nation.wealth / 500);
            nation.military = nation.military.saturating_add(nation.materials / 300);
            if let Some(capital) = nation
                .settlements
                .iter_mut()
                .find(|settlement| settlement.id == nation.capital)
            {
                capital.population = nation.population;
                capital.infrastructure = capital
                    .infrastructure
                    .saturating_add(u16::try_from(opportunity / 25).unwrap_or(u16::MAX));
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

    fn local_conditions(&self, nation: &Nation) -> (u32, u32) {
        nation
            .territory
            .iter()
            .fold((0, 0), |(fertility, minerals), coord| {
                let tile = self.tile(*coord);
                let fertile = u32::from(
                    tile.has_resource(ResourceKind::FertileSoil)
                        || tile.has_resource(ResourceKind::FreshWater),
                );
                let metal = u32::from(
                    tile.has_resource(ResourceKind::Iron)
                        || tile.has_resource(ResourceKind::Coal)
                        || matches!(tile.terrain, TerrainType::Hills | TerrainType::Mountains),
                );
                (fertility + fertile, minerals + metal)
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
                    self.trade_routes.push(route);
                    return;
                }
            }
        }
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
        let claimed: Vec<TileCoord> = self
            .nations
            .iter()
            .flat_map(|nation| nation.territory.iter().copied())
            .collect();
        let mut expansions = Vec::new();
        for (index, nation) in self.nations.iter().enumerate() {
            if nation.population < nation.territory.len() as u64 * 700 || nation.stability < 35 {
                continue;
            }
            if let Some(coord) = nation
                .territory
                .iter()
                .flat_map(|coord| coord.cardinal_neighbors())
                .find(|coord| !claimed.contains(coord) && self.tile(*coord).is_viable_spawn())
            {
                expansions.push((index, coord));
            }
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

fn nation_name(seed: u64, index: usize) -> String {
    const START: [&str; 8] = [
        "Aster", "Belen", "Corin", "Daran", "Elden", "Faryn", "Galen", "Haran",
    ];
    const END: [&str; 8] = [
        "reach", "vale", "mark", "hold", "coast", "mere", "crest", "ward",
    ];
    let shift = usize::try_from(seed % u64::try_from(START.len()).unwrap_or(1)).unwrap_or(0);
    format!(
        "{}{}",
        START[(index + shift) % START.len()],
        END[(index * 3 + shift) % END.len()]
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
    fn world_advances_without_activity() {
        let mut world = World::new_demo(12, 4);
        let before = world.tick;
        world.advance(0);
        assert!(world.tick > before);
    }

    #[test]
    fn long_run_remains_valid() {
        let mut world = World::new_demo(12, 4);
        for _ in 0..10_000 {
            world.advance(8);
        }
        assert!(world.validate());
        assert!(!world.history.is_empty());
    }
}
