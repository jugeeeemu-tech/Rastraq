use anyhow::Result;
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

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

#[derive(Clone)]
pub struct FastEmbedProvider {
    model: EmbeddingModel,
    model_name: &'static str,
    embedder: Arc<Mutex<Option<TextEmbedding>>>,
}

impl FastEmbedProvider {
    pub fn bge_small_en_v15() -> Self {
        Self {
            model: EmbeddingModel::BGESmallENV15,
            model_name: "BAAI/bge-small-en-v1.5",
            embedder: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl LlmProvider for FastEmbedProvider {
    async fn summarize_and_embed(&self, title: &str, content: &str) -> Result<ProcessedContent> {
        let (summary, key_points) = lightweight_summary(title, content);
        let input = format!("passage: {title}\n{content}");
        let model = self.model.clone();
        let embedder = Arc::clone(&self.embedder);
        let embedding = tokio::task::spawn_blocking(move || -> Result<Vec<f32>> {
            let mut guard = embedder.lock().expect("fastembed mutex poisoned");
            if guard.is_none() {
                *guard = Some(TextEmbedding::try_new(InitOptions::new(model))?);
            }
            let embeddings = guard
                .as_mut()
                .expect("fastembed initialized")
                .embed(vec![input], None)?;
            Ok(embeddings.into_iter().next().unwrap_or_default())
        })
        .await??;

        Ok(ProcessedContent {
            provider: "fastembed".to_string(),
            model: self.model_name.to_string(),
            summary,
            key_points,
            embedding,
        })
    }
}

#[derive(Debug, Default)]
pub struct DeterministicMockProvider;

#[async_trait]
impl LlmProvider for DeterministicMockProvider {
    async fn summarize_and_embed(&self, title: &str, content: &str) -> Result<ProcessedContent> {
        let (summary, key_points) = lightweight_summary(title, content);
        let mut embedding = Vec::with_capacity(16);
        let digest = Sha256::digest(format!("{title}\n{content}").as_bytes());
        for pair in digest.chunks_exact(2).take(16) {
            let value = u16::from_be_bytes([pair[0], pair[1]]) as f32 / u16::MAX as f32;
            embedding.push((value * 2.0) - 1.0);
        }

        Ok(ProcessedContent {
            provider: "deterministic-mock".to_string(),
            model: "mock-summary-embedding-v1".to_string(),
            summary,
            key_points,
            embedding,
        })
    }
}

fn lightweight_summary(title: &str, content: &str) -> (String, Vec<String>) {
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let excerpt = compact.chars().take(180).collect::<String>();
    (format!("{title}: {excerpt}"), vec![excerpt])
}
