# ThreadNations

<img src="assets/Icon.png" alt="ThreadNations icon" width="128">

ThreadNations is a Rust-built autonomous civilization simulation that runs continuously as a persistent desktop companion.

It appears in its own resizable window, which can be placed on any monitor and left running while the user works. The world does not wait for commands. Nations emerge, grow, trade, govern themselves, form religions, fight wars, industrialize, collapse, migrate, and leave history behind entirely on their own.

AI and coding activity do not determine what a nation becomes. Thread subject matter is never translated into agriculture, industry, religion, government, military doctrine, or culture. Activity only supplies neutral simulation momentum.

The result is a living desktop world inspired by the emergent history of Dwarf Fortress, the political and economic simulation of Victoria 3, the passive progression of an idle game, and the ambient presence of a desktop companion.

## Core Concept

ThreadNations runs as a persistent desktop companion window that can be moved to any monitor and left open for long periods of time.

There is no direct gameplay input. The user does not issue orders, build cities, choose governments, declare wars, assign production, or steer national development.

The simulation is fully autonomous.

AI usage, coding activity, and active work sessions act only as neutral energy for the world. They may affect how quickly simulation opportunities accumulate, when new nations appear, or how much developmental pressure enters the world, but they never determine the content or identity of a nation.

A Rust coding thread does not automatically create an engineering state. A research session does not automatically create a scientific republic. A conversation about farming does not create an agricultural nation.

Instead, every nation develops from the world itself:

- Geography
- Climate
- Natural resources
- Population pressure
- Trade access
- Neighboring powers
- Internal politics
- Religion and culture
- Historical events
- War and diplomacy
- Procedural variation

AI activity creates motion. The simulation decides what that motion becomes.

## Design Pillars

1. **Autonomous simulation**  
   ThreadNations has no gameplay commands. The world continues autonomously whether or not the user is watching; map navigation and inspection never affect outcomes.

2. **Persistent desktop presence**  
   The simulation runs in a dedicated companion window that can be resized, repositioned, and placed on any monitor.

3. **Activity supplies momentum, not meaning**  
   AI and coding activity influence simulation intensity, timing, and growth pressure. Conversation topics, prompts, code subjects, and project content do not shape national identity.

4. **Procedural national development**  
   Nations derive their identity and specialization from geography, resources, history, population, politics, trade, and deterministic procedural systems.

5. **Autonomous history**  
   Nations expand, migrate, trade, ally, govern, industrialize, fight, collapse, fragment, and rebuild without direct intervention.

6. **Infinite procedural world**  
   The map expands as needed. Terrain, biomes, resources, hazards, settlement sites, and political borders emerge over time.

7. **Readable simulation**  
   The desktop view should communicate what is happening through map animation, labels, event notices, overlays, and automatically rotating information panels.

8. **Long-term persistence**  
   A world should be capable of running for months or years while preserving its nations, borders, institutions, conflicts, and historical record.

## Activity as Simulation Fuel

ThreadNations may observe neutral activity signals from supported AI tools, editors, terminals, or local development sessions.

The system should avoid interpreting the subject matter of that activity.

Useful signals may include:

- Session started
- Session ended
- Active session duration
- Approximate token volume
- Number of completed interactions
- Code generation volume
- Frequency of active sessions
- Time since last activity
- Provider or integration source
- Long-term activity streaks

These signals are converted into abstract simulation fuel.

Possible outputs include:

- World activity points
- Nation-spawn pressure
- Population growth pressure
- Construction capacity
- Research opportunities
- Economic activity
- Migration pressure
- Event probability
- Simulation speed bonuses
- Recovery from stagnation

The same activity signal must be capable of producing different outcomes in different nations because the simulation state, not the source content, determines the result.

For example:

```text
1,000 activity units enter the world.

Nation A has fertile land and labor shortages:
  Agricultural production expands.

Nation B has iron, coal, and dense cities:
  Heavy industry expands.

Nation C is isolated and politically unstable:
  Migration and unrest increase.

Nation D is wealthy and highly educated:
  Research institutions expand.
```

The input is neutral. Existing conditions determine the outcome.

## Privacy Boundary

ThreadNations should not require the contents of conversations, prompts, files, or generated code in order to function.

Integrations should prefer metadata and aggregate activity counters over raw content.

The simulation should not need to know what the user discussed, wrote, researched, or generated. It only needs to know that qualifying activity occurred.

Where possible:

- Process activity locally
- Store only aggregate metrics
- Avoid storing prompts or responses
- Avoid topic classification
- Avoid embeddings of conversation content
- Avoid transmitting user content to external services
- Allow integrations to operate without reading project files

