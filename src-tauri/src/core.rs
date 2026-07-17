//! High-level application operations, independent of any particular frontend
//! (Tauri commands, CLI, or the local HTTP agent server all call through here).

use crate::grading;
use crate::model::*;
use crate::parser;
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct App {
    pub storage: Storage,
}

impl App {
    pub fn new(storage: Storage) -> Self {
        App { storage }
    }

    pub fn import_quiz(&self, xml: &str, name: &str, source_file: Option<String>) -> Result<Quiz> {
        let quiz = parser::parse_quiz_xml(xml, name, source_file).map_err(|e| anyhow!(e))?;
        if quiz.questions.is_empty() {
            return Err(anyhow!("No supported questions found in the XML file"));
        }
        self.storage.save_quiz(&quiz)?;
        Ok(quiz)
    }

    pub fn list_quizzes(&self) -> Result<Vec<Quiz>> {
        self.storage.list_quizzes()
    }

    pub fn get_quiz(&self, quiz_id: &str) -> Result<Quiz> {
        self.storage
            .get_quiz(quiz_id)?
            .ok_or_else(|| anyhow!("Quiz not found: {quiz_id}"))
    }

    pub fn delete_quiz(&self, quiz_id: &str) -> Result<()> {
        self.storage.delete_quiz(quiz_id)
    }

    pub fn start_attempt(&self, quiz_id: &str, shuffle: bool) -> Result<Attempt> {
        let quiz = self.get_quiz(quiz_id)?;
        let mut order: Vec<String> = quiz.questions.iter().map(|q| q.id.clone()).collect();
        if shuffle {
            order.shuffle(&mut thread_rng());
        }
        let attempt = Attempt {
            id: uuid::Uuid::new_v4().to_string(),
            quiz_id: quiz.id.clone(),
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            question_order: order,
            responses: std::collections::HashMap::new(),
            results: None,
            total_score: None,
            max_score: None,
        };
        self.storage.save_attempt(&attempt)?;
        Ok(attempt)
    }

    pub fn get_attempt(&self, attempt_id: &str) -> Result<Attempt> {
        self.storage
            .get_attempt(attempt_id)?
            .ok_or_else(|| anyhow!("Attempt not found: {attempt_id}"))
    }

    pub fn submit_response(&self, attempt_id: &str, question_id: &str, value: ResponseValue) -> Result<Attempt> {
        let mut attempt = self.get_attempt(attempt_id)?;
        if attempt.finished_at.is_some() {
            return Err(anyhow!("Attempt is already finished"));
        }
        attempt
            .responses
            .entry(question_id.to_string())
            .or_default()
            .value = value;
        self.storage.save_attempt(&attempt)?;
        Ok(attempt)
    }

    pub fn set_flag(&self, attempt_id: &str, question_id: &str, flagged: bool) -> Result<Attempt> {
        let mut attempt = self.get_attempt(attempt_id)?;
        attempt
            .responses
            .entry(question_id.to_string())
            .or_default()
            .flagged = flagged;
        self.storage.save_attempt(&attempt)?;
        Ok(attempt)
    }

    pub fn finish_attempt(&self, attempt_id: &str) -> Result<Attempt> {
        let mut attempt = self.get_attempt(attempt_id)?;
        let quiz = self.get_quiz(&attempt.quiz_id)?;
        let (results, total, max_total) = grading::grade_attempt(&quiz, &attempt);
        attempt.results = Some(results);
        attempt.total_score = Some(total);
        attempt.max_score = Some(max_total);
        attempt.finished_at = Some(chrono::Utc::now().to_rfc3339());
        self.storage.save_attempt(&attempt)?;
        Ok(attempt)
    }

    pub fn list_attempts(&self, quiz_id: &str) -> Result<Vec<Attempt>> {
        self.storage.list_attempts_for_quiz(quiz_id)
    }

    pub fn export_json(&self, attempt_id: &str) -> Result<serde_json::Value> {
        let attempt = self.get_attempt(attempt_id)?;
        let quiz = self.get_quiz(&attempt.quiz_id)?;
        Ok(crate::export::attempt_to_json(&quiz, &attempt))
    }

    pub fn export_markdown(&self, attempt_id: &str) -> Result<String> {
        let attempt = self.get_attempt(attempt_id)?;
        let quiz = self.get_quiz(&attempt.quiz_id)?;
        Ok(crate::export::attempt_to_markdown(&quiz, &attempt))
    }

    /// Default DB path: `<data-dir>/moodle-quiz-tester/quizzes.sqlite3`.
    pub fn default_db_path() -> std::path::PathBuf {
        let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
        base.join("moodle-quiz-tester").join("quizzes.sqlite3")
    }
}
