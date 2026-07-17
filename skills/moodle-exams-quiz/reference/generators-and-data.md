# Generators, data scripts, and solutions PDFs

A folder of question `.Rmd` files is turned into one importable Moodle XML by a small generator, `generate_moodle_quiz.R`. Data-backed practice items regenerate their data per version inside their own `data-generation` chunk (the default — see below); a data-backed test/exam item, or a practice item about one genuinely fixed dataset, adds `_generate_data.R` instead. Tests/exams also add `generate_solutions.R` for a worked-answers PDF.

## `generate_moodle_quiz.R` — the Moodle XML generator

Run from inside the question folder (`source("generate_moodle_quiz.R")` or `Rscript`). Canonical form:

```r
library(exams)

set.seed(2026)                 # reproducible randomisation across the whole batch

exercises <- c(
  "question_one.Rmd",
  "question_two.Rmd",
  "sim_three.Rmd"
)

exams2moodle(
  file      = exercises,
  n         = 5,                                       # versions per question (see below)
  name      = "MODULE Topic Name",                   # Moodle question CATEGORY name
  dir       = getwd(),                                 # where the .xml is written
  schoice   = list(shuffle = TRUE),                    # shuffle options at display time
  cloze     = list(cloze_schoice_display = "MULTICHOICE_V"),  # REQUIRED if any cloze
  converter = "pandoc-mathjax",                        # LaTeX → MathJax for Moodle
  quiet     = FALSE
)
```

### Argument reference

- **`file`** — character vector of question filenames, in the order they should appear.
- **`n`** — number of randomised versions generated **per question**. The N versions become a Moodle "question pool"; Moodle hands each student one at random.
  - **Practice quizzes: `n = 5`.** Gives a pool so students can re-attempt with fresh numbers.
  - **Tests/exams: `n = 1`.** A one-shot assessment needs a single fixed version. (Items with their own internal `set.seed` are already deterministic regardless.)
- **`name`** — sets the Moodle question-bank **category** the questions import into. Use the quiz's human name.
- **`dir`** — output directory for the `.xml`. `getwd()` keeps it beside the sources.
- **`schoice = list(shuffle = TRUE)`** — Moodle shuffles option order again when displaying. Complementary to `exshuffle: TRUE` in each file (which shuffles at generation).
- **`cloze = list(cloze_schoice_display = "MULTICHOICE_V")`** — **REQUIRED whenever any file is a cloze.** Forces every cloze schoice sub-question to render as a vertical radio-button list instead of the package-default dropdown. Without it, long sentence-options get truncated in a dropdown and the question becomes far harder than intended. (The package default is hybrid — radio for math-containing options, dropdown for plain text — which produces inconsistent quizzes; setting this explicitly fixes that.) The multi-response analogue, if you ever use mchoice clozes, is `cloze_mchoice_display = "MULTIRESPONSE_S"`.
- **`converter = "pandoc-mathjax"`** — renders LaTeX math as MathJax, which Moodle displays. Don't change it.

### Output filename quirk

`exams2moodle()` derives the `.xml` filename from `name`, replacing spaces with underscores in some versions and keeping them in others. Either way one `.xml` lands in `dir`. The README/CLAUDE note may say `MODULE_Topic.xml` while the file is `MODULE Topic.xml` — confirm by listing the folder after generating.

## Importing into Moodle

1. Course Administration → **Question Bank → Import**.
2. Format: **Moodle XML format**.
3. Upload the `.xml`. The questions land in the category named by `name`.
4. Build the quiz activity and add the questions (or a random selection from the category).

**Immutability:** once a student has begun an attempt, Moodle blocks edits to the deployed questions. Re-importing a fixed XML does not patch a live quiz. Verify before importing; fixes only reach next year's deployment and the exam. The lecturer maintains the deployed pool by hand — don't propose automating the upload.

## Per-version regenerated data (practice quizzes, `n > 1`) — the default

For a practice pool (`n > 1`), prefer regenerating the data **fresh inside each question's `data-generation` chunk** over shipping one fixed CSV. Every re-attempt then gets genuinely new numbers to work with, not a re-shuffled copy of the same file. Reach for a fixed `_generate_data.R` dataset instead only when the exercise is fundamentally about one *given*, immutable dataset (see the next section), or for `n = 1` tests/exams.

### The mechanism: `compute()` + rejection sampling + a `script_template`

