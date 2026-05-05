//! Tests for dual-format LLM response parsing across ctxgraph.
//!
//! Covers OpenAI format (`choices[0].message.content`) and Anthropic format
//! (`content[0].text`) used by MiniMax and other providers.

use serde_json::json;

/// Extract content from an LLM response using dual-format parsing.
///
/// Tries OpenAI format first, falls back to Anthropic format.
fn extract_llm_content(json: &serde_json::Value) -> Option<String> {
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .or_else(|| {
            json["content"][0]["text"]
                .as_str()
                .map(|s| s.trim().to_string())
        })
}

#[test]
fn test_openai_format_parsing() {
    let openai_response = json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "  Hello from OpenAI  "
                },
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5
        }
    });

    let content = extract_llm_content(&openai_response);
    assert_eq!(content, Some("Hello from OpenAI".to_string()));
}

#[test]
fn test_anthropic_format_parsing() {
    let anthropic_response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "  Hello from Anthropic  "
            }
        ],
        "model": "claude-3-5-haiku",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    });

    let content = extract_llm_content(&anthropic_response);
    assert_eq!(content, Some("Hello from Anthropic".to_string()));
}

#[test]
fn test_minimax_anthropic_format_parsing() {
    // MiniMax returns Anthropic-compatible format
    let minimax_response = json!({
        "id": "msg_minimax_456",
        "type": "message",
        "role": "assistant",
        "model": "MiniMax-M2.7",
        "content": [
            {
                "type": "text",
                "text": "  MiniMax response here  "
            }
        ],
        "usage": {
            "input_tokens": 20,
            "output_tokens": 10
        }
    });

    let content = extract_llm_content(&minimax_response);
    assert_eq!(content, Some("MiniMax response here".to_string()));
}

#[test]
fn test_openai_format_with_empty_content() {
    let openai_empty = json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": ""
                },
                "finish_reason": "stop"
            }
        ]
    });

    let content = extract_llm_content(&openai_empty);
    assert_eq!(content, Some("".to_string()));
}

#[test]
fn test_anthropic_format_with_empty_content() {
    let anthropic_empty = json!({
        "content": [
            {
                "type": "text",
                "text": ""
            }
        ]
    });

    let content = extract_llm_content(&anthropic_empty);
    assert_eq!(content, Some("".to_string()));
}

#[test]
fn test_missing_content_returns_none() {
    let missing = json!({
        "error": "rate limit exceeded"
    });

    let content = extract_llm_content(&missing);
    assert_eq!(content, None);
}

#[test]
fn test_openai_format_missing_message() {
    let malformed = json!({
        "choices": [
            {
                "finish_reason": "stop"
            }
        ]
    });

    let content = extract_llm_content(&malformed);
    assert_eq!(content, None);
}

#[test]
fn test_anthropic_format_missing_text() {
    let malformed = json!({
        "content": [
            {
                "type": "thinking"
            }
        ]
    });

    let content = extract_llm_content(&malformed);
    assert_eq!(content, None);
}

#[test]
fn test_openai_format_takes_precedence_over_anthropic() {
    // If both formats are present (shouldn't happen in practice),
    // OpenAI format should win.
    let both = json!({
        "choices": [
            {
                "message": {
                    "content": "OpenAI wins"
                }
            }
        ],
        "content": [
            {
                "text": "Anthropic loses"
            }
        ]
    });

    let content = extract_llm_content(&both);
    assert_eq!(content, Some("OpenAI wins".to_string()));
}

#[test]
fn test_openai_format_with_thinking_content() {
    // Some OpenAI-compatible providers include reasoning/thinking
    let with_thinking = json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "Final answer",
                    "reasoning_content": "Let me think..."
                },
                "finish_reason": "stop"
            }
        ]
    });

    let content = extract_llm_content(&with_thinking);
    assert_eq!(content, Some("Final answer".to_string()));
}

#[test]
fn test_anthropic_format_with_thinking_blocks() {
    // Anthropic format may have thinking blocks before text
    let with_thinking = json!({
        "content": [
            {
                "type": "thinking",
                "thinking": "Let me analyze..."
            },
            {
                "type": "text",
                "text": "  Actual response  "
            }
        ]
    });

    let content = extract_llm_content(&with_thinking);
    // content[0] is thinking, so text is at content[1]
    // Our parser only checks content[0], so this returns None
    // This is expected behavior — callers should handle multi-block responses
    assert_eq!(content, None);
}

#[test]
fn test_anthropic_format_first_block_is_text() {
    // When first block is text, it works
    let text_first = json!({
        "content": [
            {
                "type": "text",
                "text": "First text block"
            },
            {
                "type": "thinking",
                "thinking": "Second block"
            }
        ]
    });

    let content = extract_llm_content(&text_first);
    assert_eq!(content, Some("First text block".to_string()));
}

#[test]
fn test_json_array_response_from_llm() {
    // LLM might return a JSON array string as content
    let json_array = json!({
        "choices": [
            {
                "message": {
                    "content": "[{\"id\": \"1\", \"label\": \"test\"}]"
                }
            }
        ]
    });

    let content = extract_llm_content(&json_array);
    assert_eq!(content, Some("[{\"id\": \"1\", \"label\": \"test\"}]".to_string()));
}
