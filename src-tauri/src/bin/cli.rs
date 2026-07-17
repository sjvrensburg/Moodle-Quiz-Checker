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
    /// Lint a Moodle XML file: format errors, grading traps, missing attachments,
    /// unsupported question types, and a random-guess score baseline.
    /// Exits non-zero if any errors are found.
    Lint {
        path: PathBuf,
        /// Emit the full report as JSON instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
    /// Answer-key round-trip test: submit the intended-correct answer for every
    /// auto-gradeable question and assert full marks; also submit a deliberately
    /// wrong answer and assert the grader discriminates. Exits non-zero on failure.
    Autotest {
        /// Quiz id of an already-imported quiz.
        #[arg(required_unless_present = "file", conflicts_with = "file")]
        quiz_id: Option<String>,
        /// Test a Moodle XML file directly without importing it.
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Compare randomised versions of the same items and flag answer keys that
    /// never vary. Pass several files (positional alignment) or one file
    /// containing all versions (grouped by question name). Exits non-zero when
    /// any constant answer column is flagged.
    Compare {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Group versions by question name instead of by position (default for a single file).
        #[arg(long)]
        group_by_name: bool,
        #[arg(long)]
        json: bool,
    },
    /// Export a reviewer document for a quiz: every question with its answer
    /// key, weights, tolerances, and feedback inline.
    ExportQuiz {
        quiz_id: String,
        #[arg(long, value_enum, default_value = "markdown")]
        format: ExportFormat,
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
            let (quiz, warnings) =
                app.import_quiz_with_warnings(&xml, &name, Some(path.to_string_lossy().to_string()))?;
            for w in &warnings {
                eprintln!("warning: {w}");
            }
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
        Command::Lint { path, json } => {
            let xml = std::fs::read_to_string(&path)?;
            let report = App::lint_xml(&xml)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", report.to_text());
            }
            if report.errors > 0 {
                std::process::exit(1);
            }
        }
        Command::Autotest { quiz_id, file, json } => {
            let report = match (quiz_id, file) {
                (_, Some(path)) => {
                    let xml = std::fs::read_to_string(&path)?;
                    App::autotest_xml(&xml)?
                }
                (Some(id), None) => app.autotest_quiz(&id)?,
                (None, None) => unreachable!("clap enforces quiz_id or --file"),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", report.to_text());
            }
            if !report.pass {
                std::process::exit(1);
            }
        }
        Command::Compare { files, group_by_name, json } => {
            let mut sources = Vec::new();
            for path in &files {
                let xml = std::fs::read_to_string(path)?;
                let label = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                sources.push((label, xml));
            }
            let report = App::compare_xml(&sources, group_by_name)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", report.to_text());
            }
            if report.flagged_items > 0 {
                std::process::exit(1);
            }
        }
        Command::ExportQuiz { quiz_id, format } => match format {
            ExportFormat::Json => {
                let quiz = app.get_quiz(&quiz_id)?;
                println!("{}", serde_json::to_string_pretty(&quiz)?);
            }
            ExportFormat::Markdown => println!("{}", app.export_quiz_markdown(&quiz_id)?),
        },
    }

    Ok(())
}
