# Adversarial-student review protocol

The mechanical gates (lint, autotest, compare) prove the quiz *grades*
correctly. This protocol probes whether the questions *measure* anything: an
agent attempts the quiz the way a student would — and the way a test-wise
student would — and the failure patterns diagnose specific authoring defects.

Run it after the mechanical gates pass. It needs an imported quiz and a
scratch DB (see SKILL.md).

## Hard rule: two separate passes, blind first

The value of this protocol depends entirely on the blind pass actually being
blind. In pass 1 you may read ONLY:

- each question's `question_text` (and its attachments' contents),
- the option/answer texts shown to a student (multichoice options, matching
  answer pool, cloze dropdown options),
- the quiz name and question names.

You may NOT read: `fraction` values, `numerical_answers`, `match_pairs`
(the paired assignments — the answer *pool* is fine), tolerance values,
feedback of any kind, or any autotest/compare output. Practically: extract a
student view first, then never open the full `show` JSON until pass 2.

```bash
# Student view: question text + shuffled-safe option texts only
mqt-cli --db "$DB" show <quiz-id> \
  | jq '[.questions[] | {id, name, qtype, question_text,
         options: [.answers[]?.text],
         cloze: [.cloze_items[]? | {index, kind, options: [.options[].text]}],
         match_stems: [.match_pairs[]?.question_text],
         match_pool: [.match_pairs[]?.answer_text],
         files: [.files[]?.name]}]' > /tmp/student-view.json
```

## Pass 1 — honest student

Attempt every question from the student view alone, doing the actual work
(compute the numbers, run the attached dataset through the analysis if the
question expects it). Submit via `answer`, then `grade`.

Log, per question, before seeing any key:

- your answer and your confidence (sure / probable / guess),
- what information you used,
- anything that made the question harder than its content: ambiguous
  phrasing, missing information, unclear units/rounding expectations,
  notation that conflicts with itself, ESL-hostile constructions.

## Pass 2 — test-wise student (no subject knowledge allowed)

Start a **second attempt**. This time, deliberately answer using only
test-taking heuristics, pretending you do not know the material:

- longest / most detailed / most qualified option ("usually", "may"),
- grammatical agreement with the question stem,
- the option that is oddly precise among round numbers (or vice versa),
- "all of the above" / the union option,
- overlap elimination: options that contradict each other can't both be
  wrong in a single-answer question with an "all/none" option, etc.,
- answer leakage: information in another question's text, feedback, or an
  attachment that gives this one away,
- in multi-response questions with no visible penalty cue: select everything.

Log which heuristic you used per question. Then grade.

## Reading the results

| Signal | Diagnosis |
|---|---|
| Honest pass: confident answer graded wrong | Ambiguous question, wrong key, or tolerance/rounding mismatch — compare against the key in the full `show` JSON and decide which |
| Honest pass: couldn't determine an answer at all | Question under-specified: missing data, undefined notation, or the needed attachment is absent/inadequate |
| Test-wise pass scores well above the lint chance baseline | Weak distractors / cue leakage — name the heuristic that worked and the option(s) that gave it away |
| Test-wise heuristic picks the correct answer for a *specific* reason (length, grammar, precision) | Rewrite that option set: make distractors homogeneous in length, grammar, and precision |
| Both passes agree and match the key, chance baseline low | Question is probably sound — check distractor quality against the checklist rubric anyway |

Score expectations: the honest pass should approach 100% (you have the
material in front of you); every shortfall is a finding, not your failure.
The test-wise pass should land near the lint report's random-guess baseline;
every point above it is a finding.

Feed all findings into the review report (`review-report-template.md`),
citing question names and quoting the offending text.
