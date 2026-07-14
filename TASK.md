# TASK: Build the ThreadNations MVP

## Objective

Implement the first playable version of ThreadNations as a Rust-based, zero-input, persistent desktop civilization simulation.

ThreadNations must run continuously in its own resizable desktop window. The window can be moved to any monitor and left open while the user works. The simulated world must continue operating while the application is unfocused.

The user does not control the world. There are no gameplay commands, build tools, unit controls, policy choices, map controls, nation-selection mechanics, or other direct interventions.

AI and coding activity may provide neutral simulation fuel, but the subject matter of that activity must never influence what a nation becomes.

The MVP must prove this complete loop:

```text
neutral background activity
    -> abstract world activity points
    -> autonomous simulation pressure
    -> nation-specific outcomes based on world state
    -> visible changes on the desktop map
    -> persistent history saved to disk
```

Do not stop at project scaffolding, placeholder modules, architecture notes, or mock screens. Deliver a working desktop application with a visibly evolving world.

---

## Read Before Coding

Treat the project README as the authoritative product vision.

Inspect the existing repository before changing anything:

1. Read `README.md`.
2. Read `AGENTS.md` if present.
3. Inspect the current Rust workspace, dependencies, tests, and assets.
4. Reuse working code and conventions where practical.
5. Do not discard existing functionality without a concrete reason.
6. If the repository is empty or incomplete, create the structure described in this task.

Make reasonable implementation decisions without repeatedly asking for clarification. Document material assumptions in the final summary.

---

## Non-Negotiable Product Rules

### 1. No Gameplay Input

The released companion experience must not contain gameplay controls.

The user must not be able to:

- Select a nation to control
- Move the camera manually
- Zoom the map manually
- Place settlements
- Assign production
- Change laws
- Choose governments
- Declare wars
- Create alliances
- Spend activity points
- Direct research
- Rename nations
- Trigger disasters
- Alter simulation outcomes

The operating system may still provide normal window controls such as move, resize, minimize, restore, and close.

Developer tools may accept input only when compiled behind a clearly named non-default feature such as `dev-tools`. No debug interaction may be active in the normal release build.

### 2. Thread Content Must Never Shape Nations

Do not inspect, classify, embed, summarize, store, or interpret:

- Prompts
- AI responses
- Conversation subjects
- Source code contents
- Project file contents
- Thread names
- Repository names
- User messages
- Generated text

Do not implement topic classification.

Do not map programming activity to engineering, research activity to science, game design to culture, farming discussions to agriculture, or any similar content-based relationship.

Activity supplies quantity, not meaning.

### 3. Activity Is Neutral

All supported activity records must be converted into abstract points before entering the simulation.

The activity source may be retained for diagnostics, but it must not affect allocation or outcomes.

These two records must produce the same simulation pressure when their numeric metrics are equal:

```json
{
  "source": "codex",
  "active_seconds": 900,
  "interaction_count": 8,
  "token_estimate": 12000,
  "generated_bytes": 24000
}
```

```json
{
  "source": "local_editor",
  "active_seconds": 900,
  "interaction_count": 8,
  "token_estimate": 12000,
  "generated_bytes": 24000
}
```

Only the numeric activity measurements may contribute to the point calculation.

### 4. Nations Develop From Simulation Conditions

National identity and specialization must emerge from:

- Geography
- Climate
- Water access
- Fertility
- Natural resources
- Population
- Settlement density
- Trade position
- Neighboring nations
- Government
- Stability
- War
- Migration
- Historical events
- Deterministic procedural variation

A nation becomes agricultural because its land, labor, climate, demand, and history support agriculture. It does not become agricultural because the user discussed agriculture.

### 5. The World Continues Without Activity

Background activity may accelerate development or create additional opportunities, but inactivity must not pause the world.

Even with no new activity records, the simulation must continue to:

- Advance time
- Consume and produce resources
- Grow or lose population
- Move migration pressure
- Update settlements
- Maintain trade
- Evaluate diplomacy
- Resolve conflicts
- Generate historical events
- Save state

### 6. Deterministic Simulation

Given the same:

- World seed
- Initial configuration
- Ordered activity records
- Tick count

the simulation must produce the same world state.

Use fixed simulation steps and deterministic random streams. Never make simulation results depend on render frame rate, wall-clock jitter, hash-map iteration order, or nondeterministic thread scheduling.

