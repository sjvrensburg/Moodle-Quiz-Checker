# Quiz review report template

Every quiz review produced with this skill ends in a report with exactly this
structure, so reviews are comparable across quizzes, sessions, and agents.
Omit a section only when its gate was genuinely not applicable (say so);
never reorder or rename sections.

Severity vocabulary (use no other words):

- **blocker** — will misgrade students or break on import; must fix before release
- **major** — pedagogically compromising (leaks answers, guessable, ambiguous); fix strongly recommended
- **minor** — quality/polish; fix when convenient
- **note** — observation or deliberate-choice confirmation needed from the author

---

```markdown
# Quiz review: <quiz name>

**Source:** <file / generator, e.g. "exams2moodle output of practical3.Rmd">
**Reviewed:** <date> · **Reviewer:** <agent/human> · mqt-cli <version>
**Verdict:** RELEASE / FIX-THEN-RELEASE / REWORK
**Summary:** <2–4 sentences: overall state, count of findings by severity,
the single most important fix.>

## 1. Mechanical gates

| Gate | Result | Detail |
|---|---|---|
| lint | pass / N errors, M warnings | <one line> |
| autotest | pass / N of M failed | <one line> |
| compare (K versions) | pass / N items flagged / n/a | <one line> |

Random-guess baseline: quiz expected score ≈ X% (flag questions ≥50% below).

## 2. Findings

One block per finding, ordered by severity then question order:

### [severity] Qn "<question name>" — <one-line title>
- **Evidence:** <quote the offending text / key / weighting, or the gate output>
- **Impact:** <what happens to a real student or their grade>
- **Fix:** <concrete change, stated against the SOURCE (.Rmd/generator), not the XML>

## 3. Adversarial passes

- Honest-student score: X% — shortfalls listed as findings above.
- Test-wise score: X% vs chance baseline Y% — heuristics that worked, per question.

## 4. Pedagogy rubric outcomes

One line per checklist area actually reviewed (distractor quality, feedback
quality, answer leakage, independence, notation, ESL wording, randomisation
variety): pass / findings-above / not-checked-because-<reason>.

## 5. Deliberate-choice confirmations needed

Constant compare columns, unusual weightings, or design oddities that may be
intentional — each phrased as a yes/no question the author can answer.
```

---

Keep the report self-contained: quote what you refer to; the reader should
not need the XML open to understand any finding. Never include full answer
keys in a report that might circulate to students — reference question names,
not solutions, except inside Evidence where a specific value is the point.
