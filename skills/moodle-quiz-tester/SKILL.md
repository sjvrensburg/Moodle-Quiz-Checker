---
name: moodle-quiz-tester
description: |
  Import, take, and grade a Moodle XML quiz export locally — offline, without a real Moodle instance — using the moodle-quiz-tester desktop app's headless CLI (`mqt-cli`) or its localhost agent HTTP API. Use whenever the task is validating a Moodle XML export before import (does it parse, do all question types round-trip, is the grading correct), dry-running an attempt to sanity-check answer keys and partial credit, or auditing quiz quality — including the R/exams → Moodle XML workflow's output. Trigger terms: "test this Moodle quiz", "validate Moodle XML", "check this quiz XML", "moodle-quiz-tester", "mqt-cli", "dry-run a Moodle quiz", "verify Moodle grading", "quiz quality checklist", "lint this quiz", "autotest this quiz", "compare quiz versions", "adversarial quiz review". Complements (does not replace) the `moodle-exams-quiz` skill, which authors the `.Rmd`/XML in the first place — this skill checks the XML output, from any source, actually behaves correctly once imported.
version: 2.0.0
---

# Testing Moodle quizzes with moodle-quiz-tester

`moodle-quiz-tester` (repo: `~/Moodle-Quiz-Checker`) is a local, offline Tauri +
SvelteKit app that parses Moodle XML quiz exports with the same fidelity as a
real Moodle import, lets you take an attempt, and grades it with a
Moodle-faithful engine — all without a Moodle server. It is agent-first: every
operation is available headlessly via `mqt-cli` or a localhost HTTP API.

## When to use this skill

- A Moodle XML export exists (from `exams2moodle()`, hand-authored, or
  exported from a live Moodle course) and you need to confirm it **imports
  cleanly and grades correctly** before it goes anywhere near students.
- Spot-checking that partial credit, cloze blanks, matching pairs, or
  numerical tolerance behave the way the author intended.
- Auditing quiz quality — format gates plus the pedagogical review in
  `reference/quiz-quality-checklist.md` and
  `reference/adversarial-review-protocol.md`.
- Verifying randomised question-bank versions actually vary where they should
  (`mqt-cli compare`).

If the task is *authoring* new `.Rmd`/R-exams questions, use the
`moodle-exams-quiz` skill instead — that skill produces the XML this one tests.

## Prerequisites

`mqt-cli` may already be on `PATH` (check `which mqt-cli`); otherwise build it
once from the repo root:

```bash
cd ~/Moodle-Quiz-Checker/src-tauri
cargo build --release --bin mqt-cli   # → ./target/release/mqt-cli
```

Use a **scratch database** for anything that imports, so you never mix test
attempts into the user's real quiz history:

```bash
export MQT_TEST_DB=/tmp/mqt-review-$$.sqlite3
mqt-cli --db "$MQT_TEST_DB" load path/to/quiz.xml
```

**Confidentiality:** quiz XML files are frequently exam/assessment content
under embargo. Never upload them anywhere (web tools, external APIs, pastes)
— only operate on them with local commands. Delete the scratch database when
done: `rm -f "$MQT_TEST_DB"`.

## The pipeline: three mechanical gates, then a pedagogical read

Run the gates in this order — each one is cheap, deterministic, and exits
non-zero on failure, so stop and report at the first failing gate.

### Gate 1 — `lint` (format & grading traps; no import needed)

```bash
mqt-cli lint quiz.xml            # human-readable; exit 1 if any ERROR
mqt-cli lint quiz.xml --json     # machine-readable findings for fixing
```

Catches: unsupported question types (would be silently dropped by a naive
import), questions with no 100% answer, multi-response weighting that makes
"select everything" score 100%, all-wildcard shortanswer patterns that match
any input, `\[...\]` math delimiters Moodle strips, `@@PLUGINFILE@@` links
with no embedded file, attachments shared across questions, malformed cloze
blanks (no options / no correct option / empty option = unescaped `~`
signature), plus a **random-guess score baseline** per question and for the
whole quiz (warns at ≥50% per question).

Fix every ERROR at the source (`.Rmd`/generator, not the XML) and re-lint.
WARNs and INFOs need judgment — report them with your recommendation.

Known false positive: in an R/exams multi-version bank, `shared-attachment`
fires when the *five versions of one item* embed the same filename. That is
the intended per-version pattern, not a violation — the rule targets sharing
across *different* items. Check whether the flagged questions are versions
of one item (same base name), and confirm the file *contents* differ per
version (decode and hash `files[].data_base64`) before dismissing it: same
name AND same bytes across versions means the data is not being regenerated
(see Gate 3).

### Gate 2 — `autotest` (answer-key round-trip)

```bash
mqt-cli autotest --file quiz.xml           # no DB needed
mqt-cli autotest <quiz-id> --db "$MQT_TEST_DB"   # for an imported quiz
```

