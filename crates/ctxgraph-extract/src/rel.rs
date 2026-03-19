use std::path::Path;

use composable::Composable;
use gliner::model::input::relation::schema::RelationSchema;
use gliner::model::input::text::TextInput;
use gliner::model::output::decoded::SpanOutput;
use gliner::model::output::relation::RelationOutput;
use gliner::model::params::Parameters;
use gliner::model::pipeline::relation::RelationPipeline;
use gliner::model::pipeline::token::TokenPipeline;
use orp::model::Model;
use orp::params::RuntimeParameters;
use orp::pipeline::Pipeline;

use crate::ner::ExtractedEntity;
use crate::schema::ExtractionSchema;

/// A relation extracted between two entities.
#[derive(Debug, Clone)]
pub struct ExtractedRelation {
    pub head: String,
    pub relation: String,
    pub tail: String,
    pub confidence: f64,
}

/// Relation extraction engine.
///
/// Supports two modes:
/// - **Model-based**: Uses gline-rs `RelationPipeline` with the multitask ONNX model.
/// - **Heuristic**: Pattern-based extraction when no relation model is available.
pub enum RelEngine {
    ModelBased(ModelBasedRelEngine),
    Heuristic,
}

/// Model-based relation extraction using gline-rs.
///
/// Requires `gliner-multitask-large-v0.5` ONNX model.
pub struct ModelBasedRelEngine {
    model: Model,
    params: Parameters,
    tokenizer_path: String,
}

impl ModelBasedRelEngine {
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self, RelError> {
        let runtime_params = RuntimeParameters::default();
        let model = Model::new(
            model_path
                .to_str()
                .ok_or(RelError::InvalidPath(model_path.display().to_string()))?,
            runtime_params,
        )
        .map_err(|e| RelError::ModelLoad(e.to_string()))?;

        Ok(Self {
            model,
            params: Parameters::default(),
            tokenizer_path: tokenizer_path
                .to_str()
                .ok_or(RelError::InvalidPath(
                    tokenizer_path.display().to_string(),
                ))?
                .to_string(),
        })
    }

    pub fn extract(
        &self,
        text: &str,
        labels: &[&str],
        schema: &ExtractionSchema,
    ) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelation>), RelError> {
        // Build relation schema from extraction schema
        let mut relation_schema = RelationSchema::new();
        for (rel_name, spec) in &schema.relation_types {
            let heads: Vec<&str> = spec.head.iter().map(|s| s.as_str()).collect();
            let tails: Vec<&str> = spec.tail.iter().map(|s| s.as_str()).collect();
            relation_schema.push_with_allowed_labels(rel_name, &heads, &tails);
        }

        let input = TextInput::from_str(&[text], labels)
            .map_err(|e| RelError::Inference(e.to_string()))?;

        // Step 1: Run NER via TokenPipeline
        let ner_pipeline = TokenPipeline::new(&self.tokenizer_path)
            .map_err(|e| RelError::Inference(e.to_string()))?;
        let ner_composable = ner_pipeline.to_composable(&self.model, &self.params);
        let ner_output: SpanOutput = ner_composable
            .apply(input)
            .map_err(|e| RelError::Inference(e.to_string()))?;

        // Collect entities from NER output using span character offsets directly
        let mut entities = Vec::new();
        for sequence_spans in &ner_output.spans {
            for span in sequence_spans {
                let (start, end) = span.offsets();
                entities.push(ExtractedEntity {
                    text: span.text().to_string(),
                    entity_type: span.class().to_string(),
                    span_start: start,
                    span_end: end,
                    confidence: span.probability() as f64,
                });
            }
        }

        // Step 2: Run relation extraction on top of NER output
        let rel_pipeline =
            RelationPipeline::default(&self.tokenizer_path, &relation_schema)
                .map_err(|e| RelError::Inference(e.to_string()))?;
        let rel_composable = rel_pipeline.to_composable(&self.model, &self.params);
        let rel_output: RelationOutput = rel_composable
            .apply(ner_output)
            .map_err(|e| RelError::Inference(e.to_string()))?;

        // Collect relations
        let mut relations = Vec::new();
        for sequence_rels in &rel_output.relations {
            for rel in sequence_rels {
                relations.push(ExtractedRelation {
                    head: rel.subject().to_string(),
                    relation: rel.class().to_string(),
                    tail: rel.object().to_string(),
                    confidence: rel.probability() as f64,
                });
            }
        }

        Ok((entities, relations))
    }
}

impl RelEngine {
    /// Create a model-based engine if the multitask model is available,
    /// otherwise fall back to heuristic mode.
    pub fn new(model_path: Option<&Path>, tokenizer_path: Option<&Path>) -> Result<Self, RelError> {
        match (model_path, tokenizer_path) {
            (Some(mp), Some(tp)) if mp.exists() && tp.exists() => {
                let engine = ModelBasedRelEngine::new(mp, tp)?;
                Ok(Self::ModelBased(engine))
            }
            _ => Ok(Self::Heuristic),
        }
    }

    /// Extract relations between entities.
    pub fn extract(
        &self,
        text: &str,
        entities: &[ExtractedEntity],
        schema: &ExtractionSchema,
    ) -> Result<Vec<ExtractedRelation>, RelError> {
        match self {
            Self::ModelBased(engine) => {
                let labels: Vec<&str> = schema.entity_labels();
                let (_, relations) = engine.extract(text, &labels, schema)?;
                Ok(relations)
            }
            Self::Heuristic => Ok(heuristic_relations(text, entities, schema)),
        }
    }
}

