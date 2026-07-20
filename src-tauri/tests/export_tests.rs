//! Tests for the standalone question HTML render (issue #2: visual check for
//! math rendering, complementing lint's textual math-delimiters rule).

use moodle_quiz_tester_lib::export::question_to_standalone_html;
use moodle_quiz_tester_lib::parser::parse_quiz_xml;

fn wrap_quiz(questions: &str) -> String {
    format!("<?xml version=\"1.0\"?><quiz>{questions}</quiz>")
}

#[test]
fn render_preserves_raw_math_html_and_wires_up_mathjax() {
    let xml = wrap_quiz(
        r#"<question type="shortanswer">
            <name><text>math q</text></name>
            <questiontext format="html"><text><![CDATA[Solve \(x^2 = 4\) for x.]]></text></questiontext>
            <answer fraction="100"><text>2</text></answer>
        </question>"#,
    );
    let quiz = parse_quiz_xml(&xml, "bank", None).unwrap();
    let html = question_to_standalone_html(&quiz.questions[0]);

    // The raw LaTeX delimiters must survive unescaped/unstripped so MathJax
    // can actually process them (unlike the markdown reviewer export, which
    // strips all HTML).
    assert!(html.contains(r"\(x^2 = 4\)"), "{html}");
    assert!(html.contains("mathjax"), "expected a MathJax script tag: {html}");
    assert!(html.contains("<!doctype html>"));
}

#[test]
fn render_escapes_question_name_in_title_and_heading() {
    let xml = wrap_quiz(
        r#"<question type="shortanswer">
            <name><text>&lt;script&gt;evil&lt;/script&gt;</text></name>
            <questiontext format="html"><text>irrelevant</text></questiontext>
            <answer fraction="100"><text>x</text></answer>
        </question>"#,
    );
    let quiz = parse_quiz_xml(&xml, "bank", None).unwrap();
    let html = question_to_standalone_html(&quiz.questions[0]);
    assert!(!html.contains("<script>evil</script>"), "{html}");
    assert!(html.contains("&lt;script&gt;evil&lt;/script&gt;"), "{html}");
}
