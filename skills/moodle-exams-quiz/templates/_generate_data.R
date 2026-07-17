# ============================================================================
# Generate datasets for <FOLDER> — Practical/Exam data
# ============================================================================
#
# FIXED-SEED pattern: one dataset, shared by every version. Appropriate for
# n = 1 tests/exams, or a practice item deliberately about one unchanging
# dataset. For a practice quiz (n > 1) — the common case — prefer regenerating
# the data per version instead (see `cloze_regenerated.Rmd` and "Per-version
# regenerated data" in reference/generators-and-data.md); this script and its
# fixed-CSV workflow are not needed there.
#
# Produces EVERY CSV referenced by the question .Rmd files in this folder,
# under the documented seed below. Re-running overwrites the CSVs byte-for-
# byte. The shipped numbers appear in the question text and answer keys, so
# RE-RUN ONLY WITH INTENT — a changed seed invalidates a published quiz.
#
# Usage:
#   setwd("path/to/this/folder")
#   source("_generate_data.R")
# ============================================================================

set.seed(20260601L)        # documented seed

# ── Example: a heteroskedastic cross-section (BP test should reject) ────────
# Design the data so it EXHIBITS the phenomenon the question is about. If a
# specific outcome is needed (e.g. a BP p-value in [0.15, 0.30]), run a small
# seed-sweep to find a seed that lands in range, then hard-code it here.
n       <- 120
hh      <- runif(n, min = 5, max = 30)        # a regressor
temp    <- runif(n, min = 12, max = 28)       # another regressor
sigma_i <- 2 + 0.4 * hh                        # variance grows with hh
eps     <- rnorm(n, mean = 0, sd = sigma_i)
y       <- 8 + 1.6 * hh - 0.5 * temp + eps

df <- data.frame(
  id     = sprintf("S%03d", 1:n),
  hh     = round(hh,   2),
  temp   = round(temp, 2),
  y      = round(y,    2)                       # round on write: ship == re-derive
)
write.csv(df, file = "example_data.csv", row.names = FALSE)

cat("\nGenerated data files:\n  - example_data.csv (n = ", n, ")\n", sep = "")
