//! Tests for the quality tooling: lint, autotest, compare.

use moodle_quiz_tester_lib::parser::{parse_quiz_xml, parse_quiz_xml_with_warnings};
use moodle_quiz_tester_lib::quality::{autotest_quiz, compare_quizzes, lint_quiz_xml, Severity};

fn wrap_quiz(questions: &str) -> String {
    format!("<?xml version=\"1.0\"?><quiz>{questions}</quiz>")
}

fn sample_xml() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../samples/sample-quiz.xml"))
        .expect("sample quiz")
}

fn has_finding(report: &moodle_quiz_tester_lib::quality::LintReport, code: &str) -> bool {
    report.findings.iter().any(|f| f.code == code)
}

// ---------------------------------------------------------------------------
// Lint
// ---------------------------------------------------------------------------

#[test]
fn lint_sample_quiz_has_no_errors() {
    let report = lint_quiz_xml(&sample_xml()).unwrap();
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "sample quiz should lint clean, got: {errors:?}");
}

#[test]
fn lint_flags_missing_correct_answer() {
    let xml = wrap_quiz(
        r#"<question type="multichoice">
            <name><text>broken mc</text></name>
            <questiontext format="html"><text>Pick one</text></questiontext>
            <single>true</single>
            <answer fraction="0"><text>A</text></answer>
            <answer fraction="50"><text>B</text></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "no-correct-answer"));
    assert!(report.errors >= 1);
}

#[test]
fn lint_flags_select_all_strategy() {
    let xml = wrap_quiz(
        r#"<question type="multichoice">
            <name><text>free credit</text></name>
            <questiontext format="html"><text>Pick all that apply</text></questiontext>
            <single>false</single>
            <answer fraction="50"><text>A</text></answer>
            <answer fraction="50"><text>B</text></answer>
            <answer fraction="0"><text>C</text></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "select-all-strategy"));
}

#[test]
fn lint_flags_wildcard_matches_everything() {
    let xml = wrap_quiz(
        r#"<question type="shortanswer">
            <name><text>anything goes</text></name>
            <questiontext format="html"><text>Say something</text></questiontext>
            <answer fraction="100"><text>*</text></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "wildcard-matches-everything"));
}

#[test]
fn lint_flags_missing_attachment_and_math_delimiters() {
    let xml = wrap_quiz(
        r#"<question type="essay">
            <name><text>data question</text></name>
            <questiontext format="html"><text><![CDATA[Download <a href="@@PLUGINFILE@@/data.csv">the data</a> and compute \[ \bar{x} \]]]></text></questiontext>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "missing-attachment"));
    assert!(has_finding(&report, "math-delimiters"));
}

#[test]
fn lint_flags_shared_attachment() {
    let q = |name: &str| {
        format!(
            r#"<question type="essay">
                <name><text>{name}</text></name>
                <questiontext format="html"><text><![CDATA[Use <a href="@@PLUGINFILE@@/shared.csv">data</a><file name="shared.csv" encoding="base64">QUJD</file>]]></text><file name="shared.csv" encoding="base64">QUJD</file></questiontext>
            </question>"#
        )
    };
    let xml = wrap_quiz(&format!("{}{}", q("q one"), q("q two")));
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "shared-attachment"));
}

#[test]
fn lint_flags_code_underscore_risk() {
    let xml = wrap_quiz(
        r#"<question type="essay">
            <name><text>code question</text></name>
            <questiontext format="html"><text><![CDATA[Compute <code>sd(fold_errors) / sqrt(K)</code>]]></text></questiontext>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "code-underscore-risk"));
    // Plain prose underscores outside <code>/<pre> shouldn't trigger it.
    let xml_no_code = wrap_quiz(
        r#"<question type="essay">
            <name><text>plain question</text></name>
            <questiontext format="html"><text>No code spans here, just text_with_underscore</text></questiontext>
        </question>"#,
    );
    let report_no_code = lint_quiz_xml(&xml_no_code).unwrap();
    assert!(!has_finding(&report_no_code, "code-underscore-risk"));
}