For every auto-gradeable question this synthesizes the intended-correct
response straight from the answer key, grades it, and asserts 100%; it also
synthesizes a deliberately wrong response and asserts it does *not* score
100%. A FAIL means the exsolution/fraction encoding and the grading disagree
— the classic "fraction 0 where 100 was intended" R/exams bug. Essay and
description questions are SKIPped (manual grading, matching Moodle).

This replaces the old manual jq-driven "answer everything correctly and check
for 100%" loop — do not hand-drive that flow anymore unless you're debugging
a specific question's grading in isolation.

### Gate 3 — `compare` (randomisation actually randomises)

Only when the quiz has multiple randomised versions of the same item
(R/exams `exams2moodle(..., n = N)` output, or several generated files):

```bash
mqt-cli compare bank.xml                     # one file: versions grouped by question name
mqt-cli compare v1.xml v2.xml v3.xml         # several files: aligned by position
mqt-cli compare v1.xml v2.xml --group-by-name --json
```

Flags any answer-key column (whole answer, cloze blank, matching pair set)
that is **constant across every version** — the signature of trivial
arithmetic from fixed constants or self-referential construction pinning a
derived answer. Exit 1 on any flag. A constant column is only acceptable if
it is the deliberate teaching point (a fixed reference value, a mathematical
constant); confirm intent with the author before waving it through.

### Then — the pedagogical read

Mechanical gates can't judge question quality. After they pass:

1. **Adversarial student pass** — follow
   `reference/adversarial-review-protocol.md`: attempt the quiz *blind*
   (never reading answer data), log any test-taking heuristics that worked,
   then grade and diff your blind answers against the key.
2. **Checklist review** — work through
   `reference/quiz-quality-checklist.md`, including the distractor and
   feedback rubric.
3. **Report** — produce the structured report defined in
   `reference/review-report-template.md`. Always use this format so reviews
   are comparable across quizzes and agents.

For human sign-off, attach a reviewer copy of the quiz (all questions with
answer keys inline):

```bash
mqt-cli --db "$MQT_TEST_DB" export-quiz <quiz-id> > reviewer-copy.md
```

## Manual attempt driving (debugging a specific question)

```bash
BIN=mqt-cli
DB="$MQT_TEST_DB"

"$BIN" --db "$DB" load quiz.xml --name "Review"   # prints dropped-question warnings
"$BIN" --db "$DB" show <quiz-id> > /tmp/quiz.json  # the parsed model = what students see
"$BIN" --db "$DB" start-attempt <quiz-id> > /tmp/attempt.json
"$BIN" --db "$DB" answer <attempt-id> <question-id> '<value>' [--json]
"$BIN" --db "$DB" grade <attempt-id>
"$BIN" --db "$DB" export <attempt-id> --format markdown
```

### `ResponseValue` shapes for `answer --json`

| Question type | Shape | Example |
|---|---|---|
| multichoice/truefalse | array of selected answer ids | `["<answer-id>"]` |
| shortanswer/numerical/essay | plain string (no `--json`) | `answer <a> <q> "42"` |
| matching | object, pair-id → answer text | `{"<pair-id>":"Tokyo"}` |
| cloze | object, 1-based item index (as string) → value; multichoice cloze items use the option **id**, not its text | `{"1":"Paris","2":"4","3":"<option-id>"}` |

Get the exact ids from the `show` JSON — never guess them.

## Agent HTTP API (for multi-turn / non-shell agents)

Start it once (`mqt-cli --db "$DB" serve --port 4173`, binds `127.0.0.1`
only). All quality tooling is also exposed:

- `POST /lint` `{xml}` → LintReport
- `POST /autotest` `{xml}` → AutotestReport
- `GET /quizzes/:id/autotest` → AutotestReport for an imported quiz
- `POST /compare` `{sources: [{label, xml}, ...], group_by_name?}` → CompareReport
- `GET /quizzes/:id/reviewer.md` → reviewer copy with answer keys

See the repo README for the full attempt-driving endpoint table.

## Interpreting results

- **`load` prints "Dropped question ... unsupported type"**: the type (e.g.
  `ddwtos`, `gapselect`) needs a parser extension before it can be validated
  here; the question will not exist in attempts. `lint` reports the same as
  an ERROR.
- **Autotest FAIL "answer key and grading disagree"**: check the raw
  `<answer fraction="...">` / cloze `exsolution` encoding in the source XML
  first — a fraction of `0` where `100` was intended is a common authoring
  bug this tool faithfully reproduces, not hides.
- **`@@PLUGINFILE@@` links didn't resolve in the app**: expected — they're
  rewritten to same-page anchors with the file offered as a download. If
  lint reports `missing-attachment`, the `<file>` element is genuinely
  missing from the XML.
- **Essay questions never show "correct"**: expected, matches Moodle —
  essays always require manual grading.
