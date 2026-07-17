//! Moodle-like grading engine.
//!
//! Computes a fraction (0.0..=1.0, occasionally negative before clamping) for a
//! learner [`Response`] against a [`Question`], mirroring the scoring rules used
//! by Moodle's own question engine for each supported question type.

use crate::model::*;
use std::collections::BTreeMap;

pub fn grade_question(question: &Question, response: Option<&Response>) -> QuestionResult {
    let max_grade = question.default_grade;
    let (fraction, state, feedback) = match question.qtype {
        QuestionType::MultiChoice => grade_multichoice(question, response),
        QuestionType::TrueFalse => grade_truefalse(question, response),
        QuestionType::ShortAnswer => grade_shortanswer(question, response),
        QuestionType::Numerical => grade_numerical(question, response),
        QuestionType::Matching => grade_matching(question, response),
        QuestionType::Cloze => grade_cloze(question, response),
        QuestionType::Essay => grade_essay(question, response),
        QuestionType::Description => (0.0, GradeState::Ungraded, None),
        QuestionType::Unsupported => (0.0, GradeState::Ungraded, None),
    };

    let clamped = fraction.clamp(0.0, 1.0);
    QuestionResult {
        question_id: question.id.clone(),
        fraction: clamped,
        raw_grade: clamped * max_grade,
        max_grade,
        feedback,
        state,
    }
}

fn state_for_fraction(fraction: f64) -> GradeState {
    if fraction >= 0.999999 {
        GradeState::Correct
    } else if fraction <= 0.000001 {
        GradeState::Incorrect
    } else {
        GradeState::PartiallyCorrect
    }
}

