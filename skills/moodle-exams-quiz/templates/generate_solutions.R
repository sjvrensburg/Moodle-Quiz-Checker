# ============================================================================
# Generate Practical Solutions PDF — <SESSION>
# ============================================================================
#
# Produces a PDF with the question text + worked solutions for the SAME
# question set and seed that went to Moodle. Used for the memo / moderation
# bundle. Uses the local plain_href.tex template so embedded script/CSV
# download links render as plain filenames.
#
# Usage:
#   setwd("path/to/this/folder")
#   source("generate_solutions.R")
# ============================================================================

library(exams)

set.seed(2026)        # MUST match generate_moodle_quiz.R

exercises <- c(
  "question_one.Rmd",
  "question_two.Rmd",
  "cloze_mixed.Rmd"
)

exams2pdf(
  file     = exercises,
  n        = 1,        # the single fixed version students received
  name     = "MODULE_Practical_Solutions",
  dir      = getwd(),
  edir     = getwd(),
  template = "plain_href.tex"
)

cat("\nDone: MODULE_Practical_Solutions1.pdf\n")
