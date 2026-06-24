# ThreadNations

ThreadNations is a Rust-built autonomous civilization simulation where every AI coding thread can become a nation.

The player does not directly control armies, cities, governments, or economies. The player creates the world by working, coding, researching, designing, and thinking through AI conversations. Those conversations become civilizations. Those civilizations grow, trade, govern themselves, form religions, fight wars, collapse, migrate, industrialize, and leave history behind.

The goal is a living desktop world that feels like Dwarf Fortress, Victoria 3, The Political Process, an idle sim, and an AI activity companion all got thrown into a weird little nation blender.

## Core Concept

ThreadNations runs as a persistent desktop companion, transparent overlay, wallpaper layer, or standalone world viewer.

Every meaningful AI thread can create or influence a nation:

- A Codex-heavy Rust engine thread might create an engineering republic.
- A Claude research thread might create a university-driven scientific state.
- A ChatGPT worldbuilding thread might create a culture-heavy kingdom.
- A finance or tax thread might create a bureaucracy-focused trade republic.
- A farming game mod thread might create an agricultural nation with animal husbandry and crop specialization.

Each thread becomes history. Long-running projects become ancient powers. Abandoned threads may stagnate, fragment, or get swallowed by stronger neighbors.

## Design Pillars

1. **Rust first**  
   The project should be implemented in Rust, using Rust-native architecture wherever possible.

2. **Usage creates nations**  
   Nations spawn from AI usage, coding activity, thread length, project focus, provider source, and conversation metadata.

3. **Usage grows nations**  
   Token activity, session duration, code output, thread age, and topic classification become labor, research, production, culture, population pressure, and economic momentum.

4. **Autonomous simulation**  
   Nations should not wait for direct player commands. They should grow, migrate, trade, govern, ally, fight, industrialize, collapse, and rebuild on their own.

5. **Procedural infinite world**  
   The world map is borderless and ever-expanding. Land, biomes, resources, hazards, and settlement sites are generated procedurally as the simulation needs them.

6. **Readable history**  
   The user should be able to inspect the map, click nations, view population, resources, government, military strength, cities, doctrines, religions, trade routes, conflicts, and historical events.

## How Nations Spawn

Nations spawn when usage crosses configurable thresholds.

Possible spawn inputs:

- New conversation thread
- Active coding session
- Token count threshold
- Long-running project folder
- AI provider activity
- Local exported conversation
- Browser or editor activity
- Manual import of conversation logs

A new nation should prefer unclaimed land. The map should find a viable spawn site based on terrain, water access, nearby resources, climate, and distance from existing borders.

If no nearby generated region is suitable, the world expands outward and generates more land.

## Nation Identity

Each nation generates its name, culture, starting traits, and core focus from the thread that created it.

The core focus may include:

- Import/export economy
- Military industry
- Scientific research
- Farming and food production
- Religious development
- Cultural production
- Bureaucracy and law
- Technology and engineering
- Medicine and public welfare
- Isolationism
- Expansionism
- Naval trade
- Resource extraction

Example mappings:

| Thread Type | Possible Nation Result |
| --- | --- |
| Rust renderer project | Industrial engineering state |
| Stardew Valley mod discussion | Farming kingdom |
| Tax or benefits question | Legal bureaucracy republic |
| Fantasy lore thread | Religious monarchy or culture state |
| Game combat design | Militarized tactical nation |
| AI research thread | University-led technocracy |

Nation names should be generated from thread keywords, provider identity, project names, dominant topics, and procedural language rules.

## Activity to Simulation Conversion

ThreadNations translates user activity into simulation pressure.

### Tokens as Labor

Tokens spent can generate economic activity.

Example:

```text
1,000 tokens spent:
  +100 labor units
  +25 iron mined
  +15 tools manufactured
  +5 homes constructed
  +1 local invention chance
```

### Thread Length as Civilization Age

Longer threads create older, more developed civilizations.

```text
New thread:
  Camps, huts, basic food gathering, early farming

100k token thread:
  Towns, roads, schools, organized government, workshops

1m token mega-thread:
  Cities, heavy industry, universities, rail, modern military, advanced doctrine
```

### Topic Classification as National Direction

Content should influence development.

