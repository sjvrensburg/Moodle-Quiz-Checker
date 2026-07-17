# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Moodle Quiz Tester: a fully offline desktop app (Tauri 2 + SvelteKit) for importing, taking, and grading Moodle XML quiz exports without a real Moodle instance. It is agent-first: everything the GUI does is also available through a headless CLI (`mqt-cli`) and a localhost-only HTTP API, all backed by the same SQLite database.

## Commands

```bash
npm install               # once
npm run tauri dev         # run desktop app in development
npm run tauri build       # production bundles → src-tauri/target/release/bundle/
npm run check             # svelte-check (TypeScript) on the frontend

cd src-tauri
cargo test                            # all Rust tests (parser + grading)
cargo test --test parser_tests       # one test file (parser_tests, grading_tests, quality_tests)
cargo test <test_name>               # one test by name
cargo build --release --bin mqt-cli  # build the headless CLI
```

There is no lint config beyond `svelte-check`/`cargo` defaults, and no frontend tests — all tests are Rust integration tests in `src-tauri/tests/`, driven by `samples/sample-quiz.xml` (which exercises every supported question type).

CLI smoke test with an isolated DB: `mqt-cli --db /tmp/test.sqlite3 load samples/sample-quiz.xml` then `list` / `start-attempt` / `answer` / `grade` / `export`. Without `--db`, the CLI shares the desktop app's DB (`~/.local/share/moodle-quiz-tester/quizzes.sqlite3`).

## Architecture

All application logic lives in the Rust crate (`src-tauri/`); the Svelte frontend is a thin rendering layer.

**The key design invariant**: `core::App` (`src-tauri/src/core.rs`) is the single high-level API — import, attempts, responses, grading, export. Three frontends wrap it and must stay in sync:

1. **Tauri commands** in `src/lib.rs` (called from the frontend via `src/lib/api.ts` → `invoke`)
2. **`mqt-cli`** in `src/bin/cli.rs` (clap)
3. **Agent HTTP server** in `src/server.rs` (Axum, binds 127.0.0.1 only; started via `mqt serve` or the app's `start_agent_server` command)

When adding an operation, add it to `core::App` first, then expose it in all three surfaces (plus `src/lib/api.ts` and `src/lib/types.ts` on the frontend).

**Data flow within the crate**: `parser.rs` (Moodle XML → model, built on the minimal generic tree in `xmltree.rs`) → `model.rs` (Quiz/Question/Attempt/ResponseValue domain types) → `grading.rs` (Moodle-faithful grading engine) → `storage.rs` (rusqlite persistence, quizzes stored as serialized JSON) → `export.rs` (JSON/Markdown, plus a quiz-level reviewer document with answer keys).

**Quality tooling** (`quality.rs`): pure functions over XML/the model — `lint_quiz_xml` (format/grading-trap findings + analytic random-guess baseline), `autotest_quiz` (synthesizes intended-correct and deliberately-wrong responses from the answer key and asserts the grader agrees/discriminates), `compare_quizzes` (multi-version answer-key diff that flags constant columns across randomised versions). Exposed as `mqt-cli lint/autotest/compare/export-quiz` (non-zero exit on failure — used as CI gates), `/lint`, `/autotest`, `/compare`, `/quizzes/:id/reviewer.md` HTTP endpoints, and Tauri commands. The parser never drops unsupported question types silently — `parse_quiz_xml_with_warnings` reports them, and new question types must keep it that way.

**The companion skill** (`skills/moodle-quiz-tester/`) is the canonical copy of the Claude Code skill documenting the validation workflow; `~/.claude/skills/moodle-quiz-tester/` is an installed copy. When changing CLI commands, HTTP endpoints, or report formats, update the skill (and README) in the same change, then re-sync the installed copy.

**`ResponseValue`** (`model.rs`) is the answer format shared by every interface — serialized untagged: plain string (shortanswer/numerical/essay), array of answer ids (multichoice), or object (matching keyed by stem id; cloze keyed by 1-based item index). Frontend types in `src/lib/types.ts` mirror the Rust model and must be updated together.

**Frontend** (`src/`, SvelteKit SPA via adapter-static, Svelte 5): routes are quiz list (`+page.svelte`), quiz detail (`quiz/[id]`), and attempt take/review (`attempt/[id]`). `lib/components/QuestionRenderer.svelte` dispatches to per-question-type components in `lib/components/questions/`. All Moodle HTML (question text, feedback) is raw exported HTML — always sanitize with `lib/sanitize.ts` (DOMPurify) before rendering.

## Moodle fidelity rules

Grading must match Moodle's behaviour — see "Moodle fidelity notes" in README.md for the full contract. Highlights: multichoice multiple-response sums per-choice fractions (possibly negative) clamped to [0,1]; shortanswer supports `*` wildcards and `usecase`; numerical uses tolerance + unit multipliers; cloze is parsed from embedded `{n:TYPE:...}` syntax and averaged per item; essays are never auto-graded; `category` pseudo-questions attach to subsequent questions; `<file encoding="base64">` attachments are extracted and `@@PLUGINFILE@@` links rewritten to local anchors.

## Related tooling

A `moodle-quiz-tester` Claude Code skill (`~/.claude/skills/moodle-quiz-tester/`) documents driving `mqt-cli`/the HTTP API to validate quiz XML from any source.
