# Cloze questions and the paired student-script workflow

Cloze is the workhorse for applied-statistics practicals and exams. One scenario contains several graded **blanks**, each independently typed and scored. A typical practical/exam item bundles:

- a **scenario** (a regression or time-series situation),
- a downloadable **student `.R` script** with `your_code_goes_here()` blanks,
- 5–14 **blanks** mixing computed numbers (`num`) with conceptual MCQs (`schoice`),
- an answer key that **re-derives every number in R** so nothing is transcribed.

This file is the detailed guide. Start from `templates/cloze_mixed.Rmd` + `templates/cloze_mixed_paired.R`.

## Cloze meta-information

```
Meta-information
================
extype: cloze
exclozetype: num|num|num|schoice|schoice
exsolution: `r paste(c(ans_a, ans_b, ans_c, sol_d, sol_e), collapse = "|")`
extol: 0.005|0.005|0|0|0
exname: Applied — descriptive name
exshuffle: TRUE
```

The four `|`-separated fields must line up **position by position**:

| position | `exclozetype` | `exsolution` element | `extol` element |
|----------|---------------|----------------------|-----------------|
| 1 | `num` | a rounded number | numeric tolerance |
| 2 | `num` | a rounded number | numeric tolerance |
| 3 | `num` | a rounded number | `0` (or tol) |
| 4 | `schoice` | `mchoice2string(sol_d, single=TRUE)` | `0` |
| 5 | `schoice` | `mchoice2string(sol_e, single=TRUE)` | `0` |

- `exclozetype` declares each blank's type, in order.
- `exsolution` is the `|`-join of: numbers for `num` blanks, and the positional binary string for each `schoice`/`mchoice` blank.
- `extol` needs an entry for **every** blank (schoice blanks take `0`). A mismatch in counts is the most common cloze authoring bug — see the count check at the bottom.

## Blank markers in the question body: `##ANSWERn##`

Inside the `Question` text, the blanks are placed with literal markers `##ANSWER1##`, `##ANSWER2##`, … in the order they appear. The number ties the marker to the position in `exclozetype`/`exsolution`/`extol`.

```
**(a)** Report the sample mean of the series.

##ANSWER1##

**(b)** Report the sample variance.

##ANSWER2##

...

**(d)** Which statement correctly describes stationarity?

##ANSWER4##
```

The **option list for the schoice blanks** is supplied once, near the end of the `Question` section, by concatenating every option vector and rendering with `answerlist()`:

```r
all_opts <- c(opts_d, opts_e)        # each opts_* is a length-4 character vector
```

```{r questionlist, echo=FALSE, results="asis"}
answerlist(all_opts, markup = "markdown")
```

R/exams maps the options to the schoice markers in order: the first schoice blank consumes the first block of options, the second consumes the next, etc. So the *order* of `opts_*` in `all_opts` must match the order of the `schoice` markers in the prose.

## Re-deriving the answer key (the hidden chunk)

Put all computation in one hidden chunk at the top, conventionally named `data-generation`:

```r
```{r data-generation, echo=FALSE, results="hide", message=FALSE, warning=FALSE}
library(exams)
include_supplement("sim_xxx.R")          # makes the student script downloadable
# include_supplement("data.csv")         # and any data files

set.seed(75201)                          # SAME seed as the student script
# ... reproduce the exact computation a correct student would run ...

ans_mean <- round(mean(y), 4)
ans_var  <- round(var(y),  4)

# schoice blanks: option vectors + logical solution vectors
opts_d <- c("Correct statement ...", "Distractor 1 ...", "Distractor 2 ...", "Distractor 3 ...")
sol_d  <- c(TRUE, FALSE, FALSE, FALSE)

all_opts    <- c(opts_d, opts_e)
sol_strings <- c(mchoice2string(sol_d, single = TRUE),
                 mchoice2string(sol_e, single = TRUE))
```
```

Then `exsolution` is a pure `paste()` of computed objects — it can never disagree with the worked solution because both come from the same variables:

```
exsolution: `r paste(c(ans_mean, ans_var, ans_acf, sol_strings), collapse = "|")`
```

### `include_supplement()`

`include_supplement("sim_xxx.R")` (and one call per data CSV) attaches the file to the generated question so Moodle serves it as a download. The matching link in the prose is plain HTML/markdown:

```
<hr>
<mark>
**NOTE:** Right-click and "Save link as" to download
[**sim_xxx.R**](sim_xxx.R) and [**data.csv**](data.csv) into the **same**
working directory. Complete every `your_code_goes_here()` section, then run
the script with **Source**.
</mark>
<hr>
```

The supplement filename, the link text, and the `include_supplement()` argument must all match.

## Building `exclozetype`/`extol` dynamically (varied questions)

For long or programmatically-built questions, construct the type vector in R and interpolate it, instead of hand-writing a 14-element `num|num|...` string. This keeps the meta block in sync automatically when you add/remove blanks:

```r
types <- c("num","num","num","schoice","schoice")
tols  <- ifelse(types == "num", "0.005", "0")   # per-blank tolerance
```