## How Nations Emerge

Nations emerge automatically when world conditions and accumulated activity pressure allow them to.

Possible triggers include:

- Sufficient unclaimed population
- Settlement growth beyond local governance
- Migration into a remote region
- Collapse or fragmentation of an existing state
- Independence movements
- Colonial separation
- Religious schism
- Political revolution
- Accumulated world activity reaching a spawn threshold

New nations should usually arise from the simulation rather than appearing without context.

A nation may begin as:

- A frontier settlement
- A city-state
- A tribal federation
- A breakaway province
- A refugee state
- A trading colony
- A religious enclave
- A military successor state
- A union of smaller settlements

When entirely new settlement is needed, the world generator should locate viable unclaimed land based on water, food, climate, resources, travel access, and distance from existing powers.

If no suitable generated region exists, the map expands outward and creates additional land.

## Nation Identity

Nation identity is procedurally generated from simulation conditions.

A nation may develop around:

- Local geography
- Available resources
- Founding population
- Settlement structure
- Climate
- Trade position
- Military threats
- Religious traditions
- Historic disasters
- Political revolutions
- Economic dependencies
- Relations with neighboring states

Possible national tendencies include:

- Agricultural production
- Maritime trade
- Heavy industry
- Resource extraction
- Scientific research
- Religious authority
- Cultural production
- Military expansion
- Bureaucratic administration
- Public welfare
- Isolationism
- Commercial finance
- Environmental stewardship
- Colonial expansion

These tendencies are outcomes of the simulation. They are not assigned from the subject of an AI thread.

Nation names, demonyms, languages, symbols, cultures, and institutions should be generated through deterministic procedural rules tied to world seed, region, ancestry, and history.

## World Activity Conversion

Activity should enter the world through a neutral allocation system.

One possible model:

```text
Observed activity
    -> activity points
    -> global world pressure
    -> regional allocation
    -> nation-specific opportunities
    -> simulation outcomes
```

The allocation system may consider:

- Population size
- Current stability
- Infrastructure
- Education
- Resource availability
- Government efficiency
- Trade access
- War status
- Recent disasters
- Development level
- Existing shortages

Activity should not directly create specific goods.

Instead of:

```text
1,000 tokens:
  +25 iron
  +5 houses
```

Prefer:

```text
1,000 activity units:
  +development pressure
  +economic opportunity
  +construction potential
  +research potential
```

Each nation then converts that potential according to its own circumstances.

## Simulation Time

ThreadNations should continue progressing independently of active AI usage.

Activity may accelerate or energize the simulation, but inactivity should not freeze the world.

Possible time layers:

- Real-time visual updates
- Fast internal economic ticks
- Daily political and demographic ticks
- Seasonal agricultural and climate ticks
- Yearly historical and technological evaluation

The simulation may slow during long periods of inactivity, but nations should continue to age, consume resources, reproduce, migrate, negotiate, and experience events.

## Resources

The world contains procedurally placed natural resources. Nations discover, claim, extract, refine, manufacture, trade, ration, and fight over them.

### Natural Materials

- Wood
- Stone
- Coal
- Iron
- Copper
- Aluminum
- Sand
- Crystal
- Oil
- Fresh water
- Fertile soil

### Refined Materials

- Steel
- Glass
- Concrete
- Fuel
- Chemicals
- Processed lumber
- Refined copper
- Electronics-grade crystal

### Food and Agriculture

- Wheat
- Corn
- Rice
- Potatoes
- Fruit
- Vegetables
- Medicinal herbs
- Livestock
- Poultry
- Fish
- Dairy

Agricultural specialization should emerge from climate, terrain, water, domestication history, local demand, and trade opportunities.

### Industrial Goods

- Tools
- Automobiles
- Aircraft
- Electronics
- Computers
- Microprocessors
- Construction materials
- Machinery

### Abstract Military Goods

- Ammunition
- Melee weapons
- Firearms
- Explosives
- Armored vehicles
- Aircraft
- Naval vessels

Military goods remain abstract simulation resources. ThreadNations should never include real-world construction instructions for weapons, explosives, or illegal materials.

### Social and Regulated Goods

- Alcohol
- Tobacco-like goods
- Narcotic-like goods
- Medicine
- Luxury goods

These systems remain abstract and may influence health, crime, production, culture, law, trade, and unrest.

## Economy

Each nation has an economy shaped by population, geography, resources, infrastructure, technology, government, culture, laws, trade access, and historical conditions.

Economic systems should support:

