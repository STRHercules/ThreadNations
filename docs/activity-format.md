# Activity inbox format

ThreadNations reads completed lines from `data/activity-inbox.jsonl`. Each line must be one JSON object:

```json
{"occurred_at_unix_ms":1784000000000,"source":"codex","active_seconds":600,"interaction_count":5,"token_estimate":8000,"generated_bytes":16000}
{"occurred_at_unix_ms":1784000600000,"source":"local_editor","active_seconds":1200,"interaction_count":0,"token_estimate":0,"generated_bytes":42000}
```

Allowed `source` values are `codex`, `chat_gpt`, `claude`, `local_editor`, and `synthetic`. Source does not alter point conversion. Unknown fields and malformed completed lines are rejected; a partial final line is left for a later read and does not advance the saved offset.

Forbidden fields include `topic`, `prompt`, `response`, `summary`, `content`, `project`, `filename`, repository names, and any freeform text. The typed record has no place to store them.

## First-launch historical import

On first launch, ThreadNations asks whether to import local Codex history. Accepting scans every `.jsonl` session file beneath `%CODEX_HOME%\sessions` (or `%USERPROFILE%\.codex\sessions` when `CODEX_HOME` is unset). It reads file metadata only: modification time and byte length. It does not open, parse, store, classify, or transmit conversation contents, titles, paths, prompts, or responses.

Each discovered session becomes one neutral activity record and is applied in chronological metadata order before normal simulation continues. The completed decision and imported-record count are saved, so history is never imported twice.
