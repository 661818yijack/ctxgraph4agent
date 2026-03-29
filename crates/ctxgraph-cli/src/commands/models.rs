use ctxgraph_extract::model_manager::{
    ModelManager, gliner_large_v21_int8, gliner_large_v21_tokenizer, gliner_multitask_large,
    gliner_multitask_tokenizer,
};

pub fn download() -> ctxgraph::Result<()> {
    let manager = ModelManager::new().map_err(|e| {
        ctxgraph::CtxGraphError::Extraction(format!("failed to initialize model manager: {e}"))
    })?;

    println!(
        "Downloading models to {}",
        manager
            .model_path(&gliner_large_v21_int8())
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.cache/ctxgraph/models".into())
    );

    let specs = [
        ("GLiNER v2.1 NER model (INT8)", gliner_large_v21_int8()),
        ("GLiNER v2.1 tokenizer", gliner_large_v21_tokenizer()),
        (
            "GLiNER Multitask relation model (INT8)",
            gliner_multitask_large(),
        ),
        ("GLiNER Multitask tokenizer", gliner_multitask_tokenizer()),
    ];

    for (label, spec) in &specs {
        if manager.is_cached(spec) {
            println!("  {label}: cached");
        } else {
            println!("  {label}: downloading...");
            manager.get_or_download(spec).map_err(|e| {
                ctxgraph::CtxGraphError::Extraction(format!("download failed for {label}: {e}"))
            })?;
            println!("  {label}: done");
        }
    }

    // Check for optional GLiREL model
    let glirel_dir = manager
        .model_path(&gliner_large_v21_int8())
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("glirel-large-v0"));
    if let Some(ref dir) = glirel_dir {
        if dir.join("encoder.onnx").exists() && dir.join("scoring_head.onnx").exists() {
            println!("  GLiREL relation model: cached");
        } else {
            println!("  GLiREL relation model: not found (optional)");
            println!("    To enable zero-shot relation extraction, run:");
            println!("    python scripts/export_glirel_onnx.py");
        }
    }

    println!("\nAll models ready. Run `ctxgraph init` to get started.");
    Ok(())
}
