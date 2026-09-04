use std::fs;
use std::path::Path;

fn env_example() -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../../.env.example");
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}

#[test]
fn env_example_documents_agent_preferred_llm_keys() {
    let content = env_example();

    for required_key in ["ZAI_API_KEY", "MINIMAX_API_KEY"] {
        assert!(
            content.contains(required_key),
            ".env.example must document {required_key} for agent learn/extraction setup"
        );
    }

    assert!(
        content.contains("ZAI_API_KEY") && content.contains("MINIMAX_API_KEY"),
        ".env.example should document the preferred hosted providers together"
    );
}

#[test]
fn env_example_keeps_generic_fallback_and_storage_keys_documented() {
    let content = env_example();

    for required_key in [
        "OPENROUTER_API_KEY",
        "CTXGRAPH_LLM_KEY",
        "CTXGRAPH_LLM_URL",
        "CTXGRAPH_LLM_MODEL",
        "CTXGRAPH_DB",
        "CTXGRAPH_MODELS_DIR",
    ] {
        assert!(
            content.contains(required_key),
            ".env.example must keep {required_key} documented"
        );
    }
}
