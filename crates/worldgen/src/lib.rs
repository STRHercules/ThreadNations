//! Deterministic infinite world generation for `ThreadNations`.

use serde::{Deserialize, Serialize};
use threadnations_common::{
    ChunkCoord, DeterministicRng, EngineError, EngineResult, ResourceDepositId, TileCoord,
    CHUNK_SIZE,
};

/// Terrain class for a map tile.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum TerrainType {
    /// Open water or coast water.
    Water,
    /// Large bodies of open water with abundant fish.
    Ocean,
    /// Smaller inland bodies of water with moderate fish.
    Lake,
    /// Long freshwater channels that join the larger water system.
    River,
    /// Sandy shoreline between land and ocean or lake water.
    Beach,
    /// Flat grasslands suitable for early settlement.
    Plains,
    /// Dark-grass open country.
    Grassland,
    /// Light-grass open country with sparse trees.
    Prairie,
    /// Forested land rich in wood.
    Forest,
    /// Hills with common ores.
    Hills,
    /// Mountains with rare ores and defensive value.
    Mountains,
    /// Dry land with sparse water.
    Desert,
    /// Wetlands and river deltas.
    Wetlands,
}

/// Climate and biome layer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Biome {
    /// Temperate mixed climate.
    Temperate,
    /// Cold climate.
    Boreal,
    /// Hot wet climate.
    Tropical,
    /// Hot dry climate.
    Arid,
    /// Cold snowy climate.
    Tundra,
}

/// Resource category matching the README and AGENTS guidance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ResourceCategory {
    /// Natural materials.
    NaturalMaterial,
    /// Refined materials.
    RefinedMaterial,
    /// Food and agriculture.
    FoodAndAgriculture,
    /// Industrial goods.
    IndustrialGood,
    /// Abstract military goods only.
    AbstractMilitaryGood,
    /// Social and regulated goods as abstract variables.
    SocialRegulatedGood,
    /// High-technology goods.
    HighTechnologyGood,
}

/// Concrete resources in the first simulation pass.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ResourceKind {
    /// Wood.
    Wood,
    /// Stone.
    Stone,
    /// Coal.
    Coal,
    /// Iron.
    Iron,
    /// Copper.
    Copper,
    /// Aluminum.
    Aluminum,
    /// Sand.
    Sand,
    /// Crystal.
    Crystal,
    /// Fresh water.
    FreshWater,
    /// Fertile soil.
    FertileSoil,
    /// Food crops.
    FoodCrops,
    /// Livestock.
    Livestock,
    /// Fish caught from ocean, lake, and river tiles.
    Fish,
    /// Abstract ammunition stockpile.
    Ammunition,
    /// Abstract firearms industry variable.
    Firearms,
    /// Abstract explosives industry variable.
    Explosives,
    /// Electronics.
    Electronics,
    /// Computers.
    Computers,
    /// Microprocessors.
    Microprocessors,
    /// Medicine.
    Medicine,
    /// Alcohol.
    Alcohol,
    /// Narcotic-like goods.
    NarcoticLikeGoods,
}

impl ResourceKind {
    /// Returns the category for this resource.
    #[must_use]
    pub const fn category(self) -> ResourceCategory {
        match self {
            Self::Wood
            | Self::Stone
            | Self::Coal
            | Self::Iron
            | Self::Copper
            | Self::Aluminum
            | Self::Sand
            | Self::Crystal
            | Self::FreshWater
            | Self::FertileSoil => ResourceCategory::NaturalMaterial,
            Self::FoodCrops | Self::Livestock | Self::Fish => ResourceCategory::FoodAndAgriculture,
            Self::Ammunition | Self::Firearms | Self::Explosives => {
                ResourceCategory::AbstractMilitaryGood
            }
            Self::Electronics | Self::Computers | Self::Microprocessors => {
                ResourceCategory::HighTechnologyGood
            }
            Self::Medicine | Self::Alcohol | Self::NarcoticLikeGoods => {
                ResourceCategory::SocialRegulatedGood
            }
        }
    }
}

/// A natural resource deposit attached to a tile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceDeposit {
    /// Stable resource deposit ID.
    pub id: ResourceDepositId,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Tile where this resource appears.
    pub tile: TileCoord,
    /// Abundance from 1 to 100.
    pub abundance: u8,
}