/// Heuristic relation extraction using sentence-level co-occurrence.
///
/// Splits text on sentence-ending punctuation (`. `, `! `, `? `, `\n\n`) to build
/// sentence segments, then only pairs entities that appear within the same segment
/// or adjacent segments. This reduces false positives from the naive global scan
/// while preserving recall for closely co-located entities.
fn heuristic_relations(
    text: &str,
    entities: &[ExtractedEntity],
    schema: &ExtractionSchema,
) -> Vec<ExtractedRelation> {
    let patterns: &[(&str, &[&str])] = &[
        ("chose",          &["chose", "selected", "picked", "went with", "adopted"]),
        ("rejected",       &["rejected", "ruled out", "decided against", "dropped"]),
        ("replaced",       &["replaced", "migrated from", "switched from", "moved from"]),
        ("depends_on",     &["depends on", "relies on", "requires", "built on", "uses"]),
        ("fixed",          &["fixed", "resolved", "patched", "repaired", "debugged"]),
        ("introduced",     &["introduced", "added", "implemented", "created", "built"]),
        ("deprecated",     &["deprecated", "removed", "phased out", "sunset"]),
        ("caused",         &["caused", "resulted in", "led to", "triggered"]),
        ("constrained_by", &["constrained by", "limited by", "blocked by", "due to"]),
    ];

    // Build sentence boundaries: list of (start_byte, end_byte) pairs.
    // Sentence end is triggered by `. `, `! `, `? `, or `\n\n`.
    // We include a generous ±1 sentence window for cross-sentence pairs.
    let mut sentence_ranges: Vec<(usize, usize)> = Vec::new();
    {
        let bytes = text.as_bytes();
        let len = text.len();
        let mut seg_start = 0usize;
        let mut i = 0usize;
        while i < len {
            let boundary = if i + 1 < len
                && (bytes[i] == b'.' || bytes[i] == b'!' || bytes[i] == b'?')
                && bytes[i + 1] == b' '
            {
                Some(i + 1) // end is after the punctuation
            } else if i + 1 < len && bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
                Some(i) // end is before the double newline
            } else {
                None
            };

            if let Some(end) = boundary {
                sentence_ranges.push((seg_start, end));
                seg_start = end + 1;
                i = seg_start;
                continue;
            }
            i += 1;
        }
        if seg_start < len {
            sentence_ranges.push((seg_start, len));
        }
    }

    // If no sentence boundaries found, treat entire text as one sentence
    if sentence_ranges.is_empty() {
        sentence_ranges.push((0, text.len()));
    }

    let mut relations = Vec::new();
    let mut seen = std::collections::HashSet::<(String, String, String)>::new();

    for (sent_idx, &(sent_start, sent_end)) in sentence_ranges.iter().enumerate() {
        let sent_text = &text[sent_start..sent_end];
        let sent_lower = sent_text.to_lowercase();

        // Entities whose span_start falls within this sentence
        let sent_entities: Vec<&ExtractedEntity> = entities
            .iter()
            .filter(|e| e.span_start >= sent_start && e.span_start < sent_end)
            .collect();

        // Expanded window: this sentence ± 1 adjacent sentence
        let window_start = if sent_idx > 0 { sentence_ranges[sent_idx - 1].0 } else { sent_start };
        let window_end = if sent_idx + 1 < sentence_ranges.len() {
            sentence_ranges[sent_idx + 1].1
        } else {
            sent_end
        };

        let window_entities: Vec<&ExtractedEntity> = entities
            .iter()
            .filter(|e| e.span_start >= window_start && e.span_start < window_end)
            .collect();

        for (relation, keywords) in patterns {
            let rel_spec = match schema.relation_types.get(*relation) {
                Some(spec) => spec,
                None => continue,
            };

            if !keywords.iter().any(|kw| sent_lower.contains(kw)) {
                continue;
            }

            for &head in sent_entities.iter().filter(|e| rel_spec.head.contains(&e.entity_type)) {
                for &tail in window_entities.iter().filter(|&&e| {
                    !std::ptr::eq(e, head) && rel_spec.tail.contains(&e.entity_type)
                }) {
                    let in_same_sentence =
                        tail.span_start >= sent_start && tail.span_start < sent_end;
                    let confidence = if in_same_sentence { 0.6 } else { 0.45 };

                    // Use text order to assign head/tail direction
                    let (actual_head, actual_tail) = if head.span_start <= tail.span_start {
                        (&head.text, &tail.text)
                    } else {
                        (&tail.text, &head.text)
                    };

                    let key = (actual_head.clone(), relation.to_string(), actual_tail.clone());
                    if seen.insert(key) {
                        relations.push(ExtractedRelation {
                            head: actual_head.clone(),
                            relation: relation.to_string(),
                            tail: actual_tail.clone(),
                            confidence,
                        });
                    }
                }
            }
        }
    }

    relations
}

#[derive(Debug, thiserror::Error)]
pub enum RelError {
    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("failed to load model: {0}")]
    ModelLoad(String),

    #[error("inference error: {0}")]
    Inference(String),
}