```
exclozetype: `r paste(types, collapse = "|")`
extol: `r paste(tols, collapse = "|")`
```

This is the robust generalisation of the fixed-string form and is preferred for anything with many blanks.

## `expoints` — mark-weighting (tests and exams)

Practicals are usually unweighted (every blank worth 1). Tests/exams assign marks per blank with `expoints`, a `|`-separated vector aligned to the blanks:

```r
marks <- c(0.5, 0.5, 1, 2, 2)        # per-blank marks; can be fractional then scaled
marks <- as.integer(2 * marks)        # scale so the question totals an integer mark count
```

```
expoints: `r paste(marks, collapse = "|")`
```

For a single-blank question (`schoice`/`num`) `expoints` is a single number, e.g. `expoints: 2`. House rule: no non-MCQ item worth only 1 mark (see conventions reference).

## The paired student `.R` ↔ answer-key `.Rmd` contract

The student downloads `<stem>.R`; the grader renders `<stem>.Rmd`. They must stay in lockstep. The `.R` is what a student runs; the `.Rmd`'s hidden chunk must reproduce *exactly* the same computation so the key matches a correct student's output.

This `<stem>.R` can be a **static file on disk** (written once, unchanging across versions — shown below), or **written fresh per version** by the `.Rmd`'s own `data-generation` chunk via `writeLines()` + `include_supplement()` (the default for practice quizzes, `n > 1`) — see "Per-version regenerated data" in `reference/generators-and-data.md` for that mechanism; the placeholder/comment contract below is identical either way.

### The `your_code_goes_here()` placeholder

Define this helper **once at the top of every fill-in `.R` template**:

```r
your_code_goes_here <- function(...) {
  stop(
    "Placeholder not replaced. Replace each call to `your_code_goes_here()` ",
    "with your own code."
  )
}
```

Then each blank is a call with an English comment:

```r
ups_a  <- your_code_goes_here()    # draw (n + burnin) innovations
y_a[t] <- your_code_goes_here()    # AR(1) recursion
mean_a <- your_code_goes_here()    # sample mean
```

Why this and not the alternatives:

- A bare `x <- ???` parses as chained `?` calls and throws a misleading "missing bracket"-looking error far from the real line — students chase a phantom bug.
- `x <- stop("Fill in your code here ...")` parses cleanly but ESL students read the string literally and type their answer *inside* the quotes, then get their own code echoed back as an error. Structurally ambiguous.
- `your_code_goes_here()` makes the empty `()` visibly the thing to replace, the name is the instruction, the comment carries the step in plain English, and a missed blank raises a clear localised error.

### Things that must match between `.R` and `.Rmd`

- **Same `set.seed(...)`** and same fixed constants (`n`, `burnin`, coefficients, …).
- **Same variable names.** If the prose or the `.R` names a value `R2_h`, the comment, the `.R` slot, and any verification line in the `.Rmd` must all use `R2_h` — no synonyms.
- **Names on vectors.** If the answer key or question verifies `round(v["n=1000"], 6)`, the `.R` must assign `names(v) <- paste0("n=", sample_sizes)`. Without names the lookup returns `NA`.
- **`pmax`/`pmin` for element-wise** comparisons across a sample; `max`/`min` collapse to a scalar and silently break downstream numbers. If a comment describes an element-wise operation, it must say `pmax`/`pmin`.
- **Scaffolding level.** Never write the exact replacement expression in the blank's comment, in practicals or tests/exams alike — that is the answer, verbatim, and defeats the exercise. A comment may name relevant function(s) ("Consider `tolower()` and `trimws()`.") or point at the help page ("Consult `?scale`."), or describe the goal in plain English without naming the operator/argument being tested (e.g. don't write `frequency = 12` in a script when a blank asks for the frequency). If you can copy-paste the comment's content into the blank and get a working, correct answer, rewrite it.

## The `~` trap (repeat, because it bites cloze hardest)

Moodle cloze uses `~` to separate options; the exams package does **not** escape `~` inside option strings. Any `~` in an `opts_*` element — most often an R formula like `lm(y ~ x, weights = w)` — splits that option into two, creating a phantom extra option. Reword to avoid the tilde: "what `lm()` does internally when `weights` is supplied" instead of writing the formula.

## Verification before shipping a cloze

1. **Count parity.** Number of blanks must equal the lengths of `exclozetype`, `exsolution`, and `extol` (and `expoints` if present). After rendering the XML, for each cloze block the number of `~` should equal `(number_of_options − 1)` per schoice sub-question.
2. **Display format.** `grep -oE '\{1:[A-Z_]+:' *.xml | sort -u` → only `{1:MULTICHOICE_V:`.
3. **Run a filled-in student `.R`.** Replace every `your_code_goes_here()` with the intended code, run it, and confirm its printed numbers match the `exsolution` values within `extol`.
4. **No `~` in any `opts_*`.** `grep -nE 'opts_[a-z]' *.Rmd` then scan those vectors.
