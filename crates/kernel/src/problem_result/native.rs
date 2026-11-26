use super::*;
use crate::{
    AbsoluteEquationError, ExactQuadraticRoot, GroebnerError, SpecialAngleOutcome, TrigError,
    TrigLimits, evaluate_special_angle, groebner_basis_from_domain_expressions,
    ideal_membership_from_domain_expressions, partial_trig_zeros, solve_absolute_equation,
    verify_absolute_equation, verify_groebner_basis_from_domain_expressions,
    verify_ideal_membership_from_domain_expressions, verify_special_angle,
};
use mathema::Integer;

pub(super) fn is_native(operation: ProblemOperation) -> bool {
    matches!(
        operation,
        ProblemOperation::SolveAbsolute
            | ProblemOperation::EvaluateSpecialAngle
            | ProblemOperation::TrigZeros
            | ProblemOperation::GroebnerBasis
            | ProblemOperation::IdealMembership
    )
}

pub(super) fn admit(
    context: &EvaluationContext,
    statement: &ProblemStatement,
) -> Result<(), EvaluationError> {
    let limits = context.limits();
    match statement {
        ProblemStatement::Absolute(equation) => {
            for expression in [&equation.left, &equation.right] {
                if expression.layers.len() > limits.max_expression_nodes {
                    return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes));
                }
                for affine in std::iter::once(&expression.base).chain(&expression.layers) {
                    context.check_rational(&affine.slope)?;
                    context.check_rational(&affine.constant)?;
                }
            }
        }
        ProblemStatement::SpecialAngle { angle, .. } => {
            context.check_rational(angle.coefficient())?;
            if limits.max_work_units == 0 {
                return Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits));
            }
        }
        ProblemStatement::TrigZeros(_) if limits.max_work_units == 0 => {
            return Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits));
        }
        ProblemStatement::Ideal { generators, target } => {
            if generators.len() > 8 || generators.len() > limits.max_expression_nodes {
                return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes));
            }
            for source in generators.iter().chain(target.iter()) {
                admit_source(context, source)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn absolute_failure(
    request: &ProblemRequest,
    error: AbsoluteEquationError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        AbsoluteEquationError::Evaluation(error) => handle_evaluation_error(request, error),
        AbsoluteEquationError::CaseLimit => refusal(request, ResourceLimitKind::ExpressionNodes),
        AbsoluteEquationError::UnsupportedDomain => {
            Ok(unsupported(request, UnsupportedReason::Domain))
        }
        AbsoluteEquationError::MultipleVariables => {
            Ok(unsupported(request, UnsupportedReason::SeveralVariables))
        }
        AbsoluteEquationError::MissingAbsoluteValue => {
            Ok(unsupported(request, UnsupportedReason::ExpressionOutsideAlgorithm))
        }
        AbsoluteEquationError::VerificationFailed => Err(ProblemError::InvalidEvidence),
    }
}

fn trig_limits(context: &EvaluationContext) -> TrigLimits {
    TrigLimits {
        max_input_bits: TrigLimits::default()
            .max_input_bits
            .min(context.limits().max_scalar_bits)
            .min(context.limits().max_intermediate_scalar_bits.saturating_sub(5)),
    }
}

fn trig_failure(
    request: &ProblemRequest,
    error: TrigError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        TrigError::ScalarLimit | TrigError::InvalidLimits => {
            refusal(request, ResourceLimitKind::ScalarBits)
        }
        TrigError::UnsupportedDomain(_) => Ok(unsupported(request, UnsupportedReason::Domain)),
        TrigError::UnsupportedAngle | TrigError::UnsupportedEquation => {
            Ok(unsupported(request, UnsupportedReason::ExpressionOutsideAlgorithm))
        }
        TrigError::InvalidEvidence => Err(ProblemError::InvalidEvidence),
    }
}

fn groebner_failure(
    request: &ProblemRequest,
    error: GroebnerError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        GroebnerError::GeneratorLimit
        | GroebnerError::BasisLimit
        | GroebnerError::VariableNameLimit => refusal(request, ResourceLimitKind::ExpressionNodes),
        GroebnerError::RestrictedDomain { .. } => {
            Ok(unsupported(request, UnsupportedReason::ExpressionOutsideAlgorithm))
        }
        GroebnerError::Polynomial(error) => {
            expansion_failure(request, MultivariateExprError::Polynomial(error))
        }
        GroebnerError::Expression(error) => expansion_failure(request, error),
        GroebnerError::InvalidCertificate => Err(ProblemError::InvalidEvidence),
    }
}

pub(super) fn dispatch(
    context: &EvaluationContext,
    request: &ProblemRequest,
) -> Result<ProblemResult, ProblemError> {
    match &request.statement {
        ProblemStatement::Absolute(equation) => match solve_absolute_equation(context, equation) {
            Ok(candidate) => adapt(context, request, ProblemEvidence::Absolute(&candidate)),
            Err(error) => absolute_failure(request, error),
        },
        ProblemStatement::SpecialAngle { angle, function } => {
            match evaluate_special_angle(angle, *function, request.domain, trig_limits(context)) {
                Ok(candidate) => adapt(context, request, ProblemEvidence::SpecialAngle(&candidate)),
                Err(error) => trig_failure(request, error),
            }
        }
        ProblemStatement::TrigZeros(function) => {
            match partial_trig_zeros(*function, request.domain, trig_limits(context)) {
                Ok(candidate) => adapt(
                    context,
                    request,
                    ProblemEvidence::TrigZeros(candidate.known_zeros()),
                ),
                Err(error) => trig_failure(request, error),
            }
        }
        ProblemStatement::Ideal { generators, target } => {
            let limits = expansion_limits(context);
            let basis = match groebner_basis_from_domain_expressions(
                &request.variables,
                generators,
                limits,
            ) {
                Ok(candidate) => candidate,
                Err(error) => return groebner_failure(request, error),
            };
            if let Some(target) = target {
                match ideal_membership_from_domain_expressions(
                    &request.variables,
                    generators,
                    &basis,
                    target,
                    limits,
                ) {
                    Ok(candidate) => adapt(
                        context,
                        request,
                        ProblemEvidence::Membership(&basis, &candidate),
                    ),
                    Err(error) => groebner_failure(request, error),
                }
            } else {
                adapt(context, request, ProblemEvidence::Groebner(&basis))
            }
        }
        _ => Err(ProblemError::StatementMismatch),
    }
}

