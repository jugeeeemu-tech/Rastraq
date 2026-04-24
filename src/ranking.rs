use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestProfile {
    pub keywords: Vec<String>,
    pub negative_keywords: Vec<String>,
}

impl Default for InterestProfile {
    fn default() -> Self {
        Self {
            keywords: vec![
                "rust".to_string(),
                "security".to_string(),
                "ai".to_string(),
                "infrastructure".to_string(),
            ],
            negative_keywords: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateForRanking {
    pub id: i64,
    pub title: String,
    pub summary: String,
    pub source_type: String,
    pub published_at: DateTime<Utc>,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RankedItem {
    pub item_id: i64,
    pub score: f64,
    pub reason: String,
    pub features: Value,
}

pub fn rank_items(
    candidates: &[CandidateForRanking],
    profile: &InterestProfile,
    limit: usize,
) -> Vec<RankedItem> {
    let mut ranked = candidates
        .iter()
        .map(|candidate| score_candidate(candidate, profile))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.item_id.cmp(&right.item_id))
    });
    ranked.truncate(limit);
    ranked
}

fn score_candidate(candidate: &CandidateForRanking, profile: &InterestProfile) -> RankedItem {
    let haystack = format!("{} {}", candidate.title, candidate.summary).to_lowercase();
    let keyword_hits = profile
        .keywords
        .iter()
        .filter(|keyword| haystack.contains(&keyword.to_lowercase()))
        .count();
    let negative_hits = profile
        .negative_keywords
        .iter()
        .filter(|keyword| haystack.contains(&keyword.to_lowercase()))
        .count();
    let source_bonus = match candidate.source_type.as_str() {
        "security_advisory" => 1.2,
        "release_note" | "github_release" => 0.8,
        "technical_blog" | "paper" => 0.6,
        _ => 0.0,
    };
    let embedding_signal = if candidate.embedding.is_empty() {
        0.0
    } else {
        candidate.embedding.iter().map(|value| *value as f64).sum::<f64>()
            / candidate.embedding.len() as f64
    };
    let recency_signal = candidate.published_at.timestamp() as f64 / 100_000_000_000.0;
    let score =
        keyword_hits as f64 * 3.0 - negative_hits as f64 * 4.0 + source_bonus + embedding_signal
            + recency_signal;

    RankedItem {
        item_id: candidate.id,
        score,
        reason: if keyword_hits > 0 {
            format!("matched {keyword_hits} interest keyword(s)")
        } else {
            "ranked by source and embedding signals".to_string()
        },
        features: json!({
            "keyword_hits": keyword_hits,
            "negative_hits": negative_hits,
            "source_type": candidate.source_type,
            "source_bonus": source_bonus,
            "embedding_signal": embedding_signal,
            "recency_signal": recency_signal,
            "ranking_version": "explicit-v1"
        }),
    }
}
