# Mathema surface language — initial grammar

The first frontend is intentionally smaller than the eventual Mathema language.
It lives in `mathema-frontend`, not in the semantic `mathema` crate.

- One expression or one equation per input; an equation has one top-level `=`.
- ASCII nonnegative integer literals (arbitrary precision) and rational numeric
  literals `n/d`, optionally with whitespace. Denominator zero is an error.
- Exact Unicode identifier names beginning with a letter or underscore, then
  letters, digits, or underscores. No implicit multiplication.
- Explicit `+`, binary `-`, `*`, `^`, unary `+`/`-`, and parentheses.
- Power is right-associative. Unary sign binds more loosely than power; hence
  `-x^2` means `-(x^2)`, whereas `(-x)^2` is distinct.
- Slash is only permitted between integer literal tokens; it is **not** a
  general division operator. Floating point, functions, assignment, quoted
  symbols, multi-statement scripts, and comments are not implemented.
- A bounded parser rejects oversized, overly nested, or overly tokenized input.
  Errors carry UTF-8 byte offsets. The frontend first constructs a private
  syntax tree and then lowers it using the existing canonical constructors.

Examples: `(x+1)^2`, `1/3*x+2`, `2*x+4=10`.
Do not treat semantic `Expr::Display` as a guaranteed source round-trip format;
separate stable serialization and rendering remain future work.
