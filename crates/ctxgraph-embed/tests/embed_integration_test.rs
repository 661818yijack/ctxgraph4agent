/// Integration test: actually loads and runs all-MiniLM-L6-v2.
/// Requires internet on first run (downloads ~22MB model to ~/.cache/fastembed).
/// Run with: cargo test -p ctxgraph-embed --test embed_integration_test -- --ignored

#[test]
#[ignore]
fn test_embed_engine_loads_and_produces_384_dim() {
    let engine = ctxgraph_embed::EmbedEngine::new()
        .expect("EmbedEngine::new() failed — model download may have failed");

    let embedding = engine
        .embed("Chose PostgreSQL over SQLite for the billing service")
        .expect("embed() failed");

    assert_eq!(
        embedding.len(),
        384,
        "Expected 384-dim embedding, got {}",
        embedding.len()
    );

    // Verify unit-normalized (cosine sim ready)
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 0.01,
        "Expected unit-normalized embedding, norm={norm}"
    );
}

#[test]
#[ignore]
fn test_cosine_similarity_identical_texts() {
    let engine = ctxgraph_embed::EmbedEngine::new().unwrap();
    let a = engine.embed("PostgreSQL database").unwrap();
    let b = engine.embed("PostgreSQL database").unwrap();
    let sim = ctxgraph_embed::EmbedEngine::cosine_similarity(&a, &b);
    assert!(
        sim > 0.99,
        "Identical text similarity should be >0.99, got {sim}"
    );
}

#[test]
#[ignore]
fn test_cosine_similarity_related_texts() {
    let engine = ctxgraph_embed::EmbedEngine::new().unwrap();
    let a = engine.embed("Chose PostgreSQL for the database").unwrap();
    let b = engine
        .embed("Decided to use Postgres as our data store")
        .unwrap();
    let c = engine.embed("The weather is nice today").unwrap();

    let sim_related = ctxgraph_embed::EmbedEngine::cosine_similarity(&a, &b);
    let sim_unrelated = ctxgraph_embed::EmbedEngine::cosine_similarity(&a, &c);

    assert!(
        sim_related > sim_unrelated,
        "Related texts ({sim_related:.3}) should be more similar than unrelated ({sim_unrelated:.3})"
    );
    assert!(
        sim_related > 0.6,
        "Related tech texts should have similarity >0.6, got {sim_related}"
    );
}
