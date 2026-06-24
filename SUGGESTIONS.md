# SUGGESTIONS

## Next foundation tasks

1. Add persistence crate support for save/load snapshots, starting with a plain text or JSON snapshot format before SQLite migrations.
2. Add a small mock activity importer that reads sample activity records from `assets/mock_activity/` or `tests/fixtures/`.
3. Add trade route creation once nations have compatible resource needs and surpluses.
4. Add conflict pressure scoring for borders, resource scarcity, ideology, and strategic terrain.
5. Add a renderer-facing read model so UI/overlay code can inspect the world without owning simulation logic.
6. Introduce serialization dependencies once the project is ready for external crates and lockfile management.
