use anyhow::Result;
use async_trait::async_trait;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessedContent {
    pub provider: String,
    pub model: String,
    pub summary: String,
    pub key_points: Vec<String>,
    pub embedding: Vec<f32>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn summarize_and_embed(&self, title: &str, content: &str) -> Result<ProcessedContent>;
}

#[derive(Debug, Default)]
pub struct DeterministicMockProvider;

#[async_trait]
impl LlmProvider for DeterministicMockProvider {
    async fn summarize_and_embed(&self, title: &str, content: &str) -> Result<ProcessedContent> {
        let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
        let excerpt = compact.chars().take(180).collect::<String>();
        let mut embedding = Vec::with_capacity(16);
        let digest = Sha256::digest(format!("{title}\n{content}").as_bytes());
        for pair in digest.chunks_exact(2).take(16) {
            let value = u16::from_be_bytes([pair[0], pair[1]]) as f32 / u16::MAX as f32;
            embedding.push((value * 2.0) - 1.0);
        }

        Ok(ProcessedContent {
            provider: "deterministic-mock".to_string(),
            model: "mock-summary-embedding-v1".to_string(),
            summary: format!("{title}: {excerpt}"),
            key_points: vec![excerpt],
            embedding,
        })
    }
}