| Topic | Simulation Output |
| --- | --- |
| Programming | Engineers, factories, tools, computers, research |
| Game design | Culture, entertainment, happiness, doctrine variety |
| Science | Universities, medicine, innovation, advanced resources |
| Finance | Banks, trade routes, merchant fleets, imports and exports |
| Law | Courts, bureaucracy, governance, crime control |
| Survival | Food, water, defense, resilience |
| Military design | Weapons industries, command structure, military doctrine |
| Art | Culture, religion, luxury goods, tourism |

Local embeddings, keyword classification, or lightweight topic scoring can be used to classify activity.

## Resources

The world contains procedurally placed natural resources. Nations discover, claim, mine, farm, refine, manufacture, trade, and fight over them.

Resources should include raw materials, refined goods, military goods, industrial goods, food, water, drugs, alcohol, and high-technology outputs.

Example resource categories:

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

Nations should specialize in random crops and animals based on climate, geography, available water, cultural traits, and trade needs.

### Industrial Goods

- Tools
- Automobiles
- Airplanes
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

Military goods are simulation resources only. The project should never include real-world construction instructions for weapons, explosives, or illegal materials.

### Social and Regulated Goods

- Alcohol
- Tobacco-like goods
- Narcotic-like goods
- Medicine
- Luxury goods

Drug and alcohol systems should remain abstract and simulated. They can affect health, crime, production, culture, law, trade, and unrest.

## Economy

Each nation has an economy shaped by population, resources, terrain, technology level, doctrine, culture, laws, and trade.

Economic systems should support:

- Resource extraction
- Farming
- Refining
- Manufacturing
- Import needs
- Export surpluses
- Trade routes
- Labor shortages
- Inflation-like scarcity pressure
- Black markets
- Smuggling
- Infrastructure growth
- City specialization
- Industrialization

Nations should establish trade routes autonomously when they have compatible needs and surpluses.

Trade routes may be land, river, sea, air, or advanced high-speed routes depending on technology age.

## Population

Every nation tracks citizens, military population, workforce, intelligence distribution, happiness, health, crime, migration, and settlement growth.

Population systems should include:

- Citizen population count
- Military population count
- Workforce count
- Births and deaths
- Immigration
- Emigration
- Education levels
- Varying citizen intelligence levels
- Class or wealth pressure
- Public health
- Drug and alcohol use
- Crime
- Religious participation
- Political unrest

The simulation does not need individual named citizens at MVP scale. Population cohorts are fine until the design needs more detail.

## Settlements and Scale

Nations form settlements based on population and geography.

Settlement growth stages:

```text
Camp
Village
Town
City
Metropolis
Mega-region
```

Settlements should grow naturally around:

- Fresh water
- Food sources
- Trade routes
- Strategic resources
- Coastlines
- Rivers
- Defensive terrain
- Religious sites
- Industrial zones

Nations expand borders as their population grows, their infrastructure improves, and their military or political strength allows them to claim land.

## Government and Law

Nations form their own governments over time.

Possible government types:

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

Governments should be influenced by thread focus, culture, resource pressure, war, inequality, religion, technology, and historical events.

Nations should create laws and doctrines autonomously.

Example doctrines:

- Closed Borders
- Open Borders
- Dictatorship
- Universal Healthcare
- Universal Basic Income
- Birth Control Ban
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

Doctrines should have tradeoffs instead of being simple bonuses. A doctrine should create benefits, costs, risks, and cultural consequences.

## Religion and Culture

Nations may establish religions, philosophies, myths, holidays, rituals, cultural movements, art styles, and ideological traditions.

Religion and culture may influence:

- Happiness
- Law
- Birth rates
- War support
- Immigration
- Alliances
- Education
- Crime
- Trade compatibility
- Doctrines
- Government legitimacy

Religions should emerge procedurally from geography, disasters, founding topics, historic leaders, victories, collapses, and repeated national experiences.

## Diplomacy

Nations communicate with each other and build relationships autonomously.

Diplomacy should include:

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

Nations should form alliances and conflicts based on need, ideology, resources, historical grievances, military threat, trade reliance, and cultural compatibility.

## Military and Conflict

Nations can build militaries based on population, industry, technology, resources, doctrine, geography, and perceived threats.

Military systems should include:

- Military population count
- Unit categories
- Vehicle categories
- Supply capacity
- Training level
- Doctrine
- Morale
- Technology level
- Defensive structures
- Military strength score

Military strength should be derived from current units, vehicles, weapons, logistics, technology, industry, leadership, and terrain advantages.

War should emerge from simulation pressure, not direct player control.

Possible conflict causes:

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

Battles resolve automatically using abstract simulation rules.