/// A generated tile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tile {
    /// World tile coordinate.
    pub coord: TileCoord,
    /// Terrain type.
    pub terrain: TerrainType,
    /// Biome type.
    pub biome: Biome,
    /// Resource deposits on this tile.
    pub resources: Vec<ResourceDeposit>,
}

impl Tile {
    /// Returns true when the tile can host a first settlement.
    #[must_use]
    pub fn is_viable_spawn(&self) -> bool {
        if matches!(
            self.terrain,
            TerrainType::Water
                | TerrainType::Ocean
                | TerrainType::Lake
                | TerrainType::River
                | TerrainType::Beach
                | TerrainType::Mountains
                | TerrainType::Desert
        ) {
            return false;
        }

        self.has_resource(ResourceKind::FreshWater)
            || self.has_resource(ResourceKind::FertileSoil)
            || matches!(self.terrain, TerrainType::Wetlands)
    }

    /// Returns true when a tile contains the requested resource.
    #[must_use]
    pub fn has_resource(&self, resource: ResourceKind) -> bool {
        self.resources
            .iter()
            .any(|deposit| deposit.kind == resource)
    }
}

/// A square chunk of generated terrain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MapChunk {
    /// Chunk coordinate.
    pub coord: ChunkCoord,
    /// Tiles in row-major order.
    pub tiles: Vec<Tile>,
}

/// Deterministic world generator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldGenerator {
    seed: u64,
}

impl WorldGenerator {
    /// Creates a generator from a world seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Generates a chunk deterministically.
    #[must_use]
    pub fn generate_chunk(&self, coord: ChunkCoord) -> MapChunk {
        let mut tiles = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
        for local_y in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let tile_coord = TileCoord::new(
                    coord.x * CHUNK_SIZE + local_x,
                    coord.y * CHUNK_SIZE + local_y,
                );
                tiles.push(self.generate_tile(tile_coord));
            }
        }

        MapChunk { coord, tiles }
    }

    /// Generates a single tile deterministically.
    #[must_use]
    pub fn generate_tile(&self, coord: TileCoord) -> Tile {
        let mut rng = DeterministicRng::new(hash_coord(self.seed, coord));
        let (terrain, biome) = self.terrain_at(coord);
        let resources = resources_for_tile(coord, terrain, biome, &mut rng);

        Tile {
            coord,
            terrain,
            biome,
            resources,
        }
    }

    /// Returns terrain layers without allocating resource deposits.
    #[must_use]
    pub fn terrain_at(&self, coord: TileCoord) -> (TerrainType, Biome) {
        (terrain_for(self.seed, coord), biome_for(self.seed, coord))
    }

    /// Finds a viable unclaimed spawn site, expanding chunk rings as needed.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NoViableSpawnSite`] when no viable tile exists within the limit.
    pub fn find_spawn_site<F>(
        &self,
        mut is_claimed: F,
        max_radius_chunks: i32,
    ) -> EngineResult<Tile>
    where
        F: FnMut(TileCoord) -> bool,
    {
        for radius in 0..=max_radius_chunks {
            for chunk_coord in chunk_ring(radius) {
                let chunk = self.generate_chunk(chunk_coord);
                for tile in chunk.tiles {
                    if tile.is_viable_spawn() && !is_claimed(tile.coord) {
                        return Ok(tile);
                    }
                }
            }
        }

        Err(EngineError::NoViableSpawnSite)
    }
}

fn terrain_for(seed: u64, coord: TileCoord) -> TerrainType {
    match terrain_water(seed, coord) {
        TerrainType::Ocean | TerrainType::Lake => return terrain_water(seed, coord),
        _ => {}
    }
    if is_river(seed, coord) {
        return TerrainType::River;
    }
    if coord.cardinal_neighbors().iter().any(|neighbor| {
        matches!(
            terrain_water(seed, *neighbor),
            TerrainType::Ocean | TerrainType::Lake
        )
    }) {
        return TerrainType::Beach;
    }

    match layered_noise(seed ^ 0x71E2_A1A5, coord, 48) {
        0..=24 => TerrainType::Desert,
        25..=42 => TerrainType::Forest,
        43..=57 => TerrainType::Hills,
        58..=67 => TerrainType::Mountains,
        68..=83 => TerrainType::Grassland,
        _ => TerrainType::Prairie,
    }
}

fn terrain_water(seed: u64, coord: TileCoord) -> TerrainType {
    match layered_noise(seed ^ 0xB0D1_E500, coord, 96) {
        0..=15 => TerrainType::Ocean,
        16..=22 => TerrainType::Lake,
        _ => TerrainType::Plains,
    }
}

