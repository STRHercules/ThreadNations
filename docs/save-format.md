# Save format

`data/threadnations.sqlite` uses schema version `1`. It contains a migration ledger and one transactional `world_state` snapshot. Snapshot payload stores world seed, tick, calendar, generated chunk coordinates, nations, settlements, borders, trade routes, conflicts, history, activity reservoir and total, JSONL offset, and window preferences.

Schema changes add a new migration version before loading old data. Saves use SQLite transactions, so a failed write leaves the prior world snapshot intact. First launch creates a demo world; later launches resume its saved tick.
