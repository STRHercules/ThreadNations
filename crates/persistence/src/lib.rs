//! `SQLite` snapshots for persistent autonomous worlds.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use threadnations_simulation::World;

pub const SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HistoricalImportState {
    pub answered: bool,
    pub imported_records: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedWorld {
    pub world: World,
    pub activity_offset: u64,
    pub window: WindowState,
    #[serde(default)]
    pub historical_import: HistoricalImportState,
}

/// Loads latest saved world snapshot from `path`.
///
/// # Errors
///
/// Returns database or snapshot-deserialization errors.
pub fn load(path: &Path) -> Result<Option<SavedWorld>, PersistenceError> {
    let connection = open(path)?;
    let payload = connection
        .query_row("SELECT payload FROM world_state WHERE id = 1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    payload
        .map(|value| serde_json::from_str(&value).map_err(PersistenceError::from))
        .transpose()
}

/// Saves `state` in one transaction at `path`.
///
/// # Errors
///
/// Returns filesystem, database, or serialization errors without replacing the old snapshot.
pub fn save(path: &Path, state: &SavedWorld) -> Result<(), PersistenceError> {
    let connection = open(path)?;
    let payload = serde_json::to_string(state)?;
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO world_state (id, schema_version, payload) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET schema_version = excluded.schema_version, payload = excluded.payload",
        params![SCHEMA_VERSION, payload],
    )?;
    transaction.commit()?;
    Ok(())
}

fn open(path: &Path) -> Result<Connection, PersistenceError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY);
         CREATE TABLE IF NOT EXISTS world_state (
             id INTEGER PRIMARY KEY CHECK (id = 1),
             schema_version INTEGER NOT NULL,
             payload TEXT NOT NULL
         );
         INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);",
    )?;
    Ok(connection)
}

#[derive(Debug)]
pub enum PersistenceError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
}

impl From<std::io::Error> for PersistenceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<rusqlite::Error> for PersistenceError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}
impl From<serde_json::Error> for PersistenceError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PersistenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_world() {
        let path =
            std::env::temp_dir().join(format!("threadnations-{}.sqlite", std::process::id()));
        let state = SavedWorld {
            world: World::new_demo(7, 4),
            activity_offset: 9,
            window: WindowState::default(),
            historical_import: HistoricalImportState::default(),
        };
        save(&path, &state).unwrap();
        let restored = load(&path).unwrap().unwrap();
        assert_eq!(restored.world, state.world);
        let _ = std::fs::remove_file(path);
    }
}
