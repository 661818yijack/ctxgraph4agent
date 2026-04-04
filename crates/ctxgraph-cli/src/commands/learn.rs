//! `ctxgraph learn` — run the learning pipeline to extract patterns and create skills.

use std::env;

use ctxgraph::pattern::PatternDescriber;
use ctxgraph::skill::SkillSynthesizer;
use ctxgraph::{CtxGraphError, Result, SkillScope};

use super::open_graph;

/// Real LLM-based pattern describer for the CLI.
struct RealPatternDescriber {
    model: String,
}

impl RealPatternDescriber {
    fn new() -> Self {
        Self {
            model: env::var("CTXGRAPH_MODEL")
                .unwrap_or_else(|_| "glm-5-turbo".to_string()),
        }
    }
}

impl PatternDescriber for RealPatternDescriber {
    fn generate(
        &self,
        candidate: &ctxgraph::PatternCandidate,
        source_summaries: &[String],
    ) -> Result<String> {
        // Build a behavioral description using the LLM
        // For now, use a template-based approach since we don't have an LLM client in the CLI
        // The actual LLM call would be made by ctxgraph-cli with an API client
        let candidate_type = if candidate.relation_triplet.is_some() {
            "relation triplet"
        } else if candidate.entity_pair.is_some() {
            "entity pair"
        } else {
            "entity type"
        };

        let entity_info = if let Some(ref triplet) = candidate.relation_triplet {
            format!("{} --({})--> {}", triplet.0, triplet.1, triplet.2)
        } else if let Some(ref pair) = candidate.entity_pair {
            format!("{} <-> {}", pair.0, pair.1)
        } else {
            format!("types: {}", candidate.entity_types.join(", "))
        };

        let summaries_context = if source_summaries.is_empty() {
            "No source summaries available.".to_string()
        } else {
            source_summaries
                .iter()
                .take(3)
                .map(|s| format!("- {}", s))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt = format!(
            "You are a behavioral pattern analyzer for an AI agent memory system.\n\
            Based on the following co-occurrence pattern, write a 1-2 sentence behavioral \
            description that captures the actionable insight.\n\n\
            Pattern type: {}\n\
            Pattern details: {}\n\
            Occurrence count: {}\n\
            Source summaries:\n{}\n\n\
            Write a behavioral description following these rules:\n\
            - GOOD: \"When debugging Docker networking issues, the agent typically needs to restart \
            the service container and clear the network bridge — avoid assuming the daemon is healthy.\"\n\
            - GOOD: \"The user prefers using dark mode and resists configuration changes unless \
            the rationale is clearly explained.\"\n\
            - BAD: \"Entity type Component appears in 5 similar contexts.\"\n\
            - BAD: \"Entity pair (User, Postgres) appears 3 times across source summaries.\"\n\
            - DO NOT include occurrence counts or entity type names in your description\n\
            - Focus on what the agent or user DOES, PREFERS, or SHOULD AVOID\n\
            - Keep it to 1-2 sentences, max 150 characters\n\
            Description:",
            candidate_type,
            entity_info,
            candidate.occurrence_count,
            summaries_context
        );

        // Call LLM via ZAI API
        let api_key = env::var("ZAI_API_KEY").ok();
        let minimax_key = env::var("MINIMAX_API_KEY").ok();

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "max_tokens": 150,
            "temperature": 0.3
        });

        let response = if let Some(key) = api_key {
            client
                .post("https://api.z.ai/api/coding/paas/v4/chat/completions")
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
        } else if let Some(key) = minimax_key {
            client
                .post("https://api.minimax.io/anthropic")
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
        } else {
            return Err(CtxGraphError::InvalidInput(
                "ZAI_API_KEY or MINIMAX_API_KEY must be set for learn command".to_string(),
            ));
        };

        let response = response.map_err(|e| CtxGraphError::Extraction(e.to_string()))?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| CtxGraphError::Extraction("Invalid LLM response".to_string()))?;

        Ok(text.trim().to_string())
    }
}

/// Real LLM-based skill synthesizer for the CLI.
struct RealSkillSynthesizer {
    model: String,
}

impl RealSkillSynthesizer {
    fn new() -> Self {
        Self {
            model: env::var("CTXGRAPH_MODEL")
                .unwrap_or_else(|_| "glm-5-turbo".to_string()),
        }
    }
}

