# AGENTS.md

## Project

ThreadNations is a Rust-first autonomous civilization simulation. AI activity creates nations, and those nations continue to grow, trade, govern, migrate, form religions, establish doctrines, fight, collapse, and expand without direct player control.

This file gives implementation guidance for coding agents working in this repository.

## Core Rule

Build the simulation as a persistent autonomous world, not as a manually controlled strategy game.

The user creates the world by coding and using AI tools. The nations should then simulate independently.

## Technical Requirements

- Use Rust as the primary implementation language.
- Prefer Rust-native libraries and tooling.
- Do not introduce another runtime, web stack, or scripting layer unless the task explicitly asks for it.
- Keep systems deterministic where possible.
- Use stable IDs for nations, settlements, map chunks, resources, routes, wars, religions, doctrines, and activity sources.
- Prefer explicit data models over loose strings for simulation state.
- Keep simulation logic separate from rendering and UI.
- Keep integrations separate from simulation logic.
- All world state must be serializable and persistable.

## Suggested Architecture

```text
threadnations/
├── crates/
│   ├── app/
│   ├── renderer/
│   ├── simulation/
│   ├── worldgen/
│   ├── integrations/
│   ├── persistence/
│   ├── analysis/
│   └── common/
├── assets/
├── docs/
├── tests/
├── AGENTS.md
├── Cargo.toml
└── README.md
```

### Module Boundaries

- `app`: application entrypoint, runtime loop, app state wiring.
- `renderer`: world map rendering, camera, overlays, inspector visuals.
- `simulation`: autonomous systems for nations, population, economy, military, diplomacy, technology, religion, law, migration, crime, health, and disasters.
- `worldgen`: chunk generation, terrain, biomes, resource placement, spawn site selection.
- `integrations`: Codex, Claude, ChatGPT, browser monitoring, exported conversation imports, token reports, and activity duration collectors.
- `persistence`: SQLite or save files, migrations, world snapshots, activity records.
- `analysis`: thread classification, topic scoring, focus generation, activity-to-economy conversion.
- `common`: shared IDs, time, config, errors, deterministic RNG, math types.

## Simulation Design Principles

### Autonomy First

Nations should make decisions from simulation pressure:

- Population needs
- Resource scarcity
- Food and water access
- Technology level
- Government type
- Laws and doctrines
- Religion and culture
- Trade opportunities
- Military threat
- Geography
- Historical grievances
- Natural disasters
- Migration pressure

Do not hard-code player-driven decisions into core nation behavior.

### Usage Creates and Feeds Nations

Activity sources may include:

- AI provider name
- Conversation title
- Thread text or summary
- Token counts
- Session duration
- Project folder
- Code output amount
- Imported conversation logs
- Browser or editor activity

These inputs should affect:

- Nation spawning
- Nation name
- Core focus
- Starting resources
- Population growth
- Research growth
- Economic activity
- Government tendency
- Culture and religion seeds
- Technology acceleration

### Infinite World

The map must support expansion beyond initial bounds.

- Generate terrain in chunks.
- Place resources procedurally.
- Keep unclaimed land available for new nations.
- Spawn nations in viable unclaimed locations.
- Allow borders to grow based on population, infrastructure, military, and political strength.

### Abstract Sensitive Systems

The simulation includes war, crime, drugs, alcohol, weapons, firearms, explosives, and other regulated concepts as abstract systems only.

Do not add real-world instructions for manufacturing weapons, explosives, drugs, or illegal materials. Keep all such resources as game/simulation variables, production categories, risk factors, or policy objects.

## Data Model Expectations

Create strongly typed models for major concepts.

Expected entities:

- `World`
- `MapChunk`
- `Tile`
- `ResourceDeposit`
- `Nation`
- `Settlement`
- `Neighborhood`
- `PopulationCohort`
- `Government`
- `Law`
- `Doctrine`
- `Religion`
- `TradeRoute`
- `Alliance`
- `Conflict`
- `War`
- `MilitaryForce`
- `MilitaryUnitCategory`
- `TechnologyState`
- `Disaster`
- `ActivitySource`
- `HistoryEvent`

Use enums for bounded concepts and structured records for extensible simulation state.

## Resource System

Resources should be grouped by category.

Minimum categories:

- Natural materials
- Refined materials
- Food and agriculture
- Industrial goods
- Abstract military goods
- Social and regulated goods
- High-technology goods

Examples:

- Wood
- Coal
- Iron
- Aluminum
- Steel
- Sand
- Glass
- Copper
- Crystal
- Ammunition
- Melee weapons
- Firearms
- Explosives
- Electronics
- Computers
- Microprocessors
- Airplanes
- Automobiles
- Food crops
- Livestock
- Fresh water
- Medicine
- Alcohol
- Narcotic-like goods

