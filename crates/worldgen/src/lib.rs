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
    /// Flat grasslands suitable for early settlement.
    Plains,
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
            Self::FoodCrops | Self::Livestock => ResourceCategory::FoodAndAgriculture,
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
            TerrainType::Water | TerrainType::Mountains | TerrainType::Desert
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
        let terrain = terrain_from_roll(rng.range_u64(100), coord);
        let biome = biome_from_roll(rng.range_u64(100), coord);
        let resources = resources_for_tile(coord, terrain, biome, &mut rng);

        Tile {
            coord,
            terrain,
            biome,
            resources,
        }
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

fn terrain_from_roll(roll: u64, coord: TileCoord) -> TerrainType {
    if coord.x == 0 || coord.y == 0 || (coord.x + coord.y).rem_euclid(29) == 0 {
        return TerrainType::Wetlands;
    }

    match roll {
        0..=10 => TerrainType::Water,
        11..=38 => TerrainType::Plains,
        39..=58 => TerrainType::Forest,
        59..=73 => TerrainType::Hills,
        74..=84 => TerrainType::Mountains,
        85..=92 => TerrainType::Desert,
        _ => TerrainType::Wetlands,
    }
}

fn biome_from_roll(roll: u64, coord: TileCoord) -> Biome {
    let latitude_bias = coord.y.unsigned_abs() % 100;
    match roll.saturating_add(u64::from(latitude_bias / 5)) {
        0..=24 => Biome::Temperate,
        25..=42 => Biome::Boreal,
        43..=61 => Biome::Tropical,
        62..=82 => Biome::Arid,
        _ => Biome::Tundra,
    }
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
        TerrainType::Wetlands | TerrainType::Plains | TerrainType::Forest
    ) {
        maybe_push(&mut resources, coord, ResourceKind::FreshWater, 65, rng);
        maybe_push(&mut resources, coord, ResourceKind::FertileSoil, 55, rng);
    }

    match terrain {
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
        TerrainType::Plains => maybe_push(&mut resources, coord, ResourceKind::Livestock, 30, rng),
        TerrainType::Water => {}
    }

    if matches!(biome, Biome::Tropical | Biome::Temperate) {
        maybe_push(&mut resources, coord, ResourceKind::FoodCrops, 20, rng);
    }

    resources
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
    fn spawn_site_is_viable_and_unclaimed() {
        let generator = WorldGenerator::new(7);
        let spawn = generator
            .find_spawn_site(|coord| coord == TileCoord::new(0, 0), 2)
            .unwrap();

        assert!(spawn.is_viable_spawn());
        assert_ne!(spawn.coord, TileCoord::new(0, 0));
    }
}