#[test]
fn lint_flags_possible_answer_leak_across_questions() {
    // Q1's correct R² answer (0.9649 / 96.5%) verbatim in every option of Q2
    // — the "next question spoils this one's answer" trap.
    let xml = wrap_quiz(
        r#"<question type="numerical">
            <name><text>compute r2</text></name>
            <questiontext format="html"><text>Compute R^2.</text></questiontext>
            <answer fraction="100"><text>0.9649</text><tolerance>0.0001</tolerance></answer>
        </question>
        <question type="multichoice">
            <name><text>interpret r2</text></name>
            <questiontext format="html"><text>What does R^2 mean?</text></questiontext>
            <single>true</single>
            <answer fraction="100"><text>R^2=96.5% means the model explains 96.5% of variance</text></answer>
            <answer fraction="0"><text>R^2=96.5% means residuals are 96.5% of variance</text></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "possible-answer-leak"), "{}", report.to_text());
    let f = report.findings.iter().find(|f| f.code == "possible-answer-leak").unwrap();
    assert_eq!(f.question.as_deref(), Some("interpret r2"));
}

#[test]
fn lint_does_not_flag_unrelated_recurring_numbers_as_leak() {
    // A trivial/short number recurring by coincidence shouldn't trip the heuristic.
    let xml = wrap_quiz(
        r#"<question type="numerical">
            <name><text>q1</text></name>
            <questiontext format="html"><text>a=1, b=5</text></questiontext>
            <answer fraction="100"><text>5</text><tolerance>0</tolerance></answer>
        </question>
        <question type="numerical">
            <name><text>q2</text></name>
            <questiontext format="html"><text>Unrelated: there are 5 apples</text></questiontext>
            <answer fraction="100"><text>3</text><tolerance>0</tolerance></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(!has_finding(&report, "possible-answer-leak"), "{}", report.to_text());
}

#[test]
fn lint_reports_unsupported_type_as_error() {
    let xml = wrap_quiz(
        r#"<question type="ddwtos">
            <name><text>drag drop</text></name>
            <questiontext format="html"><text>Drag things</text></questiontext>
        </question>
        <question type="truefalse">
            <name><text>tf</text></name>
            <questiontext format="html"><text>True?</text></questiontext>
            <answer fraction="100"><text>true</text></answer>
            <answer fraction="0"><text>false</text></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "unsupported-question-type"));
    assert_eq!(report.question_count, 1);
}

