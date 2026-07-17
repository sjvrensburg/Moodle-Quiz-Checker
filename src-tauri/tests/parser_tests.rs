use moodle_quiz_tester_lib::model::QuestionType;
use moodle_quiz_tester_lib::parser::parse_quiz_xml;

fn sample_xml() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../samples/sample-quiz.xml")).unwrap()
}

#[test]
fn parses_all_question_types() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).expect("parse should succeed");
    assert_eq!(quiz.questions.len(), 9);

    let types: Vec<QuestionType> = quiz.questions.iter().map(|q| q.qtype).collect();
    assert!(types.contains(&QuestionType::Description));
    assert!(types.contains(&QuestionType::MultiChoice));
    assert!(types.contains(&QuestionType::TrueFalse));
    assert!(types.contains(&QuestionType::ShortAnswer));
    assert!(types.contains(&QuestionType::Numerical));
    assert!(types.contains(&QuestionType::Matching));
    assert!(types.contains(&QuestionType::Cloze));
    assert!(types.contains(&QuestionType::Essay));
}

#[test]
fn category_is_attached_to_following_questions() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).unwrap();
    for q in &quiz.questions {
        assert_eq!(q.category.as_deref(), Some("$course$/top/Sample Quiz"));
    }
}

#[test]
fn multichoice_parses_cdata_html_and_fractions() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).unwrap();
    let q = quiz
        .questions
        .iter()
        .find(|q| q.name == "Capital of France")
        .unwrap();
    assert!(q.question_text.contains("capital of"));
    assert_eq!(q.answers.len(), 4);
    let paris = q.answers.iter().find(|a| a.text == "Paris").unwrap();
    assert_eq!(paris.fraction, 100.0);
    assert!(q.single);
    assert!(q.shuffle_answers);
}

#[test]
fn truefalse_parses_two_answers() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).unwrap();
    let q = quiz.questions.iter().find(|q| q.qtype == QuestionType::TrueFalse).unwrap();
    assert_eq!(q.answers.len(), 2);
    let false_answer = q.answers.iter().find(|a| a.text == "false").unwrap();
    assert_eq!(false_answer.fraction, 100.0);
}

#[test]
fn numerical_parses_tolerance_and_units() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).unwrap();
    let q = quiz.questions.iter().find(|q| q.qtype == QuestionType::Numerical).unwrap();
    assert_eq!(q.numerical_answers.len(), 1);
    assert_eq!(q.numerical_answers[0].value, 300000.0);
    assert_eq!(q.numerical_answers[0].tolerance, 1000.0);
    assert_eq!(q.numerical_units.len(), 1);
    assert_eq!(q.numerical_units[0].0, "km/s");
}

#[test]
fn matching_parses_pairs() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).unwrap();
    let q = quiz.questions.iter().find(|q| q.qtype == QuestionType::Matching).unwrap();
    assert_eq!(q.match_pairs.len(), 3);
    assert!(q.match_pairs.iter().any(|p| p.question_text == "Japan" && p.answer_text == "Tokyo"));
}

#[test]
fn embedded_pluginfile_is_extracted_and_links_rewritten() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<quiz>
  <question type="shortanswer">
    <name><text>Dataset question</text></name>
    <questiontext format="html">
      <text><![CDATA[<p>Download <a href="@@PLUGINFILE@@/data.csv">data.csv</a> and report the mean.</p>]]></text>
      <file name="data.csv" encoding="base64">YSxiCjEsMgo=</file>
    </questiontext>
    <generalfeedback format="html"><text></text></generalfeedback>
    <defaultgrade>1.0000000</defaultgrade>
    <penalty>0.3333333</penalty>
    <hidden>0</hidden>
    <usecase>0</usecase>
    <answer fraction="100" format="plain_text"><text>1.5</text></answer>
  </question>
</quiz>"##;

    let quiz = parse_quiz_xml(xml, "Files", None).unwrap();
    let q = &quiz.questions[0];
    assert_eq!(q.files.len(), 1);
    assert_eq!(q.files[0].name, "data.csv");
    assert_eq!(q.files[0].data_base64, "YSxiCjEsMgo=");
    assert!(!q.question_text.contains("@@PLUGINFILE@@"));
    assert!(q.question_text.contains("#attachment-data.csv"));
}

#[test]
fn cloze_parses_embedded_items() {
    let quiz = parse_quiz_xml(&sample_xml(), "Sample", None).unwrap();
    let q = quiz.questions.iter().find(|q| q.qtype == QuestionType::Cloze).unwrap();
    assert_eq!(q.cloze_items.len(), 3);
    assert_eq!(q.cloze_items[0].index, 1);
    assert_eq!(q.cloze_items[2].options.len(), 3);
    let correct = q.cloze_items[2]
        .options
        .iter()
        .find(|o| o.text == "liquid")
        .unwrap();
    assert_eq!(correct.fraction, 100.0);
}
