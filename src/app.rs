use crate::{
    db::{Database, NewItem},
    llm::{DeterministicMockProvider, LlmProvider},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Database,
    llm: Arc<dyn LlmProvider>,
}

pub fn build_router(db: Database) -> Router {
    let state = AppState {
        db,
        llm: Arc::new(DeterministicMockProvider),
    };
    Router::new()
        .route("/api/health", get(health))
        .route("/api/items", post(create_item))
        .route("/api/items/:id/process", post(process_item))
        .route("/api/editions/generate", post(generate_edition))
        .route("/api/editions", get(edition_by_date))
        .route("/api/editions/today", get(today_edition))
        .route("/api/feedback", post(record_feedback))
        .route("/api/interest-keywords", get(interest_keywords))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({"ok": true}))
}

async fn create_item(
    State(state): State<AppState>,
    Json(input): Json<NewItem>,
) -> Result<impl IntoResponse, ApiError> {
    let id = state.db.insert_item(input).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn process_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let (title, content) = state.db.item_content(id).await?;
    let processed = state.llm.summarize_and_embed(&title, &content).await?;
    state
        .db
        .save_processed(
            id,
            &processed.provider,
            &processed.model,
            &processed.summary,
            &processed.key_points,
            &processed.embedding,
        )
        .await?;
    Ok(Json(json!({"id": id, "summary": processed.summary})))
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    now: Option<DateTime<Utc>>,
}

async fn generate_edition(
    State(state): State<AppState>,
    Json(input): Json<GenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let edition = state.db.generate_edition(input.now.unwrap_or_else(Utc::now)).await?;
    Ok((StatusCode::CREATED, Json(edition)))
}

#[derive(Debug, Deserialize)]
struct TodayQuery {
    now: Option<DateTime<Utc>>,
}

async fn today_edition(
    State(state): State<AppState>,
    Query(query): Query<TodayQuery>,
) -> Result<Response, ApiError> {
    match state.db.today_edition(query.now.unwrap_or_else(Utc::now)).await? {
        Some(edition) => Ok(Json(edition).into_response()),
        None => Ok((StatusCode::NOT_FOUND, Json(json!({"error": "edition not generated"}))).into_response()),
    }
}

#[derive(Debug, Deserialize)]
struct EditionQuery {
    date: NaiveDate,
}

async fn edition_by_date(
    State(state): State<AppState>,
    Query(query): Query<EditionQuery>,
) -> Result<Response, ApiError> {
    match state.db.edition_for_date(query.date).await? {
        Some(edition) => Ok(Json(edition).into_response()),
        None => Ok((StatusCode::NOT_FOUND, Json(json!({"error": "edition not generated"}))).into_response()),
    }
}

#[derive(Debug, Deserialize)]
struct FeedbackRequest {
    item_id: i64,
    event_type: String,
    payload: Option<Value>,
}

async fn record_feedback(
    State(state): State<AppState>,
    Json(input): Json<FeedbackRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id = state
        .db
        .record_feedback(
            input.item_id,
            &input.event_type,
            input.payload.unwrap_or_else(|| json!({})),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn interest_keywords(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.db.interest_keywords().await?))
}

struct ApiError(anyhow::Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": self.0.to_string()})),
        )
            .into_response()
    }
}
