//! SQLite-backed persistence for quizzes and attempts.
//!
//! Quizzes and attempts are stored as JSON blobs keyed by UUID. This keeps the
//! schema trivially forward-compatible as the question model grows, while still
//! giving us transactional local storage and simple indexed lookups.

use crate::model::{Attempt, Quiz};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct Storage {
    pub conn: Mutex<Connection>,
}

impl Storage {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        init_schema(&conn)?;
        Ok(Storage { conn: Mutex::new(conn) })
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Storage { conn: Mutex::new(conn) })
    }

    pub fn save_quiz(&self, quiz: &Quiz) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let json = serde_json::to_string(quiz)?;
        conn.execute(
            "INSERT INTO quizzes (id, name, imported_at, data) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, data = excluded.data",
            params![quiz.id, quiz.name, quiz.imported_at, json],
        )?;
        Ok(())
    }

    pub fn list_quizzes(&self) -> anyhow::Result<Vec<Quiz>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM quizzes ORDER BY imported_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            let json = row?;
            out.push(serde_json::from_str(&json)?);
        }
        Ok(out)
    }

    pub fn get_quiz(&self, id: &str) -> anyhow::Result<Option<Quiz>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM quizzes WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&json)?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_quiz(&self, id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM quizzes WHERE id = ?1", params![id])?;
        conn.execute("DELETE FROM attempts WHERE quiz_id = ?1", params![id])?;
        Ok(())
    }

    pub fn save_attempt(&self, attempt: &Attempt) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let json = serde_json::to_string(attempt)?;
        conn.execute(
            "INSERT INTO attempts (id, quiz_id, started_at, data) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![attempt.id, attempt.quiz_id, attempt.started_at, json],
        )?;
        Ok(())
    }

    pub fn get_attempt(&self, id: &str) -> anyhow::Result<Option<Attempt>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM attempts WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&json)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_attempts_for_quiz(&self, quiz_id: &str) -> anyhow::Result<Vec<Attempt>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM attempts WHERE quiz_id = ?1 ORDER BY started_at DESC")?;
        let rows = stmt.query_map(params![quiz_id], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            let json = row?;
            out.push(serde_json::from_str(&json)?);
        }
        Ok(out)
    }
}

fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS quizzes (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            imported_at TEXT NOT NULL,
            data TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS attempts (
            id TEXT PRIMARY KEY,
            quiz_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            data TEXT NOT NULL,
            FOREIGN KEY(quiz_id) REFERENCES quizzes(id)
        );
        CREATE INDEX IF NOT EXISTS idx_attempts_quiz ON attempts(quiz_id);
        ",
    )?;
    Ok(())
}
