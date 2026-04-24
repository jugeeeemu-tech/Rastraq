use rastraq::llm::{DeterministicMockProvider, LlmProvider};

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
