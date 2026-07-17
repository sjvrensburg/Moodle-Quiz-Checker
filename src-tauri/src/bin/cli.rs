//! Headless CLI for moodle-quiz-tester.
//!
//! Lets an external agentic AI (or a human) drive the whole import → attempt →
//! grade → export flow from the shell, and optionally launch the local agent
//! HTTP server for a longer-lived interactive session.

use clap::{Parser, Subcommand};
use moodle_quiz_tester_lib::core::App;
use moodle_quiz_tester_lib::model::ResponseValue;
use moodle_quiz_tester_lib::storage::Storage;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mqt-cli", version, about = "Headless CLI for moodle-quiz-tester")]
struct Cli {
    /// Path to the SQLite database. Defaults to the same file the desktop app uses.
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import a Moodle XML file into the local database.
    Load {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    /// List imported quizzes.
    List,
    /// Show full quiz detail (questions) as JSON.
    Show { quiz_id: String },
    /// Start a new attempt for a quiz.
    StartAttempt {
        quiz_id: String,
        #[arg(long)]
        shuffle: bool,
    },
    /// Submit a free-text/choice answer for one question in an attempt.
    Answer {
        attempt_id: String,
        question_id: String,
        /// Raw value. Plain text is treated as a text response; use --json for
        /// choice arrays / mapping responses (matching, cloze).
        value: String,
        #[arg(long)]
        json: bool,
    },
    /// Finish an attempt and print the graded result as JSON.
    Grade { attempt_id: String },
    /// Export a finished attempt.
    Export {
        attempt_id: String,
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,
    },
    /// Start the local agent HTTP server (foreground, Ctrl-C to stop).
    Serve {
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ExportFormat {
    Json,
    Markdown,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(App::default_db_path);
    let storage = Storage::open(&db_path)?;
    let app = std::sync::Arc::new(App::new(storage));

    match cli.command {
        Command::Load { path, name } => {
            let xml = std::fs::read_to_string(&path)?;
            let name = name.unwrap_or_else(|| {
                path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Untitled quiz".into())
            });
            let quiz = app.import_quiz(&xml, &name, Some(path.to_string_lossy().to_string()))?;
            println!("Imported '{}' ({} questions), id={}", quiz.name, quiz.questions.len(), quiz.id);
        }
        Command::List => {
            let quizzes = app.list_quizzes()?;
            for q in quizzes {
                println!("{}\t{}\t{} questions", q.id, q.name, q.questions.len());
            }
        }
        Command::Show { quiz_id } => {
            let quiz = app.get_quiz(&quiz_id)?;
            println!("{}", serde_json::to_string_pretty(&quiz)?);
        }
        Command::StartAttempt { quiz_id, shuffle } => {
            let attempt = app.start_attempt(&quiz_id, shuffle)?;
            println!("{}", serde_json::to_string_pretty(&attempt)?);
        }
        Command::Answer { attempt_id, question_id, value, json } => {
            let response_value: ResponseValue = if json {
                serde_json::from_str(&value)?
            } else {
                ResponseValue::Text(value)
            };
            let attempt = app.submit_response(&attempt_id, &question_id, response_value)?;
            println!("{}", serde_json::to_string_pretty(&attempt)?);
        }
        Command::Grade { attempt_id } => {
            let attempt = app.finish_attempt(&attempt_id)?;
            println!("{}", serde_json::to_string_pretty(&attempt)?);
        }
        Command::Export { attempt_id, format } => match format {
            ExportFormat::Json => println!("{}", serde_json::to_string_pretty(&app.export_json(&attempt_id)?)?),
            ExportFormat::Markdown => println!("{}", app.export_markdown(&attempt_id)?),
        },
        Command::Serve { port } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move { moodle_quiz_tester_lib::server::run(app, port).await })?;
        }
    }

    Ok(())
}
