# MVP architecture

`common` owns stable IDs, coordinates, ticks, and deterministic RNG. `worldgen` maps `(seed, tile)` to terrain and resources. `analysis` accepts only metadata-only activity records and converts their numeric fields into neutral points. `simulation` owns persistent world state and fixed autonomous steps. `persistence` stores a versioned world snapshot in SQLite. `app` renders a passive `eframe` desktop view.

Every fixed tick adds neutral points to a capped global reservoir, allocates a bounded amount by population and instability, then runs food, population, materials, settlement growth, trade, borders, and conflict. Activity origin is diagnostic only; it never changes points or allocation.

World generation is stateless: terrain is regenerated from world seed and tile coordinate. Saves retain generated chunk coordinates, entity IDs, and all changing simulation state. Renderer reads world state and generated tiles but never changes simulation outcomes.

The app uses a fixed calendar step with bounded catch-up. Rendering requests repaint independently, so simulation keeps advancing while the companion window is unfocused. Future integrations must emit `ActivityRecord` only; no integration may pass content into the simulation.
