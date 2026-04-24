use chrono::{TimeZone, Utc};
use rastraq::ranking::{rank_items, CandidateForRanking, InterestProfile};

#[test]
fn ranking_prefers_interest_matches_and_limits_daily_count() {
    let profile = InterestProfile {
        keywords: vec!["rust".into(), "security".into()],
        negative_keywords: vec!["sports".into()],
    };
    let candidates = vec![
        CandidateForRanking {
            id: 1,
            title: "Football result".into(),
            summary: "sports update".into(),
            source_type: "news".into(),
            published_at: Utc.with_ymd_and_hms(2026, 4, 23, 1, 0, 0).unwrap(),
            embedding: vec![0.2; 16],
        },
        CandidateForRanking {
            id: 2,
            title: "Rust security advisory".into(),
            summary: "A rust crate published a security fix".into(),
            source_type: "security_advisory".into(),
            published_at: Utc.with_ymd_and_hms(2026, 4, 23, 3, 0, 0).unwrap(),
            embedding: vec![0.8; 16],
        },
        CandidateForRanking {
            id: 3,
            title: "Database release".into(),
            summary: "Release notes for storage internals".into(),
            source_type: "release_note".into(),
            published_at: Utc.with_ymd_and_hms(2026, 4, 23, 2, 0, 0).unwrap(),
            embedding: vec![0.3; 16],
        },
    ];

    let ranked = rank_items(&candidates, &profile, 2);

    assert_eq!(ranked.iter().map(|item| item.item_id).collect::<Vec<_>>(), vec![2, 3]);
    assert!(ranked[0].score > ranked[1].score);
    assert!(ranked[0].features["keyword_hits"].as_i64().unwrap() >= 2);
}
