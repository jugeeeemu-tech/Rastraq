use rastraq::llm::{DeterministicMockProvider, FastEmbedProvider, LlmProvider};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const ONNXRUNTIME_VERSION: &str = "1.24.2";
const ONNXRUNTIME_DIR: &str = "/tmp/onnxruntime-linux-x64-1.24.2";

#[tokio::test]
async fn mock_provider_returns_stable_summary_and_embedding() {
    let provider = DeterministicMockProvider::default();
    let first = provider
        .summarize_and_embed(
            "Rust 1.90 release notes",
            "Rust 1.90 shipped a small standard library improvement and compiler fixes.",
        )
        .await
        .unwrap();
    let second = provider
        .summarize_and_embed(
            "Rust 1.90 release notes",
            "Rust 1.90 shipped a small standard library improvement and compiler fixes.",
        )
        .await
        .unwrap();

    assert_eq!(first.provider, "deterministic-mock");
    assert_eq!(first.summary, second.summary);
    assert_eq!(first.embedding, second.embedding);
    assert_eq!(first.embedding.len(), 16);
    assert!(first.summary.contains("Rust 1.90 release notes"));
}

#[tokio::test]
#[ignore = "downloads and runs the fastembed model"]
async fn fastembed_provider_returns_bge_small_embeddings() {
    prepare_onnxruntime();

    let provider = FastEmbedProvider::bge_small_en_v15();
    let processed = provider
        .summarize_and_embed(
            "Rust 1.90 release notes",
            "Rust 1.90 shipped a small standard library improvement and compiler fixes.",
        )
        .await
        .unwrap();

    assert_eq!(processed.provider, "fastembed");
    assert_eq!(processed.model, "BAAI/bge-small-en-v1.5");
    assert_eq!(processed.embedding.len(), 384);
    assert!(processed.embedding.iter().any(|value| *value != 0.0));
    assert!(processed.summary.contains("Rust 1.90 release notes"));
}

fn prepare_onnxruntime() {
    if env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }

    let lib_path = PathBuf::from(ONNXRUNTIME_DIR)
        .join("lib")
        .join("libonnxruntime.so");
    if !lib_path.exists() {
        download_onnxruntime();
    }

    let lib_dir = lib_path.parent().expect("onnxruntime lib dir");
    env::set_var("ORT_DYLIB_PATH", &lib_path);
    env::set_var("LD_LIBRARY_PATH", lib_dir);
}

fn download_onnxruntime() {
    let archive = format!("/tmp/onnxruntime-linux-x64-{ONNXRUNTIME_VERSION}.tgz");
    let url = format!(
        "https://github.com/microsoft/onnxruntime/releases/download/v{ONNXRUNTIME_VERSION}/onnxruntime-linux-x64-{ONNXRUNTIME_VERSION}.tgz"
    );

    run_command(
        Command::new("curl")
            .args(["-fL", "-o", &archive, &url]),
        "download ONNX Runtime",
    );
    run_command(
        Command::new("tar").args(["-xzf", &archive, "-C", "/tmp"]),
        "extract ONNX Runtime",
    );

    let expected = Path::new(ONNXRUNTIME_DIR)
        .join("lib")
        .join("libonnxruntime.so");
    assert!(
        expected.exists(),
        "expected ONNX Runtime library at {}",
        expected.display()
    );
}

fn run_command(command: &mut Command, label: &str) {
    let output = command.output().unwrap_or_else(|error| {
        panic!("failed to {label}: {error}");
    });
    assert!(
        output.status.success(),
        "failed to {label}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