---

## MVP Scope

Build only the systems needed to prove the autonomous desktop-companion fantasy.

The MVP must include:

1. Persistent desktop window
2. Deterministic procedural world generation
3. Autonomous nation generation
4. Population and food
5. Basic natural resources
6. Settlements and territorial borders
7. Neutral activity ingestion
8. State-based activity allocation
9. Autonomous expansion
10. Basic trade
11. Basic diplomacy and conflict
12. Passive map presentation
13. Passive event and nation information
14. Automatic camera behavior
15. SQLite persistence
16. Automated tests
17. A documented run workflow

Deep religion, complex law systems, detailed military units, advanced technology eras, individual citizens, naval simulation, aircraft, black markets, crime, disease, and full grand-strategy economics are post-MVP unless a small implementation is required to support the core loop.

---

## Preferred Technical Direction

Use Rust throughout.

Prefer the following architecture unless the existing repository already has a sound equivalent:

```text
threadnations/
├── crates/
│   ├── app/                  # Application startup, window, scheduling
│   ├── renderer/             # Map, borders, settlements, overlays, camera
│   ├── simulation/           # Autonomous world systems
│   ├── worldgen/             # Deterministic chunks, climate, resources
│   ├── activity/             # Neutral metrics and activity point conversion
│   ├── persistence/          # SQLite schema, save, load, migrations
│   ├── presentation/         # Ticker, rotating cards, world summaries
│   └── common/               # IDs, config, time, deterministic utilities
├── assets/
├── docs/
├── tests/
├── AGENTS.md
├── Cargo.toml
├── README.md
└── TASK.md
```

Preferred libraries:

- Bevy for application lifecycle, ECS, scheduling, windowing, and rendering
- `rusqlite` for SQLite persistence
- `serde` for configuration and serialized records
- `tracing` for structured logs
- A deterministic RNG such as ChaCha through `rand_chacha`
- `thiserror` or an equivalent structured error approach

Use compatible versions already supported by the repository. Keep dependencies focused and justified.

Do not introduce a web frontend, Electron, Tauri, JavaScript runtime, remote backend, or required cloud service.

---

## Required Runtime Architecture

Separate simulation, presentation, persistence, and activity ingestion.

The simulation layer must not depend on rendering types.

The renderer must observe simulation state but must not contain simulation rules.

Activity ingestion must produce neutral records and activity points. It must not directly mutate nation resources.

Persistence must save and restore simulation state without requiring the renderer.

Use explicit events and state transitions rather than hidden cross-module side effects.

Suggested high-level flow:

```text
ActivitySource
    -> ActivityRecord
    -> ActivityPointConverter
    -> GlobalActivityReservoir
    -> OpportunityAllocator
    -> NationOpportunityBudget
    -> Simulation systems
    -> HistoryEvent
    -> Presentation systems
    -> Persistence
```

---

## Core Data Model

Create strongly typed IDs rather than passing raw integers throughout the codebase.

At minimum, define:

```rust
WorldSeed
WorldTick
ChunkCoord
TileCoord
NationId
SettlementId
TradeRouteId
ConflictId
HistoryEventId
```

### World

The world state should track at least:

- Seed
- Current simulation tick
- Current calendar date or year
- Generated chunk bounds
- Global activity reservoir
- Global population
- Nations
- Settlements
- Relations
- Trade routes
- Conflicts
- Historical events
- Save schema version

### Tile

Each generated tile should contain enough information to support visible geography and national development:

- Elevation
- Moisture
- Temperature
- Water or land classification
- Biome
- Fertility
- Wood availability
- Stone availability
- Metal availability
- Fuel availability
- Traversal cost
- Habitability score

Values may be normalized or represented through small integer ranges.

### Nation

Each nation should track at least:

- Stable ID
- Procedurally generated name
- Founding tick
- Capital settlement
- Territory
- Population
- Workforce
- Food stock
- Raw materials
- Manufactured capacity
- Wealth
- Stability
- Government seed or basic government type
- Military capacity
- Research capacity
- Activity opportunity budget
- Diplomatic relations
- Current shortages
- Historical traits derived from actual simulation events

Do not store a thread topic, prompt summary, content embedding, project type, or inferred subject.

### Settlement

Each settlement should track at least:

