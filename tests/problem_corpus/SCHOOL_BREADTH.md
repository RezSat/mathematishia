# School / A-level executable breadth benchmark (W18)

Public, original calculation fixtures, separate from the 170 legacy expected-outcome regressions and W19's held-out suite. No production solver, parser or workflow changes.

Run the actual compiled executable from the Rust test harness:

```sh
cargo test -p mathematishia-cli --test real_world_corpus school_breadth --locked -- --nocapture
```

The corpus has 292 distinct valid CLI invocations, 280 candidate exact-answer cases and 12 known gaps. Four additional invalid-input tests are outside the valid denominator. These are fixture counts, not measured successes.

Each fixture specifies an exact mathematical answer obtained independently of the engine: rational arithmetic by common denominators and exponent laws, substitution/linear rearrangement, factored roots and discriminants over Q, sign charts with open poles, unit-circle values and signed minimal polynomials, polynomial antiderivatives evaluated at endpoints, derivative rules, binomial coefficients, matrix determinant/cofactor/product arithmetic, and complex multiplication or conjugate division. No production dependency or output-derived answer keys. The fixture is deliberately fixed and reviewable. Formula-only probability, statistics, geometry and mechanics entries test calculation after a human has supplied the model; they do **not** demonstrate natural-language modeling or diagram recognition.

## Mapping

The topic tags are a coarse mapping to [Sri Lanka NIE Combined Mathematics Grade 12–13 (2017)](https://nie.lk/pdffiles/tg/eALSyl%20ComMaths.pdf) and [Cambridge 9709 (2026–27)](https://www.cambridgeinternational.org/programmes-and-qualifications/cambridge-international-as-and-a-level-mathematics-9709/). Arithmetic/surds/powers/logs, algebra and equations/inequalities map to school prerequisites and pure mathematics; trigonometry, calculus, binomial and sequences map to pure mathematics; complex and vectors/coordinate geometry map to advanced pure topics; matrices are additional NIE/general CAS scope, not a claim of Cambridge 9709 matrix requirements. Probability/statistics and mechanics correspond to the applied strands. This is a sample of competencies, not exam papers, syllabus completion or Symbolab parity.

## Measurement

Every row has a category, mathematical family, input kind (typed calculation or typed formula), mode, CLI arguments and independent expectation. Distinct CLI invocations are checked against the legacy corpus. Exact answer comparison includes all lines of each expected answer field, including root/interval sets and listed restrictions; no substring success. The independently checked `steps` endpoint must emit actual nonempty rendered records and the independently keyed conclusion before teaching-step coverage is credited. Final-only answers are separate and never credited as teaching steps. The renderer verifies StepRecords against original evidence; this harness is not a new general proof checker.

Run each executable invocation twice to check determinism. Report category/kind/outcome denominators, first-call wall-clock median/p95/max microseconds (including process creation and polling overhead), output bytes and a five-second per-process deadline. Output is intentionally small; a child blocked on a full pipe is killed at the deadline. No claim of CPU or peak RSS measurement. Wrong or unexpectedly refused supported cases fail the new test; known valid gaps remain explicit and are not scored as solved. Gap answer keys are mathematical descriptions, not CLI text expectations: if a gap changes to exact, review and add an exact comparator before crediting it. Legacy cases are unmodified. Invalid inputs are separate.

## Backlog

- General radical simplification and log/exponential equations; branch and domain conditions.
- General periodic trig equations/identities, higher-degree rational inequalities, general polynomial and piecewise equations.
- Infinite/analytic limits, substitution, partial fractions, integration by parts and broader elementary primitives.
- Native sequence/summation and coefficient requests; vector products, lines/planes, distances and loci; complex polar form and roots.
- Native distributions, counting, descriptive statistics, inference, kinematics, forces, equilibrium and energy.
- Separate natural-language modeling, diagrams and proof tasks, with independent model/diagram/proof rubrics. No credit from arithmetic formula rows.
- Grow to 1,000+ original cases by adding mathematical families, not just changing constants. Add process-level memory/CPU collection and a machine-readable report. Coordinate new wire contracts with W17 and coordinator; do not leak W19 held-out fixtures.

Base: `41d78278c50ca9f42c6050592613473bee1cfb6c`. Actual measurements require running the compiled test; no measured accuracy, latency or test counts are asserted here.
