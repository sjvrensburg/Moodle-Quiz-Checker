//! Moodle XML quiz export parser.
//!
//! Converts a Moodle "Moodle XML" quiz/question bank export into our internal
//! [`Quiz`] / [`Question`] model. Handles the standard question types
//! (multichoice, truefalse, shortanswer, numerical, matching, cloze, essay,
//! description) plus `category` pseudo-questions, which are recorded onto the
//! questions that follow them (matching Moodle's own import behaviour).

use crate::model::*;
use crate::xmltree::{self, Node};
use regex::Regex;
use std::sync::OnceLock;

pub fn parse_quiz_xml(xml: &str, name: &str, source_file: Option<String>) -> Result<Quiz, String> {
    parse_quiz_xml_with_warnings(xml, name, source_file).map(|(quiz, _)| quiz)
}

/// Like [`parse_quiz_xml`], but also reports what was *not* imported: every
/// question whose `type` this parser doesn't support is returned as a warning
/// (naming the question and its type) instead of being dropped silently.
pub fn parse_quiz_xml_with_warnings(
    xml: &str,
    name: &str,
    source_file: Option<String>,
) -> Result<(Quiz, Vec<String>), String> {
    let root = xmltree::parse(xml)?;
    let quiz_root = if root.name == "quiz" {
        &root
    } else {
        root.child("quiz").unwrap_or(&root)
    };

    let mut questions = Vec::new();
    let mut warnings = Vec::new();
    let mut current_category: Option<String> = None;

    for q_node in quiz_root.children_named("question") {
        let qtype_attr = q_node.attr("type").unwrap_or("").to_string();
        if qtype_attr == "category" {
            current_category = q_node.text_of("category");
            continue;
        }

        if let Some(mut question) = parse_question(q_node, &qtype_attr) {
            question.category = current_category.clone();
            questions.push(question);
        } else {
            let qname = q_node.text_of("name").unwrap_or_else(|| "(unnamed)".to_string());
            warnings.push(format!(
                "Dropped question '{qname}': unsupported type '{qtype_attr}' (supported: multichoice, truefalse, shortanswer, numerical, matching, cloze, essay, description)"
            ));
        }
    }

    Ok((
        Quiz {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            source_file,
            questions,
            imported_at: chrono::Utc::now().to_rfc3339(),
        },
        warnings,
    ))
}

fn text_format_of(node: &Node, child: &str) -> TextFormat {
    node.child(child)
        .and_then(|n| n.attr("format"))
        .map(parse_format)
        .unwrap_or_default()
}

fn parse_format(s: &str) -> TextFormat {
    match s {
        "html" => TextFormat::Html,
        "plain_text" | "plain" => TextFormat::Plain,
        "markdown" => TextFormat::Markdown,
        "moodle_auto_format" | "moodle" => TextFormat::Moodle,
        _ => TextFormat::Html,
    }
}

fn parse_bool(s: Option<String>) -> bool {
    matches!(
        s.as_deref().map(|s| s.trim()),
        Some("1") | Some("true") | Some("TRUE") | Some("True")
    )
}

fn parse_question(node: &Node, qtype_attr: &str) -> Option<Question> {
    let qtype = match qtype_attr {
        "multichoice" => QuestionType::MultiChoice,
        "truefalse" => QuestionType::TrueFalse,
        "shortanswer" => QuestionType::ShortAnswer,
        "numerical" => QuestionType::Numerical,
        "matching" => QuestionType::Matching,
        "cloze" => QuestionType::Cloze,
        "essay" => QuestionType::Essay,
        "description" => QuestionType::Description,
        _ => QuestionType::Unsupported,
    };

    let mut q = Question::new(qtype);
    q.name = node.text_of("name").unwrap_or_default();
    q.question_text = node.text_of("questiontext").unwrap_or_default();
    q.question_text_format = text_format_of(node, "questiontext");
    q.general_feedback = node.text_of("generalfeedback").filter(|s| !s.trim().is_empty());
    q.default_grade = node
        .direct_text_of("defaultgrade")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0);
    q.penalty = node
        .direct_text_of("penalty")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    match qtype {
        QuestionType::MultiChoice => parse_multichoice(node, &mut q),
        QuestionType::TrueFalse => parse_truefalse(node, &mut q),
        QuestionType::ShortAnswer => parse_shortanswer(node, &mut q),
        QuestionType::Numerical => parse_numerical(node, &mut q),
        QuestionType::Matching => parse_matching(node, &mut q),
        QuestionType::Cloze => parse_cloze(node, &mut q),
        QuestionType::Essay => parse_essay(node, &mut q),
        QuestionType::Description => {}
        QuestionType::Unsupported => return None,
    }

    q.files = node
        .find_all("file")
        .into_iter()
        .filter_map(|f| {
            let name = f.attr("name")?.to_string();
            let data_base64 = f.text.trim().to_string();
            if data_base64.is_empty() {
                return None;
            }
            Some(QuestionFile { name, data_base64 })
        })
        .collect();

    if !q.files.is_empty() {
        rewrite_pluginfile_links(&mut q.question_text, &q.files);
        if let Some(fb) = q.general_feedback.as_mut() {
            rewrite_pluginfile_links(fb, &q.files);
        }
    }

    Some(q)
}