- Stable ID
- Nation ID
- Tile location
- Name
- Population
- Settlement stage
- Food production
- Resource production
- Infrastructure
- Local stability
- Strategic value

Suggested stages:

```text
Camp
Village
Town
City
Metropolis
```

### History Event

Every meaningful autonomous change should be representable as a historical event.

Examples:

- Nation founded
- Settlement founded
- Settlement advanced a stage
- Famine began
- Famine ended
- Trade route established
- Trade route collapsed
- Border dispute began
- War began
- War ended
- Territory changed hands
- Nation fragmented
- Nation collapsed
- New nation emerged

Each event should include:

- Tick
- Event type
- Relevant entity IDs
- Short human-readable description
- Optional location
- Severity or importance

---

## World Generation

Implement a deterministic, chunk-based world.

Recommended starting scale:

- Chunk size: configurable, with 32 x 32 tiles as a reasonable default
- Initial generated area: enough chunks to display multiple separated regions
- Initial founding nations: 3 to 6, chosen procedurally
- Additional chunks: generated when expansion approaches the current world edge or no viable emergence site exists

World generation must create coherent:

- Land and water
- Elevation
- Temperature
- Moisture
- Biomes
- Fertile regions
- Forests
- Stone
- Metals
- Fuel resources
- Habitable settlement sites

The same seed and chunk coordinate must always generate the same chunk.

Avoid generating the entire infinite world in memory. Generate and persist chunks on demand.

### Viable Nation Sites

Founding sites should favor:

- Fresh water
- Food potential
- Habitable climate
- Some nearby resources
- Separation from other capitals
- Sufficient contiguous land

A nation must not be assigned its economic identity directly. Its early specialization should be calculated later from actual local conditions.

---

## Autonomous Nation Formation

At first launch, bootstrap a small number of founding nations so the world is immediately alive.

After bootstrap, additional nations must emerge from simulation conditions such as:

- Large ungoverned settlements
- Remote population clusters
- Migration into unclaimed regions
- State fragmentation
- Independence pressure
- Collapse of an existing nation
- Accumulated world activity pressure combined with a viable population and location

Do not spawn nations solely because a new AI thread exists.

Do not name nations after providers, projects, repositories, prompts, or files.

Build a deterministic procedural naming system using seed, region, ancestry, and linguistic syllable rules.

Generate nation names, settlement names, and demonyms without requiring external AI calls.

---

## Neutral Activity System

Implement an activity abstraction that can support real integrations later without changing simulation code.

### Activity Record

Use a metadata-only record similar to:

```rust
pub struct ActivityRecord {
    pub occurred_at_unix_ms: i64,
    pub source: ActivitySourceKind,
    pub active_seconds: u64,
    pub interaction_count: u32,
    pub token_estimate: u64,
    pub generated_bytes: u64,
}
```

Do not add content, prompt, response, topic, filename, project name, repository name, or freeform text fields.

### Activity Sources

For the MVP, implement:

1. `SyntheticActivitySource`
   - Allows the world to demonstrate the complete loop without external integrations.
   - Emits deterministic sample activity on a configurable schedule.
   - Enabled by default only for a fresh demo world or explicit configuration.

2. `JsonlActivitySource`
   - Reads appended metadata-only records from a local JSONL inbox file.
   - Tracks the last consumed offset.
   - Handles partial final lines safely.
   - Rejects malformed records without crashing.
   - Does not watch or inspect any other files.

Suggested path:

```text
data/activity-inbox.jsonl
```

### Point Conversion

Convert numeric activity metrics into a bounded activity score.

Use configurable weights and diminishing returns so extreme token counts cannot instantly destabilize the world.

Example shape:

```text
points =
    duration_component
  + interaction_component
  + token_component
  + generated_bytes_component
```

The `source` field must not change the calculated point value.

Add unit tests proving source neutrality.

### Global Reservoir

Activity points first enter a global reservoir.

Do not immediately convert them into specific resources, buildings, technologies, or military units.

Cap the reservoir or apply controlled decay to avoid unlimited accumulation.

---

## Opportunity Allocation

Implement a deterministic allocator that distributes a limited amount of global activity pressure to nations.

Allocation may consider:

- Population
- Stability
- Infrastructure
- Education or research capacity
- Resource access
- Current shortages
- War status
- Recent disasters
- Development level
- Unused opportunity budget

The allocator must not consider activity source or content.

