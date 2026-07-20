//! Quiz quality tooling.
//!
//! Three pre-import gates, all pure functions over Moodle XML / the parsed model:
//!
//! - [`lint_quiz_xml`]: mechanical format checks (missing correct answers,
//!   free-credit multi-response weighting, match-everything wildcards, broken
//!   `@@PLUGINFILE@@` attachments, stripped math delimiters, malformed cloze
//!   items, unsupported question types) plus an analytic random-guess baseline
//!   per question.
//! - [`autotest_quiz`]: answer-key round-trip — synthesizes the intended-correct
//!   response for every auto-gradeable question straight from the parsed model,
//!   grades it, and asserts full marks; also synthesizes a deliberately wrong
//!   response and asserts the grader discriminates.
//! - [`compare_quizzes`]: multi-version diff for randomised (e.g. R/exams)
//!   question banks — aligns versions of the same item and flags answer-key
//!   columns that stay constant across every version.

use crate::grading;
use crate::model::*;
use crate::parser;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Lint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintFinding {
    pub code: String,
    pub severity: Severity,
    /// Question name (or a positional label) the finding applies to; None for quiz-level findings.
    pub question: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChanceEntry {
    pub question: String,
    pub qtype: QuestionType,
    /// Expected score fraction (0..=1) under uniform random guessing; None when
    /// guessing is meaningless (essay/description) or the type is text-entry.
    pub expected_fraction: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    pub question_count: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub findings: Vec<LintFinding>,
    pub chance: Vec<ChanceEntry>,
    /// Quiz-level expected score fraction under random guessing (auto-gradeable questions only).
    pub chance_quiz_expected: Option<f64>,
}

impl LintReport {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Lint: {} question(s) — {} error(s), {} warning(s), {} info\n",
            self.question_count, self.errors, self.warnings, self.infos
        ));
        for f in &self.findings {
            let sev = match f.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARN ",
                Severity::Info => "INFO ",
            };
            match &f.question {
                Some(q) => out.push_str(&format!("{sev} [{}] {}: {}\n", f.code, q, f.message)),
                None => out.push_str(&format!("{sev} [{}] {}\n", f.code, f.message)),
            }
        }
        if let Some(exp) = self.chance_quiz_expected {
            out.push_str(&format!(
                "Random-guess baseline: expected quiz score ≈ {:.1}% (per-question detail in JSON output)\n",
                exp * 100.0
            ));
        }
        out
    }
}

fn finding(
    findings: &mut Vec<LintFinding>,
    code: &str,
    severity: Severity,
    question: Option<&str>,
    message: String,
) {
    findings.push(LintFinding {
        code: code.to_string(),
        severity,
        question: question.map(|s| s.to_string()),
        message,
    });
}

