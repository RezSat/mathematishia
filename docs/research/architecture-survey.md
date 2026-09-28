# Architecture survey

This record captures design lessons, not copied implementation.

- [SymPy](https://docs.sympy.org/latest/): immutable structural expressions,
  controlled constructor normalization, and specialized polynomial/domain
  representations.
- [Wolfram Language](https://reference.wolfram.com/language/): a uniform
  expression/evaluator model is powerful; Mathematishia does not make universal
  repeated evaluation its central kernel model.
- [Mathics3](https://mathics.org/): tightly coupled function application,
  rewriting, and evaluator infrastructure carries practical complexity.
- [Maple](https://www.maplesoft.com/): structural sharing and hash-consing can
  help, but are invasive and await benchmark evidence.
- [SageMath](https://doc.sagemath.org/), [FriCAS](https://fricas.github.io/),
  and [Axiom](https://axiom-developer.org/): domains, parents, and coercions
  matter; Mathematishia begins with a smaller explicit-domain model.
- [GiNaC](https://www.ginac.de/): opaque expression handles are useful; storage
  should not be public semantics.
- [Maxima](https://maxima.sourceforge.io/) and [REDUCE](https://reduce-algebra.sourceforge.io/):
  valuable historical algorithm references, not implementation templates.
- [Cadabra](https://cadabra.science/): mathematical properties and context can
  remain separate from expression structure.
- [Julia Symbolics](https://symbolics.juliasymbolics.org/) and
  [SymbolicUtils](https://github.com/JuliaSymbolics/SymbolicUtils.jl): rewriting
  can be separated from expression storage and user-facing wrappers.
- [egg](https://egraphs-good.github.io/): equality saturation may help selected
  equality-search problems later, but is not the universal evaluator or
  derivation representation.
- [FLINT](https://flintlib.org/) and [Arb](https://arblib.org/): specialized
  representations and algorithms matter for performance; certified ball
  arithmetic differs from ordinary approximation.
- [mpmath](https://mpmath.org/): arbitrary precision does not by itself imply
  rigorous error certification.

No code from these projects is claimed as copied or incorporated.
