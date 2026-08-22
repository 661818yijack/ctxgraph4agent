//! `ctxgraph learn` — run the learning pipeline to extract patterns and create skills.

use std::collections::HashMap;
use std::env;

use ctxgraph::pattern::BatchLabelDescriber;
use ctxgraph::{CtxGraphError, PatternCandidate, Result, SkillScope};

use super::open_graph;

/// Real LLM-based batch label describer for the CLI.
///
/// Makes a single LLM call for all candidates and returns one label per candidate.
struct RealBatchLabelDescriber {
    model: String,
}

impl RealBatchLabelDescriber {
    fn new() -> Self {
        Self {
            model: env::var("CTXGRAPH_MODEL").unwrap_or_else(|_| "glm-5-turbo".to_string()),
        }
    }

    async fn call_llm(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let api_key = env::var("ZAI_API_KEY").ok();
        let minimax_key = env::var("MINIMAX_API_KEY").ok();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": 0.3
        });

        let response = if let Some(key) = api_key {
            client
                .post("https://api.z.ai/api/coding/paas/v4/chat/completions")
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
        } else if let Some(key) = minimax_key {
            client
                .post("https://api.minimax.io/anthropic")
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
        } else {
            return Err(CtxGraphError::InvalidInput(
                "ZAI_API_KEY or MINIMAX_API_KEY must be set for learn command".to_string(),
            ));
        };

        let response = response.map_err(|e| CtxGraphError::Extraction(e.to_string()))?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        // Dual-format parser: try OpenAI format first, fall back to Anthropic format.
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string())
            .or_else(|| {
                json["content"][0]["text"]
                    .as_str()
                    .map(|s| s.trim().to_string())
            });

        content.ok_or_else(|| CtxGraphError::Extraction("Invalid LLM response".to_string()))
    }
}

impl BatchLabelDescriber for RealBatchLabelDescriber {
    async fn describe_batch(
        &self,
        candidates: &[PatternCandidate],
        source_summaries: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<(String, String)>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Build numbered pattern list for the prompt
        let patterns_text: String = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let detail = if let Some(ref triplet) = c.relation_triplet {
                    format!("{} --({})--> {}", triplet.0, triplet.1, triplet.2)
                } else if let Some(ref pair) = c.entity_pair {
                    format!("{} <-> {}", pair.0, pair.1)
                } else {
                    format!("types: {}", c.entity_types.join(", "))
                };
                let summaries = source_summaries
                    .get(&c.id)
                    .map(|v| v.iter().take(2).cloned().collect::<Vec<_>>().join("; "))
                    .unwrap_or_default();
                let ctx = if summaries.is_empty() {
                    String::new()
                } else {
                    format!(" | context: {}", summaries)
                };
                format!(
                    "{}. {} | episodes: {}{}",
                    i + 1,
                    detail,
                    c.occurrence_count,
                    ctx
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "You are a behavioral pattern analyzer. For each pattern below, generate a \
            1-2 sentence behavioral label describing what the agent or user does/should do.\n\n\
            Patterns:\n{}\n\n\
            Output JSON array:\n\
            [{{\n  \"id\": \"1\",\n  \"label\": \"...\"\n}}, ...]\n\n\
            Rules:\n\
            - Max 150 chars per label\n\
            - Focus on observable behaviors, not metadata\n\
            - DO NOT include episode counts or entity type names\n\
            - id must match the pattern number exactly",
            patterns_text
        );

        let max_tokens = (candidates.len() as u32) * 60 + 100;
        let raw = self.call_llm(&prompt, max_tokens).await?;

        parse_batch_label_response(&raw, candidates)
    }
}

fn parse_batch_label_response(
    raw: &str,
    candidates: &[PatternCandidate],
) -> Result<Vec<(String, String)>> {
    let parsed: serde_json::Value = serde_json::from_str(raw.trim()).map_err(|e| {
        CtxGraphError::Extraction(format!("Failed to parse batch labels JSON: {}", e))
    })?;

    let arr = parsed
        .as_array()
        .ok_or_else(|| CtxGraphError::Extraction("Expected JSON array from LLM".to_string()))?;

    let mut results = Vec::new();
    for item in arr {
        let Some(idx) = parse_one_based_label_id(&item["id"]) else {
            continue;
        };
        let label = item["label"].as_str().unwrap_or("").to_string();

        // Map 1-based index back to candidate id
        if idx >= 1 && idx <= candidates.len() {
            results.push((candidates[idx - 1].id.clone(), label));
        }
    }

    Ok(results)
}