fn is_river(seed: u64, coord: TileCoord) -> bool {
    let horizontal = i64::try_from(layered_noise(
        seed ^ 0xC0FF_EE11,
        TileCoord::new(coord.x, 0),
        96,
    ))
    .unwrap_or_default()
        - 50;
    let vertical = i64::try_from(layered_noise(
        seed ^ 0xA11C_E551,
        TileCoord::new(0, coord.y),
        96,
    ))
    .unwrap_or_default()
        - 50;
    i64::from(coord.y).saturating_sub(horizontal).unsigned_abs() <= 1
        || i64::from(coord.x).saturating_sub(vertical).unsigned_abs() <= 1
}

fn biome_for(seed: u64, coord: TileCoord) -> Biome {
    let roll = smooth_noise(seed ^ 0xB10B_E500, coord, 128);
    let latitude_bias = coord.y.unsigned_abs() % 100;
    match roll.saturating_add(u64::from(latitude_bias / 5)) {
        0..=24 => Biome::Temperate,
        25..=42 => Biome::Boreal,
        43..=61 => Biome::Tropical,
        62..=82 => Biome::Arid,
        _ => Biome::Tundra,
    }
}

fn smooth_noise(seed: u64, coord: TileCoord, scale: i32) -> u64 {
    let x = coord.x.div_euclid(scale);
    let y = coord.y.div_euclid(scale);
    let local_x = u64::try_from(coord.x.rem_euclid(scale)).unwrap_or_default();
    let local_y = u64::try_from(coord.y.rem_euclid(scale)).unwrap_or_default();
    let scale = u64::try_from(scale).unwrap_or(1);
    let value = |x, y| hash_coord(seed, TileCoord::new(x, y)) % 101;
    let top = value(x, y).saturating_mul(scale - local_x) + value(x + 1, y).saturating_mul(local_x);
    let bottom = value(x, y + 1).saturating_mul(scale - local_x)
        + value(x + 1, y + 1).saturating_mul(local_x);
    (top.saturating_mul(scale - local_y) + bottom.saturating_mul(local_y))
        / scale.saturating_mul(scale)
}

fn layered_noise(seed: u64, coord: TileCoord, scale: i32) -> u64 {
    let broad = smooth_noise(seed, coord, scale);
    let detail = smooth_noise(seed ^ 0x6A09_E667_F3BC_C909, coord, (scale / 3).max(1));
    broad.saturating_mul(3).saturating_add(detail) / 4
}

fn resources_for_tile(
    coord: TileCoord,
    terrain: TerrainType,
    biome: Biome,
    rng: &mut DeterministicRng,
) -> Vec<ResourceDeposit> {
    let mut resources = Vec::new();

    if matches!(
        terrain,
        TerrainType::Wetlands
            | TerrainType::Plains
            | TerrainType::Grassland
            | TerrainType::Prairie
            | TerrainType::Forest
    ) {
        maybe_push(&mut resources, coord, ResourceKind::FreshWater, 65, rng);
        maybe_push(&mut resources, coord, ResourceKind::FertileSoil, 55, rng);
    }

    match terrain {
        TerrainType::Ocean => push_resource(&mut resources, coord, ResourceKind::Fish, 90),
        TerrainType::Lake => push_resource(&mut resources, coord, ResourceKind::Fish, 55),
        TerrainType::River => {
            push_resource(&mut resources, coord, ResourceKind::FreshWater, 100);
            push_resource(&mut resources, coord, ResourceKind::Fish, 20);
        }
        TerrainType::Beach => maybe_push(&mut resources, coord, ResourceKind::Sand, 85, rng),
        TerrainType::Forest => maybe_push(&mut resources, coord, ResourceKind::Wood, 90, rng),
        TerrainType::Hills => {
            maybe_push(&mut resources, coord, ResourceKind::Stone, 70, rng);
            maybe_push(&mut resources, coord, ResourceKind::Coal, 35, rng);
            maybe_push(&mut resources, coord, ResourceKind::Iron, 30, rng);
            maybe_push(&mut resources, coord, ResourceKind::Copper, 25, rng);
        }
        TerrainType::Mountains => {
            maybe_push(&mut resources, coord, ResourceKind::Stone, 95, rng);
            maybe_push(&mut resources, coord, ResourceKind::Iron, 45, rng);
            maybe_push(&mut resources, coord, ResourceKind::Aluminum, 20, rng);
            maybe_push(&mut resources, coord, ResourceKind::Crystal, 12, rng);
        }
        TerrainType::Desert => maybe_push(&mut resources, coord, ResourceKind::Sand, 95, rng),
        TerrainType::Wetlands => {
            maybe_push(&mut resources, coord, ResourceKind::FoodCrops, 35, rng);
        }
        TerrainType::Plains | TerrainType::Grassland | TerrainType::Prairie => {
            maybe_push(&mut resources, coord, ResourceKind::Livestock, 30, rng);
        }
        TerrainType::Water => {}
    }

    if matches!(biome, Biome::Tropical | Biome::Temperate) {
        maybe_push(&mut resources, coord, ResourceKind::FoodCrops, 20, rng);
    }

    resources
}

