# Simulation Engine Foundation

This document records the first Rust engine foundation for ThreadNations.

## Crate Boundaries

- `threadnations-common`: stable IDs, coordinates, deterministic RNG, time, and shared errors.
- `threadnations-analysis`: activity source models, topic scoring, and usage-pressure conversion.
- `threadnations-worldgen`: deterministic chunks, terrain, biomes, resources, and spawn-site generation.
- `threadnations-simulation`: long-lived world state, nation bootstrap, population growth, resource inventory, territory growth, and history events.
- `threadnations-app`: tiny CLI smoke-test entrypoint that proves the crates wire together.

## Current Simulation Loop

1. An `ActivitySource` models a thread, coding session, imported conversation, or other usage record.
2. `analysis` classifies the activity and converts tokens, duration, code output, and thread age into `UsagePressure`.
3. `worldgen` finds viable unclaimed land in deterministic chunks.
4. `simulation` creates a `Nation` with a stable ID, generated name, focus, government tendency, population cohort, inventory, settlement, territory, military strength, and history event.
5. `World::advance_one_tick` applies renewed usage pressure, grows population, adds abstract resources, and expands borders into unclaimed neighboring land when capacity pressure is high.

## Intentional Tradeoffs

- This is not a renderer or UI yet. Rendering should read simulation state later.
- This does not add external crates yet, so local validation can run without network dependency resolution.
- Sensitive systems such as firearms, explosives, drugs, and war remain abstract resource or pressure values only.
- Persistence is not implemented yet, but the state is modeled with explicit structs and stable IDs so save/load can be added cleanly.

## Validation

The foundation includes unit tests for:

- Deterministic chunk generation.
- Viable unclaimed spawn selection.
- Nation spawning from activity.
- Growth from usage input.
- Border expansion under population pressure.