fn parse_one_based_label_id(value: &serde_json::Value) -> Option<usize> {
    if let Some(id) = value.as_u64() {
        return usize::try_from(id).ok();
    }

    value.as_str()?.parse::<usize>().ok()
}

pub struct LearnOptions {
    pub dry_run: bool,
    pub scope: SkillScope,
    pub limit: usize,
    pub agent: String,
    pub format: String,
}

pub async fn run(options: LearnOptions) -> Result<()> {
    let graph = open_graph()?;
    let describer = RealBatchLabelDescriber::new();

    if options.dry_run {
        let config = ctxgraph::PatternExtractorConfig::default();
        let candidates = graph.extract_pattern_candidates(&config)?;

        if candidates.is_empty() {
            println!("No patterns found. Add more experiences and try again.");
            return Ok(());
        }

        println!("ctxgraph learn (dry-run)");
        println!("{}", "-".repeat(40));
        println!("Patterns found: {}", candidates.len());
        println!(
            "Skills that would be created: ~{} (limit: {})",
            candidates.len().min(options.limit),
            options.limit
        );
        println!("Scope: {:?}", options.scope);
        println!("Agent: {}", options.agent);
        println!();
        println!("To persist, run without --dry-run");
        return Ok(());
    }

    let outcome = graph
        .run_learning_pipeline(&options.agent, options.scope, &describer, options.limit)
        .await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str) -> PatternCandidate {
        PatternCandidate {
            id: id.to_string(),
            entity_types: vec!["Decision".to_string()],
            relation_triplet: None,
            entity_pair: None,
            occurrence_count: 1,
            source_groups: vec![format!("episode-{id}")],
            confidence: 0.8,
            description: None,
        }
    }

    fn candidates() -> Vec<PatternCandidate> {
        vec![candidate("candidate-a"), candidate("candidate-b")]
    }

    #[test]
    fn parse_batch_label_response_accepts_numeric_ids() {
        let parsed = parse_batch_label_response(
            r#"[{"id":1,"label":"first pattern"},{"id":2,"label":"second pattern"}]"#,
            &candidates(),
        )
        .unwrap();

        assert_eq!(
            parsed,
            vec![
                ("candidate-a".to_string(), "first pattern".to_string()),
                ("candidate-b".to_string(), "second pattern".to_string()),
            ]
        );
    }

    #[test]
    fn parse_batch_label_response_preserves_string_ids() {
        let parsed =
            parse_batch_label_response(r#"[{"id":"1","label":"first pattern"}]"#, &candidates())
                .unwrap();

        assert_eq!(
            parsed,
            vec![("candidate-a".to_string(), "first pattern".to_string())]
        );
    }

    #[test]
    fn parse_batch_label_response_ignores_invalid_ids() {
        let parsed = parse_batch_label_response(
            r#"[
                {"id":0,"label":"zero"},
                {"id":3,"label":"out of range"},
                {"id":"not-a-number","label":"bad string"},
                {"id":1.5,"label":"decimal"},
                {"label":"missing"},
                {"id":1,"label":"valid"}
            ]"#,
            &candidates(),
        )
        .unwrap();

        assert_eq!(
            parsed,
            vec![("candidate-a".to_string(), "valid".to_string())]
        );
    }

    #[test]
    fn parse_batch_label_response_reports_json_errors() {
        let malformed = parse_batch_label_response("not json", &candidates()).unwrap_err();
        assert!(
            malformed
                .to_string()
                .contains("Failed to parse batch labels JSON")
        );

        let non_array =
            parse_batch_label_response(r#"{"id":1,"label":"x"}"#, &candidates()).unwrap_err();
        assert!(
            non_array
                .to_string()
                .contains("Expected JSON array from LLM")
        );
    }
}
