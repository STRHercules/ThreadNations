//! Activity analysis for turning AI usage into simulation pressure.

use threadnations_common::ActivitySourceId;

/// Origin provider for an activity source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivityProvider {
    /// OpenAI ChatGPT conversations.
    ChatGpt,
    /// OpenAI Codex or coding-agent sessions.
    Codex,
    /// Anthropic Claude conversations.
    Claude,
    /// Local imported files or exported logs.
    LocalImport,
    /// Browser or editor activity.
    BrowserOrEditor,
    /// Unknown provider.
    Unknown(String),
}

/// Quantified activity signals used by simulation systems.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActivityMetrics {
    /// Input and output token count approximation.
    pub tokens: u64,
    /// Session duration in minutes.
    pub duration_minutes: u32,
    /// Lines or chunks of generated code.
    pub code_output_units: u32,
    /// Thread age in days.
    pub thread_age_days: u32,
}

/// A meaningful thread, session, project, or imported activity record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivitySource {
    /// Stable source ID.
    pub id: ActivitySourceId,
    /// Provider where the activity came from.
    pub provider: ActivityProvider,
    /// Human-readable title.
    pub title: String,
    /// Optional project folder or workspace name.
    pub project: Option<String>,
    /// Short summary or extracted thread text.
    pub summary: String,
    /// Usage and productivity metrics.
    pub metrics: ActivityMetrics,
}

/// High-level topic category used for national focus and pressure conversion.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TopicKind {
    /// Programming, software, tools, and engineering work.
    Programming,
    /// Game design, gameplay systems, entertainment, and modding.
    GameDesign,
    /// Science, research, medicine, and discovery.
    Science,
    /// Finance, commerce, taxes, trade, and banking.
    Finance,
    /// Law, benefits, rules, and bureaucracy.
    Law,
    /// Survival, preparedness, food, water, and resilience.
    Survival,
    /// Military design, abstract conflict systems, and defense planning.
    MilitaryDesign,
    /// Art, culture, writing, branding, and aesthetic design.
    Art,
}

/// Generated national direction derived from activity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NationFocus {
    /// Engineering, tools, factories, and computers.
    TechnologyAndEngineering,
    /// Entertainment, cultural production, and doctrine variety.
    CulturalProduction,
    /// Universities, medicine, and research acceleration.
    ScientificResearch,
    /// Import/export, banking, merchant activity, and trade routes.
    ImportExportEconomy,
    /// Courts, public administration, and legal institutions.
    BureaucracyAndLaw,
    /// Food, water, defense, and resilience.
    SurvivalistProduction,
    /// Abstract military industry and strategic doctrine.
    MilitaryIndustry,
    /// Religion, art styles, tourism, and luxury goods.
    ReligiousAndArtisticCulture,
}

/// Economic and social pressure produced by activity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UsagePressure {
    /// Labor units available this tick or bootstrap period.
    pub labor: u32,
    /// Research momentum.
    pub research: u32,
    /// Production momentum.
    pub production: u32,
    /// Cultural momentum.
    pub culture: u32,
    /// Population pull from attention and thread age.
    pub population_pull: u32,
}

/// Classifies an activity source into its dominant topic.
#[must_use]
pub fn classify_activity(source: &ActivitySource) -> TopicKind {
    let text = searchable_text(source);
    let mut best = (TopicKind::Art, 0_u32);

    for topic in [
        TopicKind::Programming,
        TopicKind::GameDesign,
        TopicKind::Science,
        TopicKind::Finance,
        TopicKind::Law,
        TopicKind::Survival,
        TopicKind::MilitaryDesign,
        TopicKind::Art,
    ] {
        let score = score_topic(topic, &text) + provider_bias(topic, &source.provider);
        if score > best.1 {
            best = (topic, score);
        }
    }

    best.0
}

/// Converts topic kind to a national focus.
#[must_use]
pub const fn focus_for_topic(topic: TopicKind) -> NationFocus {
    match topic {
        TopicKind::Programming => NationFocus::TechnologyAndEngineering,
        TopicKind::GameDesign => NationFocus::CulturalProduction,
        TopicKind::Science => NationFocus::ScientificResearch,
        TopicKind::Finance => NationFocus::ImportExportEconomy,
        TopicKind::Law => NationFocus::BureaucracyAndLaw,
        TopicKind::Survival => NationFocus::SurvivalistProduction,
        TopicKind::MilitaryDesign => NationFocus::MilitaryIndustry,
        TopicKind::Art => NationFocus::ReligiousAndArtisticCulture,
    }
}