fn pluginfile_regex() -> Regex {
    Regex::new(r#"@@PLUGINFILE@@/([^"'\s<>\)\]]+)"#).unwrap()
}

/// Minimal percent-decoding for Moodle's URL-encoded attachment references
/// (`my%20data.csv` in the link vs `name="my data.csv"` on the element).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

const FULL: f64 = 99.999; // fraction "effectively 100%"

fn has_full_credit(answers: &[Answer]) -> bool {
    answers.iter().any(|a| a.fraction >= FULL)
}

/// A shortanswer pattern that matches any input (all wildcards / whitespace).
fn matches_everything(pattern: &str) -> bool {
    let t = pattern.trim();
    !t.is_empty() && t.chars().all(|c| c == '*')
}

pub fn lint_quiz_xml(xml: &str) -> Result<LintReport, String> {
    let (quiz, parse_warnings) = parser::parse_quiz_xml_with_warnings(xml, "lint", None)?;
    let mut findings: Vec<LintFinding> = Vec::new();

    for w in &parse_warnings {
        finding(&mut findings, "unsupported-question-type", Severity::Error, None, w.clone());
    }

    let pf_re = pluginfile_regex();

    // Attachment filenames shared across questions.
    let mut file_owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for q in &quiz.questions {
        for f in &q.files {
            file_owners.entry(f.name.clone()).or_default().push(q.name.clone());
        }
    }
    for (fname, owners) in &file_owners {
        if owners.len() > 1 {
            finding(
                &mut findings,
                "shared-attachment",
                Severity::Warning,
                None,
                format!(
                    "Attachment '{fname}' appears in {} questions ({}) — applied questions should each have uniquely-named, self-contained attachments",
                    owners.len(),
                    owners.join(", ")
                ),
            );
        }
    }

    let mut chance = Vec::new();
    for q in &quiz.questions {
        lint_question(q, &pf_re, &mut findings);
        let expected = expected_chance_fraction(q);
        if let Some(exp) = expected {
            if exp >= 0.5 {
                finding(
                    &mut findings,
                    "high-chance-score",
                    Severity::Warning,
                    Some(&q.name),
                    format!("Random guessing scores ≈ {:.0}% on this question", exp * 100.0),
                );
            }
        }
        chance.push(ChanceEntry {
            question: q.name.clone(),
            qtype: q.qtype,
            expected_fraction: expected,
        });
    }

    // Quiz-level expected chance score, weighted by default_grade.
    let mut exp_total = 0.0;
    let mut max_total = 0.0;
    for (q, c) in quiz.questions.iter().zip(&chance) {
        if matches!(q.qtype, QuestionType::Description) {
            continue;
        }
        max_total += q.default_grade;
        exp_total += c.expected_fraction.unwrap_or(0.0) * q.default_grade;
    }
    let chance_quiz_expected = if max_total > 0.0 { Some(exp_total / max_total) } else { None };

    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warnings = findings.iter().filter(|f| f.severity == Severity::Warning).count();
    let infos = findings.iter().filter(|f| f.severity == Severity::Info).count();
    Ok(LintReport {
        question_count: quiz.questions.len(),
        errors,
        warnings,
        infos,
        findings,
        chance,
        chance_quiz_expected,
    })
}

fn lint_question(q: &Question, pf_re: &Regex, findings: &mut Vec<LintFinding>) {
    let name = q.name.as_str();

    if q.question_text.trim().is_empty() && !matches!(q.qtype, QuestionType::Description) {
        finding(findings, "empty-question-text", Severity::Error, Some(name), "Question text is empty".into());
    }

    // Math delimiters Moodle's MathJax filter strips.
    let mut texts: Vec<&str> = vec![&q.question_text];
    if let Some(fb) = &q.general_feedback {
        texts.push(fb);
    }
    if texts.iter().any(|t| t.contains(r"\[") || t.contains(r"\]")) {
        finding(
            findings,
            "math-delimiters",
            Severity::Warning,
            Some(name),
            r"Contains \[ or \] math delimiters — Moodle's MathJax filter strips these; use $...$ or $$...$$".into(),
        );
    }

    // @@PLUGINFILE@@ references with no matching embedded <file>.
    let mut referenced: Vec<String> = Vec::new();
    for t in &texts {
        for cap in pf_re.captures_iter(t) {
            referenced.push(cap[1].to_string());
        }
    }
    for a in &q.answers {
        for cap in pf_re.captures_iter(&a.text) {
            referenced.push(cap[1].to_string());
        }
    }
    for r in referenced {
        let decoded = percent_decode(&r);
        let present = q.files.iter().any(|f| f.name == r || f.name == decoded);
        if !present {
            finding(
                findings,
                "missing-attachment",
                Severity::Error,
                Some(name),
                format!("References @@PLUGINFILE@@/{r} but no matching <file name=\"{decoded}\"> is embedded — the link will be dead for students"),
            );
        }
    }

    match q.qtype {
        QuestionType::MultiChoice => lint_multichoice(q, findings),
        QuestionType::TrueFalse => {
            if q.answers.len() != 2 {
                finding(
                    findings,
                    "truefalse-answer-count",
                    Severity::Error,
                    Some(name),
                    format!("True/false question has {} answers (expected 2)", q.answers.len()),
                );
            }
            if !has_full_credit(&q.answers) {
                finding(findings, "no-correct-answer", Severity::Error, Some(name), "No answer carries fraction 100".into());
            }
        }
        QuestionType::ShortAnswer => {
            if !has_full_credit(&q.answers) {
                finding(findings, "no-correct-answer", Severity::Error, Some(name), "No answer carries fraction 100".into());
            }
            for a in &q.answers {
                if a.fraction >= FULL && matches_everything(&a.text) {
                    finding(
                        findings,
                        "wildcard-matches-everything",
                        Severity::Error,
                        Some(name),
                        format!("Full-credit answer pattern '{}' is all wildcards — every response grades correct", a.text.trim()),
                    );
                }
            }
        }
        QuestionType::Numerical => {
            if !q.numerical_answers.iter().any(|a| a.fraction >= FULL) {
                finding(findings, "no-correct-answer", Severity::Error, Some(name), "No numerical answer carries fraction 100".into());
            }
            for a in &q.numerical_answers {
                if a.tolerance < 0.0 {
                    finding(
                        findings,
                        "negative-tolerance",
                        Severity::Warning,
                        Some(name),
                        format!("Answer {} has negative tolerance {} (treated as 0)", a.value, a.tolerance),
                    );
                }
            }
        }
        QuestionType::Matching => {
            if q.match_pairs.len() < 2 {
                finding(
                    findings,
                    "matching-too-few-pairs",
                    Severity::Warning,
                    Some(name),
                    format!("Matching question has only {} pair(s)", q.match_pairs.len()),
                );
            }
            for p in &q.match_pairs {
                if p.answer_text.trim().is_empty() {
                    finding(
                        findings,
                        "matching-empty-answer",
                        Severity::Error,
                        Some(name),
                        format!("Matching stem '{}' has an empty answer", truncate(&p.question_text, 40)),
                    );
                }
            }
        }
        QuestionType::Cloze => lint_cloze(q, findings),
        QuestionType::Essay | QuestionType::Description | QuestionType::Unsupported => {}
    }

    // Design smell: 1-mark non-MCQ items create disproportionate grading noise.
    let non_mcq = matches!(
        q.qtype,
        QuestionType::ShortAnswer | QuestionType::Numerical | QuestionType::Matching | QuestionType::Cloze | QuestionType::Essay
    );
    if non_mcq && q.default_grade < 2.0 {
        finding(
            findings,
            "one-mark-non-mcq",
            Severity::Info,
            Some(name),
            format!("Non-MCQ question worth only {} mark(s) — consider ≥ 2 marks or folding into an adjacent part", q.default_grade),
        );
    }
}

fn lint_multichoice(q: &Question, findings: &mut Vec<LintFinding>) {
    let name = q.name.as_str();
    if q.answers.is_empty() {
        finding(findings, "no-answers", Severity::Error, Some(name), "Multichoice question has no answers".into());
        return;
    }
    if q.single {
        if !has_full_credit(&q.answers) {
            finding(findings, "no-correct-answer", Severity::Error, Some(name), "No answer carries fraction 100".into());
        }
        let full = q.answers.iter().filter(|a| a.fraction >= FULL).count();
        if full > 1 {
            finding(
                findings,
                "multiple-full-credit",
                Severity::Warning,
                Some(name),
                format!("{full} answers carry fraction 100 in a single-response question"),
            );
        }
    } else {
        let positive_sum: f64 = q.answers.iter().map(|a| a.fraction.max(0.0)).sum();
        if (positive_sum - 100.0).abs() > 0.5 {
            finding(
                findings,
                "positive-fractions-sum",
                Severity::Warning,
                Some(name),
                format!("Positive answer fractions sum to {positive_sum:.1}% (expected 100%) — full marks may be unreachable or overshoot"),
            );
        }
        let has_wrong_option = q.answers.iter().any(|a| a.fraction <= 0.0);
        let has_negative = q.answers.iter().any(|a| a.fraction < 0.0);
        if has_wrong_option && !has_negative {
            finding(
                findings,
                "select-all-strategy",
                Severity::Warning,
                Some(name),
                "Wrong options carry no negative fraction — selecting every option scores 100%".into(),
            );
        }
        if !has_wrong_option {
            finding(
                findings,
                "select-all-strategy",
                Severity::Warning,
                Some(name),
                "Every option carries positive credit — selecting every option scores full marks".into(),
            );
        }
    }
    // Duplicate option texts confuse students and can hide an unescaped separator.
    let mut seen = BTreeMap::new();
    for a in &q.answers {
        *seen.entry(a.text.trim().to_string()).or_insert(0) += 1;
    }
    for (text, n) in seen {
        if n > 1 && !text.is_empty() {
            finding(
                findings,
                "duplicate-option",
                Severity::Warning,
                Some(name),
                format!("Option '{}' appears {n} times", truncate(&text, 40)),
            );
        }
    }
}

fn lint_cloze(q: &Question, findings: &mut Vec<LintFinding>) {
    let name = q.name.as_str();
    if q.cloze_items.is_empty() {
        finding(
            findings,
            "cloze-no-items",
            Severity::Error,
            Some(name),
            "Cloze question contains no {n:TYPE:...} markers".into(),
        );
        return;
    }
    for item in &q.cloze_items {
        if item.options.is_empty() {
            finding(
                findings,
                "cloze-empty-item",
                Severity::Error,
                Some(name),
                format!("Cloze blank {} has no options", item.index),
            );
            continue;
        }
        if !item.options.iter().any(|o| o.fraction >= FULL) {
            finding(
                findings,
                "cloze-item-no-correct",
                Severity::Error,
                Some(name),
                format!("Cloze blank {} has no option with fraction 100", item.index),
            );
        }
        for o in &item.options {
            if o.text.trim().is_empty() {
                finding(
                    findings,
                    "cloze-empty-option",
                    Severity::Warning,
                    Some(name),
                    format!("Cloze blank {} contains an empty option — often the signature of an unescaped '~' splitting an option in two", item.index),
                );
            }
        }
        if matches!(item.kind, ClozeKind::ShortAnswer | ClozeKind::ShortAnswerCaseSensitive) {
            for o in &item.options {
                if o.fraction >= FULL && matches_everything(&o.text) {
                    finding(
                        findings,
                        "wildcard-matches-everything",
                        Severity::Error,
                        Some(name),
                        format!("Cloze blank {}: full-credit pattern '{}' is all wildcards", item.index, o.text.trim()),
                    );
                }
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

// ---------------------------------------------------------------------------
// Chance (random-guess) baseline
// ---------------------------------------------------------------------------

/// Expected score fraction under uniform random guessing, computed analytically
/// (with subset enumeration / sampling for multiple-response multichoice).
/// None for essay/description/unsupported (no auto-grade to guess against).
pub fn expected_chance_fraction(q: &Question) -> Option<f64> {
    match q.qtype {
        QuestionType::MultiChoice | QuestionType::TrueFalse => {
            if q.answers.is_empty() {
                return Some(0.0);
            }
            if q.single {
                let sum: f64 = q
                    .answers
                    .iter()
                    .map(|a| (a.fraction / 100.0).clamp(0.0, 1.0))
                    .sum();
                Some(sum / q.answers.len() as f64)
            } else {
                Some(expected_multi_response(&q.answers))
            }
        }
        // Free-text entry: effectively unguessable.
        QuestionType::ShortAnswer | QuestionType::Numerical => Some(0.0),
        QuestionType::Matching => {
            let n = q.match_pairs.len();
            if n == 0 {
                Some(0.0)
            } else {
                // Expected fixed points of a random permutation is 1 → fraction 1/n.
                Some(1.0 / n as f64)
            }
        }
        QuestionType::Cloze => {
            if q.cloze_items.is_empty() {
                return Some(0.0);
            }
            let mut total = 0.0;
            for item in &q.cloze_items {
                total += match item.kind {
                    ClozeKind::MultichoiceInline | ClozeKind::MultichoiceDropdown => {
                        if item.options.is_empty() {
                            0.0
                        } else {
                            item.options
                                .iter()
                                .map(|o| (o.fraction / 100.0).clamp(0.0, 1.0))
                                .sum::<f64>()
                                / item.options.len() as f64
                        }
                    }
                    _ => 0.0,
                };
            }
            Some(total / q.cloze_items.len() as f64)
        }
        QuestionType::Essay | QuestionType::Description | QuestionType::Unsupported => None,
    }
}

/// E[clamp(Σ selected fractions, 0, 1)] when each option is independently
/// selected with p = 0.5 (uniform over all subsets). Exact enumeration up to
/// 2^16 subsets, deterministic stratified sampling beyond that.
fn expected_multi_response(answers: &[Answer]) -> f64 {
    let n = answers.len();
    let fractions: Vec<f64> = answers.iter().map(|a| a.fraction / 100.0).collect();
    if n <= 16 {
        let total = 1u32 << n;
        let mut acc = 0.0;
        for mask in 0..total {
            let mut s = 0.0;
            for (i, f) in fractions.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    s += f;
                }
            }
            acc += s.clamp(0.0, 1.0);
        }
        acc / total as f64
    } else {
        // Deterministic LCG sampling (no external RNG state needed).
        let mut state = 0x2545F4914F6CDD1Du64;
        let samples = 20000;
        let mut acc = 0.0;
        for _ in 0..samples {
            let mut s = 0.0;
            for f in &fractions {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                if state >> 63 == 1 {
                    s += f;
                }
            }
            acc += s.clamp(0.0, 1.0);
        }
        acc / samples as f64
    }
}

// ---------------------------------------------------------------------------
// Autotest: answer-key round-trip
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AutotestQuestionResult {
    pub question_id: String,
    pub name: String,
    pub qtype: QuestionType,
    /// Grade fraction obtained by submitting the intended-correct response.
    pub correct_fraction: Option<f64>,
    /// Grade fraction obtained by submitting a deliberately wrong response.
    pub wrong_fraction: Option<f64>,
    pub pass: bool,
    pub skipped: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutotestReport {
    pub quiz_name: String,
    pub questions: Vec<AutotestQuestionResult>,
    pub tested: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pass: bool,
}

impl AutotestReport {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Autotest '{}': {} tested, {} passed, {} failed, {} skipped → {}\n",
            self.quiz_name,
            self.tested,
            self.passed,
            self.failed,
            self.skipped,
            if self.pass { "PASS" } else { "FAIL" }
        ));
        for q in &self.questions {
            let status = if q.skipped {
                "SKIP".to_string()
            } else if q.pass {
                "PASS".to_string()
            } else {
                "FAIL".to_string()
            };
            let cf = q.correct_fraction.map(|f| format!("{:.0}%", f * 100.0)).unwrap_or_else(|| "-".into());
            let wf = q.wrong_fraction.map(|f| format!("{:.0}%", f * 100.0)).unwrap_or_else(|| "-".into());
            out.push_str(&format!("{status} {} — key answer scores {cf}, wrong answer scores {wf}", q.name));
            if !q.notes.is_empty() {
                out.push_str(&format!(" ({})", q.notes.join("; ")));
            }
            out.push('\n');
        }
        out
    }
}

pub fn autotest_quiz(quiz: &Quiz) -> AutotestReport {
    let mut questions = Vec::new();
    let (mut tested, mut passed, mut failed, mut skipped) = (0usize, 0usize, 0usize, 0usize);

    for q in &quiz.questions {
        let mut notes = Vec::new();
        let correct = intended_correct_response(q, &mut notes);
        let Some(correct_value) = correct else {
            skipped += 1;
            questions.push(AutotestQuestionResult {
                question_id: q.id.clone(),
                name: q.name.clone(),
                qtype: q.qtype,
                correct_fraction: None,
                wrong_fraction: None,
                pass: true,
                skipped: true,
                notes,
            });
            continue;
        };

        tested += 1;
        let correct_result = grade_with(q, correct_value);
        let correct_ok = correct_result >= 0.999;
        if !correct_ok {
            notes.push(format!(
                "intended-correct response only scored {:.1}% — answer key and grading disagree",
                correct_result * 100.0
            ));
        }

        let wrong = deliberately_wrong_response(q, &mut notes);
        let wrong_result = wrong.map(|w| grade_with(q, w));
        let wrong_ok = match wrong_result {
            Some(f) => {
                if f >= 0.999 {
                    notes.push("deliberately wrong response scored full marks — grader does not discriminate".into());
                    false
                } else {
                    true
                }
            }
            None => true,
        };

        let pass = correct_ok && wrong_ok;
        if pass {
            passed += 1;
        } else {
            failed += 1;
        }
        questions.push(AutotestQuestionResult {
            question_id: q.id.clone(),
            name: q.name.clone(),
            qtype: q.qtype,
            correct_fraction: Some(correct_result),
            wrong_fraction: wrong_result,
            pass,
            skipped: false,
            notes,
        });
    }

    AutotestReport {
        quiz_name: quiz.name.clone(),
        pass: failed == 0,
        tested,
        passed,
        failed,
        skipped,
        questions,
    }
}

fn grade_with(q: &Question, value: ResponseValue) -> f64 {
    let response = Response { value, flagged: false };
    grading::grade_question(q, Some(&response)).fraction
}

/// Turns a shortanswer wildcard pattern into a concrete string that matches it
/// (each `*` matches the empty string).
fn realize_wildcard(pattern: &str) -> String {
    pattern.replace('*', "")
}

fn best_answer(answers: &[Answer]) -> Option<&Answer> {
    answers
        .iter()
        .max_by(|a, b| a.fraction.partial_cmp(&b.fraction).unwrap_or(std::cmp::Ordering::Equal))
}

/// Synthesizes the response the answer key says deserves full marks.
/// None means the question type has no auto-gradeable key (essay/description).
pub fn intended_correct_response(q: &Question, notes: &mut Vec<String>) -> Option<ResponseValue> {
    match q.qtype {
        QuestionType::MultiChoice | QuestionType::TrueFalse => {
            if q.answers.is_empty() {
                notes.push("no answers to select".into());
                return None;
            }
            if q.single {
                let best = best_answer(&q.answers)?;
                if best.fraction < FULL {
                    notes.push(format!("best available answer is only worth {:.0}%", best.fraction));
                }
                Some(ResponseValue::Choices(vec![best.id.clone()]))
            } else {
                let ids: Vec<String> = q
                    .answers
                    .iter()
                    .filter(|a| a.fraction > 0.0)
                    .map(|a| a.id.clone())
                    .collect();
                if ids.is_empty() {
                    notes.push("no positively-weighted options".into());
                    return None;
                }
                Some(ResponseValue::Choices(ids))
            }
        }
        QuestionType::ShortAnswer => {
            let best = best_answer(&q.answers)?;
            if best.fraction < FULL {
                notes.push(format!("best available answer is only worth {:.0}%", best.fraction));
            }
            let mut text = best.text.clone();
            if text.contains('*') {
                notes.push(format!("realized wildcard pattern '{}'", text.trim()));
                text = realize_wildcard(&text);
            }
            Some(ResponseValue::Text(text))
        }
        QuestionType::Numerical => {
            let best = q
                .numerical_answers
                .iter()
                .max_by(|a, b| a.fraction.partial_cmp(&b.fraction).unwrap_or(std::cmp::Ordering::Equal))?;
            if best.fraction < FULL {
                notes.push(format!("best available answer is only worth {:.0}%", best.fraction));
            }
            Some(ResponseValue::Text(format!("{}", best.value)))
        }
        QuestionType::Matching => {
            if q.match_pairs.is_empty() {
                return None;
            }
            let mut m = BTreeMap::new();
            for p in &q.match_pairs {
                m.insert(p.id.clone(), p.answer_text.clone());
            }
            Some(ResponseValue::Mapping(m))
        }
        QuestionType::Cloze => {
            if q.cloze_items.is_empty() {
                return None;
            }
            let mut m = BTreeMap::new();
            for item in &q.cloze_items {
                let best = best_answer(&item.options)?;
                if best.fraction < FULL {
                    notes.push(format!("cloze blank {}: best option only worth {:.0}%", item.index, best.fraction));
                }
                let value = match item.kind {
                    ClozeKind::MultichoiceInline | ClozeKind::MultichoiceDropdown => best.id.clone(),
                    ClozeKind::ShortAnswer | ClozeKind::ShortAnswerCaseSensitive => realize_wildcard(&best.text),
                    ClozeKind::Numerical => best.text.clone(),
                };
                m.insert(item.index.to_string(), value);
            }
            Some(ResponseValue::Mapping(m))
        }
        QuestionType::Essay | QuestionType::Description | QuestionType::Unsupported => None,
    }
}

const WRONG_TEXT: &str = "zzz-mqt-deliberately-wrong-7391";

/// Synthesizes a response that should NOT earn full marks. None when no such
/// response can be constructed (e.g. every option carries full credit).
pub fn deliberately_wrong_response(q: &Question, notes: &mut Vec<String>) -> Option<ResponseValue> {
    match q.qtype {
        QuestionType::MultiChoice | QuestionType::TrueFalse => {
            if q.single {
                let worst = q
                    .answers
                    .iter()
                    .min_by(|a, b| a.fraction.partial_cmp(&b.fraction).unwrap_or(std::cmp::Ordering::Equal))?;
                if worst.fraction >= FULL {
                    notes.push("every option carries full credit — no wrong answer exists".into());
                    return None;
                }
                Some(ResponseValue::Choices(vec![worst.id.clone()]))
            } else {
                let zero_ids: Vec<String> = q
                    .answers
                    .iter()
                    .filter(|a| a.fraction <= 0.0)
                    .map(|a| a.id.clone())
                    .collect();
                if !zero_ids.is_empty() {
                    Some(ResponseValue::Choices(zero_ids))
                } else {
                    let worst = q
                        .answers
                        .iter()
                        .min_by(|a, b| a.fraction.partial_cmp(&b.fraction).unwrap_or(std::cmp::Ordering::Equal))?;
                    if worst.fraction >= FULL {
                        notes.push("every option carries full credit — no wrong answer exists".into());
                        return None;
                    }
                    Some(ResponseValue::Choices(vec![worst.id.clone()]))
                }
            }
        }
        QuestionType::ShortAnswer => {
            // Make sure our sentinel doesn't accidentally match a wildcard key.
            let matches_full = q.answers.iter().any(|a| {
                a.fraction >= FULL && shortanswer_pattern_matches(&a.text, WRONG_TEXT, q.case_sensitive)
            });
            if matches_full {
                notes.push("could not construct a wrong answer — a full-credit pattern matches arbitrary text".into());
                return None;
            }
            Some(ResponseValue::Text(WRONG_TEXT.to_string()))
        }
        QuestionType::Numerical => {
            let v = value_outside_all_windows(&q.numerical_answers)?;
            Some(ResponseValue::Text(format!("{v}")))
        }
        QuestionType::Matching => {
            let n = q.match_pairs.len();
            if n < 2 {
                notes.push("only one matching pair — cannot construct a mismatched assignment".into());
                return None;
            }
            let mut m = BTreeMap::new();
            for (i, p) in q.match_pairs.iter().enumerate() {
                let rotated = &q.match_pairs[(i + 1) % n];
                m.insert(p.id.clone(), rotated.answer_text.clone());
            }
            Some(ResponseValue::Mapping(m))
        }
        QuestionType::Cloze => {
            if q.cloze_items.is_empty() {
                return None;
            }
            let mut m = BTreeMap::new();
            for item in &q.cloze_items {
                let value = match item.kind {
                    ClozeKind::MultichoiceInline | ClozeKind::MultichoiceDropdown => {
                        let worst = item
                            .options
                            .iter()
                            .min_by(|a, b| a.fraction.partial_cmp(&b.fraction).unwrap_or(std::cmp::Ordering::Equal))?;
                        worst.id.clone()
                    }
                    ClozeKind::ShortAnswer | ClozeKind::ShortAnswerCaseSensitive => WRONG_TEXT.to_string(),
                    ClozeKind::Numerical => {
                        let windows: Vec<NumericalTolerance> = item
                            .options
                            .iter()
                            .filter_map(|o| {
                                o.text.trim().parse::<f64>().ok().map(|v| NumericalTolerance {
                                    value: v,
                                    tolerance: 1e-9,
                                    fraction: o.fraction,
                                    feedback: None,
                                })
                            })
                            .collect();
                        format!("{}", value_outside_all_windows(&windows).unwrap_or(1.0e12))
                    }
                };
                m.insert(item.index.to_string(), value);
            }
            Some(ResponseValue::Mapping(m))
        }
        QuestionType::Essay | QuestionType::Description | QuestionType::Unsupported => None,
    }
}

fn shortanswer_pattern_matches(pattern: &str, value: &str, case_sensitive: bool) -> bool {
    let (p, v) = if case_sensitive {
        (pattern.trim().to_string(), value.trim().to_string())
    } else {
        (pattern.trim().to_lowercase(), value.trim().to_lowercase())
    };
    if !p.contains('*') {
        return p == v;
    }
    let escaped: Vec<String> = p.split('*').map(regex::escape).collect();
    Regex::new(&format!("^{}$", escaped.join(".*")))
        .map(|re| re.is_match(&v))
        .unwrap_or(false)
}

/// A numeric value guaranteed to fall outside every accepted tolerance window.
fn value_outside_all_windows(answers: &[NumericalTolerance]) -> Option<f64> {
    let max_edge = answers
        .iter()
        .map(|a| a.value.abs() + a.tolerance.abs())
        .fold(0.0_f64, f64::max);
    let mut candidate = max_edge + 1013.771;
    for _ in 0..100 {
        let inside = answers.iter().any(|a| (candidate - a.value).abs() <= a.tolerance.max(0.0));
        if !inside {
            return Some(candidate);
        }
        candidate = candidate * 2.0 + 17.31;
    }
    None
}

// ---------------------------------------------------------------------------
// Compare: multi-version answer-key diff
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CompareColumn {
    /// What this column is: "answer", "blank 2", "pair 1", ...
    pub label: String,
    /// One answer-key signature per version, in version order.
    pub values: Vec<String>,
    pub constant: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareItem {
    /// Group key: question name (group-by-name) or "Q<n>" (positional).
    pub key: String,
    pub versions: usize,
    pub question_text_constant: bool,
    pub columns: Vec<CompareColumn>,
    /// True when at least one answer column never varies across versions.
    pub flagged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareReport {
    pub items: Vec<CompareItem>,
    pub flagged_items: usize,
    /// Groups with a single version (nothing to compare) — reported, not flagged.
    pub singletons: Vec<String>,
    pub notes: Vec<String>,
}

impl CompareReport {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for n in &self.notes {
            out.push_str(&format!("note: {n}\n"));
        }
        for item in &self.items {
            let mark = if item.flagged { "⚠" } else { "✓" };
            out.push_str(&format!(
                "{mark} {} ({} versions){}\n",
                item.key,
                item.versions,
                if item.question_text_constant { " [question text identical across versions]" } else { "" }
            ));
            for col in &item.columns {
                if col.constant {
                    out.push_str(&format!(
                        "    CONSTANT {} = '{}' in every version\n",
                        col.label,
                        truncate(&col.values[0], 60)
                    ));
                } else {
                    out.push_str(&format!("    varies   {}\n", col.label));
                }
            }
        }
        if !self.singletons.is_empty() {
            out.push_str(&format!(
                "single-version (not compared): {}\n",
                self.singletons.join(", ")
            ));
        }
        out.push_str(&format!(
            "{} of {} compared item(s) flagged. A constant answer column is only fine if it is the deliberate teaching point.\n",
            self.flagged_items,
            self.items.len()
        ));
        out
    }
}

/// Compares versions of the same item across quizzes and flags answer-key
/// columns that never vary. With `group_by_name`, questions sharing a name are
/// treated as versions of one item (R/exams multi-replication exports put all
/// versions in one file under the same name); otherwise questions are aligned
/// positionally across the given quizzes.
pub fn compare_quizzes(quizzes: &[Quiz], group_by_name: bool) -> CompareReport {
    let mut notes = Vec::new();
    let mut groups: Vec<(String, Vec<&Question>)> = Vec::new();

    if group_by_name || quizzes.len() == 1 {
        if quizzes.len() == 1 && !group_by_name {
            notes.push("single file given — grouping versions by question name".into());
        }
        // Normalised (replicate-stripped) names get their own namespace
        // ("\0"-prefixed bucket key, stripped back off for display) so a
        // question that isn't itself an R/exams replicate but happens to be
        // named exactly like one's stripped base (e.g. a plain question
        // named "q1_why_cv" alongside replicates "R1 Q1 : q1_why_cv", "R2 Q1
        // : q1_why_cv", ...) is never silently folded into that group —
        // only names that actually matched the replicate pattern merge with
        // each other.
        let mut by_name: BTreeMap<String, (String, Vec<&Question>)> = BTreeMap::new();
        let mut collapsed = 0usize;
        for quiz in quizzes {
            for q in &quiz.questions {
                let normalized = normalize_replicate_name(&q.name);
                let matched = normalized != q.name;
                if matched {
                    collapsed += 1;
                }
                let bucket_key = if matched { format!("\0{normalized}") } else { q.name.clone() };
                by_name.entry(bucket_key).or_insert_with(|| (normalized, Vec::new())).1.push(q);
            }
        }
        if collapsed > 0 {
            notes.push(format!(
                "grouped by normalised name — stripped replicate prefix on {collapsed} question(s)"
            ));
        }
        groups.extend(by_name.into_values());
    } else {
        let counts: Vec<usize> = quizzes.iter().map(|z| z.questions.len()).collect();
        if counts.windows(2).any(|w| w[0] != w[1]) {
            notes.push(format!(
                "question counts differ across files ({counts:?}) — positional alignment may be wrong; consider --group-by-name"
            ));
        }
        let max_len = counts.iter().copied().max().unwrap_or(0);
        for i in 0..max_len {
            let vs: Vec<&Question> = quizzes.iter().filter_map(|z| z.questions.get(i)).collect();
            let key = vs
                .first()
                .map(|q| format!("Q{} ({})", i + 1, q.name))
                .unwrap_or_else(|| format!("Q{}", i + 1));
            groups.push((key, vs));
        }
    }

    let mut items = Vec::new();
    let mut singletons = Vec::new();
    for (key, versions) in groups {
        if versions.len() < 2 {
            singletons.push(key);
            continue;
        }
        let texts: Vec<String> = versions.iter().map(|q| normalize_text_signature(&q.question_text)).collect();
        let question_text_constant = texts.windows(2).all(|w| w[0] == w[1]);

        // Collect per-version key columns; only compare labels present in every version.
        let per_version: Vec<BTreeMap<String, String>> =
            versions.iter().map(|q| answer_key_columns(q)).collect();
        let mut columns = Vec::new();
        if let Some(first) = per_version.first() {
            for label in first.keys() {
                if !per_version.iter().all(|m| m.contains_key(label)) {
                    continue;
                }
                let values: Vec<String> = per_version.iter().map(|m| m[label].clone()).collect();
                let constant = values.windows(2).all(|w| w[0] == w[1]);
                columns.push(CompareColumn {
                    label: label.clone(),
                    values,
                    constant,
                });
            }
        }
        let flagged = columns.iter().any(|c| c.constant);
        items.push(CompareItem {
            key,
            versions: versions.len(),
            question_text_constant,
            columns,
            flagged,
        });
    }

    let flagged_items = items.iter().filter(|i| i.flagged).count();
    CompareReport {
        items,
        flagged_items,
        singletons,
        notes,
    }
}

/// Strips R/exams' `exams2moodle(..., n = N)` replicate prefix (e.g.
/// `"R1 Q1 : q1_why_cv"`) so that versions of the same underlying item
/// collapse back into one group when grouping by name. Names that don't
/// match the pattern are returned unchanged.
fn normalize_replicate_name(name: &str) -> String {
    let re = Regex::new(r"^R\d+\s+Q\d+\s*:\s*(.+)$").unwrap();
    re.captures(name)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| name.to_string())
}

fn normalize_text_signature(html: &str) -> String {
    let tag_re = Regex::new(r"<[^>]*>").unwrap();
    let no_tags = tag_re.replace_all(html, " ");
    no_tags.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The answer-key "signature" of a question, one entry per independently
/// gradeable column (whole answer, cloze blank, matching pair, ...).
fn answer_key_columns(q: &Question) -> BTreeMap<String, String> {
    let mut cols = BTreeMap::new();
    match q.qtype {
        QuestionType::MultiChoice | QuestionType::TrueFalse | QuestionType::ShortAnswer => {
            let mut correct: Vec<String> = q
                .answers
                .iter()
                .filter(|a| a.fraction > 0.0)
                .map(|a| format!("{} ({:.0}%)", normalize_text_signature(&a.text), a.fraction))
                .collect();
            correct.sort();
            cols.insert("answer".to_string(), correct.join(" | "));
        }
        QuestionType::Numerical => {
            let mut correct: Vec<String> = q
                .numerical_answers
                .iter()
                .filter(|a| a.fraction > 0.0)
                .map(|a| format!("{}±{}", a.value, a.tolerance))
                .collect();
            correct.sort();
            cols.insert("answer".to_string(), correct.join(" | "));
        }
        QuestionType::Matching => {
            let mut pairs: Vec<String> = q
                .match_pairs
                .iter()
                .map(|p| {
                    format!(
                        "{} → {}",
                        normalize_text_signature(&p.question_text),
                        normalize_text_signature(&p.answer_text)
                    )
                })
                .collect();
            pairs.sort();
            cols.insert("pairs".to_string(), pairs.join(" | "));
        }
        QuestionType::Cloze => {
            for item in &q.cloze_items {
                let mut correct: Vec<String> = item
                    .options
                    .iter()
                    .filter(|o| o.fraction > 0.0)
                    .map(|o| format!("{} ({:.0}%)", o.text.trim(), o.fraction))
                    .collect();
                correct.sort();
                cols.insert(format!("blank {}", item.index), correct.join(" | "));
            }
        }
        QuestionType::Essay | QuestionType::Description | QuestionType::Unsupported => {}
    }
    cols
}