/// Rewrites Moodle's `@@PLUGINFILE@@/name` placeholder links (which only
/// resolve inside a live Moodle instance) into local same-page anchors that
/// point at the matching entry in the question's rendered attachments list.
fn rewrite_pluginfile_links(text: &mut String, files: &[QuestionFile]) {
    for file in files {
        let placeholder = format!("@@PLUGINFILE@@/{}", file.name);
        let anchor = format!("#attachment-{}", attachment_slug(&file.name));
        *text = text.replace(&placeholder, &anchor);
    }
}

pub fn attachment_slug(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '-' })
        .collect()
}

fn parse_answer_node(a: &Node) -> Answer {
    Answer {
        id: uuid::Uuid::new_v4().to_string(),
        text: a.own_text(),
        format: a.attr("format").map(parse_format).unwrap_or_default(),
        fraction: a
            .attr("fraction")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0),
        feedback: a.text_of("feedback").filter(|s| !s.trim().is_empty()),
    }
}

fn parse_multichoice(node: &Node, q: &mut Question) {
    q.single = node
        .direct_text_of("single")
        .map(|s| parse_bool(Some(s)))
        .unwrap_or(true);
    q.shuffle_answers = node
        .direct_text_of("shuffleanswers")
        .map(|s| parse_bool(Some(s)))
        .unwrap_or(false);
    q.correct_feedback = node.text_of("correctfeedback").filter(|s| !s.trim().is_empty());
    q.partially_correct_feedback = node
        .text_of("partiallycorrectfeedback")
        .filter(|s| !s.trim().is_empty());
    q.incorrect_feedback = node.text_of("incorrectfeedback").filter(|s| !s.trim().is_empty());
    q.answers = node.children_named("answer").map(parse_answer_node).collect();
}

fn parse_truefalse(node: &Node, q: &mut Question) {
    q.single = true;
    q.answers = node.children_named("answer").map(parse_answer_node).collect();
}

fn parse_shortanswer(node: &Node, q: &mut Question) {
    q.case_sensitive = node
        .direct_text_of("usecase")
        .map(|s| parse_bool(Some(s)))
        .unwrap_or(false);
    q.answers = node.children_named("answer").map(parse_answer_node).collect();
}

fn parse_numerical(node: &Node, q: &mut Question) {
    for a in node.children_named("answer") {
        let text = a.own_text();
        let value = text.trim().parse::<f64>().unwrap_or(0.0);
        let tolerance = a
            .direct_text_of("tolerance")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let fraction = a
            .attr("fraction")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let feedback = a.text_of("feedback").filter(|s| !s.trim().is_empty());
        q.numerical_answers.push(NumericalTolerance {
            value,
            tolerance,
            fraction,
            feedback,
        });
    }
    if let Some(units_node) = node.child("units") {
        for unit in units_node.children_named("unit") {
            let uname = unit.direct_text_of("unit_name").unwrap_or_default();
            let mult = unit
                .direct_text_of("multiplier")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0);
            q.numerical_units.push((uname, mult));
        }
    }
}

fn parse_matching(node: &Node, q: &mut Question) {
    q.shuffle_answers = node
        .direct_text_of("shuffleanswers")
        .map(|s| parse_bool(Some(s)))
        .unwrap_or(true);
    for sub in node.children_named("subquestion") {
        let question_text = sub.own_text();
        let answer_text = sub.child("answer").map(|a| a.own_text()).unwrap_or_default();
        if question_text.trim().is_empty() && answer_text.trim().is_empty() {
            continue;
        }
        q.match_pairs.push(MatchPair {
            id: uuid::Uuid::new_v4().to_string(),
            question_text,
            answer_text,
        });
    }
}