fn push_resource(
    resources: &mut Vec<ResourceDeposit>,
    coord: TileCoord,
    kind: ResourceKind,
    abundance: u8,
) {
    resources.push(ResourceDeposit {
        id: ResourceDepositId::new(hash_resource_id(coord, kind)),
        kind,
        tile: coord,
        abundance,
    });
}

fn maybe_push(
    resources: &mut Vec<ResourceDeposit>,
    coord: TileCoord,
    kind: ResourceKind,
    chance_percent: u64,
    rng: &mut DeterministicRng,
) {
    if rng.range_u64(100) < chance_percent {
        resources.push(ResourceDeposit {
            id: ResourceDepositId::new(hash_resource_id(coord, kind)),
            kind,
            tile: coord,
            abundance: u8::try_from(rng.range_u64(100).max(1)).unwrap_or(u8::MAX),
        });
    }
}

fn chunk_ring(radius: i32) -> Vec<ChunkCoord> {
    if radius == 0 {
        return vec![ChunkCoord::new(0, 0)];
    }

    let mut coords = Vec::new();
    for x in -radius..=radius {
        coords.push(ChunkCoord::new(x, -radius));
        coords.push(ChunkCoord::new(x, radius));
    }
    for y in (-radius + 1)..radius {
        coords.push(ChunkCoord::new(-radius, y));
        coords.push(ChunkCoord::new(radius, y));
    }
    coords
}

fn hash_coord(seed: u64, coord: TileCoord) -> u64 {
    let x = i64::from(coord.x).cast_unsigned();
    let y = i64::from(coord.y).cast_unsigned();
    seed ^ x.wrapping_mul(0xA24B_AED4_963E_E407) ^ y.wrapping_mul(0x9FB2_1C65_1E98_DF25)
}

fn hash_resource_id(coord: TileCoord, kind: ResourceKind) -> u64 {
    hash_coord(kind as u64 + 17, coord)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_creates_same_world_chunk() {
        let generator = WorldGenerator::new(42);
        let first = generator.generate_chunk(ChunkCoord::new(3, -2));
        let second = generator.generate_chunk(ChunkCoord::new(3, -2));

        assert_eq!(first, second);
    }

    #[test]
    fn terrain_only_lookup_matches_generated_tile() {
        let generator = WorldGenerator::new(42);
        let coord = TileCoord::new(17, -31);

        assert_eq!(generator.terrain_at(coord), {
            let tile = generator.generate_tile(coord);
            (tile.terrain, tile.biome)
        });
    }

    #[test]
    fn spawn_site_is_viable_and_unclaimed() {
        let generator = WorldGenerator::new(7);
        let spawn = generator
            .find_spawn_site(|coord| coord == TileCoord::new(0, 0), 2)
            .unwrap();

        assert!(spawn.is_viable_spawn());
        assert_ne!(spawn.coord, TileCoord::new(0, 0));
    }

    #[test]
    fn water_types_supply_expected_fish_abundance() {
        let coord = TileCoord::new(4, -9);
        let mut rng = DeterministicRng::new(12);
        let ocean = resources_for_tile(coord, TerrainType::Ocean, Biome::Temperate, &mut rng);
        let river = resources_for_tile(coord, TerrainType::River, Biome::Temperate, &mut rng);

        assert!(ocean
            .iter()
            .any(|resource| resource.kind == ResourceKind::Fish && resource.abundance == 90));
        assert!(river
            .iter()
            .any(|resource| resource.kind == ResourceKind::Fish && resource.abundance == 20));
    }
}