The same amount of allocated opportunity must produce different outcomes according to each nation’s state.

Examples:

- Fertile nation with food shortages invests more into agricultural output.
- Mineral-rich nation with strong infrastructure expands extraction or manufacturing.
- Wealthy educated nation increases research capacity.
- Unstable nation may convert opportunity poorly or experience migration and unrest.
- War-torn nation may spend more capacity on logistics and defense.

Implement this as state-based weighted choices, not hard-coded activity-topic mappings.

Record important outcomes as history events.

---

## Simulation Clock and Scheduling

Use a fixed simulation timestep.

Rendering may run at a variable frame rate, but simulation state must update in deterministic fixed steps.

Define explicit schedules for:

- Frequent resource updates
- Population updates
- Settlement growth
- Territorial expansion
- Activity allocation
- Trade evaluation
- Diplomacy evaluation
- Conflict evaluation
- History aggregation
- Autosave

Make the calendar scale configurable.

A reasonable MVP default is to advance several simulated days or one simulated month per real-world second, but use a named configuration value rather than scattering constants.

The world must continue simulating when the window is unfocused.

Avoid unbounded catch-up loops after the computer wakes from sleep. Apply a configurable maximum catch-up duration or maximum ticks per frame.

---

## Economy and Population

Implement a compact economy that creates meaningful national differences without attempting full Victoria-scale modeling.

### Required Goods

At minimum, model:

- Food
- Wood
- Stone
- Metal
- Fuel
- Manufactured goods
- Wealth

### Production

Settlement production should depend on:

- Local tile and neighboring tile resources
- Population and workforce
- Infrastructure
- Stability
- Technology or research capacity
- Trade access
- Shortages
- Opportunity budget

### Consumption

Population must consume food.

Settlements should consume some materials when growing.

Shortages should affect:

- Population growth
- Stability
- Migration
- Settlement advancement
- Conflict risk

### Population

Model at least:

- Births
- Deaths
- Food-limited growth
- Migration pressure
- War losses
- Famine losses

Use cohorts or aggregate values. Do not simulate individual citizens in the MVP.

---

## Settlements and Borders

Settlements must grow autonomously.

Possible advancement:

```text
Camp -> Village -> Town -> City -> Metropolis
```

Advancement should require combinations of:

- Population
- Food security
- Infrastructure
- Materials
- Stability
- Time

Nations must expand territory into adjacent viable tiles when they have:

- Population pressure
- Administrative capacity
- Access from existing territory
- Sufficient stability
- No stronger competing claim

Territory changes must be deterministic and visible.

Avoid expensive full-map border recomputation every frame. Recalculate only when ownership changes and cache render geometry where practical.

---

## Trade

Implement simple autonomous trade routes.

A route may form when:

- Two nations are not at war
- A traversable connection exists
- One nation has a meaningful surplus
- The other has a meaningful shortage
- Relations are above a minimum threshold
- Trade value exceeds route cost

Trade routes should:

- Move abstract goods over time
- Improve wealth or stability when successful
- Be visible on the map
- End when war, collapse, route loss, or prolonged lack of value makes them invalid
- Produce historical events when created or destroyed

Pathfinding may be coarse for the MVP. Prefer a bounded and understandable implementation over a theoretically perfect one.

---

## Diplomacy and Conflict

Implement a small autonomous relationship model.

Each pair of neighboring or trading nations may track:

- Opinion
- Trust
- Rivalry
- Border tension
- Trade dependence
- War state

Relations should change because of simulation events such as:

- Trade
- Resource competition
- Border pressure
- Territory disputes
- Shared enemies
- War history
- Broken routes
- Relative military threat

War may begin when hostility and pressure cross deterministic thresholds.

Conflict resolution may remain abstract.

At minimum, war should affect:

- Population
- Food and wealth
- Stability
- Military capacity
- Border ownership
- Trade routes
- Historical events

Wars must end through exhaustion, decisive advantage, loss of objectives, collapse, or negotiated settlement.

Do not add tactical unit control.

---

## Passive Desktop Presentation

Create a clean, readable desktop companion rather than a debug visualization.

The normal application window should include:

1. **World map**
   - Terrain and water
   - Nation territory colors
   - Borders
   - Settlement markers
   - Capital markers
   - Trade routes
   - Conflict indicators
   - Important event locations