fn zero_angles(function: TrigFunction) -> Option<[Rational; 2]> {
    let ratio = |n, d| {
        Rational::new(Integer::from(n), Integer::from(d)).expect("nonzero denominator")
    };
    match function {
        TrigFunction::Sin => Some([ratio(0_i64, 1_i64), ratio(1, 1)]),
        TrigFunction::Cos => Some([ratio(1, 2), ratio(3, 2)]),
        TrigFunction::Tan => None,
    }
}

pub(super) fn adapt(
    context: &EvaluationContext,
    request: &ProblemRequest,
    evidence: ProblemEvidence<'_>,
) -> Result<ProblemResult, ProblemError> {
    let (outcome, proof, detail) = match (&request.statement, evidence) {
        (ProblemStatement::Absolute(equation), ProblemEvidence::Absolute(candidate)) => {
            match verify_absolute_equation(context, equation, candidate) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(error) => return absolute_failure(request, error),
            }
            let answer = ExactAnswer::Absolute(candidate.clone());
            if candidate.solutions.is_empty() {
                (ProblemOutcome::NoSolutions, CheckedBy::AbsoluteSignCases, Some(answer))
            } else {
                (ProblemOutcome::SolvedExact(answer), CheckedBy::AbsoluteSignCases, None)
            }
        }
        (
            ProblemStatement::SpecialAngle { angle, function },
            ProblemEvidence::SpecialAngle(candidate),
        ) => {
            match verify_special_angle(
                angle,
                *function,
                request.domain,
                candidate,
                trig_limits(context),
            ) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(error) => return trig_failure(request, error),
            }
            let admission = match &candidate.outcome {
                SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(value)) => {
                    context.check_rational(value)
                }
                SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
                    minimal_polynomial, ..
                }) => context.check_polynomial(minimal_polynomial),
                SpecialAngleOutcome::UndefinedTangentPole => Ok(()),
            };
            if let Err(error) = admission {
                return handle_evaluation_error(request, error);
            }
            let answer = ExactAnswer::SpecialAngle(Box::new(candidate.clone()));
            if candidate.outcome == SpecialAngleOutcome::UndefinedTangentPole {
                (
                    ProblemOutcome::Undefined(UndefinedReason::TangentPole),
                    CheckedBy::SpecialAngleIdentity,
                    Some(answer),
                )
            } else {
                (ProblemOutcome::SolvedExact(answer), CheckedBy::SpecialAngleIdentity, None)
            }
        }
        (ProblemStatement::TrigZeros(function), ProblemEvidence::TrigZeros(candidates)) => {
            let Some(angles) = zero_angles(*function) else {
                return Ok(unsupported(request, UnsupportedReason::ExpressionOutsideAlgorithm));
            };
            if candidates.len() != angles.len() {
                return Err(ProblemError::InvalidEvidence);
            }
            let zero = SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(Rational::from(
                Integer::zero(),
            )));
            for (angle, candidate) in angles.into_iter().zip(candidates) {
                if candidate.outcome != zero {
                    return Err(ProblemError::InvalidEvidence);
                }
                match verify_special_angle(
                    &PiMultiple::new(angle),
                    *function,
                    request.domain,
                    candidate,
                    trig_limits(context),
                ) {
                    Ok(true) => {}
                    Ok(false) => return Err(ProblemError::InvalidEvidence),
                    Err(error) => return trig_failure(request, error),
                }
            }
            (
                ProblemOutcome::PartialScope {
                    known: ExactAnswer::TrigZeros(candidates.to_vec()),
                    reason: UnsupportedReason::PeriodicCompletenessUnavailable,
                },
                CheckedBy::TrigZeroSubstitution,
                None,
            )
        }
        (
            ProblemStatement::Ideal { generators, target: None },
            ProblemEvidence::Groebner(candidate),
        ) => {
            match verify_groebner_basis_from_domain_expressions(
                &request.variables,
                generators,
                candidate,
                expansion_limits(context),
            ) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(error) => return groebner_failure(request, error),
            }
            (
                ProblemOutcome::SolvedExact(ExactAnswer::Groebner(Box::new(candidate.clone()))),
                CheckedBy::GroebnerCriterion,
                None,
            )
        }
        (
            ProblemStatement::Ideal { generators, target: Some(target) },
            ProblemEvidence::Membership(basis, candidate),
        ) => {
            match verify_ideal_membership_from_domain_expressions(
                &request.variables,
                generators,
                basis,
                target,
                candidate,
                expansion_limits(context),
            ) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(error) => return groebner_failure(request, error),
            }
            (
                ProblemOutcome::SolvedExact(ExactAnswer::Membership {
                    basis: Box::new(basis.clone()),
                    membership: Box::new(candidate.clone()),
                }),
                CheckedBy::IdealReduction,
                None,
            )
        }
        _ => return Err(ProblemError::StatementMismatch),
    };
    let mut checked = checked_result(request, outcome, proof, Vec::new());
    checked.detail = detail;
    Ok(checked)
}