Resource availability should depend on geography, technology, infrastructure, labor, government, trade, and conflict.

## Nation Systems

Each nation should eventually support:

- Generated name
- Core focus
- Government
- Laws
- Doctrines
- Religion or philosophy
- Citizen population count
- Military population count
- Military strength score
- Resource inventory
- Imports and exports
- Food production
- Water access
- Crop specialization
- Animal farming specialization
- Crime level
- Drug and alcohol use
- Education and intelligence distribution
- Immigration and emigration
- Cities, towns, neighborhoods, and borders
- Technology age
- Diplomatic relationships
- History log

## Technology Progression

Nations should progress from primitive to advanced ages.

Suggested ages:

- Stone Age
- Bronze Age
- Iron Age
- Classical Age
- Medieval Age
- Early Industrial Age
- Industrial Age
- Modern Age
- Atomic Age
- Information Age
- Space Age
- Post-Space Age

Progression should not be global. Nations can advance unevenly.

## UI and Map Viewer

The user must be able to inspect the world.

Minimum viewer features:

- Pan and zoom world map
- Show nation borders
- Show unclaimed land
- Show cities, towns, and neighborhoods
- Show resources and resource deposits
- Show trade routes
- Show active conflicts
- Click a nation for details
- View population, military, government, laws, doctrines, religion, economy, imports, exports, and history

Rendering should read simulation state. Rendering should not own simulation logic.

## Persistence Requirements

Worlds are long-lived.

Save and reload:

- World seed
- Time state
- Generated chunks
- Resource deposits
- Nations
- Borders
- Settlements
- Population cohorts
- Governments
- Laws and doctrines
- Religions
- Trade routes
- Alliances
- Wars and conflicts
- Disasters
- Activity source links
- History events

When changing persisted structures, add migration notes or compatibility handling when practical.

## Development Priorities

When implementing features, prefer this order:

1. Core Rust workspace and shared data types.
2. Deterministic world generation with chunks and resources.
3. Activity source model and mock activity importer.
4. Nation spawning from activity sources.
5. Population, food, water, and border growth.
6. Economy and resource production.
7. Trade route creation.
8. Government, laws, and doctrines.
9. Diplomacy, alliances, and conflicts.
10. Military strength and automatic battle resolution.
11. Religion and culture generation.
12. Immigration, emigration, crime, drug use, and alcohol use.
13. Disasters based on geography.
14. Technology age progression.
15. Map viewer and inspector UI.
16. Persistence and long-run simulation stability.

## Coding Standards

- Keep functions small and named clearly.
- Prefer pure simulation functions where practical.
- Avoid hidden global mutable state.
- Use deterministic RNG passed through systems, not random calls scattered through the codebase.
- Add unit tests for simulation rules.
- Add integration tests for world generation, save/load, nation spawning, trade route creation, and conflict creation.
- Use `Result` for fallible operations.
- Use clear error types.
- Keep code formatted with `cargo fmt`.
- Keep code clean with `cargo clippy` where practical.

## Testing Expectations

Important tests:

- Same seed creates same world chunks.
- Nations spawn in unclaimed viable land.
- Nations grow from usage input.
- Borders expand with population and capacity.
- Resources appear according to terrain rules.
- Trade routes form when needs and surpluses match.
- Conflicts emerge from scarcity, ideology, borders, or strategic pressure.
- Military strength changes when units, technology, industry, or supply changes.
- Save and reload preserves world state.
- Technology progression is possible but not identical for every nation.

## Documentation Expectations

Update documentation when behavior changes.

At minimum:

- Update `README.md` when changing the project concept, architecture, setup, or user-facing behavior.
- Add docs for new simulation systems.
- Document assumptions and tradeoffs.
- Keep examples abstract and simulation-focused.

## Do Not Do

- Do not turn this into a manually controlled RTS.
- Do not make the user directly command units by default.
- Do not couple UI and simulation state tightly.
- Do not make the world finite unless explicitly asked.
- Do not hard-code one nation per AI provider only. Thread-based nations must remain supported.
- Do not store unstructured blobs where typed simulation data is needed.
- Do not include real-world weapon, explosive, or drug manufacturing instructions.
- Do not remove autonomous behavior to simplify implementation.

## Current Product Fantasy

The user opens the app and sees a world that exists because they have been coding, researching, and building with AI.

A Rust MMO design thread might become a military-industrial republic. A Stardew mod thread might become a farming nation. A benefits paperwork thread might become a bureaucracy-heavy state. These nations grow, specialize, trade, migrate, pass laws, form religions, fight over resources, and sometimes disappear.

The user is watching their own thinking activity become world history.
