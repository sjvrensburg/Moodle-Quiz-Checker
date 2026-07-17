# Pitfalls and authoring conventions

Two parts: (A) concrete traps that have produced student-facing bugs — check every one before shipping; (B) higher-level design conventions for tests/practicals. Both come from real STAT-module review cycles.

## A. Pitfalls (the pre-publish checklist)

### A1. Cloze schoice display — always `MULTICHOICE_V`
Every `exams2moodle()` call that includes a cloze must pass
`cloze = list(cloze_schoice_display = "MULTICHOICE_V")`.
Otherwise long sentence-options render as a dropdown that truncates and hides alternatives, making the question much harder than intended. The package default is inconsistent (radio for math options, dropdown for plain text). **Verify:** `grep -oE '\{1:[A-Z_]+:' *.xml | sort -u` → only `{1:MULTICHOICE_V:`.

### A2. No `~` inside cloze option text
Moodle cloze uses `~` as the option separator and exams does not escape it. An R formula like `lm(y ~ x, weights = w)` inside an `opts_*` element splits into a phantom extra option. Reword to remove the tilde. **Check:** `grep -nE 'opts_[a-z]' *.Rmd`, then eyeball those vectors for `~`. After rendering, the `~` count per schoice block should be `options − 1`.

### A3. Math delimiters: `$...$` and `$$...$$`, never `\[...\]`
Moodle strips the backslash from `\[ \]`, leaving bare brackets in the rendered question. Inline `$...$`, display `$$...$$`. (In handout `.Rmd` notes `\[...\]` is fine — but **not** in exams questions.)

### A4. Names parity on numeric vectors
If the answer key or question verifies `round(v["n=1000"], 6)`, the student `.R` must construct `v` with `names(v) <- paste0("n=", sample_sizes)`. Without names, the lookup returns `NA` and a correct student appears wrong.

### A5. Variable-name drift between `.R`, `.Rmd`, and prose
When the question text names a variable (`R2_h`, `n`, `mean_a`), the `.R` slot, its comment, and any `.Rmd` verification line must use the *same* identifier — no synonyms. Mismatches force students to invent locals and second-guess the spec.

### A6. `pmax`/`pmin`, not `max`/`min`, for element-wise work
Mathematical `max(δ, e_i²)` over a sample is element-wise; R's `max()` collapses to a scalar and silently breaks downstream numbers. If a comment describes an element-wise operation, write `pmax`/`pmin` and ideally add "use pmax — max collapses to a scalar".

### A7. Placeholder = `your_code_goes_here()`, never `???` or `stop("Fill in...")`
- `x <- ???` parses as chained `?` operators and throws a misleading missing-bracket-looking error far from the real line.
- `x <- stop("Fill in your code here ...")` parses but ESL students type their answer inside the quotes and get it echoed back as an error.
- Define once per `.R`:
  ```r
  your_code_goes_here <- function(...) {
    stop("Placeholder not replaced. Replace each call to `your_code_goes_here()` with your own code.")
  }
  ```
  Each blank: `eps <- your_code_goes_here()  # Step 1: draw error vector`.

### A8. `exsolution`/`exclozetype`/`extol` count parity
Every blank needs an entry in `exclozetype`, `exsolution`, and `extol` (and `expoints` if present). Build them by `paste(..., collapse = "|")` over R objects so they can't drift. Schoice blanks take `extol` `0`.

### A9. Compute, never transcribe
Every number in the prose, the solution, and `exsolution` must come from an inline `` `r ... ` `` referencing the same R object. A hand-typed number that should track a random draw is the classic version-drift bug.

