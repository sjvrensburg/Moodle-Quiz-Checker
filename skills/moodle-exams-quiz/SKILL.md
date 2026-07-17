---
name: moodle-exams-quiz
description: |
  Author randomised Moodle quizzes in R Markdown using R's `exams` package (exams2moodle / exams2pdf). Use whenever the task is creating, editing, or debugging quiz questions, practicals, or the practical component of a test/exam that will be imported into Moodle — including single-choice (schoice), multiple-response (mchoice), numeric (num), and mixed cloze questions, the paired student `.R` / answer-key `.Rmd` "fill-in-the-blank" workflow, data-generation scripts, and the `generate_moodle_quiz.R` / `generate_solutions.R` generators. Trigger terms: "Moodle quiz", "exams package", "exams2moodle", "cloze question", "schoice/mchoice", "practical question", "randomised quiz", "R/exams", "generate Moodle XML". Carries opinionated assessment-design conventions from university statistics teaching, but works for any R/exams → Moodle workflow.
version: 1.0.0
---

# Moodle quizzes with R/exams

Produce randomised, auto-graded Moodle questions from R Markdown using the `exams` package. A question is a single `.Rmd` file; a generator script bundles questions into one Moodle XML file you import into the Question Bank.

## When to use this skill

- Writing or editing a quiz/practical question (`.Rmd`) for Moodle.
- Building or fixing the `generate_moodle_quiz.R` generator for a folder of questions.
- Authoring the paired student-facing `.R` script + answer-key `.Rmd` ("fill-in-the-blank") used by simulation/applied questions.
- Generating a worked-solutions PDF (`exams2pdf`) or a reproducible data-generation script (`_generate_data.R`).
- Building the practical component of a test or exam (same engine, `n = 1`, mark-weighted with `expoints`).

If the request is about typeset lecture notes or slides (not auto-graded questions), this is the wrong skill.

## The big picture

```
question_xxx.Rmd  ─┐
question_yyy.Rmd   ├─► generate_moodle_quiz.R ──exams2moodle()──► "Quiz Name.xml" ──import──► Moodle Question Bank
sim_zzz.Rmd  + sim_zzz.R (student downloads) ─┘
_generate_data.R ──► data.csv (shipped to students + read by the answer key)
generate_solutions.R ──exams2pdf()──► Solutions.pdf
```

Every question `.Rmd` has four sections delimited by **underline headers** (NOT `#` markdown headings):

```
Question
========
<question text, may include an R chunk that randomises values>

Solution
========
<worked solution>

Meta-information
================
extype: schoice
exsolution: 1000
exname: Short descriptive name
exshuffle: TRUE
```

## Question types at a glance

| `extype` | Use for | Answer encoding |
|----------|---------|-----------------|
| `schoice` | single-choice MCQ (exactly one correct) | `exsolution` binary string, e.g. `0100` = 2nd option correct |
| `mchoice` | multiple-response (≥1 correct, "select all") | `exsolution` binary string, e.g. `10110` |
| `num` | a single numeric answer | `exsolution` = the number; `extol` = tolerance |
| `cloze` | several blanks in one question (mix of num/schoice/…) | `exclozetype` lists the per-blank types; `exsolution`/`extol` are `|`-separated |

`cloze` is the workhorse for applied-statistics practicals and exams — one scenario, a download script, and 5–14 graded blanks mixing computed numbers with conceptual MCQs. **Weight your attention here.**

## The non-negotiable rules

These cause silent, student-facing breakage. Full detail in `reference/pitfalls-and-conventions.md` — but never ship without checking them.

