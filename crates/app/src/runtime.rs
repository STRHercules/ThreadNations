use std::{
    env,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub const INITIAL_NATIONS: u8 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub world_seed: u64,
    pub calendar_step_ms: u64,
    pub autosave_seconds: u64,
    pub synthetic_activity: bool,
    pub synthetic_interval_ticks: u64,
    pub jsonl_inbox: String,
    pub window_width: f32,
    pub window_height: f32,
    pub borderless: bool,
    pub always_on_top: bool,
    pub max_catchup_ticks: u32,
    pub history_retention_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            world_seed: random_world_seed(),
            calendar_step_ms: 60_000,
            autosave_seconds: 30,
            synthetic_activity: true,
            synthetic_interval_ticks: 4,
            jsonl_inbox: application_data_dir()
                .join("activity-inbox.jsonl")
                .to_string_lossy()
                .into_owned(),
            window_width: 1_240.0,
            window_height: 760.0,
            borderless: false,
            always_on_top: false,
            max_catchup_ticks: 8,
            history_retention_limit: 2_000,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = config_path();
        migrate_legacy_file("config.toml", &path);
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::write(
                    &path,
                    toml::to_string_pretty(&config).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                Ok(config)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::write(
            config_path(),
            toml::to_string_pretty(self).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}

fn application_data_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA").map_or_else(
        || PathBuf::from("data"),
        |path| PathBuf::from(path).join("ThreadNations"),
    )
}

fn config_path() -> PathBuf {
    application_data_dir().join("config.toml")
}

pub fn random_world_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
        });
    now ^ now.rotate_left(23) ^ u64::from(std::process::id())
}

pub fn world_database_path() -> PathBuf {
    let path = application_data_dir().join("threadnations.sqlite");
    migrate_legacy_file("threadnations.sqlite", &path);
    path
}

fn migrate_legacy_file(name: &str, destination: &Path) {
    if destination.exists() {
        return;
    }
    let Some(root) = env::current_exe().ok().and_then(|path| {
        path.ancestors()
            .find(|ancestor| ancestor.join("Cargo.toml").exists())
            .map(Path::to_path_buf)
    }) else {
        return;
    };
    let source = root.join("data").join(name);
    if source.exists() {
        if let Some(parent) = destination.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(source, destination);
    }
}