### A10. Reproducibility for shipped values
A question whose values appear in a shipped CSV, or in an exam, must set its own `set.seed(...)` inside the chunk (matching the student `.R`'s seed), so the key is byte-stable across renders.

### A11. Every applied question is independent — its own scenario, its own uniquely-named data
Within one quiz, no two applied questions may share a downloaded dataset, and no question's scenario may depend on a prior question ("Continuing with `hospital_admissions.csv`, …"). Moodle randomises question order and pools versions independently, so a student who doesn't re-download a file used by an earlier question (or whose earlier attempt used a different pooled version) gets silently wrong answers on the later one — and reviewing past attempts becomes impossible because the "same" filename may have meant different data each time.
- **Prefer per-version regenerated data** (see `reference/generators-and-data.md`, "Per-version regenerated data"): each applied item's `.Rmd` draws its own seed and rejection-samples its own data internally, then ships a freshly-written script and/or CSV — this is automatically independent, nothing to check. This is also the fix for A12 (constant answers despite a random seed): the same per-version regeneration that gives filename independence gives answer variety, as long as the counts/parameters that matter are randomised too, not just the seed.
- **If a question must ship a fixed CSV** (e.g. a `n = 1` test/exam item, or a data-cleaning exercise deliberately about one *given*, unchanging messy dataset), give **every applied question its own CSV with a filename used by no other question in the quiz** — a distinct scenario (different entity/site/cohort), not just a re-seeded copy under the same name. Do not have two applied items `read.csv()` the same shipped file.
- Two previously-separate applied questions may be **merged into one** cloze item if their content is genuinely one scenario — that's fine and often better than inventing a second dataset for a thin second question.
- **Check before shipping:** for each applied `.Rmd`, grep its `include_supplement(...)` / `read.csv(...)` filename across every other applied `.Rmd` in the same folder — it must not appear in any other file. After generating the XML, `grep -c '<that filename>' *.xml` should attribute every hit to that one question's `R# Q#` blocks only (see `practical_01_data_fundamentals/` for a worked fix — three applied items were rewritten from one shared `hospital_admissions.csv` to three independent facility datasets).
- Concept MCQs that merely *mention* a filename in prose (no download, no `read.csv`) aren't subject to this rule — only questions that actually import/ship data.

### A12. A random seed doesn't guarantee a varying answer — verify per-version variety
For any practice item (`n > 1`) that regenerates its own data (see `reference/generators-and-data.md`, "Per-version regenerated data"), a fresh `seed` each version doesn't automatically make every blank vary. Two real failure modes, both caught only by decoding actual XML output and comparing versions, never by eyeballing the code:
- **Trivial arithmetic from fixed constants.** `n_train = 0.8 * n` is identical every version if `n` and the split proportion are both hard-coded — the seed never touches it. Usually tolerable as one "warm-up" blank if the exercise's substantive blanks (coefficients, RMSE, counts of injected messiness, …) genuinely vary.
- **Self-referential construction.** A class label built as `ifelse(score > quantile(score, 0.65), ...)` pins the majority-class baseline accuracy to `0.65` in *every* version, because the labelling rule always carves the same split out of whatever `score` turns out to be — the "randomness" never reaches that particular answer. This kind is worth fixing: randomise the pinning parameter itself (here, the quantile probability) the same way the seed is randomised.
- **Check:** after generating a batch, decode each version's cloze answers from the XML (not from whatever file is left on disk — rendering interleaves files/versions, so the last write isn't necessarily the last version) and diff column-by-column across the `n` versions. A constant column is fine only if it's the intended teaching point (a fixed grid point, a math constant like `dnorm(0)`, a method's genuine invariant like "ridge never zeros a coefficient exactly") — otherwise it's a bug.
- Applies to CSV-shipping practicals too: randomise the *counts* of injected messiness (`rcount(lo, hi)` per category, per version), not just the seed — a fixed count (e.g. always exactly 6 invalid ages) makes that count's answer identical across every re-attempt even though the underlying row values differ.

## B. Design conventions (tests and practicals)