- Resource extraction
- Farming
- Refining
- Manufacturing
- Domestic consumption
- Import needs
- Export surpluses
- Trade routes
- Labor shortages
- Scarcity and inflation pressure
- Black markets
- Smuggling
- Infrastructure development
- City specialization
- Industrialization
- Economic collapse
- Recovery and reconstruction

Nations establish trade routes autonomously when they have compatible needs, surpluses, diplomatic access, and viable transportation.

Routes may travel by land, river, sea, air, or advanced transport networks depending on geography and technological development.

## Population

Every nation tracks population through cohorts rather than requiring individually simulated citizens.

Population systems should include:

- Total population
- Civilian population
- Military population
- Workforce
- Births and deaths
- Immigration
- Emigration
- Education
- Skill distribution
- Wealth and class pressure
- Public health
- Crime
- Substance use
- Religious participation
- Political unrest
- Urbanization
- Displacement
- Refugees

Population behavior should respond to food, housing, safety, opportunity, law, culture, war, disease, climate, and migration routes.

## Settlements and Scale

Nations form and expand settlements according to population and geography.

```text
Camp
Village
Town
City
Metropolis
Mega-region
```

Settlements should emerge around:

- Fresh water
- Food sources
- Trade routes
- Strategic resources
- Coastlines
- Rivers
- Defensive terrain
- Religious sites
- Industrial zones
- Transportation hubs

Cities should develop distinct functions over time, such as ports, capitals, mining centers, university cities, military strongholds, manufacturing hubs, or religious centers.

## Government and Law

Nations form and transform their own governments.

Possible government types include:

- Tribal council
- Monarchy
- Theocracy
- Dictatorship
- Military junta
- Republic
- Parliamentary democracy
- Corporate state
- Technocracy
- Socialist state
- Federal union
- Failed state

Government formation should be influenced by:

- Founding conditions
- Population structure
- Wealth distribution
- Religion
- Military power
- Resource pressure
- War
- Technology
- Education
- Historic institutions
- Revolutions
- Foreign influence

Nations create laws and doctrines autonomously.

Possible doctrines include:

- Closed Borders
- Open Borders
- Universal Healthcare
- Universal Basic Income
- Military State
- Religious Law
- Free Trade
- State-Owned Industry
- Isolationism
- Expansionism
- Environmental Protection
- Forced Conscription
- Public Education
- Corporate Rule
- Anti-Drug Policy
- Legalized Alcohol
- Prohibition

Doctrines should create benefits, costs, risks, and social consequences rather than simple bonuses.

## Religion and Culture

Nations may develop religions, philosophies, myths, holidays, rituals, artistic traditions, political ideologies, and cultural movements.

Religion and culture may influence:

- Happiness
- Legitimacy
- Law
- Birth rates
- War support
- Immigration
- Alliances
- Education
- Crime
- Trade compatibility
- Government stability

Religions and cultural traditions should emerge from geography, disasters, migrations, founding myths, historic leaders, victories, defeats, collapses, and repeated national experiences.

## Diplomacy

Nations communicate and build relationships autonomously.

Diplomatic systems should include:

- Recognition
- Alliances
- Rivalries
- Trade agreements
- Migration agreements
- Border treaties
- Religious disputes
- Resource disputes
- Non-aggression pacts
- Sanctions
- Defensive blocs
- Federations
- Proxy conflicts
- Tributary relationships
- Peace settlements

Relationships should respond to ideology, resources, historical grievances, military threat, trade dependence, culture, geography, and alliance obligations.

## Military and Conflict

Nations build militaries according to population, industry, technology, resources, doctrine, geography, logistics, and perceived threats.

Military systems should include:

- Military population
- Unit categories
- Vehicle categories
- Supply capacity
- Training
- Morale
- Technology
- Defensive structures
- Command quality
- Military strength

War should emerge from simulation pressure.

Possible causes include:

- Border disputes
- Resource scarcity
- Ideological conflict
- Religious conflict
- Trade disruption
- Strategic ports
- Water access
- Migration pressure
- Revenge
- Alliance obligations
- Internal collapse
- Independence movements

Battles resolve automatically through abstract simulation rules.

## Technology Progression

Nations progress through technological eras at different rates.

```text
Stone Age
Bronze Age
Iron Age
Classical Age
Medieval Age
Early Industrial Age
Industrial Age
Modern Age
Atomic Age
Information Age
Space Age
Post-Space Age
```

Technological progress should depend on:

- Research output
- Education
- Resources
- Trade
- Stability
- Institutions
- Population density
- Imported knowledge
- War pressure
- Economic surplus

