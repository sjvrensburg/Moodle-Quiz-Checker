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
fn render_sanitizes_script_tags_in_question_and_answer_html() {
    // This document is served directly by the agent HTTP server as
    // text/html (with permissive CORS) and opened in a browser by the CLI
    // workflow, so untrusted Moodle question/answer/feedback HTML must not
    // be able to run script in either context.
    let xml = wrap_quiz(
        r#"<question type="shortanswer">
            <name><text>xss q</text></name>
            <questiontext format="html"><text><![CDATA[<p>Solve \(x=1\)</p><script>alert(1)</script><img src=x onerror=alert(2)>]]></text></questiontext>
            <answer fraction="100"><text><![CDATA[<script>alert(3)</script>ok]]></text></answer>
        </question>"#,
    );
    let quiz = parse_quiz_xml(&xml, "bank", None).unwrap();
    let html = question_to_standalone_html(&quiz.questions[0]);

    // The document legitimately embeds two <script> tags of its own (the
    // MathJax config + loader) — count them rather than asserting zero, so
    // this only catches a *third* script tag injected from question content.
    assert_eq!(html.matches("<script").count(), 2, "{html}");
    assert!(!html.contains("onerror"), "{html}");
    assert!(!html.contains("alert("), "{html}");
    // Benign markup and math delimiters must survive sanitization.
    assert!(html.contains("<p>Solve"), "{html}");
    assert!(html.contains(r"\(x=1\)"), "{html}");
    assert!(html.contains("ok</li>"), "{html}");
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