### B1. No 1-mark non-MCQ items
Every non-MCQ question or subpart is worth ≥ 2 marks; MCQs sit at 1 mark each. One-mark short-answer items create partial-credit grading confusion. Fold a naturally-1-mark sub-task into an adjacent part. (Implement per-blank marks with `expoints`.)

### B2. Proofs as single blocks
A 5–9 mark proof is one question with an internal roadmap ("Your proof should: (i)…, (ii)…, (iii)…"), not four 2-mark subparts. Subparts lock the student into one argument order and break mathematical flow. Reserve subparts for genuinely independent content.

### B3. Reveal-checking across the paper
No question's setup, model answer, or the formula sheet may leak the answer to another question. State modified setups via equations ("$\boldsymbol{\Sigma}$ is not a scalar multiple of the identity"), not via cross-references to another question's numbered assumptions. After drafting, read every question against every other and against the formula sheet.

### B4. Subpart independence
A student who fails subpart (a) must still be able to attempt (b), (c)…. For cloze items, every numeric blank must be computable from the data + question text, not from an earlier blank's value. If (b) genuinely needs (a)'s output, merge them into one question (sequential proofs are the exception — handle via B2).

### B5. Formula-sheet hygiene
The formula sheet stays with the student through both theory and practical sections — every formula on it is a freebie. List only general building blocks (model setup, estimator forms, projection/annihilator definitions, trace algebra, critical values). Never list a formula that is itself the punchline answer to a question on the paper (e.g. $s^2 = \mathbf{e}'\mathbf{e}/(n-k)$ when a question asks for it).

### B6. Test scaffolding < practical scaffolding — but neither gets the verbatim answer
Never write the exact replacement expression in a blank's comment, in practicals or tests/exams alike (see A7 / SKILL.md rule 7) — that hands over the answer. Both formats get a hint (function name, help-page pointer, or a plain-English description of the goal); tests/exams get a *stricter* hint that additionally avoids naming the function or argument value the MCQs are testing. "Pull the estimated slope coefficient out of the fitted model" works for both; a practical may add "— consult `coef()`", a test may not go further than that. Inline comments in a test read `# Step N`, not `# Step N: draw with rnorm(n, 0, sigma_i)` (that names the exact call). Genuinely obscure API mechanics (reaching an SE inside `summary(fit)$coefficients[2, "Std. Error"]`) are fair to hint even in a test, since the target is statistical understanding, not R-API navigation.

### B7. Notation must match the lecture notes
Use the notes' terminology, not generic-textbook synonyms, so students don't have to translate between paper and notes. Build an explicit convention list for your module and apply it in every question. (Example of what such a list looks like, from one econometrics module: `Cov(·)` for matrix covariances, `V[·]` for scalar variances; "homoskedasticity"/"scalar covariance assumption", never "spherical errors"; $\boldsymbol{\Sigma}$ not $\sigma^2\boldsymbol{\Omega}$; the course's own assumption numbering, not Wooldridge's `MLR.*`.)

### B8. ESL-friendly wording
The cohort includes second-language English speakers; ambiguity costs them disproportionately. Prefer plain constructions ("only the provincial totals are released" over "the data custodian releases the data only as provincial totals"), avoid jargon-y nouns, and avoid templates that look like fillable slots (the `your_code_goes_here()` helper exists for this reason — see A7).

## Workflow corollary

Author the **model-answers** version first — it is the source of truth — then derive the student paper by toggling solutions off and dropping answer paragraphs. For a fixed-dataset item (test/exam, or a practical deliberately about one unchanging dataset): generate data via `_generate_data.R` (documented seed; seed-sweep if a specific outcome is needed). For a practice item (the common case): regenerate data per version inside its own chunk via rejection sampling (A12). Either way, cross-verify the numbers across (1) the model-answers PDF, (2) the cloze `.Rmd` answer keys, and (3) a filled-in run of the student `.R` — and for `n > 1`, confirm genuine variety across versions (A12) — before generating the Moodle XML.