Development should remain uneven. A small research-focused state may lead in computing while struggling to feed itself. A large agrarian empire may possess enormous population power while remaining technologically limited.

## Natural Disasters and Geography

Natural events should arise from geography, climate, terrain, and procedural world rules.

Examples include:

- Earthquakes
- Floods
- Droughts
- Wildfires
- Hurricanes
- Tornadoes
- Volcanic eruptions
- Blizzards
- Heat waves
- Famine
- Disease outbreaks

Disasters affect food, water, population, infrastructure, migration, religion, government legitimacy, and conflict risk.

## World Map

The world map is procedurally generated, borderless, and capable of expanding indefinitely.

Core requirements include:

- Procedural terrain
- Procedural resources
- Unclaimed land
- Dynamic borders
- Roads
- Trade routes
- Rivers
- Coastlines
- Biomes
- Cities and towns
- Conflict zones
- Disaster zones
- Religious and cultural centers
- Migration paths
- Expanding settlement regions

Nations compete for strategic locations and respond to the world around them without direct user commands.

## Desktop Experience

ThreadNations runs in a persistent desktop window.

The window should:

- Be resizable
- Be movable between monitors
- Remember its position and size
- Continue simulating while unfocused
- Support borderless and always-on-top modes
- Scale cleanly across display resolutions
- Remain readable at compact window sizes
- Avoid demanding attention

The map can be panned and zoomed for inspection without affecting the simulation.

Information may be communicated through:

- Automatically rotating nation panels
- Map labels
- Animated borders
- Trade-route movement
- Settlement growth
- Conflict indicators
- Disaster effects
- Short event notices
- Population and economy counters
- A passive historical ticker
- Map navigation and nation inspection
- Periodic world summaries

The user may reposition or configure the application window at the operating-system level, but there are no gameplay controls and no direct control over the world.

## Technical Architecture

ThreadNations should be written in Rust.

Suggested layout:

```text
threadnations/
├── crates/
│   ├── app/                  # Desktop application and window lifecycle
│   ├── renderer/             # World rendering, camera, overlays, animation
│   ├── simulation/           # Autonomous civilization simulation
│   ├── worldgen/             # Infinite terrain, climate, and resources
│   ├── activity/             # Neutral activity metrics and integrations
│   ├── persistence/          # SQLite and world-state saves
│   ├── presentation/         # Passive panels, event ticker, summaries
│   └── common/               # Shared types, IDs, config, time
├── assets/
├── docs/
├── tests/
├── AGENTS.md
├── Cargo.toml
└── README.md
```

Recommended Rust-friendly technologies:

- `bevy` or another ECS-oriented architecture for simulation systems
- `egui` for internal tools, diagnostics, and optional configuration
- `wgpu` for custom rendering
- `winit` for desktop window behavior
- `sqlite` for persistent world state
- `serde` for save and configuration formats
- Deterministic random generation for reproducible worlds

Debug and development interfaces may accept input, but the released companion experience should remain free of gameplay controls.

## Simulation Modules

The simulation should be modular from the beginning.

```text
simulation/
├── nations
├── population
├── economy
├── resources
├── settlements
├── governments
├── laws
├── religion
├── diplomacy
├── military
├── technology
├── disasters
├── migration
├── crime
├── health
└── history
```

Systems should communicate through explicit events and state transitions rather than hidden side effects.

## Persistence

ThreadNations is intended to run for months or years.

Persistence requirements include:

- Save world seed
- Save generated chunks
- Save nations
- Save populations
- Save settlements
- Save resources
- Save governments, laws, doctrines, and religions
- Save trade routes, alliances, wars, and historical events
- Save aggregate activity totals
- Save simulation time and scheduling state
- Support save-version migration when practical

Raw prompts, conversation contents, and generated code should not be required for persistence.

## MVP Scope

The first playable version should prove the autonomous desktop-companion concept.

MVP goals:

1. Run in a persistent resizable desktop window.
2. Generate a deterministic chunk-based world map.
3. Spawn procedurally generated nations without using conversation subject matter.
4. Simulate population, food, basic resources, settlements, and borders.
5. Convert mock or locally observed activity metrics into neutral world activity points.
6. Allow nations to use those points according to their own conditions.
7. Create simple autonomous trade relationships.
8. Create simple autonomous conflicts over borders and resources.
9. Display events through passive labels, notices, and rotating panels.
10. Continue simulating while the window is unfocused.
11. Persist and reload the complete world.

Post-MVP work can deepen governments, religion, migration, crime, disasters, military organization, industry, diplomacy, and technology.

## Long-Term Vision

