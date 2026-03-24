//! DeBERTa cross-encoder relation classifier.
//!
//! Fine-tuned DeBERTa-v3-xsmall that classifies entity pairs into one of
//! 10 relation classes (9 relations + "none"). Uses entity markers
//! `[E1]`/`[/E1]` and `[E2]`/`[/E2]` around head and tail entities.
//!
//! The model takes tokenized text (input_ids + attention_mask) and outputs
//! logits of shape [batch, 10].

use std::collections::HashMap;
use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use crate::rel::RelError;

/// Maximum sequence length for the DeBERTa model.
const MAX_SEQ_LEN: usize = 128;

/// Index of the "none" class.
const NONE_CLASS_IDX: usize = 9;

/// DeBERTa cross-encoder relation classifier.
pub struct DebertaClassifier {
    session: Session,
    tokenizer: Tokenizer,
    label_map: Vec<String>,
}

impl DebertaClassifier {
    /// Load DeBERTa ONNX model and tokenizer from disk.
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self, RelError> {
        let session = Session::builder()
            .and_then(|b| b.with_intra_threads(1))
            .and_then(|b| b.commit_from_file(model_path))
            .map_err(|e| RelError::ModelLoad(e.to_string()))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| RelError::ModelLoad(format!("tokenizer: {e}")))?;

        // Load label map from model directory
        let label_map = if let Some(parent) = model_path.parent() {
            let label_path = parent.join("label_map.json");
            load_label_map(&label_path).unwrap_or_else(default_label_map)
        } else {
            default_label_map()
        };

        Ok(Self {
            session,
            tokenizer,
            label_map,
        })
    }

    /// Classify a relation between head and tail entities in the given text.
    ///
    /// Returns `(predicted_label, confidence, none_probability)`.
    pub fn classify(
        &self,
        text: &str,
        head: &str,
        tail: &str,
    ) -> Result<(String, f32, f32), RelError> {
        let marked = insert_markers(text, head, tail);
        let encoding = self
            .tokenizer
            .encode(marked, true)
            .map_err(|e| RelError::Inference(format!("tokenize: {e}")))?;

        let mut input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mut attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        // Truncate to MAX_SEQ_LEN
        input_ids.truncate(MAX_SEQ_LEN);
        attention_mask.truncate(MAX_SEQ_LEN);

        // Pad to MAX_SEQ_LEN
        while input_ids.len() < MAX_SEQ_LEN {
            input_ids.push(0);
            attention_mask.push(0);
        }

        let seq_len = input_ids.len();
        let ids_tensor = Tensor::from_array(([1, seq_len], input_ids))
            .map_err(|e| RelError::Inference(e.to_string()))?;
        let mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))
            .map_err(|e| RelError::Inference(e.to_string()))?;

        let inputs = ort::inputs![
            "input_ids" => ids_tensor,
            "attention_mask" => mask_tensor,
        ]
        .map_err(|e| RelError::Inference(e.to_string()))?;

        let outputs = self
            .session
            .run(inputs)
            .map_err(|e| RelError::Inference(e.to_string()))?;

        let logits_view = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| RelError::Inference(e.to_string()))?;
        let logits = logits_view
            .as_slice()
            .ok_or_else(|| RelError::Inference("non-contiguous logits".into()))?;

        let num_classes = logits.len().min(self.label_map.len());
        let probs = softmax(&logits[..num_classes]);

        let (best_idx, best_prob) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let none_prob = if NONE_CLASS_IDX < probs.len() {
            probs[NONE_CLASS_IDX]
        } else {
            0.0
        };

        let label = if best_idx < self.label_map.len() {
            self.label_map[best_idx].clone()
        } else {
            "none".to_string()
        };

        Ok((label, *best_prob, none_prob))
    }

    /// Check if a heuristic relation should be filtered out.
    ///
    /// Extracts the sentence(s) containing both entities and classifies that
    /// focused context instead of the full text (which would be truncated
    /// at 128 tokens, losing entity positions).
    ///
    /// Returns `true` if the DeBERTa model thinks this entity pair has no
    /// relation or predicts a different relation type.
    pub fn should_filter(
        &self,
        text: &str,
        head: &str,
        tail: &str,
        heuristic_relation: &str,
        none_threshold: f32,
    ) -> bool {
        // Extract focused context around the entity pair
        let context = extract_entity_context(text, head, tail);
        match self.classify(&context, head, tail) {
            Ok((predicted, confidence, none_prob)) => {
                // Filter if model says "none" above threshold
                if none_prob > none_threshold {
                    return true;
                }
                // If model confidently predicts a different relation, filter
                if predicted != "none"
                    && predicted != heuristic_relation
                    && confidence > 0.60
                    && none_prob < 0.20
                {
                    return true;
                }
                false
            }
            Err(_) => false, // On error, don't filter
        }
    }
}

