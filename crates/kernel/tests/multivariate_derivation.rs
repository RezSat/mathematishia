use mathema::{Integer, Rational, Symbol};
use mathematishia_kernel::{
    DivisionExplanationKind, DivisionMonomialOrder, MultivariateDerivationError,
    MultivariateDivisionError, MultivariateError, MultivariateLimits, MultivariatePolynomial,
    Relation, divide_multivariate, render_multivariate_division, verify_multivariate_derivation,
};

fn sym(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn rat(value: i64) -> Rational {
    Rational::from(Integer::from(value))
}

fn frac(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn poly(terms: &[(i32, i32, i64)]) -> MultivariatePolynomial {
    MultivariatePolynomial::try_new(
        vec![sym("x"), sym("y")],
        terms.iter().map(|&(x, y, c)| (vec![x, y], rat(c))),
        MultivariateLimits::default(),
    )
    .unwrap()
}

#[test]
fn preserves_exact_quotients_recomposition_and_reduced_remainder() {
    let limits = MultivariateLimits::default();
    let dividend = poly(&[(2, 0, 1), (0, 1, 1)]);
    let divisor = poly(&[(1, 0, 1), (0, 0, 1)]);
    let evidence = divide_multivariate(&dividend, std::slice::from_ref(&divisor), limits).unwrap();
    let rendered =
        render_multivariate_division(&dividend, std::slice::from_ref(&divisor), &evidence, limits)
            .unwrap();
    assert_eq!(rendered.ordered_divisors, vec![divisor.clone()]);
    assert_eq!(rendered.quotients, evidence.quotients());
    assert_eq!(rendered.remainder, *evidence.remainder());
    assert_eq!(
        rendered.monomial_order,
        DivisionMonomialOrder::Lexicographic
    );
    assert_eq!(rendered.recomposition.relation, Relation::Equal);
    assert_eq!(rendered.recomposition.ordered_products[0].divisor_index, 0);
    assert_eq!(
        rendered.recomposition.ordered_products[0].quotient,
        rendered.quotients[0]
    );
    assert_eq!(rendered.recomposition.ordered_products[0].divisor, divisor);
    assert_eq!(
        rendered.reduced_remainder.leading_exponents,
        vec![vec![1, 0]]
    );
    assert_eq!(rendered.reported_iterations, evidence.steps());
    assert_eq!(rendered.lines.len(), 7);
    assert_eq!(rendered.lines[0].kind, DivisionExplanationKind::Dividend);
    assert_eq!(
        rendered.lines[1].kind,
        DivisionExplanationKind::AmbientRingAndOrder
    );
    assert!(rendered.lines[1].display.ends_with("divisor priority: g[0]"));
    assert!(!rendered.lines[1].display.contains("g[1]"));
    assert_eq!(
        rendered.lines[2].kind,
        DivisionExplanationKind::Divisor { index: 0 }
    );
    assert_eq!(
        rendered.lines[3].kind,
        DivisionExplanationKind::Quotient { index: 0 }
    );
    assert_eq!(rendered.lines[4].kind, DivisionExplanationKind::Remainder);
    assert_eq!(
        rendered.lines[5].kind,
        DivisionExplanationKind::Recomposition
    );
    assert_eq!(
        rendered.lines[6].kind,
        DivisionExplanationKind::ReducedRemainder
    );
    assert!(rendered.lines[5].display.contains(" = "));
    assert!(rendered.lines[5].display.contains(" * "));
    assert!(
        rendered.lines[6]
            .explanation
            .contains("no unique normal form")
    );

    // Independently recompose using exact polynomial arithmetic.
    let mut sum = rendered.recomposition.remainder.clone();
    for product in &rendered.recomposition.ordered_products {
        sum = sum
            .add(
                &product.quotient.multiply(&product.divisor, limits).unwrap(),
                limits,
            )
            .unwrap();
    }
    assert_eq!(sum, dividend);
    for powers in rendered.reduced_remainder.remainder.terms().keys() {
        assert!(
            rendered
                .reduced_remainder
                .leading_exponents
                .iter()
                .all(|leading| !leading.iter().zip(powers).all(|(a, b)| a <= b))
        );
    }
    assert_eq!(
        verify_multivariate_derivation(
            &dividend,
            std::slice::from_ref(&divisor),
            &evidence,
            &rendered,
            limits,
        ),
        Ok(())
    );
    assert_eq!(
        render_multivariate_division(&dividend, std::slice::from_ref(&divisor), &evidence, limits,)
            .unwrap(),
        rendered
    );
}

#[test]
fn divisor_priority_changes_exact_quotients_without_claiming_unique_form() {
    let limits = MultivariateLimits::default();
    let f = poly(&[(1, 1, 1)]);
    let x = poly(&[(1, 0, 1)]);
    let y = poly(&[(0, 1, 1)]);
    let xy = [x.clone(), y.clone()];
    let yx = [y.clone(), x.clone()];
    let first = divide_multivariate(&f, &xy, limits).unwrap();
    let second = divide_multivariate(&f, &yx, limits).unwrap();
    let first_out = render_multivariate_division(&f, &xy, &first, limits).unwrap();
    let second_out = render_multivariate_division(&f, &yx, &second, limits).unwrap();
    assert_eq!(first_out.quotients, vec![y.clone(), poly(&[])]);
    assert_eq!(second_out.quotients, vec![x.clone(), poly(&[])]);
    assert_ne!(first_out, second_out);
    assert_eq!(first_out.recomposition.ordered_products[0].divisor, x);
    assert_eq!(second_out.recomposition.ordered_products[0].divisor, y);
    assert!(
        first_out.lines[1].display.ends_with("divisor priority: g[0] then g[1]")
    );
    assert_eq!(first_out.remainder, poly(&[]));
    assert_eq!(second_out.remainder, poly(&[]));
    assert_eq!(
        verify_multivariate_derivation(&f, &xy, &first, &first_out, limits),
        Ok(())
    );
    assert_eq!(
        verify_multivariate_derivation(&f, &yx, &second, &second_out, limits),
        Ok(())
    );
    assert_eq!(
        render_multivariate_division(&f, &yx, &first, limits),
        Err(MultivariateDerivationError::InvalidDivisionEvidence)
    );
    assert_eq!(
        render_multivariate_division(&f, &xy, &second, limits),
        Err(MultivariateDerivationError::InvalidDivisionEvidence)
    );
}

#[test]
fn zero_remainder_and_zero_dividend_keep_the_ambient_ring() {
    let limits = MultivariateLimits::default();
    let zero = poly(&[]);
    let x = poly(&[(1, 0, 1)]);
    for dividend in [zero.clone(), poly(&[(1, 0, 1)])] {
        let evidence = divide_multivariate(&dividend, std::slice::from_ref(&x), limits).unwrap();
        let output =
            render_multivariate_division(&dividend, std::slice::from_ref(&x), &evidence, limits)
                .unwrap();
        assert_eq!(output.remainder, zero);
        assert!(output.lines.last().unwrap().display.contains("vacuously"));
        assert!(output.lines.last().unwrap().display.contains("LM(g[0])"));
        if dividend.is_zero() {
            assert_eq!(output.quotients, vec![zero.clone()]);
            assert_eq!(output.reported_iterations, 0);
        }
        assert_eq!(
            verify_multivariate_derivation(
                &dividend,
                std::slice::from_ref(&x),
                &evidence,
                &output,
                limits,
            ),
            Ok(())
        );
    }
}

#[test]
fn exact_constant_division_works_in_zero_variable_ring() {
    let limits = MultivariateLimits::default();
    let f = MultivariatePolynomial::constant(vec![], frac(7, 3), limits).unwrap();
    let g = MultivariatePolynomial::constant(vec![], frac(1, 2), limits).unwrap();
    let evidence = divide_multivariate(&f, std::slice::from_ref(&g), limits).unwrap();
    let output =
        render_multivariate_division(&f, std::slice::from_ref(&g), &evidence, limits).unwrap();
    assert_eq!(
        output.quotients,
        vec![MultivariatePolynomial::constant(vec![], frac(14, 3), limits).unwrap()]
    );
    assert_eq!(
        output.remainder,
        MultivariatePolynomial::zero(vec![], limits).unwrap()
    );
    assert!(output.lines[1].display.contains("Q[]"));
    assert_eq!(
        verify_multivariate_derivation(&f, std::slice::from_ref(&g), &evidence, &output, limits,),
        Ok(())
    );
}

#[test]
fn shuffled_ambient_ring_constructor_is_canonical_and_deterministic() {
    let limits = MultivariateLimits::default();
    let dividend = MultivariatePolynomial::try_new(
        vec![sym("y"), sym("x")],
        [(vec![1, 1], frac(3, 2)), (vec![0, 0], frac(-5, 3))],
        limits,
    )
    .unwrap();
    let one = MultivariatePolynomial::constant(vec![sym("x"), sym("y")], rat(1), limits).unwrap();
    let evidence = divide_multivariate(&dividend, std::slice::from_ref(&one), limits).unwrap();
    let output =
        render_multivariate_division(&dividend, std::slice::from_ref(&one), &evidence, limits)
            .unwrap();
    assert_eq!(dividend.variables(), &[sym("x"), sym("y")]);
    assert_eq!(output.recomposition.ordered_products[0].quotient, dividend);
    assert_eq!(output.remainder, poly(&[]));
    assert!(output.lines[1].display.contains("Q[\"x\", \"y\"]"));
    assert_eq!(
        verify_multivariate_derivation(
            &dividend,
            std::slice::from_ref(&one),
            &evidence,
            &output,
            limits,
        ),
        Ok(())
    );
    for _ in 0..12 {
        assert_eq!(
            render_multivariate_division(&dividend, std::slice::from_ref(&one), &evidence, limits,)
                .unwrap(),
            output
        );
    }
}

#[test]
fn fails_closed_on_invalid_divisor_sets_and_mismatched_ring() {
    let limits = MultivariateLimits::default();
    let f = poly(&[(1, 0, 1)]);
    let g = poly(&[(1, 0, 1)]);
    let evidence = divide_multivariate(&f, std::slice::from_ref(&g), limits).unwrap();
    assert_eq!(
        render_multivariate_division(&f, &[], &evidence, limits),
        Err(MultivariateDerivationError::Division(
            MultivariateDivisionError::NoDivisors
        ))
    );
    assert_eq!(
        render_multivariate_division(&f, &[poly(&[])], &evidence, limits),
        Err(MultivariateDerivationError::Division(
            MultivariateDivisionError::ZeroDivisor
        ))
    );
    assert_eq!(
        render_multivariate_division(&f, &vec![g.clone(); 33], &evidence, limits),
        Err(MultivariateDerivationError::Division(
            MultivariateDivisionError::TooManyDivisors
        ))
    );
    let different = MultivariatePolynomial::constant(vec![sym("z")], rat(1), limits).unwrap();
    assert_eq!(
        render_multivariate_division(&f, &[different], &evidence, limits),
        Err(MultivariateDerivationError::Division(
            MultivariateDivisionError::Polynomial(MultivariateError::VariableMismatch)
        ))
    );
    let other = poly(&[(1, 0, 2)]);
    assert_eq!(
        render_multivariate_division(&other, &[g], &evidence, limits),
        Err(MultivariateDerivationError::InvalidDivisionEvidence)
    );
}

#[test]
fn detects_fabricated_mathematical_claims_and_display_lines() {
    let limits = MultivariateLimits::default();
    let dividend = poly(&[(2, 0, 1), (0, 1, 1)]);
    let divisor = poly(&[(1, 0, 1), (0, 0, 1)]);
    let supplied = [divisor];
    let evidence = divide_multivariate(&dividend, &supplied, limits).unwrap();
    let original = render_multivariate_division(&dividend, &supplied, &evidence, limits).unwrap();
    let mut cases = Vec::new();
    let mut forged = original.clone();
    forged.quotients[0] = poly(&[(0, 0, 42)]);
    cases.push(forged);
    let mut forged = original.clone();
    forged.recomposition.relation = Relation::Equivalent;
    cases.push(forged);
    let mut forged = original.clone();
    forged.recomposition.ordered_products[0].divisor_index = 42;
    cases.push(forged);
    let mut forged = original.clone();
    forged.recomposition.ordered_products[0].quotient = poly(&[(0, 0, 42)]);
    cases.push(forged);
    let mut forged = original.clone();
    forged.reduced_remainder.leading_exponents = vec![vec![0, 0]];
    cases.push(forged);
    let mut forged = original.clone();
    forged.reduced_remainder.remainder = poly(&[]);
    cases.push(forged);
    let mut forged = original.clone();
    forged.monomial_order = DivisionMonomialOrder::Lexicographic;
    forged.lines.pop();
    cases.push(forged);
    let mut forged = original.clone();
    forged.lines[5].display = "false equality".into();
    cases.push(forged);
    let mut forged = original.clone();
    forged.lines[0].kind = DivisionExplanationKind::Recomposition;
    cases.push(forged);
    let mut forged = original.clone();
    forged.reported_iterations += 1;
    cases.push(forged);
    for forged in &cases {
        assert_eq!(
            verify_multivariate_derivation(&dividend, &supplied, &evidence, forged, limits),
            Err(MultivariateDerivationError::InvalidExplanation)
        );
    }
    assert_eq!(
        verify_multivariate_derivation(&dividend, &supplied, &evidence, &original, limits,),
        Ok(())
    );
}

#[test]
fn rejects_invalid_resource_limits_and_oversized_variable_labels() {
    let limits = MultivariateLimits::default();
    let f = poly(&[(2, 0, 1), (0, 0, 1)]);
    let g = poly(&[(1, 0, 1), (0, 0, 1)]);
    let evidence = divide_multivariate(&f, std::slice::from_ref(&g), limits).unwrap();
    let tiny = MultivariateLimits {
        max_work_units: 0,
        ..limits
    };
    assert_eq!(
        render_multivariate_division(&f, std::slice::from_ref(&g), &evidence, tiny),
        Err(MultivariateDerivationError::ResourceLimit)
    );

    let huge_name = sym(&format!("x{}", "q".repeat(257)));
    let huge =
        MultivariatePolynomial::try_new(vec![huge_name.clone()], [(vec![1], rat(1))], limits)
            .unwrap();
    let one = MultivariatePolynomial::constant(vec![huge_name], rat(1), limits).unwrap();
    let evidence = divide_multivariate(&huge, std::slice::from_ref(&one), limits).unwrap();
    assert_eq!(
        render_multivariate_division(&huge, &[one], &evidence, limits),
        Err(MultivariateDerivationError::ResourceLimit)
    );
}

#[test]
fn bounded_formatter_rejects_oversized_polynomial_text() {
    let limits = MultivariateLimits::default();
    let variables: Vec<_> = (0..16)
        .map(|i| sym(&format!("v{i}_{}", "q".repeat(235))))
        .collect();
    let terms = (1..=5).map(|k| {
        let mut powers = vec![1_i32; 16];
        powers[0] = k;
        (powers, rat(1))
    });
    let f = MultivariatePolynomial::try_new(variables.clone(), terms, limits).unwrap();
    let one = MultivariatePolynomial::constant(variables, rat(1), limits).unwrap();
    let evidence = divide_multivariate(&f, std::slice::from_ref(&one), limits).unwrap();
    assert_eq!(
        render_multivariate_division(&f, &[one], &evidence, limits),
        Err(MultivariateDerivationError::ResourceLimit)
    );
}