After months of continuous use, a world might contain:

```text
World Age: 127 years
Nations: 34
Total Population: 483,000,000
Active Wars: 7
Known Religions: 19
Doctrines: 284
Trade Routes: 3,417
Largest Empire: The Veyran Union
Oldest Surviving Nation: The Orrow Compact
Most Advanced Nation: The Telic Republic
Most Unstable Region: The Glass Coast
```

Some nations may survive for centuries. Others may collapse shortly after forming. A minor settlement may become a continental empire because of geography, trade, political stability, and luck. A wealthy industrial state may disintegrate after war, famine, or internal division.

None of those outcomes are chosen by the user, and none are inferred from what the user discussed with an AI.

ThreadNations is a persistent autonomous civilization ecosystem that turns background activity into motion while allowing the simulated world to decide its own history.

## Run

Install stable Rust, then run:

```powershell
cargo run --release -p threadnations-app
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

### Godot viewer (in progress)

The Godot project lives at the repository root and uses the existing Rust simulation and SQLite save as its authority. The normal launch is one command:

```powershell
.\scripts\run-godot.ps1
```

Use `.\scripts\run-godot.ps1 -Editor` to open the project in the Godot editor, or `-Release` to run a release-mode Rust bridge. The script starts a fresh local bridge and replaces any earlier bridge; it stays alive after Godot exits so the simulation keeps advancing. To run the two processes manually:

```powershell
cargo run -p threadnations-app --bin godot_bridge
& 'C:\Godot\Godot_v4.7.1-stable_win64.exe' --editor project.godot
```

The bridge writes local map snapshots under `.tmp/` and reads the Godot viewer's requested map center from the same folder; it does not open a network port. Do not run the legacy eframe app and the Godot bridge against the same save at once: both are autonomous simulation hosts during this transition. The current Godot slice provides terrain/sprite rendering, borders/walls, buildings, workers, wildlife, trade paths, conflicts, pan/zoom, nation selection, and nation/history inspection; reset, backup, restore, and historical-import controls remain in the eframe app for now.

First launch chooses a new random world seed and stores `config.toml` plus `threadnations.sqlite` in `%LOCALAPPDATA%\ThreadNations`. New worlds begin with one nation and later powers emerge from autonomous growth. The default calendar advances one tick per minute, or one simulated year every twelve minutes; change `calendar_step_ms` in that config file to tune that cadence. The normal build has no gameplay controls: drag the map to pan, use the mouse wheel to zoom, click a nation to inspect it, or use the collapsible **Nations** menu to focus it. The right panel has Nation and History tabs; each nation has a persistent named ruler whose title reflects its government. Capitals display a small set of representative profession markers—such as farmers, teachers, engineers, soldiers, scientists, and historians—rather than every citizen. These controls never affect the autonomous simulation.

The map renderer uses the bundled Miniworld sprites at native 16px grid scale. Buildings, permanent roads, specialist workers, renewable tree stumps and rocks, and animated land/water wildlife are saved world state; the renderer only visualizes them. Nations persist a complete medieval job roster, while visible lumberjacks, stonemasons, and hunters gather their matching world resources. Land animals are scattered across bare grass, while fish populate rivers, lakes, and ocean tiles; both use their directional sprite-sheet rows rather than cycling unrelated frames. Idle workers wander on clear tiles within their nation. Each nation receives a stable coloured building palette, starting with huts before progressing to houses. Farms use the dedicated Farm sprites, retain an adjacent wheat field, and are kept clear of the settlement core. Construction is limited to bare grass tiles within a nation's territory. Rocks are scarce on grassland and concentrate in beach and desert terrain. Roads are rebuilt as a shared branch network, and the supplied wall sprites trace the exposed perimeter of every nation. Each keep occupies the wall gate and serves as the settlement entrance. Use **Borders** to show high-contrast colored territory outlines. Persistent local roads and trade corridors are drawn on the minimap, while capital markers remain above them and remain within its frame. The History tab condenses repeated border growth into one summary per nation.

Use **Home** to return the camera to the opening view. **Options** can reset to a new seed, back up the current SQLite world through the Windows file picker, or restore a selected backup.

`data/activity-inbox.jsonl` is optional. It accepts only numeric, metadata-only activity records; the application never reads prompts, responses, project names, file names, source code, titles, or topic text. See [activity format](docs/activity-format.md).

First launch asks whether to import existing local Codex sessions. Accepting imports every discoverable session as neutral metadata-derived activity, then immediately advances the autonomous world. Conversation content remains unread; see [activity format](docs/activity-format.md) for exact boundaries.