/// Extract sentence-level context containing both entities.
///
/// Returns the minimal span of sentences that covers both entity mentions.
/// If entities aren't found, returns the original text.
fn extract_entity_context(text: &str, head: &str, tail: &str) -> String {
    let text_lower = text.to_lowercase();
    let head_pos = text_lower.find(&head.to_lowercase());
    let tail_pos = text_lower.find(&tail.to_lowercase());

    let (hp, tp) = match (head_pos, tail_pos) {
        (Some(h), Some(t)) => (h, t),
        _ => return text.to_string(),
    };

    // Find the range covering both entities
    let start = hp.min(tp);
    let end = (hp + head.len()).max(tp + tail.len());

    // Expand to sentence boundaries
    let sent_start = text[..start].rfind(". ").map(|i| i + 2).unwrap_or(0);
    let sent_end = text[end..]
        .find(". ")
        .map(|i| end + i + 1)
        .or_else(|| text[end..].find(".\n").map(|i| end + i + 1))
        .unwrap_or(text.len());

    text[sent_start..sent_end].trim().to_string()
}

/// Insert [E1]/[/E1] and [E2]/[/E2] markers around entities in text.
fn insert_markers(text: &str, head: &str, tail: &str) -> String {
    let text_lower = text.to_lowercase();
    let head_lower = head.to_lowercase();
    let tail_lower = tail.to_lowercase();

    let head_pos = text_lower.find(&head_lower);
    let tail_pos = text_lower.find(&tail_lower);

    match (head_pos, tail_pos) {
        (Some(hp), Some(tp)) if hp < tp && hp + head.len() <= tp => {
            let he = hp + head.len();
            let te = tp + tail.len();
            format!(
                "{}[E1]{}[/E1]{}[E2]{}[/E2]{}",
                &text[..hp],
                &text[hp..he],
                &text[he..tp],
                &text[tp..te],
                &text[te..]
            )
        }
        (Some(hp), Some(tp)) if tp < hp && tp + tail.len() <= hp => {
            let te = tp + tail.len();
            let he = hp + head.len();
            format!(
                "{}[E2]{}[/E2]{}[E1]{}[/E1]{}",
                &text[..tp],
                &text[tp..te],
                &text[te..hp],
                &text[hp..he],
                &text[he..]
            )
        }
        _ => {
            // Fallback: prepend markers
            format!("[E1]{}[/E1] [E2]{}[/E2] {}", head, tail, text)
        }
    }
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|&l| (l - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.into_iter().map(|e| e / sum).collect()
}

fn load_label_map(path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let map: HashMap<String, String> = serde_json::from_str(&content).ok()?;
    let max_idx = map.keys().filter_map(|k| k.parse::<usize>().ok()).max()?;
    let mut labels = vec!["none".to_string(); max_idx + 1];
    for (k, v) in &map {
        if let Ok(idx) = k.parse::<usize>() {
            labels[idx] = v.clone();
        }
    }
    Some(labels)
}

fn default_label_map() -> Vec<String> {
    vec![
        "chose",
        "rejected",
        "replaced",
        "depends_on",
        "fixed",
        "introduced",
        "deprecated",
        "caused",
        "constrained_by",
        "none",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
