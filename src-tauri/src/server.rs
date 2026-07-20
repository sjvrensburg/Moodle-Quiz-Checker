//! Local-only HTTP agent interface.
//!
//! Binds to 127.0.0.1 only (never 0.0.0.0) so external agentic tools running on
//! the same machine (e.g. a CLI-driven LLM agent) can list quizzes, drive an
//! attempt, and read back grading/feedback without embedding any LLM in this
//! app itself.

use crate::core::App;
use crate::model::*;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as AxumResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

type SharedApp = Arc<App>;

pub async fn run(app: SharedApp, port: u16) -> anyhow::Result<()> {
    let router = build_router(app);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("moodle-quiz-tester agent server listening on http://{addr}");
    axum::serve(listener, router).await?;
    Ok(())
}

pub fn build_router(app: SharedApp) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/quizzes", get(list_quizzes).post(import_quiz))
        .route("/quizzes/:quiz_id", get(get_quiz).delete(delete_quiz))
        .route("/quizzes/:quiz_id/attempts", get(list_attempts).post(start_attempt))
        .route("/attempts/:attempt_id", get(get_attempt))
        .route("/attempts/:attempt_id/responses/:question_id", post(submit_response))
        .route("/attempts/:attempt_id/flag/:question_id", post(set_flag))
        .route("/attempts/:attempt_id/finish", post(finish_attempt))
        .route("/attempts/:attempt_id/export.json", get(export_json))
        .route("/attempts/:attempt_id/export.md", get(export_markdown))
        .route("/lint", post(lint))
        .route("/autotest", post(autotest_xml))
        .route("/quizzes/:quiz_id/autotest", get(autotest_quiz))
        .route("/compare", post(compare))
        .route("/quizzes/:quiz_id/reviewer.md", get(export_quiz_markdown))
        .route(
            "/quizzes/:quiz_id/questions/:question_id/render.html",
            get(render_question_html),
        )
        .layer(CorsLayer::permissive())
        .with_state(app)
}

fn err_response(e: anyhow::Error) -> AxumResponse {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct ImportBody {
    xml: String,
    name: String,
    source_file: Option<String>,
}

async fn import_quiz(State(app): State<SharedApp>, Json(body): Json<ImportBody>) -> AxumResponse {
    match app.import_quiz(&body.xml, &body.name, body.source_file) {
        Ok(quiz) => Json(quiz).into_response(),
        Err(e) => err_response(e),
    }
}

async fn list_quizzes(State(app): State<SharedApp>) -> AxumResponse {
    match app.list_quizzes() {
        Ok(quizzes) => Json(quizzes).into_response(),
        Err(e) => err_response(e),
    }
}

async fn get_quiz(State(app): State<SharedApp>, Path(quiz_id): Path<String>) -> AxumResponse {
    match app.get_quiz(&quiz_id) {
        Ok(quiz) => Json(quiz).into_response(),
        Err(e) => err_response(e),
    }
}

async fn delete_quiz(State(app): State<SharedApp>, Path(quiz_id): Path<String>) -> AxumResponse {
    match app.delete_quiz(&quiz_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize, Default)]
struct StartAttemptBody {
    #[serde(default)]
    shuffle: bool,
}

async fn start_attempt(
    State(app): State<SharedApp>,
    Path(quiz_id): Path<String>,
    body: Option<Json<StartAttemptBody>>,
) -> AxumResponse {
    let shuffle = body.map(|b| b.0.shuffle).unwrap_or(false);
    match app.start_attempt(&quiz_id, shuffle) {
        Ok(attempt) => Json(attempt).into_response(),
        Err(e) => err_response(e),
    }
}

async fn list_attempts(State(app): State<SharedApp>, Path(quiz_id): Path<String>) -> AxumResponse {
    match app.list_attempts(&quiz_id) {
        Ok(attempts) => Json(attempts).into_response(),
        Err(e) => err_response(e),
    }
}

async fn get_attempt(State(app): State<SharedApp>, Path(attempt_id): Path<String>) -> AxumResponse {
    match app.get_attempt(&attempt_id) {
        Ok(attempt) => Json(attempt).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct SubmitResponseBody {
    value: ResponseValue,
}

async fn submit_response(
    State(app): State<SharedApp>,
    Path((attempt_id, question_id)): Path<(String, String)>,
    Json(body): Json<SubmitResponseBody>,
) -> AxumResponse {
    match app.submit_response(&attempt_id, &question_id, body.value) {
        Ok(attempt) => Json(attempt).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct FlagBody {
    flagged: bool,
}

async fn set_flag(
    State(app): State<SharedApp>,
    Path((attempt_id, question_id)): Path<(String, String)>,
    Json(body): Json<FlagBody>,
) -> AxumResponse {
    match app.set_flag(&attempt_id, &question_id, body.flagged) {
        Ok(attempt) => Json(attempt).into_response(),
        Err(e) => err_response(e),
    }
}

async fn finish_attempt(State(app): State<SharedApp>, Path(attempt_id): Path<String>) -> AxumResponse {
    match app.finish_attempt(&attempt_id) {
        Ok(attempt) => Json(attempt).into_response(),
        Err(e) => err_response(e),
    }
}

async fn export_json(State(app): State<SharedApp>, Path(attempt_id): Path<String>) -> AxumResponse {
    match app.export_json(&attempt_id) {
        Ok(json) => Json(json).into_response(),
        Err(e) => err_response(e),
    }
}

async fn export_markdown(State(app): State<SharedApp>, Path(attempt_id): Path<String>) -> AxumResponse {
    match app.export_markdown(&attempt_id) {
        Ok(md) => ([("content-type", "text/markdown; charset=utf-8")], md).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct XmlBody {
    xml: String,
}

async fn lint(Json(body): Json<XmlBody>) -> AxumResponse {
    match App::lint_xml(&body.xml) {
        Ok(report) => Json(report).into_response(),
        Err(e) => err_response(e),
    }
}

async fn autotest_xml(Json(body): Json<XmlBody>) -> AxumResponse {
    match App::autotest_xml(&body.xml) {
        Ok(report) => Json(report).into_response(),
        Err(e) => err_response(e),
    }
}

async fn autotest_quiz(State(app): State<SharedApp>, Path(quiz_id): Path<String>) -> AxumResponse {
    match app.autotest_quiz(&quiz_id) {
        Ok(report) => Json(report).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct CompareBody {
    /// Versions to compare: [{label, xml}, ...].
    sources: Vec<CompareSource>,
    #[serde(default)]
    group_by_name: bool,
}

#[derive(Deserialize)]
struct CompareSource {
    label: String,
    xml: String,
}

async fn compare(Json(body): Json<CompareBody>) -> AxumResponse {
    let sources: Vec<(String, String)> = body.sources.into_iter().map(|s| (s.label, s.xml)).collect();
    match App::compare_xml(&sources, body.group_by_name) {
        Ok(report) => Json(report).into_response(),
        Err(e) => err_response(e),
    }
}

async fn export_quiz_markdown(State(app): State<SharedApp>, Path(quiz_id): Path<String>) -> AxumResponse {
    match app.export_quiz_markdown(&quiz_id) {
        Ok(md) => ([("content-type", "text/markdown; charset=utf-8")], md).into_response(),
        Err(e) => err_response(e),
    }
}

async fn render_question_html(
    State(app): State<SharedApp>,
    Path((quiz_id, question_id)): Path<(String, String)>,
) -> AxumResponse {
    match app.render_question_html(&quiz_id, &question_id) {
        Ok(html) => ([("content-type", "text/html; charset=utf-8")], html).into_response(),
        Err(e) => err_response(e),
    }
}