impl SkillSynthesizer for RealSkillSynthesizer {
    fn synthesize(
        &self,
        draft: &ctxgraph::DraftSkill,
    ) -> Result<(String, String, String, String)> {
        let prompt = format!(
            "You are a skill synthesizer for an AI agent memory system.\n\
            Given a draft skill derived from behavioral patterns, generate:\n\
            1. A short skill NAME (max 10 words)\n\
            2. A TRIGGER CONDITION: when to apply this skill (1 sentence, specific scenario)\n\
            3. An ACTION: what to do when triggered (1-2 sentences, specific steps)\n\
            4. A DESCRIPTION: brief overview (1 sentence)\n\n\
            Draft skill:\n\
            - Entity types: {:?}\n\
            - Success count: {} (times this pattern led to success)\n\
            - Failure count: {} (times this pattern led to failure)\n\
            - Source summaries: {}\n\n\
            Output format (JSON):\n\
            {{\"name\": \"...\", \"trigger\": \"...\", \"action\": \"...\", \"description\": \"...\"}}\n\n\
            Rules:\n\
            - GOOD trigger: \"When debugging Docker networking issues involving container-to-container connectivity\"\n\
            - GOOD action: \"Restart the service container, clear the network bridge, verify DNS resolution — do NOT assume the daemon is healthy\"\n\
            - BAD trigger: \"When entity types [Component, Network] appear together\"\n\
            - BAD action: \"Apply pattern 3\"\n\
            - Do NOT mention entity type names, counts, or co-occurrence metadata in any field\n\
            - Focus on observable behaviors and specific actions",
            draft.entity_types,
            draft.success_count,
            draft.failure_count,
            draft.source_summaries.join(" | ")
        );

        let api_key = env::var("ZAI_API_KEY").ok();
        let minimax_key = env::var("MINIMAX_API_KEY").ok();

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "max_tokens": 300,
            "temperature": 0.3
        });

        let response = if let Some(key) = api_key {
            client
                .post("https://api.z.ai/api/coding/paas/v4/chat/completions")
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
        } else if let Some(key) = minimax_key {
            client
                .post("https://api.minimax.io/anthropic")
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
        } else {
            return Err(CtxGraphError::InvalidInput(
                "ZAI_API_KEY or MINIMAX_API_KEY must be set for learn command".to_string(),
            ));
        };

        let response = response.map_err(|e| CtxGraphError::Extraction(e.to_string()))?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| CtxGraphError::Extraction("Invalid LLM response".to_string()))?;

        // Parse JSON from response
        let parsed: serde_json::Value = serde_json::from_str(text.trim())
            .map_err(|e| CtxGraphError::Extraction(format!("Failed to parse skill JSON: {}", e)))?;

        let name = parsed["name"].as_str().unwrap_or("Untitled Skill").to_string();
        let trigger = parsed["trigger"].as_str().unwrap_or("").to_string();
        let action = parsed["action"].as_str().unwrap_or("").to_string();
        let description = parsed["description"].as_str().unwrap_or("").to_string();

        Ok((name, trigger, action, description))
    }
}

pub struct LearnOptions {
    pub dry_run: bool,
    pub scope: SkillScope,
    pub limit: usize,
    pub agent: String,
    pub format: String,
}

pub fn run(options: LearnOptions) -> Result<()> {
    let graph = open_graph()?;
    let describer = RealPatternDescriber::new();
    let synthesizer = RealSkillSynthesizer::new();

    if options.dry_run {
        // Dry run: just show what would be learned without persisting
        let config = ctxgraph::PatternExtractorConfig::default();
        let candidates = graph.extract_pattern_candidates(&config)?;

        if candidates.is_empty() {
            println!("No patterns found. Run compression first: ctxgraph compress");
            return Ok(());
        }

        println!("ctxgraph learn (dry-run)");
        println!("{}", "-".repeat(40));
        println!("Patterns found: {}", candidates.len());
        println!("Skills that would be created: ~{} (limit: {})", candidates.len().min(options.limit), options.limit);
        println!("Scope: {:?}", options.scope);
        println!("Agent: {}", options.agent);
        println!();
        println!("To persist, run without --dry-run");
        return Ok(());
    }

    let outcome = graph.run_learning_pipeline(
        &options.agent,
        options.scope,
        &describer,
        &synthesizer,
        options.limit,
    )?;

    match options.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&outcome)?);
        }
        _ => {
            println!("Learning complete");
            println!("{}", "-".repeat(30));
            println!("  Patterns found:  {}", outcome.patterns_found);
            println!("  Patterns new:    {}", outcome.patterns_new);
            println!("  Skills created:  {}", outcome.skills_created);
            println!("  Skills updated:  {}", outcome.skills_updated);
            if !outcome.skill_ids.is_empty() {
                println!("  Skill IDs: {}", outcome.skill_ids.join(", "));
            }
        }
    }

    Ok(())
}
