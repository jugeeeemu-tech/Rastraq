use crate::{
    ranking::{CandidateForRanking, InterestProfile, RankedItem},
    time::previous_local_date,
};
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemView {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub source_type: String,
    pub published_at: DateTime<Utc>,
    pub summary: Option<String>,
    pub key_points: Value,
    pub score: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditionView {
    pub id: i64,
    pub target_date: NaiveDate,
    pub timezone: String,
    pub daily_limit: i64,
    pub items: Vec<ItemView>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewItem {
    pub url: String,
    pub title: String,
    pub source_type: String,
    pub published_at: DateTime<Utc>,
    pub raw_content: String,
}

impl Database {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        for statement in SCHEMA.split("\n\n").filter(|part| !part.trim().is_empty()) {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        sqlx::query(
            "insert or ignore into users (id, timezone, daily_limit) values (1, 'Asia/Tokyo', 5)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "insert or ignore into interest_profile_snapshots \
             (id, user_id, keywords_json, negative_keywords_json, source, created_at) \
             values (1, 1, ?, ?, 'seed', ?)",
        )
        .bind(json!(InterestProfile::default().keywords).to_string())
        .bind(json!(InterestProfile::default().negative_keywords).to_string())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_item(&self, input: NewItem) -> Result<i64> {
        let hash = content_hash(&input.url, &input.raw_content);
        let result = sqlx::query(
            "insert into items (url, title, source_type, published_at, discovered_at, raw_content, content_hash) \
             values (?, ?, ?, ?, ?, ?, ?) \
             on conflict(content_hash) do update set title = excluded.title \
             returning id",
        )
        .bind(input.url)
        .bind(input.title)
        .bind(input.source_type)
        .bind(input.published_at)
        .bind(Utc::now())
        .bind(input.raw_content)
        .bind(hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.get("id"))
    }

    pub async fn item_content(&self, item_id: i64) -> Result<(String, String)> {
        let row = sqlx::query("select title, raw_content from items where id = ?")
            .bind(item_id)
            .fetch_one(&self.pool)
            .await?;
        Ok((row.get("title"), row.get("raw_content")))
    }

    pub async fn save_processed(
        &self,
        item_id: i64,
        provider: &str,
        model: &str,
        summary: &str,
        key_points: &[String],
        embedding: &[f32],
    ) -> Result<()> {
        sqlx::query(
            "insert into item_summaries \
             (item_id, provider, model, summary, key_points_json, embedding_json, processed_at) \
             values (?, ?, ?, ?, ?, ?, ?) \
             on conflict(item_id) do update set \
             provider = excluded.provider, model = excluded.model, summary = excluded.summary, \
             key_points_json = excluded.key_points_json, embedding_json = excluded.embedding_json, \
             processed_at = excluded.processed_at",
        )
        .bind(item_id)
        .bind(provider)
        .bind(model)
        .bind(summary)
        .bind(json!(key_points).to_string())
        .bind(json!(embedding).to_string())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn generate_edition(&self, now: DateTime<Utc>) -> Result<EditionView> {
        let settings = self.user_settings().await?;
        let target_date = previous_local_date(now, &settings.timezone)?;
        let profile = self.interest_profile().await?;
        let candidates = self.candidates_for_date(target_date, &settings.timezone).await?;
        let ranked = crate::ranking::rank_items(&candidates, &profile, settings.daily_limit as usize);

        let edition_id = sqlx::query(
            "insert into daily_editions (user_id, target_date, timezone, daily_limit, generated_at) \
             values (1, ?, ?, ?, ?) \
             on conflict(user_id, target_date) do update set generated_at = daily_editions.generated_at \
             returning id",
        )
        .bind(target_date)
        .bind(&settings.timezone)
        .bind(settings.daily_limit)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?
        .get("id");

        let existing_count: i64 =
            sqlx::query("select count(*) as count from daily_edition_items where edition_id = ?")
                .bind(edition_id)
                .fetch_one(&self.pool)
                .await?
                .get("count");
        if existing_count == 0 {
            self.insert_ranked_items(edition_id, &ranked).await?;
        }
        self.edition_by_id(edition_id).await
    }

    pub async fn today_edition(&self, now: DateTime<Utc>) -> Result<Option<EditionView>> {
        let settings = self.user_settings().await?;
        let target_date = previous_local_date(now, &settings.timezone)?;
        self.edition_by_date(target_date).await
    }

    pub async fn edition_for_date(&self, target_date: NaiveDate) -> Result<Option<EditionView>> {
        self.edition_by_date(target_date).await
    }

    pub async fn record_feedback(
        &self,
        item_id: i64,
        event_type: &str,
        payload: Value,
    ) -> Result<i64> {
        let id = sqlx::query(
            "insert into feedback_events (user_id, item_id, event_type, payload_json, created_at) \
             values (1, ?, ?, ?, ?) returning id",
        )
        .bind(item_id)
        .bind(event_type)
        .bind(payload.to_string())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?
        .get("id");
        if event_type == "interested" {
            self.refresh_interest_from_feedback().await?;
        }
        Ok(id)
    }

    pub async fn interest_keywords(&self) -> Result<Value> {
        let profile = self.interest_profile().await?;
        Ok(json!({
            "keywords": profile.keywords,
            "negative_keywords": profile.negative_keywords,
            "source": "rastraq-feedback-v1"
        }))
    }

    async fn user_settings(&self) -> Result<UserSettings> {
        let row = sqlx::query("select timezone, daily_limit from users where id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(UserSettings {
            timezone: row.get("timezone"),
            daily_limit: row.get("daily_limit"),
        })
    }

    async fn interest_profile(&self) -> Result<InterestProfile> {
        let row = sqlx::query(
            "select keywords_json, negative_keywords_json from interest_profile_snapshots \
             where user_id = 1 order by id desc limit 1",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(InterestProfile {
            keywords: serde_json::from_str(row.get("keywords_json"))?,
            negative_keywords: serde_json::from_str(row.get("negative_keywords_json"))?,
        })
    }

    async fn candidates_for_date(
        &self,
        target_date: NaiveDate,
        timezone: &str,
    ) -> Result<Vec<CandidateForRanking>> {
        let rows = sqlx::query(
            "select items.id, items.title, items.source_type, items.published_at, \
             item_summaries.summary, item_summaries.embedding_json \
             from items join item_summaries on item_summaries.item_id = items.id",
        )
        .fetch_all(&self.pool)
        .await?;
        let tz: chrono_tz::Tz = timezone.parse()?;
        let mut candidates = Vec::new();
        for row in rows {
            let published_at: DateTime<Utc> = row.get("published_at");
            if published_at.with_timezone(&tz).date_naive() != target_date {
                continue;
            }
            let embedding_json: String = row.get("embedding_json");
            candidates.push(CandidateForRanking {
                id: row.get("id"),
                title: row.get("title"),
                source_type: row.get("source_type"),
                published_at,
                summary: row.get("summary"),
                embedding: serde_json::from_str(&embedding_json)?,
            });
        }
        Ok(candidates)
    }

    async fn insert_ranked_items(&self, edition_id: i64, ranked: &[RankedItem]) -> Result<()> {
        for (index, item) in ranked.iter().enumerate() {
            sqlx::query(
                "insert into daily_edition_items \
                 (edition_id, item_id, rank, score, reason, features_json) values (?, ?, ?, ?, ?, ?)",
            )
            .bind(edition_id)
            .bind(item.item_id)
            .bind(index as i64 + 1)
            .bind(item.score)
            .bind(&item.reason)
            .bind(item.features.to_string())
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn edition_by_date(&self, target_date: NaiveDate) -> Result<Option<EditionView>> {
        let row = sqlx::query(
            "select id from daily_editions where user_id = 1 and target_date = ? limit 1",
        )
        .bind(target_date)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => Ok(Some(self.edition_by_id(row.get("id")).await?)),
            None => Ok(None),
        }
    }

    async fn edition_by_id(&self, edition_id: i64) -> Result<EditionView> {
        let edition = sqlx::query(
            "select id, target_date, timezone, daily_limit from daily_editions where id = ?",
        )
        .bind(edition_id)
        .fetch_one(&self.pool)
        .await?;
        let item_rows = sqlx::query(
            "select items.id, items.url, items.title, items.source_type, items.published_at, \
             item_summaries.summary, item_summaries.key_points_json, \
             daily_edition_items.score, daily_edition_items.reason \
             from daily_edition_items \
             join items on items.id = daily_edition_items.item_id \
             left join item_summaries on item_summaries.item_id = items.id \
             where daily_edition_items.edition_id = ? order by daily_edition_items.rank asc",
        )
        .bind(edition_id)
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::new();
        for row in item_rows {
            let key_points: String = row.get("key_points_json");
            items.push(ItemView {
                id: row.get("id"),
                url: row.get("url"),
                title: row.get("title"),
                source_type: row.get("source_type"),
                published_at: row.get("published_at"),
                summary: row.get("summary"),
                key_points: serde_json::from_str(&key_points).unwrap_or_else(|_| json!([])),
                score: row.get("score"),
                reason: row.get("reason"),
            });
        }
        Ok(EditionView {
            id: edition.get("id"),
            target_date: edition.get("target_date"),
            timezone: edition.get("timezone"),
            daily_limit: edition.get("daily_limit"),
            items,
        })
    }

    async fn refresh_interest_from_feedback(&self) -> Result<()> {
        let rows = sqlx::query(
            "select items.title from feedback_events \
             join items on items.id = feedback_events.item_id \
             where feedback_events.event_type = 'interested' order by feedback_events.id desc limit 25",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut keywords = InterestProfile::default().keywords;
        for row in rows {
            for token in tokenize_keywords(row.get("title")) {
                if !keywords.iter().any(|existing| existing == &token) {
                    keywords.push(token);
                }
            }
        }
        sqlx::query(
            "insert into interest_profile_snapshots \
             (user_id, keywords_json, negative_keywords_json, source, created_at) values (1, ?, ?, ?, ?)",
        )
        .bind(json!(keywords).to_string())
        .bind(json!([] as [String; 0]).to_string())
        .bind("feedback")
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug)]
struct UserSettings {
    timezone: String,
    daily_limit: i64,
}

fn content_hash(url: &str, content: &str) -> String {
    let digest = Sha256::digest(format!("{url}\n{content}").as_bytes());
    format!("{digest:x}")
}

fn tokenize_keywords(title: &str) -> Vec<String> {
    title
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() >= 4)
        .map(|token| token.to_lowercase())
        .collect()
}

const SCHEMA: &str = r#"
create table if not exists users (
    id integer primary key,
    timezone text not null,
    daily_limit integer not null
);

create table if not exists items (
    id integer primary key autoincrement,
    url text not null,
    title text not null,
    source_type text not null,
    published_at text not null,
    discovered_at text not null,
    raw_content text not null,
    content_hash text not null unique
);

create table if not exists item_summaries (
    item_id integer primary key references items(id),
    provider text not null,
    model text not null,
    summary text not null,
    key_points_json text not null,
    embedding_json text not null,
    processed_at text not null
);

create table if not exists daily_editions (
    id integer primary key autoincrement,
    user_id integer not null references users(id),
    target_date text not null,
    timezone text not null,
    daily_limit integer not null,
    generated_at text not null,
    unique(user_id, target_date)
);

create table if not exists daily_edition_items (
    edition_id integer not null references daily_editions(id),
    item_id integer not null references items(id),
    rank integer not null,
    score real not null,
    reason text not null,
    features_json text not null,
    primary key (edition_id, item_id)
);

create table if not exists feedback_events (
    id integer primary key autoincrement,
    user_id integer not null references users(id),
    item_id integer not null references items(id),
    event_type text not null,
    payload_json text not null,
    created_at text not null
);

create table if not exists interest_profile_snapshots (
    id integer primary key autoincrement,
    user_id integer not null references users(id),
    keywords_json text not null,
    negative_keywords_json text not null,
    source text not null,
    created_at text not null
);
"#;
