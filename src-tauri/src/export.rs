//! Result export: JSON and Markdown summaries of a graded attempt, plus a
//! quiz-level reviewer document (all questions with answer keys inline).

use crate::model::{Attempt, GradeState, Question, QuestionType, Quiz};
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

/// Quiz-level reviewer document: every question rendered with its answer key,
/// weights, tolerances, and feedback inline — for human moderation/sign-off,
/// as opposed to the per-attempt exports above.
pub fn quiz_to_markdown(quiz: &Quiz) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {} — reviewer copy (includes answer key)\n\n", quiz.name));
    let gradeable = quiz
        .questions
        .iter()
        .filter(|q| !matches!(q.qtype, QuestionType::Description))
        .count();
    let total_marks: f64 = quiz
        .questions
        .iter()
        .filter(|q| !matches!(q.qtype, QuestionType::Description))
        .map(|q| q.default_grade)
        .sum();
    out.push_str(&format!(
        "{} questions ({} gradeable, {:.1} marks total). Imported: {}\n\n---\n\n",
        quiz.questions.len(),
        gradeable,
        total_marks,
        quiz.imported_at
    ));

    for (i, q) in quiz.questions.iter().enumerate() {
        out.push_str(&format!("## Q{}. {} `[{}]`\n\n", i + 1, q.name, qtype_label(q.qtype)));
        if let Some(cat) = &q.category {
            out.push_str(&format!("Category: `{}`  \n", cat));
        }
        out.push_str(&format!("Marks: {:.1}\n\n", q.default_grade));
        out.push_str(&format!("{}\n\n", strip_html(&q.question_text)));

        if !q.files.is_empty() {
            let names: Vec<&str> = q.files.iter().map(|f| f.name.as_str()).collect();
            out.push_str(&format!("**Attachments:** {}\n\n", names.join(", ")));
        }

        match q.qtype {
            QuestionType::MultiChoice | QuestionType::TrueFalse | QuestionType::ShortAnswer => {
                render_answer_table(&mut out, q);
            }
            QuestionType::Numerical => {
                out.push_str("**Accepted values:**\n\n");
                for a in &q.numerical_answers {
                    out.push_str(&format!(
                        "- {} ± {} ({:.0}%){}\n",
                        a.value,
                        a.tolerance,
                        a.fraction,
                        a.feedback
                            .as_deref()
                            .map(|f| format!(" — feedback: {}", strip_html(f)))
                            .unwrap_or_default()
                    ));
                }
                if !q.numerical_units.is_empty() {
                    let units: Vec<String> = q
                        .numerical_units
                        .iter()
                        .map(|(u, m)| format!("{u} (×{m})"))
                        .collect();
                    out.push_str(&format!("\nUnits: {}\n", units.join(", ")));
                }
                out.push('\n');
            }
            QuestionType::Matching => {
                out.push_str("**Correct pairs:**\n\n");
                for p in &q.match_pairs {
                    out.push_str(&format!(
                        "- {} → **{}**\n",
                        strip_html(&p.question_text),
                        strip_html(&p.answer_text)
                    ));
                }
                out.push('\n');
            }
            QuestionType::Cloze => {
                for item in &q.cloze_items {
                    out.push_str(&format!("**Blank {} ({:?}):**\n\n", item.index, item.kind));
                    for o in &item.options {
                        let mark = if o.fraction >= 99.999 { "✓" } else if o.fraction > 0.0 { "◐" } else { "✗" };
                        out.push_str(&format!(
                            "- {mark} {} ({:.0}%){}\n",
                            strip_html(&o.text),
                            o.fraction,
                            o.feedback
                                .as_deref()
                                .map(|f| format!(" — feedback: {}", strip_html(f)))
                                .unwrap_or_default()
                        ));
                    }
                    out.push('\n');
                }
            }
            QuestionType::Essay => {
                out.push_str("_Essay — manually graded._\n\n");
            }
            QuestionType::Description | QuestionType::Unsupported => {}
        }

        let mut fb_lines = Vec::new();
        if let Some(f) = &q.general_feedback {
            fb_lines.push(format!("- General: {}", strip_html(f)));
        }
        if let Some(f) = &q.correct_feedback {
            fb_lines.push(format!("- If correct: {}", strip_html(f)));
        }
        if let Some(f) = &q.partially_correct_feedback {
            fb_lines.push(format!("- If partially correct: {}", strip_html(f)));
        }
        if let Some(f) = &q.incorrect_feedback {
            fb_lines.push(format!("- If incorrect: {}", strip_html(f)));
        }
        if !fb_lines.is_empty() {
            out.push_str("**Whole-question feedback:**\n\n");
            out.push_str(&fb_lines.join("\n"));
            out.push_str("\n\n");
        }
        out.push_str("---\n\n");
    }
    out
}