#[test]
fn lint_flags_high_chance_score() {
    // True/false: random guessing scores 50%.
    let xml = wrap_quiz(
        r#"<question type="truefalse">
            <name><text>coin flip</text></name>
            <questiontext format="html"><text>True?</text></questiontext>
            <answer fraction="100"><text>true</text></answer>
            <answer fraction="0"><text>false</text></answer>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "high-chance-score"));
    let entry = &report.chance[0];
    assert!((entry.expected_fraction.unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn lint_flags_cloze_empty_option() {
    // Unescaped ~ leaves an empty option: "Paris~~London" splits into 3 with one empty.
    let xml = wrap_quiz(
        r#"<question type="cloze">
            <name><text>tilde bug</text></name>
            <questiontext format="html"><text>Capital: {1:MULTICHOICE:%100%Paris~~London}</text></questiontext>
        </question>"#,
    );
    let report = lint_quiz_xml(&xml).unwrap();
    assert!(has_finding(&report, "cloze-empty-option"));
}

// ---------------------------------------------------------------------------
// Parser warnings
// ---------------------------------------------------------------------------

#[test]
fn parser_reports_dropped_questions() {
    let xml = wrap_quiz(
        r#"<question type="gapselect">
            <name><text>gap select q</text></name>
            <questiontext format="html"><text>Fill gaps</text></questiontext>
        </question>"#,
    );
    let (quiz, warnings) = parse_quiz_xml_with_warnings(&xml, "t", None).unwrap();
    assert!(quiz.questions.is_empty());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("gapselect"));
    assert!(warnings[0].contains("gap select q"));
}

// ---------------------------------------------------------------------------
// Autotest
// ---------------------------------------------------------------------------

#[test]
fn autotest_sample_quiz_passes() {
    let quiz = parse_quiz_xml(&sample_xml(), "sample", None).unwrap();
    let report = autotest_quiz(&quiz);
    assert!(report.pass, "sample quiz should autotest clean: {}", report.to_text());
    assert!(report.tested > 0);
    for q in &report.questions {
        if !q.skipped {
            assert!(q.correct_fraction.unwrap() >= 0.999, "{}: {:?}", q.name, q.notes);
        }
    }
}

#[test]
fn autotest_catches_answer_key_grading_disagreement() {
    // Author intended "Paris" but weighted it 0 — the classic exsolution bug.
    let xml = wrap_quiz(
        r#"<question type="multichoice">
            <name><text>bad key</text></name>
            <questiontext format="html"><text>Capital of France?</text></questiontext>
            <single>true</single>
            <answer fraction="0"><text>Paris</text></answer>
            <answer fraction="0"><text>London</text></answer>
        </question>"#,
    );
    let quiz = parse_quiz_xml(&xml, "t", None).unwrap();
    let report = autotest_quiz(&quiz);
    assert!(!report.pass);
    assert_eq!(report.failed, 1);
}

#[test]
fn autotest_skips_essay_and_description() {
    let xml = wrap_quiz(
        r#"<question type="essay">
            <name><text>essay q</text></name>
            <questiontext format="html"><text>Discuss.</text></questiontext>
        </question>
        <question type="description">
            <name><text>info</text></name>
            <questiontext format="html"><text>Read this first.</text></questiontext>
        </question>"#,
    );
    let quiz = parse_quiz_xml(&xml, "t", None).unwrap();
    let report = autotest_quiz(&quiz);
    assert!(report.pass);
    assert_eq!(report.skipped, 2);
    assert_eq!(report.tested, 0);
}

#[test]
fn autotest_wrong_answer_discriminates_numerical() {
    let xml = wrap_quiz(
        r#"<question type="numerical">
            <name><text>num q</text></name>
            <questiontext format="html"><text>2+2?</text></questiontext>
            <answer fraction="100"><text>4</text><tolerance>0.5</tolerance></answer>
        </question>"#,
    );
    let quiz = parse_quiz_xml(&xml, "t", None).unwrap();
    let report = autotest_quiz(&quiz);
    assert!(report.pass, "{}", report.to_text());
    let q = &report.questions[0];
    assert!(q.correct_fraction.unwrap() >= 0.999);
    assert!(q.wrong_fraction.unwrap() < 0.001);
}

// ---------------------------------------------------------------------------
// Compare
// ---------------------------------------------------------------------------

fn version_xml(name: &str, data_a: u32, data_b: u32, answer: u32) -> String {
    wrap_quiz(&format!(
        r#"<question type="numerical">
            <name><text>{name}</text></name>
            <questiontext format="html"><text>Given a={data_a} and b={data_b}, compute the result.</text></questiontext>
            <answer fraction="100"><text>{answer}</text><tolerance>0.01</tolerance></answer>
        </question>"#
    ))
}

#[test]
fn compare_flags_constant_answer_across_versions() {
    // Data varies, answer key doesn't — the R/exams self-referential trap.
    let v1 = parse_quiz_xml(&version_xml("ex1", 3, 5, 42), "v1", None).unwrap();
    let v2 = parse_quiz_xml(&version_xml("ex1", 7, 2, 42), "v2", None).unwrap();
    let v3 = parse_quiz_xml(&version_xml("ex1", 9, 4, 42), "v3", None).unwrap();
    let report = compare_quizzes(&[v1, v2, v3], false);
    assert_eq!(report.flagged_items, 1, "{}", report.to_text());
    let item = &report.items[0];
    assert!(!item.question_text_constant);
    assert!(item.columns.iter().any(|c| c.constant));
}

#[test]
fn compare_passes_when_answers_vary() {
    let v1 = parse_quiz_xml(&version_xml("ex1", 3, 5, 15), "v1", None).unwrap();
    let v2 = parse_quiz_xml(&version_xml("ex1", 7, 2, 14), "v2", None).unwrap();
    let report = compare_quizzes(&[v1, v2], false);
    assert_eq!(report.flagged_items, 0, "{}", report.to_text());
}

#[test]
fn compare_groups_by_name_within_single_file() {
    // One file holding two versions of the same named item (R/exams style).
    let xml = wrap_quiz(&format!(
        "{}{}",
        r#"<question type="numerical">
            <name><text>exA</text></name>
            <questiontext format="html"><text>a=1</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#,
        r#"<question type="numerical">
            <name><text>exA</text></name>
            <questiontext format="html"><text>a=2</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#
    ));
    let quiz = parse_quiz_xml(&xml, "bank", None).unwrap();
    let report = compare_quizzes(&[quiz], false);
    assert_eq!(report.items.len(), 1);
    assert!(report.items[0].flagged);
    assert_eq!(report.items[0].versions, 2);
}

#[test]
fn compare_groups_rexams_replicate_names_in_single_file() {
    // R/exams' exams2moodle(..., n = N) bakes the replicate label into the
    // name itself ("R1 Q1 : q1_why_cv", "R2 Q1 : q1_why_cv", ...). Those must
    // collapse into one group, not be treated as N singletons (issue #1).
    let xml = wrap_quiz(&format!(
        "{}{}{}",
        r#"<question type="numerical">
            <name><text>R1 Q1 : q1_why_cv</text></name>
            <questiontext format="html"><text>a=1</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#,
        r#"<question type="numerical">
            <name><text>R2 Q1 : q1_why_cv</text></name>
            <questiontext format="html"><text>a=2</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#,
        r#"<question type="numerical">
            <name><text>R3 Q1 : q1_why_cv</text></name>
            <questiontext format="html"><text>a=3</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#
    ));
    let quiz = parse_quiz_xml(&xml, "bank", None).unwrap();
    let report = compare_quizzes(&[quiz], false);
    assert_eq!(report.items.len(), 1, "{}", report.to_text());
    assert_eq!(report.items[0].key, "q1_why_cv");
    assert_eq!(report.items[0].versions, 3);
    assert!(report.items[0].flagged, "constant answer=10 across replicates should be flagged");
    assert!(
        report.notes.iter().any(|n| n.contains("stripped replicate prefix")),
        "{:?}",
        report.notes
    );
}

#[test]
fn compare_non_rexams_names_unaffected_by_replicate_normalisation() {
    // Non-matching names (no "R<n> Q<n> :" prefix) must keep exact-name
    // grouping — the normalisation is additive, not a behaviour change.
    let v1 = parse_quiz_xml(&version_xml("ex1", 3, 5, 42), "v1", None).unwrap();
    let v2 = parse_quiz_xml(&version_xml("ex1", 7, 2, 42), "v2", None).unwrap();
    let report = compare_quizzes(&[v1, v2], false);
    assert_eq!(report.items.len(), 1);
    assert_eq!(report.items[0].key, "Q1 (ex1)");
    assert!(!report.notes.iter().any(|n| n.contains("stripped replicate prefix")));
}

#[test]
fn compare_does_not_merge_unrelated_question_sharing_replicate_base_name() {
    // A plain question literally named "q1_why_cv" must stay its own group,
    // not get silently folded into the "R<n> Q1 : q1_why_cv" replicate
    // group just because the stripped base happens to match.
    let xml = wrap_quiz(&format!(
        "{}{}{}",
        r#"<question type="numerical">
            <name><text>R1 Q1 : q1_why_cv</text></name>
            <questiontext format="html"><text>a=1</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#,
        r#"<question type="numerical">
            <name><text>R2 Q1 : q1_why_cv</text></name>
            <questiontext format="html"><text>a=2</text></questiontext>
            <answer fraction="100"><text>10</text><tolerance>0</tolerance></answer>
        </question>"#,
        r#"<question type="numerical">
            <name><text>q1_why_cv</text></name>
            <questiontext format="html"><text>unrelated question</text></questiontext>
            <answer fraction="100"><text>99</text><tolerance>0</tolerance></answer>
        </question>"#
    ));
    let quiz = parse_quiz_xml(&xml, "bank", None).unwrap();
    let report = compare_quizzes(&[quiz], false);
    assert_eq!(report.items.len(), 1, "{}", report.to_text());
    assert_eq!(report.items[0].key, "q1_why_cv");
    assert_eq!(report.items[0].versions, 2, "the unrelated plain-named question must not join the replicate group");
    assert_eq!(report.singletons.len(), 1);
    assert_eq!(report.singletons[0], "q1_why_cv");
}
