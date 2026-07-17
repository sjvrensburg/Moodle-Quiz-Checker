# Question types: schoice, mchoice, num, simple cloze

Every R/exams question is one `.Rmd` with the four underline-delimited sections (`Question`, `Solution`, `Meta-information`, and an implicit answer list). This file covers the three single-blank types and the structure shared by all of them. Multi-blank cloze (the main practical/exam workhorse) has its own file: `cloze-and-paired-scripts.md`.

## Anatomy shared by all types

```
Question
========

```{r, echo=FALSE, results="hide"}
# (optional) randomise values, build the answer vector and the
# logical solution vector here. Keep all randomisation in ONE chunk.
```

Question text. Inline math $x^2$, display math $$\mathbf{X}'\mathbf{X}$$.

```{r questionlist, echo=FALSE, results="asis"}
answerlist(answers, markup = "markdown")
```

Solution
========

Worked explanation. Reference the *correct* answer and why each
distractor is wrong.

```{r solutionlist, echo=FALSE, results="asis"}
answerlist(explanations, markup = "markdown")
```

Meta-information
================
extype: schoice
exsolution: `r mchoice2string(sol, single = TRUE)`
exname: Short descriptive name
exshuffle: TRUE
```

Key conventions:

- **Underline headers** (`====`), not `#`. The exams package keys off these exactly.
- **`echo=FALSE, results="hide"`** for the computation chunk; **`results="asis"`** for the `answerlist()` chunks.
- **`answerlist(x, markup = "markdown")`** renders a character vector as the list of options (in `Question`) and the matching per-option explanations (in `Solution`). The two lists must be the same length and in the same order.
- **`exname`** — a short human label shown in the Moodle question bank. Make it descriptive and unique within the folder.
- **`exshuffle: TRUE`** — randomise option order at *generation* time. (Independent of `schoice = list(shuffle = TRUE)` in the generator, which shuffles again at *display* time. Both are standard.)
- **`exsolution` is positional and refers to source order**, before any shuffle. Always build it with `mchoice2string()`, never hand-typed, so it can't drift from the `sol` vector.

## schoice — single-choice MCQ

Exactly one correct option. `exsolution` is a binary string with a single `1`.

```
Meta-information
================
extype: schoice
exsolution: `r mchoice2string(sol, single = TRUE)`   # e.g. 1000 = first option correct
exname: Dimensions of a matrix product
exshuffle: TRUE
```

In the chunk, `sol` is a logical vector with one `TRUE`:

```r
answers <- c(correct, distractor1, distractor2, distractor3)
sol     <- c(TRUE, FALSE, FALSE, FALSE)
explanations <- c(
  "Correct. ...why...",
  "Incorrect. ...the specific misconception this distractor targets...",
  "Incorrect. ...",
  "Incorrect. ..."
)
```

`single = TRUE` enforces exactly one correct answer. Distractors should each encode a *specific, common* student mistake, and the matching explanation should name that mistake. See `templates/schoice.Rmd`.

## mchoice — multiple-response ("select all that apply")

One or more correct. `exsolution` is a binary string that may have several `1`s. Use `mchoice2string(sol)` (no `single = TRUE`).

```
extype: mchoice
exsolution: `r mchoice2string(sol)`   # e.g. 10110
```

Tell the student explicitly that more than one may be correct ("select **all** correct statements... more than one statement is correct"). A robust pattern is to keep a pool of true statements and a pool of false statements, then sample a random number from each so the count of correct answers varies between versions:

```r
n_true  <- sample(2:3, 1)
n_false <- 5L - n_true
items   <- c(sample(true_pool, n_true), sample(false_pool, n_false))
items   <- items[sample(length(items))]          # shuffle
answers <- sapply(items, `[[`, "text")
sol     <- sapply(items, `[[`, "correct")
```

Moodle scores mchoice with partial credit and penalties for wrong ticks — keep the option count modest (≈5). See `templates/mchoice.Rmd`.

## num — single numeric answer

The answer is a number; Moodle grades it within a tolerance.

```
extype: num
exsolution: `r round(answer, 4)`
extol: 0.01
exname: Sample variance
exshuffle: TRUE
```

- **`exsolution`** is the numeric value. Compute and `round()` it in the chunk; never transcribe a number by hand.
- **`extol`** is the absolute tolerance. Match it to the rounding you ask of students: if you ask for 4 dp, `extol` of `0.0005`–`0.005` is typical; for a count/integer answer use `extol: 0`. For large-magnitude answers, widen it (a variance of ~10 might use `extol: 0.5`).
- State the rounding in the question ("Round to **four decimal places**") so the tolerance and the student's effort line up.

There is no `answerlist()` for a pure `num` question — the `Solution` section just shows the worked value. See `templates/num.Rmd`.

## Randomising values in the question chunk

The whole point of R/exams is per-student randomisation. Draw the random pieces at the top of the single hidden chunk, then express *every* number in the prose and the solution via inline `` `r ... ` `` so they stay consistent:

```r
dims <- sample(2:6, 3)
m <- dims[1]; n <- dims[2]; p <- dims[3]
```

```
Given $\mathbf{A}$ is $`r m` \times `r n`$ and $\mathbf{B}$ is $`r n` \times `r p`$, ...
```

Rules:

- **One source of truth.** Never write a literal number that you also computed in R — interpolate it. A hard-coded `5` that should track a random draw is the classic version-drift bug.
- **Reproducible draws.** The generator sets a global `set.seed(...)`; questions that must be byte-stable across renders (exams, or any question whose values appear in a shipped CSV) should *also* set their own `set.seed(...)` inside the chunk.
- **Guard against degenerate draws.** If some random configurations break the question (e.g. a singular matrix when you need an invertible one, or a hypothesis test that should reject but sometimes doesn't), wrap the draw in a `while (bad) { ... }` rejection loop that re-samples until the configuration satisfies your constraints.
- **Distractors from mistakes.** Generate distractors *from* the random values by applying the wrong operation (transpose instead of inverse, `n-1` instead of `n`, etc.) so they're plausible and track the randomisation.

## Meta-information field reference

| Field | Applies to | Meaning |
|-------|-----------|---------|
| `extype` | all | `schoice` / `mchoice` / `num` / `cloze` / `string` |
| `exsolution` | all | the answer (binary string for choice; number for num; `\|`-joined for cloze) |
| `extol` | num, cloze-num blanks | numeric tolerance (absolute) |
| `exname` | all | short label in the question bank |
| `exshuffle` | choice types | `TRUE` to shuffle options at generation |
| `exclozetype` | cloze | `\|`-separated per-blank types (see cloze reference) |
| `expoints` | all | mark weight (per blank for cloze, `\|`-separated); used in tests/exams |
| `exextra[numwidth,numeric]` | num/cloze | controls numeric field width if needed |

For `expoints`, cloze marking, and everything multi-blank, continue to `cloze-and-paired-scripts.md`.
