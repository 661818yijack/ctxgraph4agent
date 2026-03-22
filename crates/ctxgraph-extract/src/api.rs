//! API-based relation extraction using OpenAI, Anthropic, or compatible endpoints.
//!
//! Tier 2 extraction: highest quality (~0.85-0.90 F1) but requires an API key
//! and sends data to an external service.
//!
//! Set `CTXGRAPH_API_KEY` to enable. Optionally set `CTXGRAPH_API_URL` and
//! `CTXGRAPH_API_MODEL` to customize the endpoint.
//!
//! Anthropic detection: if `CTXGRAPH_API_URL` contains "anthropic.com" or
//! `CTXGRAPH_API_MODEL` starts with "claude-", the Anthropic Messages API
//! format is used automatically.

use serde::{Deserialize, Serialize};

use crate::ner::ExtractedEntity;
use crate::rel::ExtractedRelation;
use crate::schema::ExtractionSchema;

const DEFAULT_OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1-mini";
const DEFAULT_ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";

/// Which API provider format to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiProvider {
    OpenAI,
    Anthropic,
}

/// API-based relation extraction engine.
pub struct ApiRelEngine {
    api_url: String,
    api_key: String,
    model: String,
    provider: ApiProvider,
}

// --- OpenAI request/response types ---

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

// --- Anthropic request/response types ---

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    temperature: f64,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlmTriple {
    head: String,
    relation: String,
    tail: String,
}

/// Detect provider from URL and model name.
fn detect_provider(api_url: &str, model: &str) -> ApiProvider {
    if api_url.contains("anthropic.com") || model.starts_with("claude-") {
        ApiProvider::Anthropic
    } else {
        ApiProvider::OpenAI
    }
}

impl ApiRelEngine {
    /// Create a new API engine from environment variables.
    /// Returns `None` if `CTXGRAPH_API_KEY` is not set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("CTXGRAPH_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }

        let explicit_url = std::env::var("CTXGRAPH_API_URL").ok();
        let explicit_model = std::env::var("CTXGRAPH_API_MODEL").ok();

        // Detect provider from whatever hints are available.
        let provider = detect_provider(
            explicit_url.as_deref().unwrap_or(""),
            explicit_model.as_deref().unwrap_or(""),
        );

        let (default_url, default_model) = match provider {
            ApiProvider::Anthropic => (DEFAULT_ANTHROPIC_URL, DEFAULT_ANTHROPIC_MODEL),
            ApiProvider::OpenAI => (DEFAULT_OPENAI_URL, DEFAULT_OPENAI_MODEL),
        };

        Some(Self {
            api_url: explicit_url.unwrap_or_else(|| default_url.to_string()),
            api_key,
            model: explicit_model.unwrap_or_else(|| default_model.to_string()),
            provider,
        })
    }

    /// Extract relations using the API.
    pub fn extract(
        &self,
        text: &str,
        entities: &[ExtractedEntity],
        schema: &ExtractionSchema,
    ) -> Result<Vec<ExtractedRelation>, ApiError> {
        let system_prompt = build_system_prompt(schema);
        let user_prompt = build_user_prompt(text, entities, schema);

        let content = match self.provider {
            ApiProvider::Anthropic => self.call_anthropic(&system_prompt, &user_prompt)?,
            ApiProvider::OpenAI => self.call_openai(&system_prompt, &user_prompt)?,
        };

        parse_response(&content, entities, schema)
    }

    /// Send request using OpenAI chat completions format.
    fn call_openai(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, ApiError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".into(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature: 0.0,
            max_tokens: 512,
        };

        let response = reqwest::blocking::Client::new()
            .post(&self.api_url)
            .timeout(std::time::Duration::from_secs(30))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| ApiError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            return Err(ApiError::HttpStatus(status, body));
        }

        let body = response
            .json::<ChatResponse>()
            .map_err(|e| ApiError::Parse(e.to_string()))?;

        Ok(body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    /// Send request using Anthropic Messages API format.
    fn call_anthropic(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, ApiError> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 512,
            temperature: 0.0,
            system: system_prompt.to_string(),
            messages: vec![AnthropicMessage {
                role: "user".into(),
                content: user_prompt.to_string(),
            }],
        };

        let response = reqwest::blocking::Client::new()
            .post(&self.api_url)
            .timeout(std::time::Duration::from_secs(30))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| ApiError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            return Err(ApiError::HttpStatus(status, body));
        }

        let body = response
            .json::<AnthropicResponse>()
            .map_err(|e| ApiError::Parse(e.to_string()))?;

        // Extract text from the first text content block.
        Ok(body
            .content
            .iter()
            .find(|c| c.content_type == "text")
            .and_then(|c| c.text.clone())
            .unwrap_or_default())
    }
}

