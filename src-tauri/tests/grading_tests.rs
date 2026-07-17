use moodle_quiz_tester_lib::grading::grade_question;
use moodle_quiz_tester_lib::model::*;

fn mc_question(single: bool) -> Question {
    let mut q = Question::new(QuestionType::MultiChoice);
    q.default_grade = 1.0;
    q.single = single;
    if single {
        q.answers = vec![
            Answer { id: "a".into(), text: "Paris".into(), format: TextFormat::Html, fraction: 100.0, feedback: None },
            Answer { id: "b".into(), text: "London".into(), format: TextFormat::Html, fraction: 0.0, feedback: None },
        ];
    } else {
        q.answers = vec![
            Answer { id: "a".into(), text: "2".into(), format: TextFormat::Html, fraction: 25.0, feedback: None },
            Answer { id: "b".into(), text: "3".into(), format: TextFormat::Html, fraction: 25.0, feedback: None },
            Answer { id: "c".into(), text: "4".into(), format: TextFormat::Html, fraction: -25.0, feedback: None },
            Answer { id: "d".into(), text: "5".into(), format: TextFormat::Html, fraction: 25.0, feedback: None },
        ];
    }
    q
}

#[test]
fn single_choice_correct_gets_full_marks() {
    let q = mc_question(true);
    let response = Response { value: ResponseValue::Choices(vec!["a".into()]), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert_eq!(result.fraction, 1.0);
    assert_eq!(result.state, GradeState::Correct);
}

#[test]
fn single_choice_incorrect_gets_zero() {
    let q = mc_question(true);
    let response = Response { value: ResponseValue::Choices(vec!["b".into()]), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert_eq!(result.fraction, 0.0);
    assert_eq!(result.state, GradeState::Incorrect);
}

#[test]
fn multi_response_partial_credit_sums_and_clamps() {
    let q = mc_question(false);
    // Select both correct ones (2, 3) -> 25 + 25 = 50%
    let response = Response { value: ResponseValue::Choices(vec!["a".into(), "b".into()]), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert!((result.fraction - 0.5).abs() < 1e-9);
    assert_eq!(result.state, GradeState::PartiallyCorrect);
}

#[test]
fn multi_response_wrong_answer_reduces_score() {
    let q = mc_question(false);
    // 2, 3, 4 (wrong) -> 25 + 25 - 25 = 25%
    let response = Response {
        value: ResponseValue::Choices(vec!["a".into(), "b".into(), "c".into()]),
        flagged: false,
    };
    let result = grade_question(&q, Some(&response));
    assert!((result.fraction - 0.25).abs() < 1e-9);
}

#[test]
fn no_response_is_ungraded() {
    let q = mc_question(true);
    let result = grade_question(&q, None);
    assert_eq!(result.state, GradeState::Ungraded);
}

#[test]
fn shortanswer_wildcard_and_case_insensitivity() {
    let mut q = Question::new(QuestionType::ShortAnswer);
    q.default_grade = 1.0;
    q.case_sensitive = false;
    q.answers = vec![Answer {
        id: "a".into(),
        text: "H*O".into(),
        format: TextFormat::Plain,
        fraction: 100.0,
        feedback: None,
    }];
    let response = Response { value: ResponseValue::Text("h2o".into()), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert_eq!(result.fraction, 1.0);
}

#[test]
fn numerical_within_tolerance_is_correct() {
    let mut q = Question::new(QuestionType::Numerical);
    q.default_grade = 1.0;
    q.numerical_answers = vec![NumericalTolerance { value: 300000.0, tolerance: 1000.0, fraction: 100.0, feedback: None }];
    let response = Response { value: ResponseValue::Text("300500".into()), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert_eq!(result.fraction, 1.0);

    let response_out = Response { value: ResponseValue::Text("500000".into()), flagged: false };
    let result_out = grade_question(&q, Some(&response_out));
    assert_eq!(result_out.fraction, 0.0);
}

#[test]
fn matching_partial_credit_by_pair_count() {
    let mut q = Question::new(QuestionType::Matching);
    q.default_grade = 1.0;
    q.match_pairs = vec![
        MatchPair { id: "p1".into(), question_text: "Japan".into(), answer_text: "Tokyo".into() },
        MatchPair { id: "p2".into(), question_text: "Italy".into(), answer_text: "Rome".into() },
    ];
    let mut mapping = std::collections::BTreeMap::new();
    mapping.insert("p1".to_string(), "Tokyo".to_string());
    mapping.insert("p2".to_string(), "Wrong".to_string());
    let response = Response { value: ResponseValue::Mapping(mapping), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert!((result.fraction - 0.5).abs() < 1e-9);
}

#[test]
fn essay_is_always_ungraded() {
    let mut q = Question::new(QuestionType::Essay);
    q.default_grade = 1.0;
    let response = Response { value: ResponseValue::Text("My reflection.".into()), flagged: false };
    let result = grade_question(&q, Some(&response));
    assert_eq!(result.state, GradeState::Ungraded);
}