2. **World summary**
   - World year
   - Nation count
   - Total population
   - Active wars
   - Active trade routes
   - Recent major event

3. **Automatically rotating nation card**
   - Nation name
   - Population
   - Capital
   - Settlement count
   - Food status
   - Wealth
   - Stability
   - Government
   - Current tendency derived from simulation state
   - Current war or trade status

4. **Historical ticker**
   - Show recent meaningful events
   - Prioritize high-importance events
   - Keep text short and readable

### Automatic Camera

The user must not manually operate the camera.

Implement an automatic camera director that can:

- Slowly pan across inhabited regions
- Pause over major events
- Frame a featured nation
- Move to wars, new nations, disasters, or rapidly growing cities
- Return to a broad world overview
- Avoid rapid or nauseating motion

Camera decisions should be presentation-only and must not affect simulation state.

### Window Behavior

The application must:

- Run in a normal desktop window
- Be resizable
- Be movable between monitors
- Continue updating while unfocused
- Remember window size and position when practical
- Support configurable borderless mode
- Support configurable always-on-top mode
- Scale UI for high-DPI displays
- Start at a sensible default size
- Avoid blocking modal dialogs during normal operation

Store window preferences in a local configuration file.

No gameplay settings panel is required. Configuration may be file-based for the MVP.

---

## Persistence

Use SQLite.

Create a schema version from the beginning.

Persist at least:

- World metadata
- World seed
- Current tick and calendar
- Generated chunks
- Tile ownership
- Nations
- Settlements
- Relations
- Trade routes
- Conflicts
- History events
- Global activity reservoir
- Activity totals
- JSONL source offset
- Window state
- Configuration version

Save periodically in a transaction.

Also save during a graceful application shutdown.

A save failure must be logged and surfaced non-destructively. It must not silently corrupt the existing database.

Use either normalized tables or clearly versioned serialized payloads stored in SQLite. Favor a design that allows practical schema migration.

On startup:

1. Open or create the database.
2. Apply migrations.
3. Load the previous world if one exists.
4. Otherwise create a new world from the configured seed.
5. Resume simulation from the saved tick.

Add a persistence round-trip test.

---

## Configuration

Provide a documented local configuration file, for example:

```text
data/config.toml
```

Support at least:

- World seed
- Calendar speed
- Autosave interval
- Initial nation count
- Synthetic activity enabled
- Synthetic activity rate
- JSONL inbox path
- Window width and height
- Borderless mode
- Always-on-top mode
- Maximum catch-up ticks
- History retention limit

Create sensible defaults automatically when the file does not exist.

Configuration changes may require restart in the MVP.

---

## Logging and Diagnostics

Use structured logging.

Log:

- Startup and shutdown
- Loaded or created world
- Database migrations
- Autosave success or failure
- Generated chunks
- Nation formation
- Trade route changes
- War changes
- Activity record ingestion
- Rejected activity records
- Catch-up limiting
- Fatal errors

Do not log raw prompts, responses, source code, filenames, project names, or conversation content.

Provide useful error context without panicking during recoverable failures.

---

## Performance Requirements

Target a smooth desktop-companion experience.

At MVP scale:

- Keep rendering responsive at common desktop resolutions.
- Avoid running expensive simulation systems every frame.
- Keep simulation work bounded through fixed schedules.
- Avoid scanning every tile for every nation on every tick.
- Cache derived border and route visuals.
- Keep historical events bounded through retention or archiving.
- Generate chunks only when required.
- Do not block the render loop during autosave.
- Avoid uncontrolled memory growth during multi-hour runs.

A first-run world with several nations should remain stable during an automated long-run test.

---

## Testing Requirements

Add automated tests for the following.

### Determinism

- Same seed produces the same initial world.
- Same seed and same activity record sequence produce the same state after a fixed tick count.
- Save and reload does not alter the deterministic result.

### Activity Neutrality

- Equal numeric metrics from different sources produce equal activity points.
- Activity source does not change nation allocation.
- Activity records cannot contain prompt or topic content in the typed model.

### World Generation

- Same chunk coordinate and seed produce identical tiles.
- Different chunk coordinates do not reuse identical generated data accidentally.
- Founding sites satisfy minimum habitability rules.

### Simulation

