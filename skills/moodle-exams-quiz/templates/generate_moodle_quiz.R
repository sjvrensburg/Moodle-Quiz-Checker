# ============================================================================
# Generate Moodle Quiz — <TOPIC NAME>
# ============================================================================
#
# Bundles the question .Rmd files in this folder into one Moodle XML file.
# Import via: Course Administration > Question Bank > Import > Moodle XML.
#
# Usage:
#   setwd("path/to/this/folder")
#   source("generate_moodle_quiz.R")
#
# PRE-REQUISITE: if any question reads a CSV, run _generate_data.R first.
# ============================================================================

library(exams)

set.seed(2026)        # reproducible randomisation across the whole batch

exercises <- c(
  # --- concept MCQs (schoice / mchoice) ---
  "question_one.Rmd",
  "question_two.Rmd",
  # --- applied / simulation cloze items (each paired with a .R script) ---
  "cloze_mixed.Rmd"
)

exams2moodle(
  file      = exercises,
  n         = 5,        # PRACTICE: 5 versions per question (a pool).
                        # EXAM/TEST: set n = 1 (a single fixed version).
  name      = "MODULE Topic Name",                          # Moodle category
  dir       = getwd(),
  schoice   = list(shuffle = TRUE),                          # shuffle at display
  cloze     = list(cloze_schoice_display = "MULTICHOICE_V"), # REQUIRED for cloze
  converter = "pandoc-mathjax",                              # LaTeX -> MathJax
  quiet     = FALSE
)

cat("\nDone! Import the generated .xml into Moodle.\n")

# ── Post-generation checks (run in a shell) ────────────────────────────────
#   grep -oE '\{1:[A-Z_]+:' *.xml | sort -u      # expect only {1:MULTICHOICE_V:
#   # then open the preview HTML / import into a sandbox course to eyeball.
