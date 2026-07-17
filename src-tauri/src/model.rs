//! Core domain model for Moodle quizzes, questions, and attempts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single answer choice (used by multichoice, truefalse, matching subquestions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub id: String,
    /// Raw HTML text of the answer, as authored in Moodle.
    pub text: String,
    pub format: TextFormat,
    /// Fraction of full credit this answer is worth, e.g. 100.0, 50.0, -100.0.
    pub fraction: f64,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextFormat {
    Html,
    Plain,
    Markdown,
    Moodle,
}

impl Default for TextFormat {
    fn default() -> Self {
        TextFormat::Html
    }
}

/// A matching sub-question: a stem paired with its correct match text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPair {
    pub id: String,
    pub question_text: String,
    pub answer_text: String,
}

/// A cloze (embedded answer) sub-item, parsed out of the {1:SHORTANSWER:...} syntax
/// embedded in the question text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClozeItem {
    pub id: String,
    pub index: u32,
    pub kind: ClozeKind,
    pub options: Vec<Answer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClozeKind {
    MultichoiceInline, // MC / MULTICHOICE
    MultichoiceDropdown, // MCVS / MULTICHOICE_V / MULTICHOICE_VS
    ShortAnswer,       // SA / SHORTANSWER
    ShortAnswerCaseSensitive, // SAC
    Numerical,         // NM / NUMERICAL
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    MultiChoice,
    TrueFalse,
    ShortAnswer,
    Numerical,
    Matching,
    Cloze,
    Essay,
    Description,
    Unsupported,
}

/// A file embedded in the question XML via Moodle's `<file encoding="base64">`
/// mechanism (e.g. a CSV dataset or script linked from the question text with
/// an `@@PLUGINFILE@@/name` URL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionFile {
    pub name: String,
    /// Base64-encoded file contents, as found in the XML.
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericalTolerance {
    pub value: f64,
    pub tolerance: f64,
    pub fraction: f64,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub qtype: QuestionType,
    pub name: String,
    pub question_text: String,
    pub question_text_format: TextFormat,
    pub general_feedback: Option<String>,
    pub default_grade: f64,
    pub penalty: f64,
    pub category: Option<String>,
    pub shuffle_answers: bool,
    pub single: bool, // multichoice: single vs multiple response

    // multichoice / truefalse
    pub answers: Vec<Answer>,

    // shortanswer
    pub case_sensitive: bool,

    // numerical
    pub numerical_answers: Vec<NumericalTolerance>,
    pub numerical_units: Vec<(String, f64)>,

    // matching
    pub match_pairs: Vec<MatchPair>,

    // cloze
    pub cloze_items: Vec<ClozeItem>,

    // feedback for whole-answer states (multichoice/truefalse/etc.)
    pub correct_feedback: Option<String>,
    pub partially_correct_feedback: Option<String>,
    pub incorrect_feedback: Option<String>,

    // essay
    pub essay_response_format: Option<String>,
    pub essay_lines: Option<u32>,

    // files embedded via @@PLUGINFILE@@ (datasets, scripts, images, ...)
    pub files: Vec<QuestionFile>,
}

impl Question {
    pub fn new(qtype: QuestionType) -> Self {
        Question {
            id: Uuid::new_v4().to_string(),
            qtype,
            name: String::new(),
            question_text: String::new(),
            question_text_format: TextFormat::Html,
            general_feedback: None,
            default_grade: 1.0,
            penalty: 0.0,
            category: None,
            shuffle_answers: false,
            single: true,
            answers: Vec::new(),
            case_sensitive: false,
            numerical_answers: Vec::new(),
            numerical_units: Vec::new(),
            match_pairs: Vec::new(),
            cloze_items: Vec::new(),
            correct_feedback: None,
            partially_correct_feedback: None,
            incorrect_feedback: None,
            essay_response_format: None,
            essay_lines: None,
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quiz {
    pub id: String,
    pub name: String,
    pub source_file: Option<String>,
    pub questions: Vec<Question>,
    pub imported_at: String,
}

/// A single learner response to one question within an attempt.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Response {
    /// For multichoice: selected answer id(s). For matching: "stemid=answerid" pairs.
    /// For cloze: "itemindex:value" pairs. For shortanswer/numerical/essay: free text.
    pub value: ResponseValue,
    pub flagged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseValue {
    Text(String),
    Choices(Vec<String>),
    Mapping(std::collections::BTreeMap<String, String>),
    Empty,
}

impl Default for ResponseValue {
    fn default() -> Self {
        ResponseValue::Empty
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResult {
    pub question_id: String,
    pub fraction: f64, // 0.0..=1.0 (can be negative if penalties push below 0, clamped)
    pub raw_grade: f64,
    pub max_grade: f64,
    pub feedback: Option<String>,
    pub state: GradeState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeState {
    Correct,
    PartiallyCorrect,
    Incorrect,
    Ungraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub id: String,
    pub quiz_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub question_order: Vec<String>,
    pub responses: std::collections::HashMap<String, Response>,
    pub results: Option<Vec<QuestionResult>>,
    pub total_score: Option<f64>,
    pub max_score: Option<f64>,
}
