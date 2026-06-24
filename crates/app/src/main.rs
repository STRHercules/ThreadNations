//! Minimal CLI entrypoint for the ThreadNations engine foundation.

use threadnations_analysis::{ActivityMetrics, ActivityProvider, ActivitySource};
use threadnations_common::ActivitySourceId;
use threadnations_simulation::World;

fn main() {
    let activity = ActivitySource {
        id: ActivitySourceId::new(1),
        provider: ActivityProvider::Codex,
        title: "ThreadNations Rust simulation engine".to_owned(),
        project: Some("ThreadNations".to_owned()),
        summary: "Rust workspace with deterministic worldgen and autonomous nation spawning".to_owned(),
        metrics: ActivityMetrics {
            tokens: 12_000,
            duration_minutes: 60,
            code_output_units: 24,
            thread_age_days: 1,
        },
    };

    let mut world = World::new(0x5448_5245_4144);
    let nation_id = world
        .spawn_nation_from_activity(&activity)
        .expect("mock activity should produce a viable nation");
    world.advance_one_tick(&[activity]);

    let nation = world
        .nation(nation_id)
        .expect("spawned nation should be inspectable");
    println!(
        "{}: population={}, territory_tiles={}, military_strength={}",
        nation.name,
        nation.population.citizens,
        nation.territory.len(),
        nation.military_strength
    );
}