- Population consumes food.
- Food shortages reduce growth and stability.
- Settlements can advance stages.
- Nations can expand territory.
- Trade forms under compatible surplus and shortage conditions.
- Trade does not form while nations are at war.
- Conflict can begin and end.
- Inactivity does not stop simulation time.

### Persistence

- New database initializes correctly.
- World state round-trips correctly.
- Schema version is stored.
- Malformed activity lines do not corrupt the saved offset or crash the app.

### Long-Run Smoke Test

Run a headless or simulation-only test for at least 10,000 fixed ticks and verify:

- No panic
- No invalid populations
- No negative stockpiles unless explicitly represented as debt
- No orphaned settlement ownership
- No trade routes referencing deleted nations
- No conflicts referencing deleted nations
- World time advances
- History is generated
- State remains serializable

---

## Documentation Deliverables

Update or create:

### `README.md`

Keep the product vision intact and add:

- Build prerequisites
- Run commands
- Test commands
- Data directory location
- Configuration location
- Save database location
- JSONL activity record example
- Explanation that activity content is never read

### `docs/architecture.md`

Document:

- Crate responsibilities
- Simulation schedules
- Deterministic RNG strategy
- Activity flow
- Persistence strategy
- Main data model
- Presentation data flow
- Extension points for future integrations

### `docs/activity-format.md`

Document the metadata-only JSONL format and explicitly prohibit content fields.

### `docs/save-format.md`

Document the schema version and migration approach.

---

## Required JSONL Example

The application should accept records shaped like:

```json
{"occurred_at_unix_ms":1784000000000,"source":"codex","active_seconds":600,"interaction_count":5,"token_estimate":8000,"generated_bytes":16000}
{"occurred_at_unix_ms":1784000600000,"source":"local_editor","active_seconds":1200,"interaction_count":0,"token_estimate":0,"generated_bytes":42000}
```

Unknown fields should either be rejected with a clear log message or ignored through an explicitly documented compatibility policy.

Never add a `topic`, `prompt`, `summary`, `content`, `project`, or `filename` field.

---

## Out of Scope

Do not spend MVP time on:

- Reading AI conversation content
- Topic classification
- Embeddings
- LLM calls
- Cloud accounts
- User authentication
- Multiplayer
- Direct user control
- Tactical battles
- Individual citizens
- Detailed family trees
- Full religion simulation
- Full law and doctrine simulation
- Complex political parties
- Detailed crime simulation
- Complex disease simulation
- Nuclear warfare
- Space colonization
- Workshop or mod support
- Mobile versions
- Web versions
- Remote synchronization
- Steam integration
- Achievements
- Monetization

Create clean extension points where appropriate, but do not implement these systems now.

---

## Acceptance Criteria

The task is complete only when all of the following are true:

1. `cargo run --release` launches a working desktop application.
2. A procedurally generated world is visible without user setup.
3. Multiple autonomous nations exist and visibly change over time.
4. The user cannot control nations or the camera in the normal build.
5. The simulation continues while the window is unfocused.
6. The map displays terrain, borders, settlements, trade routes, and conflict state.
7. Nation identities and tendencies are derived from world conditions.
8. No conversation subject matter is read or used.
9. Synthetic activity or JSONL activity produces neutral activity points.
10. Equal activity metrics produce equal points regardless of source.
11. Nations convert opportunity into different outcomes according to their own state.
12. Population, food, settlements, expansion, trade, and conflict operate autonomously.
13. Important changes appear in a passive historical ticker.
14. The camera automatically frames the world and major events.
15. Closing and reopening the app restores the same world.
16. Autosave works without freezing normal rendering.
17. Automated tests pass.
18. The long-run smoke test passes.
19. The repository contains clear run and architecture documentation.
20. The implementation is functional rather than placeholder-only.

---

## Final Verification

Before reporting completion:

1. Format the workspace with `cargo fmt`.
2. Run the full test suite.
3. Run clippy with warnings treated seriously.
4. Build the release application.
5. Launch the application and verify the visible autonomous loop.
6. Confirm there are no normal-build gameplay controls.
7. Confirm no code path analyzes thread or project subject matter.
8. Confirm save and reload behavior.
9. Confirm synthetic and JSONL activity both use the same neutral point converter.
10. Summarize changed files, architecture decisions, test results, and remaining post-MVP limitations.

Do not claim completion if the project only compiles but does not visibly simulate and persist an autonomous world.