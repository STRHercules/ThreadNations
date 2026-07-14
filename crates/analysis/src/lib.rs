//! Metadata-only activity ingestion. Content never enters this crate.

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};

/// Metadata origin retained only for diagnostics. It never affects points.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivitySourceKind {
    Codex,
    ChatGpt,
    Claude,
    LocalEditor,
    Synthetic,
}

/// Strictly numeric activity record accepted from local JSONL.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityRecord {
    pub occurred_at_unix_ms: i64,
    pub source: ActivitySourceKind,
    pub active_seconds: u64,
    pub interaction_count: u32,
    pub token_estimate: u64,
    pub generated_bytes: u64,
}

/// Converts numeric metrics into bounded neutral world pressure.
#[must_use]
pub fn activity_points(record: &ActivityRecord) -> u32 {
    let duration = (record.active_seconds / 60).min(120);
    let interactions = u64::from(record.interaction_count).min(80) * 3;
    let tokens = integer_sqrt(record.token_estimate / 100).min(180);
    let bytes = integer_sqrt(record.generated_bytes / 128).min(120);
    (duration + interactions + tokens + bytes).min(400) as u32
}

/// Reads completed JSONL lines after `offset`; leaves a partial last line untouched.
///
/// # Errors
///
/// Returns filesystem read errors for an existing inbox file.
pub fn read_jsonl(path: &Path, offset: u64) -> std::io::Result<ActivityRead> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ActivityRead::default())
        }
        Err(error) => return Err(error),
    };
    let start = usize::try_from(offset)
        .ok()
        .filter(|offset| *offset <= bytes.len())
        .unwrap_or(bytes.len());
    let unread = &bytes[start..];
    let Some(last_newline) = unread.iter().rposition(|byte| *byte == b'\n') else {
        return Ok(ActivityRead::default());
    };
    let consumed = &unread[..=last_newline];
    let mut records = Vec::new();
    let mut rejected = 0;
    for line in consumed
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        match serde_json::from_slice(line) {
            Ok(record) => records.push(record),
            Err(_) => rejected += 1,
        }
    }
    Ok(ActivityRead {
        records,
        next_offset: u64::try_from(start + consumed.len()).unwrap_or(u64::MAX),
        rejected,
    })
}

#[derive(Debug, Default)]
pub struct ActivityRead {
    pub records: Vec<ActivityRecord>,
    pub next_offset: u64,
    pub rejected: usize,
}

/// Builds neutral records from every local Codex session file beneath `root`.
/// File contents, names, and paths are never read or retained.
///
/// # Errors
///
/// Returns directory-enumeration or metadata errors.
pub fn import_codex_session_metadata(root: &Path) -> std::io::Result<Vec<ActivityRecord>> {
    let mut files = Vec::new();
    collect_jsonl_files(root, &mut files)?;
    files.sort();
    let mut records = Vec::with_capacity(files.len());
    for path in files {
        let metadata = fs::metadata(path)?;
        let occurred_at_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or_default();
        records.push(ActivityRecord {
            occurred_at_unix_ms,
            source: ActivitySourceKind::Codex,
            active_seconds: (metadata.len() / 1_024).clamp(60, 7_200),
            interaction_count: 1,
            token_estimate: 0,
            generated_bytes: metadata.len(),
        });
    }
    records.sort_by_key(|record| record.occurred_at_unix_ms);
    Ok(records)
}

fn collect_jsonl_files(root: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_jsonl_files(&entry.path(), files)?;
        } else if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "jsonl")
        {
            files.push(entry.path());
        }
    }
    Ok(())
}

fn integer_sqrt(value: u64) -> u64 {
    let mut low = 0;
    let mut high = value.min(4_294_967_295);
    while low < high {
        let middle = (low + high).div_ceil(2);
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(source: ActivitySourceKind) -> ActivityRecord {
        ActivityRecord {
            occurred_at_unix_ms: 1,
            source,
            active_seconds: 900,
            interaction_count: 8,
            token_estimate: 12_000,
            generated_bytes: 24_000,
        }
    }

    #[test]
    fn equal_metrics_are_source_neutral() {
        assert_eq!(
            activity_points(&record(ActivitySourceKind::Codex)),
            activity_points(&record(ActivitySourceKind::LocalEditor))
        );
    }

    #[test]
    fn partial_line_is_not_consumed() {
        let path = std::env::temp_dir().join(format!(
            "threadnations-activity-{}.jsonl",
            std::process::id()
        ));
        fs::write(&path, b"{\"occurred_at_unix_ms\":1,\"source\":\"codex\",\"active_seconds\":1,\"interaction_count\":0,\"token_estimate\":0,\"generated_bytes\":0}\n{").unwrap();
        let result = read_jsonl(&path, 0).unwrap();
        assert_eq!(result.records.len(), 1);
        assert!(result.next_offset > 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn historical_import_uses_file_metadata_only() {
        let root =
            std::env::temp_dir().join(format!("threadnations-history-{}", std::process::id()));
        let nested = root.join("2026");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("session.jsonl"), b"private conversation text").unwrap();
        let records = import_codex_session_metadata(&root).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, ActivitySourceKind::Codex);
        assert!(records[0].generated_bytes > 0);
        let _ = fs::remove_dir_all(root);
    }
}
