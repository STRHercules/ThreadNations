//! Shared primitives for the ThreadNations Rust engine.

use std::fmt::{Display, Formatter};

/// Size of one square map chunk in tiles.
pub const CHUNK_SIZE: i32 = 16;

/// Result alias for engine operations.
pub type EngineResult<T> = Result<T, EngineError>;

/// Shared engine errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineError {
    /// A requested entity could not be found.
    MissingEntity { entity: &'static str, id: u64 },
    /// Worldgen could not find viable unclaimed land.
    NoViableSpawnSite,
    /// Input failed validation.
    InvalidInput(String),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity { entity, id } => write!(f, "missing {entity} with id {id}"),
            Self::NoViableSpawnSite => write!(f, "no viable unclaimed spawn site found"),
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
        }
    }
}

impl std::error::Error for EngineError {}

macro_rules! stable_id {
    ($name:ident) => {
        #[doc = concat!("Stable ID for ", stringify!($name), ".")]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u64);

        impl $name {
            /// Creates an ID from a raw value.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the raw value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

stable_id!(ActivitySourceId);
stable_id!(NationId);
stable_id!(SettlementId);
stable_id!(ResourceDepositId);
stable_id!(HistoryEventId);
stable_id!(TradeRouteId);
stable_id!(ConflictId);
stable_id!(ReligionId);
stable_id!(DoctrineId);
stable_id!(MapChunkId);

/// Monotonic stable ID allocator for a world save.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdAllocator {
    next: u64,
}

impl IdAllocator {
    /// Creates an allocator starting at one.
    #[must_use]
    pub const fn new() -> Self {
        Self { next: 1 }
    }

    /// Allocates the next raw ID value.
    pub fn allocate_raw(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunk-space coordinate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkCoord {
    /// Chunk x coordinate.
    pub x: i32,
    /// Chunk y coordinate.
    pub y: i32,
}

impl ChunkCoord {
    /// Creates a chunk coordinate.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Tile-space coordinate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TileCoord {
    /// Tile x coordinate.
    pub x: i32,
    /// Tile y coordinate.
    pub y: i32,
}

impl TileCoord {
    /// Creates a tile coordinate.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Returns the owning chunk coordinate.
    #[must_use]
    pub fn chunk(self) -> ChunkCoord {
        ChunkCoord {
            x: self.x.div_euclid(CHUNK_SIZE),
            y: self.y.div_euclid(CHUNK_SIZE),
        }
    }

    /// Returns cardinal neighbors for basic border growth.
    #[must_use]
    pub const fn cardinal_neighbors(self) -> [Self; 4] {
        [
            Self::new(self.x + 1, self.y),
            Self::new(self.x - 1, self.y),
            Self::new(self.x, self.y + 1),
            Self::new(self.x, self.y - 1),
        ]
    }
}

/// Lightweight deterministic RNG based on SplitMix64.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    /// Creates a deterministic RNG from a seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next pseudo-random u64.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a number in `0..exclusive_max`.
    pub fn range_u64(&mut self, exclusive_max: u64) -> u64 {
        if exclusive_max == 0 {
            0
        } else {
            self.next_u64() % exclusive_max
        }
    }
}

/// Monotonic world simulation tick.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorldTick(pub u64);

impl WorldTick {
    /// Returns the next tick.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