/// Converts usage metrics and topic into simulation pressure.
#[must_use]
pub fn usage_pressure(source: &ActivitySource) -> UsagePressure {
    let topic = classify_activity(source);
    let token_units = source.metrics.tokens / 1_000;
    let labor = (token_units * 100).min(u32::MAX as u64) as u32;
    let duration_bonus = source.metrics.duration_minutes / 10;
    let code_bonus = source.metrics.code_output_units.saturating_mul(3);
    let age_bonus = source.metrics.thread_age_days / 7;

    let mut pressure = UsagePressure {
        labor: labor.saturating_add(duration_bonus),
        research: (token_units * 12).min(u32::MAX as u64) as u32,
        production: (token_units * 18).min(u32::MAX as u64) as u32,
        culture: (token_units * 8).min(u32::MAX as u64) as u32,
        population_pull: ((token_units / 2).min(u32::MAX as u64) as u32).saturating_add(age_bonus),
    };

    match topic {
        TopicKind::Programming => {
            pressure.research = pressure.research.saturating_add(code_bonus.saturating_mul(2));
            pressure.production = pressure.production.saturating_add(code_bonus);
        }
        TopicKind::GameDesign | TopicKind::Art => {
            pressure.culture = pressure.culture.saturating_add(40);
        }
        TopicKind::Science => {
            pressure.research = pressure.research.saturating_add(60);
        }
        TopicKind::Finance => {
            pressure.production = pressure.production.saturating_add(45);
        }
        TopicKind::Law => {
            pressure.culture = pressure.culture.saturating_add(20);
            pressure.population_pull = pressure.population_pull.saturating_add(10);
        }
        TopicKind::Survival => {
            pressure.production = pressure.production.saturating_add(25);
            pressure.population_pull = pressure.population_pull.saturating_add(15);
        }
        TopicKind::MilitaryDesign => {
            pressure.production = pressure.production.saturating_add(60);
        }
    }

    pressure
}

fn searchable_text(source: &ActivitySource) -> String {
    format!(
        "{} {} {}",
        source.title,
        source.project.as_deref().unwrap_or_default(),
        source.summary
    )
    .to_ascii_lowercase()
}

fn provider_bias(topic: TopicKind, provider: &ActivityProvider) -> u32 {
    match (topic, provider) {
        (TopicKind::Programming, ActivityProvider::Codex) => 3,
        (TopicKind::Science, ActivityProvider::Claude) => 2,
        (TopicKind::Art, ActivityProvider::ChatGpt) => 1,
        _ => 0,
    }
}

fn score_topic(topic: TopicKind, text: &str) -> u32 {
    const PROGRAMMING: &[&str] = &["rust", "code", "engine", "crate", "codex", "repo", "api", "compiler"];
    const GAME_DESIGN: &[&str] = &["game", "mod", "combat", "turn", "map", "quest", "npc", "stardew"];
    const SCIENCE: &[&str] = &["research", "science", "medical", "biology", "study", "data", "lab", "health"];
    const FINANCE: &[&str] = &["tax", "finance", "bank", "money", "trade", "market", "budget", "invoice"];
    const LAW: &[&str] = &["law", "legal", "benefits", "court", "policy", "bureaucracy", "case", "rights"];
    const SURVIVAL: &[&str] = &["survival", "food", "water", "shelter", "garden", "resilience", "prepared", "weather"];
    const MILITARY: &[&str] = &["military", "weapon", "war", "battle", "defense", "army", "doctrine", "conflict"];
    const ART: &[&str] = &["art", "design", "caption", "brand", "story", "culture", "religion", "style"];

    let keywords = match topic {
        TopicKind::Programming => PROGRAMMING,
        TopicKind::GameDesign => GAME_DESIGN,
        TopicKind::Science => SCIENCE,
        TopicKind::Finance => FINANCE,
        TopicKind::Law => LAW,
        TopicKind::Survival => SURVIVAL,
        TopicKind::MilitaryDesign => MILITARY,
        TopicKind::Art => ART,
    };

    keywords.iter().filter(|keyword| text.contains(*keyword)).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(title: &str, summary: &str) -> ActivitySource {
        ActivitySource {
            id: ActivitySourceId::new(1),
            provider: ActivityProvider::Codex,
            title: title.to_owned(),
            project: None,
            summary: summary.to_owned(),
            metrics: ActivityMetrics {
                tokens: 10_000,
                duration_minutes: 60,
                code_output_units: 15,
                thread_age_days: 14,
            },
        }
    }

    #[test]
    fn codex_rust_thread_becomes_programming_focus() {
        let activity = source("Rust renderer engine", "Implement crate modules and repo architecture");

        assert_eq!(classify_activity(&activity), TopicKind::Programming);
        assert_eq!(
            focus_for_topic(classify_activity(&activity)),
            NationFocus::TechnologyAndEngineering
        );
    }

    #[test]
    fn tokens_create_labor_pressure() {
        let activity = source("Rust sim", "code engine");
        let pressure = usage_pressure(&activity);

        assert!(pressure.labor >= 1_000);
        assert!(pressure.research > 0);
        assert!(pressure.production > 0);
    }
}
