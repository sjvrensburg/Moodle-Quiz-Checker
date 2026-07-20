pub mod core;
pub mod export;
pub mod grading;
pub mod model;
pub mod parser;
pub mod quality;
pub mod server;
pub mod storage;
pub mod xmltree;

use crate::core::App;
use crate::model::*;
use std::sync::Arc;
use tauri::State;

pub struct AppState(pub Arc<App>);

#[tauri::command]
fn import_quiz_xml(
    state: State<AppState>,
    xml: String,
    name: String,
    source_file: Option<String>,
) -> Result<Quiz, String> {
    state.0.import_quiz(&xml, &name, source_file).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_quizzes(state: State<AppState>) -> Result<Vec<Quiz>, String> {
    state.0.list_quizzes().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_quiz(state: State<AppState>, quiz_id: String) -> Result<Quiz, String> {
    state.0.get_quiz(&quiz_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_quiz(state: State<AppState>, quiz_id: String) -> Result<(), String> {
    state.0.delete_quiz(&quiz_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_attempt(state: State<AppState>, quiz_id: String, shuffle: bool) -> Result<Attempt, String> {
    state.0.start_attempt(&quiz_id, shuffle).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_attempt(state: State<AppState>, attempt_id: String) -> Result<Attempt, String> {
    state.0.get_attempt(&attempt_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn submit_response(
    state: State<AppState>,
    attempt_id: String,
    question_id: String,
    value: ResponseValue,
) -> Result<Attempt, String> {
    state
        .0
        .submit_response(&attempt_id, &question_id, value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_flag(state: State<AppState>, attempt_id: String, question_id: String, flagged: bool) -> Result<Attempt, String> {
    state.0.set_flag(&attempt_id, &question_id, flagged).map_err(|e| e.to_string())
}

#[tauri::command]
fn finish_attempt(state: State<AppState>, attempt_id: String) -> Result<Attempt, String> {
    state.0.finish_attempt(&attempt_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_attempts(state: State<AppState>, quiz_id: String) -> Result<Vec<Attempt>, String> {
    state.0.list_attempts(&quiz_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_json(state: State<AppState>, attempt_id: String) -> Result<serde_json::Value, String> {
    state.0.export_json(&attempt_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_markdown(state: State<AppState>, attempt_id: String) -> Result<String, String> {
    state.0.export_markdown(&attempt_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn lint_xml(xml: String) -> Result<quality::LintReport, String> {
    App::lint_xml(&xml).map_err(|e| e.to_string())
}

#[tauri::command]
fn autotest_quiz(state: State<AppState>, quiz_id: String) -> Result<quality::AutotestReport, String> {
    state.0.autotest_quiz(&quiz_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_quiz_markdown(state: State<AppState>, quiz_id: String) -> Result<String, String> {
    state.0.export_quiz_markdown(&quiz_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn render_question_html(state: State<AppState>, quiz_id: String, question_id: String) -> Result<String, String> {
    state.0.render_question_html(&quiz_id, &question_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_agent_server(state: State<AppState>, port: u16) -> Result<String, String> {
    let app = state.0.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
        rt.block_on(async move {
            if let Err(e) = server::run(app, port).await {
                eprintln!("agent server error: {e}");
            }
        });
    });
    Ok(format!("http://127.0.0.1:{port}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let storage = storage::Storage::open(&App::default_db_path()).expect("failed to open database");
    let app_state = AppState(Arc::new(App::new(storage)));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            import_quiz_xml,
            list_quizzes,
            get_quiz,
            delete_quiz,
            start_attempt,
            get_attempt,
            submit_response,
            set_flag,
            finish_attempt,
            list_attempts,
            export_json,
            export_markdown,
            lint_xml,
            autotest_quiz,
            export_quiz_markdown,
            render_question_html,
            start_agent_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
