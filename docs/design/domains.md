# Mathematical domains

Generic expressions are not universal algorithm representations. Specialized
domains will own the data structures and algorithms suited to their
mathematics. Candidate domains include ZZ, QQ, polynomial rings, matrix
domains, formal series, and numerical domains.

Conversions between semantic expressions and specialized domains are explicit
and fallible. Promotion and coercion should follow concrete requirements; this
project is not introducing a general category or coercion system now.

The initial implementation direction is pure Rust. Mature native libraries
such as FLINT may be evaluated behind suitable boundaries if measurements
justify them. No backend trait is warranted solely because multiple backends
might exist later.