fn parse_essay(node: &Node, q: &mut Question) {
    q.essay_response_format = node.direct_text_of("responseformat");
    q.essay_lines = node
        .direct_text_of("responsefieldlines")
        .and_then(|s| s.parse::<u32>().ok());
}

fn cloze_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\{(\d*):([A-Za-z_]+):([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}").unwrap()
    })
}

fn cloze_option_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Splits on `~` at top level, e.g. `%100%Paris#Correct~%0%London#Wrong`
    RE.get_or_init(|| Regex::new(r"~").unwrap())
}

/// Parses embedded cloze markers out of the question text, e.g.
/// `{1:MULTICHOICE:%100%Paris#Correct~%0%London#Wrong}`
/// `{1:SHORTANSWER:=Paris#Correct~%50%London#Half}`
/// `{1:NUMERICAL:=5:0.5#Correct}`
fn parse_cloze(_node: &Node, q: &mut Question) {
    let text = q.question_text.clone();
    let mut index = 0u32;
    for caps in cloze_regex().captures_iter(&text) {
        index += 1;
        let kind_raw = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_uppercase();
        let body = caps.get(3).map(|m| m.as_str()).unwrap_or("");

        let kind = match kind_raw.as_str() {
            "MC" | "MULTICHOICE" => ClozeKind::MultichoiceInline,
            "MCV" | "MCVS" | "MULTICHOICE_V" | "MULTICHOICE_VS" | "MULTICHOICE_H" | "MULTICHOICE_HS" => {
                ClozeKind::MultichoiceDropdown
            }
            "SA" | "SHORTANSWER" => ClozeKind::ShortAnswer,
            "SAC" | "SHORTANSWER_C" => ClozeKind::ShortAnswerCaseSensitive,
            "NM" | "NUMERICAL" => ClozeKind::Numerical,
            _ => ClozeKind::ShortAnswer,
        };

        let options = parse_cloze_options(body, kind);

        q.cloze_items.push(ClozeItem {
            id: uuid::Uuid::new_v4().to_string(),
            index,
            kind,
            options,
        });
    }
}

fn parse_cloze_options(body: &str, kind: ClozeKind) -> Vec<Answer> {
    let parts: Vec<&str> = cloze_option_regex().split(body).collect();
    let mut out = Vec::new();
    for (i, raw) in parts.iter().enumerate() {
        let mut raw = raw.trim();
        if raw.is_empty() && i == 0 {
            continue;
        }
        // Leading '=' means fraction 100 for shortanswer/numerical style.
        let mut fraction = 100.0;
        if let Some(stripped) = raw.strip_prefix('=') {
            fraction = 100.0;
            raw = stripped;
        } else if let Some(stripped) = raw.strip_prefix('%') {
            if let Some(end) = stripped.find('%') {
                if let Ok(f) = stripped[..end].parse::<f64>() {
                    fraction = f;
                }
                raw = &stripped[end + 1..];
            }
        } else if i > 0 || matches!(kind, ClozeKind::MultichoiceInline | ClozeKind::MultichoiceDropdown) {
            // No explicit weight and not the first "correct" marker -> wrong answer.
            fraction = 0.0;
        }

        let (answer_part, feedback_part) = match raw.split_once('#') {
            Some((a, f)) => (a, Some(f.to_string())),
            None => (raw, None),
        };

        if matches!(kind, ClozeKind::Numerical) {
            // NUMERICAL body: `value:tolerance#feedback`
            let (value_str, _tolerance_str) = answer_part.split_once(':').unwrap_or((answer_part, "0"));
            out.push(Answer {
                id: uuid::Uuid::new_v4().to_string(),
                text: value_str.trim().to_string(),
                format: TextFormat::Plain,
                fraction,
                feedback: feedback_part,
            });
        } else {
            out.push(Answer {
                id: uuid::Uuid::new_v4().to_string(),
                text: answer_part.trim().to_string(),
                format: TextFormat::Html,
                fraction,
                feedback: feedback_part,
            });
        }
    }
    out
}