fn selected_choices(response: Option<&Response>) -> Vec<String> {
    match response.map(|r| &r.value) {
        Some(ResponseValue::Choices(v)) => v.clone(),
        Some(ResponseValue::Text(s)) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn grade_multichoice(q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    let selected = selected_choices(response);
    if selected.is_empty() {
        return (0.0, GradeState::Ungraded, None);
    }

    if q.single {
        let chosen_id = &selected[0];
        let answer = q.answers.iter().find(|a| &a.id == chosen_id);
        let fraction = answer.map(|a| a.fraction / 100.0).unwrap_or(0.0);
        let feedback = combine_feedback(
            answer.and_then(|a| a.feedback.clone()),
            overall_feedback(q, fraction),
        );
        (fraction, state_for_fraction(fraction), feedback)
    } else {
        // Multiple-response: sum positive fractions for selected correct answers,
        // sum (usually negative) fractions for selected incorrect answers, per Moodle's
        // "each choice's own weight" model. Total is clamped to [0,1] by the caller.
        let mut total = 0.0;
        let mut fb_parts = Vec::new();
        for id in &selected {
            if let Some(a) = q.answers.iter().find(|a| &a.id == id) {
                total += a.fraction / 100.0;
                if let Some(fb) = &a.feedback {
                    if !fb.trim().is_empty() {
                        fb_parts.push(fb.clone());
                    }
                }
            }
        }
        let feedback = combine_feedback(
            if fb_parts.is_empty() { None } else { Some(fb_parts.join("<br/>")) },
            overall_feedback(q, total),
        );
        (total, state_for_fraction(total.clamp(0.0, 1.0)), feedback)
    }
}

fn grade_truefalse(q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    grade_multichoice(q, response)
}

fn normalize_text(s: &str, case_sensitive: bool) -> String {
    let trimmed = s.trim();
    if case_sensitive {
        trimmed.to_string()
    } else {
        trimmed.to_lowercase()
    }
}

/// Moodle shortanswer supports simple `*` wildcards.
fn wildcard_match(pattern: &str, value: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == value;
    }
    let escaped_parts: Vec<String> = pattern.split('*').map(regex::escape).collect();
    let re_str = format!("^{}$", escaped_parts.join(".*"));
    regex::Regex::new(&re_str).map(|re| re.is_match(value)).unwrap_or(false)
}

fn grade_shortanswer(q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    let text = match response.map(|r| &r.value) {
        Some(ResponseValue::Text(s)) if !s.trim().is_empty() => s.clone(),
        _ => return (0.0, GradeState::Ungraded, None),
    };
    let normalized_response = normalize_text(&text, q.case_sensitive);

    let mut best: Option<(&Answer, f64)> = None;
    for a in &q.answers {
        let pattern = normalize_text(&a.text, q.case_sensitive);
        if wildcard_match(&pattern, &normalized_response) {
            let fraction = a.fraction / 100.0;
            if best.map(|(_, f)| fraction > f).unwrap_or(true) {
                best = Some((a, fraction));
            }
        }
    }

    match best {
        Some((a, fraction)) => {
            let feedback = combine_feedback(a.feedback.clone(), overall_feedback(q, fraction));
            (fraction, state_for_fraction(fraction), feedback)
        }
        None => (0.0, GradeState::Incorrect, overall_feedback(q, 0.0)),
    }
}

fn grade_numerical(q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    let text = match response.map(|r| &r.value) {
        Some(ResponseValue::Text(s)) if !s.trim().is_empty() => s.clone(),
        _ => return (0.0, GradeState::Ungraded, None),
    };

    // Strip a trailing recognized unit, if any (best-effort; unit multiplier applied).
    let mut numeric_part = text.trim().to_string();
    let mut multiplier = 1.0;
    for (unit, mult) in &q.numerical_units {
        if !unit.is_empty() && numeric_part.ends_with(unit.as_str()) {
            numeric_part = numeric_part[..numeric_part.len() - unit.len()].trim().to_string();
            multiplier = *mult;
            break;
        }
    }

    let value: f64 = match numeric_part.parse() {
        Ok(v) => v,
        Err(_) => return (0.0, GradeState::Incorrect, None),
    };
    let value = value * multiplier;

    let mut best: Option<(&NumericalTolerance, f64)> = None;
    for a in &q.numerical_answers {
        if (value - a.value).abs() <= a.tolerance.max(0.0) {
            let fraction = a.fraction / 100.0;
            if best.map(|(_, f)| fraction > f).unwrap_or(true) {
                best = Some((a, fraction));
            }
        }
    }

    match best {
        Some((a, fraction)) => {
            let feedback = combine_feedback(a.feedback.clone(), overall_feedback(q, fraction));
            (fraction, state_for_fraction(fraction), feedback)
        }
        None => (0.0, GradeState::Incorrect, overall_feedback(q, 0.0)),
    }
}

fn grade_matching(q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    let mapping = match response.map(|r| &r.value) {
        Some(ResponseValue::Mapping(m)) => m.clone(),
        _ => return (0.0, GradeState::Ungraded, None),
    };
    if q.match_pairs.is_empty() {
        return (0.0, GradeState::Ungraded, None);
    }

    let total = q.match_pairs.len();
    let mut correct = 0usize;
    for pair in &q.match_pairs {
        if let Some(chosen_answer_text) = mapping.get(&pair.id) {
            if chosen_answer_text.trim() == pair.answer_text.trim() {
                correct += 1;
            }
        }
    }
    let fraction = correct as f64 / total as f64;
    (fraction, state_for_fraction(fraction), overall_feedback(q, fraction))
}

fn grade_cloze(q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    let mapping = match response.map(|r| &r.value) {
        Some(ResponseValue::Mapping(m)) => m.clone(),
        _ => return (0.0, GradeState::Ungraded, None),
    };
    if q.cloze_items.is_empty() {
        return (0.0, GradeState::Ungraded, None);
    }

    let mut total_fraction = 0.0;
    let mut answered_any = false;
    for item in &q.cloze_items {
        let key = item.index.to_string();
        let given = mapping.get(&key).cloned().unwrap_or_default();
        if given.trim().is_empty() {
            continue;
        }
        answered_any = true;
        let item_fraction = best_cloze_option_fraction(item, &given);
        total_fraction += item_fraction;
    }

    if !answered_any {
        return (0.0, GradeState::Ungraded, None);
    }

    let fraction = total_fraction / q.cloze_items.len() as f64;
    (fraction, state_for_fraction(fraction), overall_feedback(q, fraction))
}

fn best_cloze_option_fraction(item: &ClozeItem, given: &str) -> f64 {
    match item.kind {
        ClozeKind::MultichoiceInline | ClozeKind::MultichoiceDropdown => item
            .options
            .iter()
            .find(|o| o.id == given || o.text == given)
            .map(|o| o.fraction / 100.0)
            .unwrap_or(0.0),
        ClozeKind::ShortAnswer => item
            .options
            .iter()
            .filter(|o| wildcard_match(&o.text.to_lowercase(), &given.trim().to_lowercase()))
            .map(|o| o.fraction / 100.0)
            .fold(0.0_f64, f64::max),
        ClozeKind::ShortAnswerCaseSensitive => item
            .options
            .iter()
            .filter(|o| wildcard_match(&o.text, given.trim()))
            .map(|o| o.fraction / 100.0)
            .fold(0.0_f64, f64::max),
        ClozeKind::Numerical => {
            let value: f64 = match given.trim().parse() {
                Ok(v) => v,
                Err(_) => return 0.0,
            };
            item.options
                .iter()
                .filter(|o| o.text.trim().parse::<f64>().map(|v| (v - value).abs() < 1e-9).unwrap_or(false))
                .map(|o| o.fraction / 100.0)
                .fold(0.0_f64, f64::max)
        }
    }
}

fn grade_essay(_q: &Question, response: Option<&Response>) -> (f64, GradeState, Option<String>) {
    // Essay questions require manual grading in Moodle; we mark them as
    // "ungraded" unless a response was given, in which case they stay ungraded
    // but are recorded as answered so the UI can show "submitted, pending review".
    let answered = matches!(response.map(|r| &r.value), Some(ResponseValue::Text(s)) if !s.trim().is_empty());
    if answered {
        (0.0, GradeState::Ungraded, Some("This essay response requires manual grading.".to_string()))
    } else {
        (0.0, GradeState::Ungraded, None)
    }
}

fn overall_feedback(q: &Question, fraction: f64) -> Option<String> {
    if fraction >= 0.999999 {
        q.correct_feedback.clone()
    } else if fraction <= 0.000001 {
        q.incorrect_feedback.clone()
    } else {
        q.partially_correct_feedback.clone()
    }
}

fn combine_feedback(specific: Option<String>, overall: Option<String>) -> Option<String> {
    let parts: Vec<String> = [specific, overall]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("<br/>"))
    }
}

/// Grades every question in a quiz for the given attempt, applying the quiz's
/// question order, and returns the aggregate score.
pub fn grade_attempt(quiz: &Quiz, attempt: &Attempt) -> (Vec<QuestionResult>, f64, f64) {
    let mut results = Vec::new();
    let mut total = 0.0;
    let mut max_total = 0.0;
    for qid in &attempt.question_order {
        if let Some(question) = quiz.questions.iter().find(|q| &q.id == qid) {
            let response = attempt.responses.get(qid);
            let result = grade_question(question, response);
            if !matches!(question.qtype, QuestionType::Description) {
                total += result.raw_grade;
                max_total += result.max_grade;
            }
            results.push(result);
        }
    }
    (results, total, max_total)
}

#[allow(dead_code)]
pub fn empty_mapping() -> BTreeMap<String, String> {
    BTreeMap::new()
}