/// Renders a single question's *raw* HTML (question text, answers, feedback
/// — unstripped, unlike [`quiz_to_markdown`]) into a standalone HTML document
/// with the MathJax runtime wired up, so it can be opened in a browser (or
/// screenshotted by a browser-automation tool) to visually confirm math
/// actually renders. This is a text-vs-visual complement to the `lint` gate's
/// `math-delimiters` rule, which can only catch the one known textual failure
/// mode (literal `\[...\]`/`\(...\)` display-math delimiters that Moodle's
/// filter strips) and can't tell whether other LaTeX is malformed or whether
/// the exams→HTML conversion step introduced a broken form not present in the
/// source.
///
/// MathJax is loaded from a CDN rather than vendored, so viewing the
/// rendered result requires the browser opening the file to have network
/// access — generating the file itself stays fully offline.
pub fn question_to_standalone_html(q: &Question) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "<h1>{}</h1>\n<p class=\"meta\">Type: {} &middot; Marks: {:.1}</p>\n",
        html_escape::encode_text(&q.name),
        qtype_label(q.qtype),
        q.default_grade
    ));
    body.push_str("<section class=\"question-text\">\n");
    body.push_str(&q.question_text);
    body.push_str("\n</section>\n");

    if !q.answers.is_empty() {
        body.push_str("<section class=\"answers\">\n<h2>Options / accepted answers</h2>\n<ul>\n");
        for a in &q.answers {
            body.push_str(&format!(
                "<li><span class=\"fraction\">({:.0}%)</span> {}</li>\n",
                a.fraction, a.text
            ));
        }
        body.push_str("</ul>\n</section>\n");
    }

    for (label, fb) in [
        ("General feedback", &q.general_feedback),
        ("If correct", &q.correct_feedback),
        ("If partially correct", &q.partially_correct_feedback),
        ("If incorrect", &q.incorrect_feedback),
    ] {
        if let Some(fb) = fb {
            body.push_str(&format!(
                "<section class=\"feedback\"><h3>{label}</h3>\n{fb}\n</section>\n"
            ));
        }
    }

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script>
  window.MathJax = {{
    tex: {{
      inlineMath: [['\\(', '\\)'], ['$', '$']],
      displayMath: [['\\[', '\\]'], ['$$', '$$']]
    }}
  }};
</script>
<script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" defer></script>
<style>
  body {{ font-family: system-ui, sans-serif; max-width: 50rem; margin: 2rem auto; padding: 0 1rem; line-height: 1.5; }}
  .meta {{ color: #666; }}
  .answers ul {{ padding-left: 1.2rem; }}
  .fraction {{ color: #888; font-size: 0.9em; }}
  section {{ margin-top: 1.5rem; }}
</style>
</head>
<body>
{body}
</body>
</html>
"#,
        title = html_escape::encode_text(&q.name),
        body = body
    )
}

fn render_answer_table(out: &mut String, q: &Question) {
    out.push_str("**Options / accepted answers:**\n\n");
    for a in &q.answers {
        let mark = if a.fraction >= 99.999 { "✓" } else if a.fraction > 0.0 { "◐" } else { "✗" };
        out.push_str(&format!(
            "- {mark} {} ({:.0}%){}\n",
            strip_html(&a.text),
            a.fraction,
            a.feedback
                .as_deref()
                .map(|f| format!(" — feedback: {}", strip_html(f)))
                .unwrap_or_default()
        ));
    }
    out.push('\n');
}

fn qtype_label(t: QuestionType) -> &'static str {
    match t {
        QuestionType::MultiChoice => "multichoice",
        QuestionType::TrueFalse => "truefalse",
        QuestionType::ShortAnswer => "shortanswer",
        QuestionType::Numerical => "numerical",
        QuestionType::Matching => "matching",
        QuestionType::Cloze => "cloze",
        QuestionType::Essay => "essay",
        QuestionType::Description => "description",
        QuestionType::Unsupported => "unsupported",
    }
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