```r
compute <- function(seed) {
  set.seed(seed)
  n <- 200
  x <- rnorm(n, 50, 10)
  y <- 3 + 0.8 * x + rnorm(n, 0, 5)
  fit <- lm(y ~ x)
  list(slope = round(coef(fit)["x"], 4), r2 = round(summary(fit)$r.squared, 4))
}

## Per-version seed: rejection-sample so the story is clean (a strong,
## unambiguous, positive slope every version — not "sometimes flat").
repeat {
  seed <- sample(10000:99999, 1)
  a <- compute(seed)
  if (a$r2 >= 0.5 && a$slope > 0) break
}

## Write THIS version's student script and attach it for download.
script_template <- '# ...
set.seed(__SEED__)
n <- 200
x <- rnorm(n, 50, 10)
y <- 3 + 0.8 * x + rnorm(n, 0, 5)
fit <- your_code_goes_here()   # Consult ?lm.
'
writeLines(gsub("__SEED__", seed, script_template, fixed = TRUE), "applied_xyz.R")
include_supplement("applied_xyz.R")
```

This lives inside the exercise's hidden `data-generation` chunk (`echo=FALSE, results="hide"`), same as any other answer-key computation.

- **`compute(seed)`** does exactly what the finished student script does — same `set.seed()` placement, same code — so its return value is both the answer key *and* the thing checked by the rejection loop.
- **The rejection loop** draws a fresh seed each time `exams2moodle()` renders this version, and only accepts one that produces a clean pedagogical story (a clear effect, a non-degenerate split, an unambiguous "which model wins" answer). Bound the criteria to what the question actually needs — over-constraining just makes the loop slower.
- **`script_template`** is a string with `__SEED__` (and other `__PLACEHOLDER__`s as needed — see below) substituted via `gsub(..., fixed = TRUE)`, then written out and `include_supplement()`d. The student downloads a script customised to *their* version but otherwise identical in structure every time.
- The top-level `set.seed(2026)` in `generate_moodle_quiz.R` makes the whole batch reproducible: re-running the generator with an unchanged question set reproduces the same `n` versions in the same order.

### Shipping a per-version CSV instead of inline-simulated data

Some exercises are about *cleaning a given messy dataset*, not fitting a model to freshly simulated numbers — the download has to be an actual CSV with realistic problems (missing values, inconsistent text, impossible entries), not something the student's own script re-simulates. The same rejection-sampling idea still applies, just building a `data.frame` and `write.csv()`-ing it per version instead of substituting into a script string:

```r
repeat {
  seed <- sample(10000:99999, 1)
  df <- make_messy_data(seed, bad_n = rcount(4, 8), na_n = rcount(8, 16), ...)
  if (<structural guarantees hold, e.g. all categories survive cleaning>) break
}
write.csv(df, "this_question.csv", row.names = FALSE)
include_supplement("this_question.R")   # the static paired script, unchanged
include_supplement("this_question.csv") # THIS version's freshly written data
```

Randomise the **messiness counts**, not just the seed — `rcount(lo, hi)` (`sample(lo:hi, 1)`) drawn once per category, per version. A seed alone reshuffles *positions*; if the counts themselves are fixed constants (e.g. always exactly 6 bad ages), the count-based answers stay constant across every version even though the seed changed (rule 10) — the counts have to vary too.

**Gotcha: `exams` knits each exercise in an isolated sandbox.** Each `.Rmd` is rendered in a temporary directory containing *only that file* — a chunk's `source("helper.R")` will fail even though `helper.R` sits right next to the `.Rmd` on disk, because sibling files aren't copied in (only `include_supplement()`-registered files are, and that also marks them for student download, which you don't want for an internal generator). If several applied questions in one folder share a data-generating function, **inline it identically in each question's chunk** rather than sourcing a shared file, with a comment noting the duplication and to keep the copies in sync if the generator changes.

### Verifying genuine variety (rule 10)

A random seed does not guarantee a varying answer — some quantities are pinned by the exercise's own construction regardless of the seed. Two real examples caught this way:

- A holdout split's `n_train`/`n_test` are `0.8 * n` and `0.2 * n` — if `n` and the split proportion are both fixed constants, these are deterministic arithmetic, identical in every version (usually fine as a single trivial "warm-up" blank if the *substantive* blanks — slopes, RMSE, etc. — already vary).
- A binary label built as `ifelse(score > quantile(score, 0.65), ...)` makes the **majority-class baseline accuracy** equal `0.65` in every version, because the labelling rule is self-referential: it always carves the same 65/35 split out of whatever `score` turns out to be. This one is worth fixing (randomise the quantile probability itself, threaded through the same `__PLACEHOLDER__` mechanism as the seed) because it silently defeats the point of the exercise — a student never actually has to inspect their data to answer it.

After generating a batch, decode each version's answers straight from the XML (not from whatever `.csv`/`.R` happens to be left on disk — rendering order interleaves files and versions, so the last file written isn't necessarily the last version) and diff them column-by-column:

