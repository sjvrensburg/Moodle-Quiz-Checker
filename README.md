# Moodle Quiz Tester

A fully offline, desktop-first application for importing, taking, and grading
**Moodle XML** quiz exports — without needing a real Moodle instance, a
network connection, or a cloud service. Built with [Tauri 2](https://tauri.app)
(Rust backend) and [SvelteKit](https://kit.svelte.dev) (frontend).

It's also **agent-first**: every piece of quiz state and every action (import,
answer, grade, export) is available through a scriptable CLI and a localhost
HTTP API, so an external agentic AI (or a shell script, or a human) can drive
a full quiz attempt without touching the GUI. No LLM is embedded in the app —
you bring your own agent (an Ollama model, Claude, whatever).

## Features

- **Import** Moodle XML quiz/question-bank exports (`<quiz>...</quiz>`), including
  category markers, CDATA HTML question text, and per-answer feedback.
- **Question types**: multiple choice (single/multiple response), true/false,
  short answer, numerical (with tolerance and units), matching, cloze
  (embedded answers, e.g. `{1:SHORTANSWER:=Paris#Correct}`), essay, and
  description.
- **Moodle-faithful grading**: fractional partial credit, per-answer and
  whole-question feedback (correct / partially correct / incorrect / general),
  shortanswer wildcards (`*`), numerical tolerance and unit multipliers.
- **Local, persistent storage** via SQLite (no server, no account).
- **Attempt flow**: question navigation grid, flagging, elapsed-time display,
  review mode with per-question feedback after finishing.
- **Export**: JSON, Markdown, and print-to-PDF (via the OS print dialog).
- **Agent interfaces**: a headless CLI (`mqt-cli`) and an optional localhost-only
  HTTP API for driving attempts programmatically.
- **Dark mode**, keyboard-friendly navigation.

## Installing a prebuilt release

After `npm run tauri build` (see below), Linux bundles land in
`src-tauri/target/release/bundle/`:

```bash
# .deb (Debian/Ubuntu)
sudo dpkg -i src-tauri/target/release/bundle/deb/moodle-quiz-tester_*.deb

# .rpm (Fedora/openSUSE)
sudo rpm -i src-tauri/target/release/bundle/rpm/moodle-quiz-tester-*.rpm

# .AppImage (portable, no install) — drop it anywhere on PATH, e.g. ~/bin
cp src-tauri/target/release/bundle/appimage/moodle-quiz-tester_*.AppImage ~/bin/moodle-quiz-tester
chmod +x ~/bin/moodle-quiz-tester

# mqt-cli is a separate binary (not embedded in the AppImage's entrypoint) —
# install it alongside for headless/agent use:
cp src-tauri/target/release/mqt-cli ~/bin/mqt-cli
```

## Claude Code skills

The repo ships two complementary skills under [`skills/`](skills/) that
together cover the full author → validate pipeline for agent-driven Moodle
quiz development:

- **`moodle-quiz-tester`** (validation) — teaches Claude Code (or any agent
  that reads it) how to drive `mqt-cli`/the agent HTTP API to validate a
  Moodle XML quiz: the mechanical gates (`lint` → `autotest` → `compare`),
  an adversarial-student review protocol, a distractor/feedback rubric, and
  a structured review-report template.
- **`moodle-exams-quiz`** (authoring) — how to author randomised Moodle
  quizzes in R Markdown with R's [`exams`](https://www.r-exams.org) package
  (`exams2moodle`): schoice/mchoice/num/cloze question templates, the paired
  student-`.R`/answer-key-`.Rmd` workflow, per-version data regeneration,
  generator scripts, and the pitfalls that cause silent student-facing
  breakage. Module names/notation are generic placeholders — adapt the
  conventions to your own course.

Install by copying into your skills directory:

```bash
cp -r skills/moodle-quiz-tester skills/moodle-exams-quiz ~/.claude/skills/
```

Then invoke by name: `moodle-exams-quiz` when writing questions,
`moodle-quiz-tester` when checking the generated XML — from this repo or any
other.

## Project layout

```
src-tauri/           Rust backend (Tauri app, CLI binary, core library)
  src/model.rs        Domain model (Quiz, Question, Attempt, ...)
  src/xmltree.rs       Minimal generic XML tree used by the parser
  src/parser.rs        Moodle XML -> internal model
  src/grading.rs        Moodle-like grading engine
  src/storage.rs        SQLite persistence
  src/core.rs            Shared app logic (used by Tauri, CLI, and the HTTP server)
  src/server.rs           Localhost-only agent HTTP API (Axum)
  src/lib.rs               Tauri commands + app wiring
  src/main.rs                Desktop app entry point
  src/quality.rs              Quality tooling: lint, autotest, compare, chance baseline
  src/bin/cli.rs               `mqt-cli` headless CLI entry point
  tests/                        Parser + grading + quality unit tests
src/                  SvelteKit frontend (SPA, adapter-static)
  routes/               Quiz list, quiz detail, attempt (take + review)
  lib/components/         Question renderers per Moodle question type
samples/              Sample Moodle XML export used by the parser tests
skills/               Claude Code skill for agent-driven quiz validation
```

## Building & running

### Prerequisites

- Rust (stable) + Cargo
- Node.js 18+ and npm
- Linux: standard Tauri webview dependencies (`webkit2gtk`, etc. — see the
  [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/))

### Desktop app (development)

```bash
npm install
npm run tauri dev
```

### Desktop app (production build)

```bash
npm install
npm run tauri build
```

This produces platform installers/bundles (`.deb`, `.AppImage` on Linux;
`.msi`/`.exe` on Windows; `.dmg`/`.app` on macOS) under
`src-tauri/target/release/bundle/`.

### Rust tests

```bash
cd src-tauri
cargo test
```

Covers the XML parser (all question types, categories, CDATA HTML) and the
grading engine (single/multi-response multichoice, shortanswer wildcards,
numerical tolerance, matching partial credit, essay-is-manual).

## The CLI (`mqt-cli`)

Everything the desktop app can do is also available headlessly, reading from
and writing to the *same* SQLite database (`~/.local/share/moodle-quiz-tester/quizzes.sqlite3`
on Linux, or pass `--db <path>` to use a separate one, e.g. for CI or scripted
testing):

```bash
cd src-tauri
cargo build --release --bin mqt-cli
alias mqt=./target/release/mqt-cli

# Import a quiz
mqt load ../samples/sample-quiz.xml --name "Sample Quiz"

# List imported quizzes
mqt list

# Inspect a quiz's full question data
mqt show <quiz-id>

# Start an attempt (question order shuffled or not)
mqt start-attempt <quiz-id> --shuffle

# Answer a question. Plain text answers by default; pass --json for
# choice arrays (multichoice) or objects (matching/cloze), matching the
# Rust ResponseValue shape.
mqt answer <attempt-id> <question-id> "Paris"
mqt answer <attempt-id> <question-id> '["answer-id-1"]' --json
mqt answer <attempt-id> <question-id> '{"pair-id-1":"Tokyo"}' --json

# Finish & grade the attempt
mqt grade <attempt-id>

# Export results
mqt export <attempt-id> --format json
mqt export <attempt-id> --format markdown

# Run the local agent HTTP server in the foreground
mqt serve --port 4173
```

### Quality tooling

Three pre-import gates plus a reviewer export, designed for CI / agent
pipelines (each gate exits non-zero on failure; add `--json` for
machine-readable reports):

```bash
# Gate 1: format lint — grading traps, missing attachments, malformed cloze,
# unsupported question types, and a random-guess score baseline per question.
mqt lint quiz.xml

# Gate 2: answer-key round-trip — synthesizes the intended-correct response
# for every auto-gradeable question and asserts it grades 100%, plus a
# deliberately wrong response and asserts it doesn't.
mqt autotest --file quiz.xml          # or: mqt autotest <quiz-id>

# Gate 3: randomised versions actually vary — flags answer-key columns that
# are constant across every version of an item.
mqt compare bank.xml                  # one file: versions grouped by question name
mqt compare v1.xml v2.xml v3.xml      # several files: aligned by position

# Reviewer copy: every question with its answer key, weights, and feedback
# inline, for human moderation/sign-off.
mqt export-quiz <quiz-id> > reviewer-copy.md
```

`mqt load` also now warns on stderr about any question it had to drop because
of an unsupported type, instead of dropping it silently.

## The agent HTTP API

The same server can also be started from inside the desktop app (sidebar
button "Start agent API") via the `start_agent_server` Tauri command, or from
the CLI with `mqt serve`. It binds to `127.0.0.1` only — it is never reachable
from the network.

| Method & path                                         | Description                          |
|--------------------------------------------------------|---------------------------------------|
| `GET  /health`                                          | Liveness check                        |
| `GET  /quizzes`                                          | List imported quizzes                 |
| `POST /quizzes` `{xml, name, source_file?}`               | Import a quiz from raw XML             |
| `GET  /quizzes/:id`                                        | Full quiz detail (all questions)        |
| `DELETE /quizzes/:id`                                        | Delete a quiz and its attempts           |
| `GET  /quizzes/:id/attempts`                                  | List attempts for a quiz                  |
| `POST /quizzes/:id/attempts` `{shuffle?}`                       | Start a new attempt                        |
| `GET  /attempts/:id`                                              | Attempt state (responses, results if graded)|
| `POST /attempts/:id/responses/:qid` `{value}`                      | Submit/replace a response                    |
| `POST /attempts/:id/flag/:qid` `{flagged}`                           | Flag/unflag a question                        |
| `POST /attempts/:id/finish`                                            | Grade the attempt and lock it in               |
| `GET  /attempts/:id/export.json`                                        | JSON export                                     |
| `GET  /attempts/:id/export.md`                                           | Markdown export                                  |
| `POST /lint` `{xml}`                                                       | Lint a Moodle XML export (nothing persisted)      |
| `POST /autotest` `{xml}`                                                    | Answer-key round-trip test on raw XML              |
| `GET  /quizzes/:id/autotest`                                                 | Answer-key round-trip test on an imported quiz      |
| `POST /compare` `{sources: [{label, xml}], group_by_name?}`                   | Multi-version answer-key diff                        |
| `GET  /quizzes/:id/reviewer.md`                                                | Reviewer copy (all questions with answer keys)        |

`value` in a submit-response call matches the Rust `ResponseValue` enum
(serialized untagged): a plain string for short-answer/numerical/essay, an
array of answer ids for multichoice, or an object (`{stemId: answerText}` for
matching, `{"1": "Paris", "2": "4"}` for cloze, keyed by 1-based item index).

Example session with `curl`:

```bash
curl -s http://127.0.0.1:4173/quizzes \
  -X POST -H 'content-type: application/json' \
  -d "{\"xml\": $(jq -Rs . < samples/sample-quiz.xml), \"name\": \"Sample\"}"

curl -s http://127.0.0.1:4173/quizzes/<quiz-id>/attempts -X POST -d '{"shuffle": false}'

curl -s http://127.0.0.1:4173/attempts/<attempt-id>/responses/<question-id> \
  -X POST -d '{"value": ["<answer-id>"]}'

curl -s http://127.0.0.1:4173/attempts/<attempt-id>/finish -X POST
```

### Using a local LLM (Ollama) as the agent

This app does not embed any LLM. To have a local model drive a quiz attempt,
point any Ollama-based agent/script at the HTTP API above — e.g. a small
Python/Node script that: `GET`s the quiz, asks the local Ollama model
(`http://localhost:11434/api/generate`) for an answer to each question's
text, then `POST`s the response back to `/attempts/:id/responses/:qid` and
finally calls `/attempts/:id/finish`. Because the API is plain JSON over
localhost HTTP, this requires no SDK — `curl`/`fetch`/`requests` is enough.

## Moodle fidelity notes

- Question text and feedback are stored and rendered as the raw HTML Moodle
  exports (from `<text><![CDATA[...]]></text>`), sanitized client-side with
  DOMPurify before rendering.
- Multichoice: single-response takes the fraction of the selected answer;
  multiple-response sums the (possibly negative) fraction of every selected
  answer and clamps to `[0, 1]`, matching Moodle's per-choice weighting model.
- Shortanswer supports Moodle's `*` wildcard and per-quiz case sensitivity
  (`usecase`).
- Numerical answers match within `tolerance` and support a unit multiplier
  table (`<units>`).
- Matching questions grade the fraction of correctly matched pairs.
- Cloze questions are parsed out of the embedded `{n:TYPE:...}` syntax in the
  question text (`MULTICHOICE`, `SHORTANSWER`/`SAC`, `NUMERICAL` and their
  common Moodle aliases) and graded per-item, averaged across items.
- Essay questions are never auto-graded (matching Moodle, where they require
  manual marking) — they're recorded as "submitted, pending review".
- `category` pseudo-questions in the XML are attached to every question that
  follows them until the next category marker, mirroring Moodle's own
  question-bank import behaviour.
- **Embedded files**: `<file encoding="base64">` attachments under
  `<questiontext>` (e.g. CSV datasets or scripts linked via
  `@@PLUGINFILE@@/name`, common in R/exams-generated quizzes) are extracted,
  offered as downloadable attachments on the question, and the
  `@@PLUGINFILE@@` links in the question text are rewritten to same-page
  anchors pointing at them — since those Moodle-relative URLs can't resolve
  outside a live Moodle instance.

## Sample data

`samples/sample-quiz.xml` exercises every supported question type and is used
by the Rust parser/grading tests. Import it from the app's drag-and-drop zone,
or via `mqt-cli load samples/sample-quiz.xml`, to try the whole flow quickly.

## License

MIT — see [LICENSE](LICENSE).
