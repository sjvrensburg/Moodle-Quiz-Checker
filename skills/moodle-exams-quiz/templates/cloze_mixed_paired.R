# ============================================================================
# MODULE — Practical — Sampling distribution of the mean
# ============================================================================
#
# Scenario: draw one sample of n = 60 from a Normal(mu = 100, sigma = 15)
# population, then compute the sample mean, the sample variance, and the
# standard error of the mean. Round numerical answers to four decimal places.
#
# Complete every section marked `your_code_goes_here()`, then run the whole
# script with Source. The printed values are what you enter into Moodle.
# ============================================================================

your_code_goes_here <- function(...) {
  stop(
    "Placeholder not replaced. Replace each call to `your_code_goes_here()` ",
    "with your own code."
  )
}

# ── Setup (do not change) ──────────────────────────────────────────────────
# The seed and constants below MUST match the answer key. Do not edit them.
set.seed(4242)
n     <- 60       # sample size
mu    <- 100      # population mean
sigma <- 15       # population standard deviation

# ── Step 1: draw the sample ────────────────────────────────────────────────
# Draw n observations from a Normal(mu, sigma) population.
x <- your_code_goes_here()                 # rnorm(...) with the constants above

# ── Step 2: sample mean ────────────────────────────────────────────────────
sample_mean <- your_code_goes_here()       # the sample mean of x

# ── Step 3: sample variance (n - 1 denominator) ────────────────────────────
sample_var <- your_code_goes_here()        # the sample variance of x

# ── Step 4: standard error of the mean ─────────────────────────────────────
# Consider sqrt(sample_var) and the sample size n.
se_mean <- your_code_goes_here()

# ── Final reporting ────────────────────────────────────────────────────────
cat("Sample mean              :", round(sample_mean, 4), "\n")
cat("Sample variance (n - 1)  :", round(sample_var,  4), "\n")
cat("Standard error of mean   :", round(se_mean,     4), "\n")
