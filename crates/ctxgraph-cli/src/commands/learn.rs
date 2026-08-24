//! `ctxgraph learn` — run the learning pipeline to extract patterns and create skills.

use std::collections::HashMap;
use std::env;

use ctxgraph::pattern::BatchLabelDescriber;
use ctxgraph::{CtxGraphError, PatternCandidate, Result, SkillScope};

use super::open_graph;

/// Real LLM-based batch label describer for the CLI.
///
/// Makes a single LLM call for all candidates and returns one label per candidate.
struct RealBatchLabelDescriber;

#[derive(Debug)]
struct LlmRequestConfig {
    url: String,
    model: String,
    headers: HashMap<String, String>,
    body: serde_json::Value,
}

impl RealBatchLabelDescriber {
    fn new() -> Self {
        Self
    }

    fn request_config_from_values(
        zai_key: Option<String>,
        minimax_key: Option<String>,
        model_override: Option<String>,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<LlmRequestConfig> {
        if let Some(key) = zai_key {
            let model = model_override.unwrap_or_else(|| "glm-5-turbo".to_string());
            let mut headers = HashMap::new();
            headers.insert("Authorization".to_string(), format!("Bearer {key}"));
            headers.insert("Content-Type".to_string(), "application/json".to_string());

            return Ok(LlmRequestConfig {
                url: "https://api.z.ai/api/coding/paas/v4/chat/completions".to_string(),
                model: model.clone(),
                headers,
                body: serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": prompt}],
                    "max_tokens": max_tokens,
                    "temperature": 0.3
                }),
            });
        }

        if let Some(key) = minimax_key {
            let model = model_override.unwrap_or_else(|| "MiniMax-M2.7".to_string());
            let mut headers = HashMap::new();
            headers.insert("x-api-key".to_string(), key);
            headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
            headers.insert("Content-Type".to_string(), "application/json".to_string());

            return Ok(LlmRequestConfig {
                url: "https://api.minimax.io/anthropic/v1/messages".to_string(),
                model: model.clone(),
                headers,
                body: serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": prompt}],
                    "max_tokens": max_tokens
                }),
            });
        }

        Err(CtxGraphError::InvalidInput(
            "ZAI_API_KEY or MINIMAX_API_KEY must be set for learn command".to_string(),
        ))
    }

    fn request_config_from_env(&self, prompt: &str, max_tokens: u32) -> Result<LlmRequestConfig> {
        Self::request_config_from_values(
            env::var("ZAI_API_KEY").ok().filter(|key| !key.is_empty()),
            env::var("MINIMAX_API_KEY")
                .ok()
                .filter(|key| !key.is_empty()),
            env::var("CTXGRAPH_MODEL")
                .ok()
                .filter(|model| !model.is_empty()),
            prompt,
            max_tokens,
        )
    }

    async fn call_llm(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let config = self.request_config_from_env(prompt, max_tokens)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let mut request = client.post(&config.url);
        for (name, value) in &config.headers {
            request = request.header(name.as_str(), value.as_str());
        }

        let response = request.json(&config.body).send().await;

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

        // Parse JSON array response
        let parsed: serde_json::Value = serde_json::from_str(raw.trim()).map_err(|e| {
            CtxGraphError::Extraction(format!("Failed to parse batch labels JSON: {}", e))
        })?;

        let arr = parsed
            .as_array()
            .ok_or_else(|| CtxGraphError::Extraction("Expected JSON array from LLM".to_string()))?;

        let mut results = Vec::new();
        for item in arr {
            let id_str = item["id"]
                .as_str()
                .or_else(|| item["id"].as_u64().map(|_| ""))
                .unwrap_or("")
                .to_string();
            let label = item["label"].as_str().unwrap_or("").to_string();

            // Map 1-based index back to candidate id
            if let Ok(idx) = id_str.parse::<usize>()
                && idx >= 1
                && idx <= candidates.len()
            {
                results.push((candidates[idx - 1].id.clone(), label));
            }
        }

        Ok(results)
    }
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

    #[test]
    fn minimax_request_config_uses_anthropic_endpoint_headers_and_default_model() {
        let config = RealBatchLabelDescriber::request_config_from_values(
            None,
            Some("test-minimax-key".to_string()),
            None,
            "prompt",
            220,
        )
        .expect("MiniMax key should produce a request config");

        assert_eq!(config.url, "https://api.minimax.io/anthropic/v1/messages");
        assert_eq!(config.model, "MiniMax-M2.7");
        assert_eq!(
            config.headers.get("x-api-key").map(String::as_str),
            Some("test-minimax-key")
        );
        assert_eq!(
            config.headers.get("anthropic-version").map(String::as_str),
            Some("2023-06-01")
        );
        assert!(!config.headers.contains_key("Authorization"));
        assert_eq!(config.body["model"], "MiniMax-M2.7");
        assert_eq!(config.body["max_tokens"], 220);
        assert_eq!(config.body["messages"][0]["content"], "prompt");
        assert!(config.body.get("temperature").is_none());
    }

    #[test]
    fn zai_request_config_takes_priority_and_keeps_openai_shape() {
        let config = RealBatchLabelDescriber::request_config_from_values(
            Some("test-zai-key".to_string()),
            Some("test-minimax-key".to_string()),
            Some("custom-zai-model".to_string()),
            "prompt",
            120,
        )
        .expect("ZAI key should produce a request config");

        assert_eq!(
            config.url,
            "https://api.z.ai/api/coding/paas/v4/chat/completions"
        );
        assert_eq!(config.model, "custom-zai-model");
        assert_eq!(
            config.headers.get("Authorization").map(String::as_str),
            Some("Bearer test-zai-key")
        );
        assert_eq!(config.body["temperature"], 0.3);
        assert_eq!(config.body["messages"][0]["content"], "prompt");
        assert!(!config.headers.contains_key("x-api-key"));
    }
}
