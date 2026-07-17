//! Result export: JSON and Markdown summaries of a graded attempt.

use crate::model::{Attempt, GradeState, Quiz};
use serde_json::json;

pub fn attempt_to_json(quiz: &Quiz, attempt: &Attempt) -> serde_json::Value {
    json!({
        "quiz": { "id": quiz.id, "name": quiz.name },
        "attempt": attempt,
    })
}

pub fn attempt_to_markdown(quiz: &Quiz, attempt: &Attempt) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", quiz.name));
    if let (Some(total), Some(max)) = (attempt.total_score, attempt.max_score) {
        let pct = if max > 0.0 { total / max * 100.0 } else { 0.0 };
        out.push_str(&format!("**Score:** {:.2} / {:.2} ({:.1}%)\n\n", total, max, pct));
    }
    out.push_str(&format!("Attempt: `{}`  \n", attempt.id));
    out.push_str(&format!("Started: {}\n\n", attempt.started_at));
    if let Some(finished) = &attempt.finished_at {
        out.push_str(&format!("Finished: {}\n\n", finished));
    }
    out.push_str("---\n\n");

    let results = attempt.results.as_deref().unwrap_or(&[]);
    for (i, qid) in attempt.question_order.iter().enumerate() {
        let Some(question) = quiz.questions.iter().find(|q| &q.id == qid) else { continue };
        let result = results.iter().find(|r| &r.question_id == qid);

        out.push_str(&format!("## Q{}. {}\n\n", i + 1, question.name));
        out.push_str(&format!("{}\n\n", strip_html(&question.question_text)));

        if !question.files.is_empty() {
            let names: Vec<&str> = question.files.iter().map(|f| f.name.as_str()).collect();
            out.push_str(&format!("**Attachments:** {}\n\n", names.join(", ")));
        }

        if let Some(response) = attempt.responses.get(qid) {
            out.push_str(&format!("**Response:** {}\n\n", format_response(&response.value)));
        } else {
            out.push_str("**Response:** _(no answer)_\n\n");
        }

        if let Some(r) = result {
            let state_label = match r.state {
                GradeState::Correct => "Correct",
                GradeState::PartiallyCorrect => "Partially correct",
                GradeState::Incorrect => "Incorrect",
                GradeState::Ungraded => "Not graded",
            };
            out.push_str(&format!(
                "**Result:** {} ({:.2} / {:.2})\n\n",
                state_label, r.raw_grade, r.max_grade
            ));
            if let Some(fb) = &r.feedback {
                out.push_str(&format!("**Feedback:** {}\n\n", strip_html(fb)));
            }
        }
        out.push_str("---\n\n");
    }

    out
}

fn format_response(value: &crate::model::ResponseValue) -> String {
    use crate::model::ResponseValue::*;
    match value {
        Text(s) => s.clone(),
        Choices(v) => v.join(", "),
        Mapping(m) => m
            .iter()
            .map(|(k, v)| format!("{k} → {v}"))
            .collect::<Vec<_>>()
            .join("; "),
        Empty => "_(no answer)_".to_string(),
    }
}

fn strip_html(s: &str) -> String {
    let style_re = regex::Regex::new(r"(?is)<style\b[^>]*>.*?</style\s*>").unwrap();
    let script_re = regex::Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").unwrap();
    let without_style = style_re.replace_all(s, "");
    let without_blocks = script_re.replace_all(&without_style, "");
    let tag_re = regex::Regex::new(r"<[^>]*>").unwrap();
    html_escape::decode_html_entities(&tag_re.replace_all(&without_blocks, ""))
        .trim()
        .to_string()
}