fn build_system_prompt(schema: &ExtractionSchema) -> String {
    format!(
        r#"You are a precise software architecture knowledge graph extractor. Extract directed relationships between entities.

Relation types with direction rules and examples:
- chose: Person/Service adopted a technology. head=chooser, tail=chosen. "Alice chose PostgreSQL" → chose(Alice,PostgreSQL)
- rejected: Person/Service rejected an alternative. head=rejector, tail=rejected. "decided against MongoDB" → rejected(Alice,MongoDB)
- replaced: NEW replaced OLD. head=NEW, tail=OLD. "migrated from MySQL to PostgreSQL" → replaced(PostgreSQL,MySQL)
- depends_on: Consumer depends on provider. head=consumer, tail=provider. "PaymentService uses Redis" → depends_on(PaymentService,Redis)
- fixed: Fixer fixed something. head=fixer, tail=fixed. "Bob patched AuthService" → fixed(Bob,AuthService)
- introduced: Added a new component. head=introducer, tail=introduced. "added Prometheus" → introduced(BillingService,Prometheus)
- deprecated: Removed/phased out. head=deprecator, tail=deprecated. "sunset the SOAP endpoint" → deprecated(Bob,SOAP)
- caused: Causal effect. head=cause, tail=effect. "Redis improved p99 latency" → caused(Redis,p99 latency)
- constrained_by: Constrained by requirement. head=constrained, tail=constraint. "must comply with SLA" → constrained_by(Service,SLA)

Critical rules:
1. "replaced": head=NEW, tail=OLD. "from X to Y" → head=Y, tail=X.
2. "depends_on": head=consumer, tail=provider.
3. "X over Y" in a choice context → chose(chooser,X) + rejected(chooser,Y).
4. Only use relation types: {relation_keys}

Output a JSON array of objects: [{{"head":"<entity>","relation":"<type>","tail":"<entity>"}}]
Use exact entity names from the provided list. Only extract relationships explicitly supported by the text."#,
        relation_keys = schema.relation_labels().join(", "),
    )
}

fn build_user_prompt(
    text: &str,
    entities: &[ExtractedEntity],
    schema: &ExtractionSchema,
) -> String {
    let entity_list: Vec<String> = entities
        .iter()
        .map(|e| format!("- {} [{}]", e.text, e.entity_type))
        .collect();

    format!(
        r#"Entities:
{entities}

Text: {text}

Extract relationships using ONLY these types: {types}
Output ONLY a JSON array, no other text."#,
        entities = entity_list.join("\n"),
        types = schema.relation_labels().join(", "),
    )
}

fn parse_response(
    content: &str,
    entities: &[ExtractedEntity],
    schema: &ExtractionSchema,
) -> Result<Vec<ExtractedRelation>, ApiError> {
    let relation_names: std::collections::HashSet<&str> =
        schema.relation_labels().into_iter().collect();

    let mut relations = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Try JSON array first
    if let Ok(triples) = serde_json::from_str::<Vec<LlmTriple>>(content.trim()) {
        for t in triples {
            add_triple(&t, entities, &relation_names, &mut seen, &mut relations);
        }
        return Ok(relations);
    }

    // JSON lines
    for line in content.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        if let Ok(triple) = serde_json::from_str::<LlmTriple>(line) {
            add_triple(&triple, entities, &relation_names, &mut seen, &mut relations);
        }
    }

    Ok(relations)
}

fn add_triple(
    triple: &LlmTriple,
    entities: &[ExtractedEntity],
    relation_names: &std::collections::HashSet<&str>,
    seen: &mut std::collections::HashSet<(String, String, String)>,
    relations: &mut Vec<ExtractedRelation>,
) {
    if !relation_names.contains(triple.relation.as_str()) {
        return;
    }

    let head = match_entity(&triple.head, entities);
    let tail = match_entity(&triple.tail, entities);

    let (head_name, tail_name) = match (head, tail) {
        (Some(h), Some(t)) if h.text != t.text => (h.text.clone(), t.text.clone()),
        _ => return,
    };

    let key = (head_name.clone(), triple.relation.clone(), tail_name.clone());
    if !seen.insert(key) {
        return;
    }

    relations.push(ExtractedRelation {
        head: head_name,
        relation: triple.relation.clone(),
        tail: tail_name,
        confidence: 0.85,
    });
}

fn match_entity<'a>(name: &str, entities: &'a [ExtractedEntity]) -> Option<&'a ExtractedEntity> {
    let name_lower = name.to_lowercase();

    // Exact
    if let Some(e) = entities.iter().find(|e| e.text == name) {
        return Some(e);
    }
    // Case-insensitive
    if let Some(e) = entities.iter().find(|e| e.text.to_lowercase() == name_lower) {
        return Some(e);
    }
    // Substring
    entities.iter().find(|e| {
        e.text.to_lowercase().contains(&name_lower)
            || name_lower.contains(&e.text.to_lowercase())
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("API request failed: {0}")]
    Request(String),

    #[error("API returned HTTP {0}: {1}")]
    HttpStatus(u16, String),

    #[error("failed to parse API response: {0}")]
    Parse(String),
}