1. **Cloze schoice display.** Every `exams2moodle()` call that includes any cloze MUST pass
   `cloze = list(cloze_schoice_display = "MULTICHOICE_V")`. Without it, long sentence-options collapse into a dropdown students can't read. Verify: `grep -oE '\{1:[A-Z_]+:' *.xml | sort -u` → only `{1:MULTICHOICE_V:`.
2. **No `~` inside cloze option text.** Moodle's cloze format uses `~` as the option separator; an R formula like `lm(y ~ x)` inside an option string splits it into a phantom extra option. Reword to remove the tilde.
3. **Math delimiters.** Inline `$...$`, display `$$...$$`. **Never `\[...\]`** — Moodle strips the backslash, leaving bare brackets.
4. **`exsolution` is positional.** For schoice/mchoice it is a binary string where `1` marks the correct option *in source order* (before display shuffle). Generate it with `mchoice2string(sol, single = TRUE)` (schoice) or `mchoice2string(sol)` (mchoice) rather than hand-typing.
5. **Names/parity between the `.R` student script and the `.Rmd` answer key.** If the answer key verifies `round(v["n=1000"], 6)`, the student script must build `v` with those names. Variable names in the script must match the names referenced in the question text.
6. **Placeholder in student scripts** = a `your_code_goes_here()` helper, never a bare `???` (leaks a confusing parser error) and never `stop("Fill in...")` (ESL students type their answer inside the string). See the paired-script reference.
7. **No verbatim answers in blank comments — hints only, in practicals and tests/exams alike.** The comment beside a `your_code_goes_here()` blank must never contain the exact code that fills it (e.g. `# trimws(tolower(df$gender))`) — that is the answer, handed over. Point at the relevant function(s) or help page instead (`# Consider tolower() and trimws().` or `# Consult ?scale.`), or describe the goal without naming the tested operator/argument. Litmus test: if pasting the comment's content into the blank produces a correct, working answer, rewrite it. See the "Scaffolding level" bullet in the paired-script reference.
8. **Quiz immutability.** Once a student has begun an attempt, Moodle won't let you fix the question. Verify everything *before* generating and importing. Fixes for a live quiz only land next year + the exam.
9. **Every applied question is independent.** Within one quiz, no two applied questions may share a downloaded dataset/filename, and no scenario may say "continuing with…" a previous question's file — Moodle randomises question order and versions, so a shared file makes correctness depend on what a student downloaded for a *different* question. Prefer per-version simulated data (each `.R`/`.Rmd` draws its own seed); if a fixed CSV must ship, give every applied question its own uniquely-named file and self-contained scenario. Previously-separate questions may be merged into one if they're genuinely one scenario. See A11 in the pitfalls reference.
10. **Practice items (`n > 1`) must regenerate their data per version — and a random seed alone doesn't prove it worked.** A blank can look randomised but still be pinned to one value across every version: either trivially (a count computed from two constants, e.g. `n_train = 0.8 * 200` when both are fixed) or non-obviously (a label built from `quantile(x, 0.65)` of that same `x` makes the majority-class baseline accuracy equal `0.65` in *every* version, regardless of seed — the labelling rule itself fixes the answer). After generating a batch, decode each version's answers from the XML and confirm every blank actually differs — except where a constant is the intended teaching point (a fixed grid point like `k = 25`, a math constant like `dnorm(0)`, or a method's invariant like "ridge never zeros a coefficient"). See "Per-version regenerated data" in the generators-and-data reference for the mechanism and a verification recipe.

## Authoring workflow

1. **Pick the type** from the table above. For anything with a downloadable script + computed answers, use `cloze` with a paired `.R`.
2. **Copy a template** from `templates/` (see below) — don't write the YAML/section skeleton from memory.
3. **Author the model answer first** for tests/exams: it is the source of truth; derive the student version from it.
4. **If data is involved, pick fixed vs per-version.** Tests/exams (`n = 1`) and any scenario that's fundamentally about a *given* fixed dataset: write/extend `_generate_data.R` with a documented `set.seed(...)`, run it once. Practice quizzes (`n > 1`, the common case): regenerate the data **inside** the `data-generation` chunk with a fresh random seed each render — see "Per-version regenerated data" in the generators-and-data reference. Default to per-version regeneration; only fall back to a fixed CSV when the exercise genuinely requires one immutable dataset.
5. **Re-derive the answer key inside the `.Rmd`** in a hidden `data-generation` chunk so `exsolution` is always computed, never transcribed.
6. **Wire it into `generate_moodle_quiz.R`** (add the filename to the `exercises` vector).
7. **Generate and verify** — render the XML, grep the checks above, run a filled-in copy of the student `.R` to confirm its numbers match the key, and (for `n > 1`) confirm genuine variety across versions per rule 10.

## Reference files (read as needed)

- `reference/question-types.md` — full annotated templates and rules for `schoice`, `mchoice`, `num`, and simple `cloze`; how `exsolution`/`extol`/`exshuffle`/`exname` work; randomising values in the question chunk.
- `reference/cloze-and-paired-scripts.md` — **the complex, high-value part.** Multi-blank cloze anatomy, `##ANSWERn##` markers, `exclozetype`/`exsolution`/`extol` alignment, building these dynamically, `include_supplement`, the paired student `.R` ↔ answer-key `.Rmd` contract, `expoints` for mark-weighting in tests/exams.
- `reference/generators-and-data.md` — `generate_moodle_quiz.R` options (`n`, `name`, `schoice`, `cloze`, `converter`), practice (`n = 5`) vs exam (`n = 1`), **per-version regenerated data** (the rejection-sampling + `script_template`/`__SEED__` mechanism, and how to verify genuine variety), `_generate_data.R` conventions for fixed datasets, worked-solutions PDF via `exams2pdf` + `plain_href.tex`, importing into Moodle.
- `reference/pitfalls-and-conventions.md` — the full pitfall checklist (the non-negotiable rules above, expanded) plus higher-level authoring conventions (mark allocation, reveal-checking, subpart independence, formula-sheet hygiene, notation, ESL wording) and the pre-publish checklist.

## Templates (copy, don't retype)

In `templates/`:
- `schoice.Rmd`, `mchoice.Rmd`, `num.Rmd` — the three simple types.
- `cloze_mixed.Rmd` + `cloze_mixed_paired.R` — a complete cloze question with a **fixed-seed** downloadable student script (`n = 1` tests/exams, or a deliberately-unchanging scenario).
- `cloze_regenerated.Rmd` — a complete cloze question that **regenerates its data and its downloadable script per version** via rejection sampling (`n > 1` practice quizzes — the default; see rule 10 and "Per-version regenerated data" in the generators-and-data reference).
- `generate_moodle_quiz.R` — the generator (practice and exam variants in comments).
- `generate_solutions.R` + `plain_href.tex` — worked-solutions PDF.
- `_generate_data.R` — reproducible data generation.

Start from the template that matches the type, then adapt the statistics/wording. The templates already bake in the non-negotiable rules above.