```r
xml <- paste(readLines("Quiz Name.xml"), collapse = "\n")
nums <- regmatches(xml, gregexpr("\\{1:(?:NUMERICAL:=|MULTICHOICE_V:)[^:}]*", xml, perl = TRUE))
# group by version (the <name> tags read "R<k> Q<j> : <exname>") and compare columns
```

Any column that's identical across all `n` versions is either an intentional constant (a fixed grid point, a math fact like `dnorm(0)`, a method's genuine invariant — leave these) or a variety bug (fix by randomising whatever parameter is silently pinning it).

## `_generate_data.R` — fixed datasets (tests/exams, or a genuinely immutable scenario)

For applied questions that ship **one** CSV to every version — appropriate for `n = 1` tests/exams, or a practice item that's deliberately about one unchanging dataset — a single script builds **every** such CSV the folder references, under documented seeds.

```r
set.seed(20260601L)            # documented seed — re-running overwrites CSVs byte-for-byte

# ── Q4: electricity demand, heteroskedastic by design ──────────────────────
n <- 120
hh      <- runif(n, 5, 30)
sigma_i <- 2 + 0.4 * hh                       # variance grows with hh
demand  <- 8 + 1.6 * hh - 0.5 * temp + rnorm(n, 0, sigma_i)
write.csv(data.frame(...), "electricity_demand.csv", row.names = FALSE)
```

Conventions:

- **One CSV per applied question, never shared.** If two applied questions in the same quiz both `read.csv()` the same file, a student who doesn't re-download it for the second question (or whose two questions pooled different versions) gets silently wrong answers, and the shared filename couples otherwise-independent items. Give each applied question its own uniquely-named CSV and its own self-contained scenario (a different entity/site/cohort), not "continuing with `X.csv`" from an earlier question. A shared underlying generator function (parameterised by seed/entity, as below) is fine — the *output files* must still be distinct. See A11 in `pitfalls-and-conventions.md`.
- **One seed, documented.** A comment at the top states the seed and what each output is. The CSV's exact numbers appear in the question text and answer key, so **re-running with a changed seed retroactively invalidates a published quiz** — re-run only with intent.
- **Design the data to exhibit the phenomenon** the question is about (heteroskedasticity that the BP test rejects; a trend + seasonal pattern with a clear lag-12 ACF spike; a near-unit-root AR(1)). If a specific outcome is needed (e.g. a BP p-value in `[0.15, 0.30]`), run a small seed-sweep to find a seed that lands in range, then hard-code it.
- **The answer-key `.Rmd` reads the same CSV** via `read.csv()` inside its hidden chunk and `include_supplement()`s it for download. Numbers are never copied between the data script and the key.
- **Round on write** (`round(x, 3)`) so the shipped CSV matches what students see and re-derive.

Run `_generate_data.R` **before** generating the quiz XML or the solutions PDF.

## `generate_solutions.R` + `plain_href.tex` — worked-answers PDF

For tests/exams you usually want a PDF of the practical questions with full worked solutions (for the memo/moderation bundle). Use `exams2pdf()` on the **same** question set and seed, with a local template that renders the script/CSV download links as plain filenames:

```r
library(exams)
set.seed(20260601L)            # SAME seed as generate_moodle_quiz.R

exercises <- c("applied_01.Rmd", "applied_02.Rmd", "sim_03.Rmd")

exams2pdf(
  file     = exercises,
  n        = 1,
  name     = "MODULE_Session_Practical_Solutions",
  dir      = getwd(),
  edir     = getwd(),
  template = "plain_href.tex"
)
```

- **`template = "plain_href.tex"`** — a minimal `article` template (in `templates/`) whose `\href` rendering shows the bare filename, so embedded R-script/CSV links read cleanly in print.
- **Same seed and `n = 1`** as the Moodle generator, so the PDF shows exactly the version students received.
- The output is `..._Solutions1.pdf`. It is part of the secure-submission memo bundle for moderated exams.

## Putting a folder together (checklist)

```
PracticalXX/
├── question_*.Rmd            # schoice / mchoice / num concept items
├── applied_*.Rmd  + .R       # cloze answer key + paired student script
│                              #   (.R may be static, or regenerated per version —
│                              #    see "Per-version regenerated data" above)
├── sim_*.Rmd      + .R       # cloze answer key + paired student script
├── _generate_data.R          # (only if some item ships one FIXED CSV) — run first
├── *.csv                     # fixed data, shipped to students (fixed-CSV items only)
├── generate_moodle_quiz.R    # → the importable XML (also generates per-version CSVs)
└── generate_solutions.R      # (tests/exams) → worked-solutions PDF
```

Order of operations: (if any item uses a fixed CSV) `_generate_data.R` → verify a filled-in student `.R` matches each key → `generate_moodle_quiz.R` → grep the XML checks → for `n > 1`, verify genuine variety per rule 10 → (tests/exams) `generate_solutions.R`.