The player watches history unfold.

## Technology Progression

Nations progress from the stone age to the nuclear and space age.

Example ages:

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

Technology progression should depend on:

- Research output
- Education
- Resources
- Trade
- War pressure
- Stability
- Imported knowledge
- Thread topic influence
- AI usage intensity

Nations can advance unevenly. A small high-research nation may have advanced computers but limited food security. A huge agrarian empire may have population power but weak industry.

## Natural Disasters and Geography

Natural disasters should be based on geography, climate, terrain, and procedural world rules.

Examples:

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

Disasters should affect food, water, population, infrastructure, migration, faith, government legitimacy, and conflict risk.

## World Map

The world map is ever expanding and has no fixed borders.

Core map requirements:

- Procedurally generated terrain
- Procedurally scattered resources
- Unclaimed land
- Expanding nations
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

Nations spawn in unclaimed land, attempt to grow their borders, and compete for strategic locations.

The user should be able to view the map, inspect nations, see what nations are where, and understand why borders, wars, trade routes, and cities emerged.

## User Experience

The user is primarily an observer.

The app should let the user:

- View the world map
- Zoom and pan
- Click nations
- Inspect cities, towns, and neighborhoods
- View population counts
- View military population counts
- View military strength
- View government type
- View laws and doctrines
- View religions
- View resources
- View imports and exports
- View active trade routes
- View alliances and wars
- View disasters
- View historical timeline events
- View the thread or activity source that created a nation

The dream version runs as a desktop overlay where little settlements, roads, ships, aircraft, conflicts, and borders appear over time.

## Technical Architecture

ThreadNations should be written in Rust.

Suggested layout:

```text
threadnations/
├── crates/
│   ├── app/                  # Desktop app entrypoint
│   ├── renderer/             # Map rendering, overlay rendering, camera
│   ├── simulation/           # Core autonomous world simulation
│   ├── worldgen/             # Infinite map generation and resources
│   ├── integrations/         # Codex, Claude, ChatGPT, browser, file imports
│   ├── persistence/          # SQLite and world state saves
│   ├── analysis/             # Topic classification and activity conversion
│   └── common/               # Shared types, IDs, config, time
├── assets/
├── docs/
├── tests/
├── AGENTS.md
├── Cargo.toml
└── README.md
```

Recommended Rust-friendly technologies:

- `bevy` or another ECS-friendly structure for large simulation systems
- `egui` for inspector panels and debug UI
- `wgpu` for rendering if custom rendering is needed
- `sqlite` for persistent world state
- `serde` for save/load formats
- Deterministic RNG for reproducible world generation

## Simulation Modules

The simulation should be modular from day one.

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

Systems should communicate through explicit events and state transitions instead of hidden side effects.

## Persistence

ThreadNations is meant to run for months or years.

Persistence requirements:

- Save world seed
- Save generated chunks
- Save nations
- Save populations
- Save settlements
- Save resources
- Save governments, laws, doctrines, religions
- Save trade routes, alliances, wars, and history
- Save activity sources and thread-to-nation links
- Support migration between save versions when practical

## MVP Scope

A practical first playable version should focus on the smallest version that proves the fantasy.

MVP goals:

1. Load activity input from mock data or imported thread summaries.
2. Spawn nations from thread records.
3. Generate an expanding tile or chunk-based world map.
4. Place resources procedurally.
5. Give each nation a name, focus, population, resources, government seed, and territory.
6. Simulate growth over time from usage activity.
7. Allow nations to expand borders into unclaimed land.
8. Allow simple trade route creation between compatible nations.
9. Allow simple conflict creation over borders or resources.
10. Render the map and allow nation inspection.
11. Persist and reload the world.

Post-MVP systems can deepen religion, doctrine, migration, crime, disasters, military units, and technology ages.

## Long-Term Vision

After months of real use, the world might look like this:

```text
World Age: 127 years
Nations: 34
Total Population: 483,000,000
Active Wars: 7
Known Religions: 19
Doctrines: 284
Trade Routes: 3,417
Largest Empire: Codexia
Oldest Surviving Nation: The Rustward Compact
Most Advanced Nation: Claudean Observatory
Most Unstable Region: The Glass Coast
```

Some nations survive for years. Some collapse after one burst of activity. Some become strange old empires because the user spent six months obsessively coding one project.

ThreadNations is not just a game. It is a persistent civilization ecosystem generated from the user's thinking activity, where every AI conversation can become part of world history.
