#!/usr/bin/env bash
set -u
printf 'RUNTIME_AUDIT_START\n'
cargo build --quiet --locked -p mathematishia-cli || exit 30
BIN=./target/debug/mathematishia
run_case() {
  local case_id="$1"
  local description="$2"
  shift 2
  printf '\nCASE_START|%s|%s\n' "$case_id" "$description"
  printf 'COMMAND|'
  printf '%q ' "$@"
  printf '\n'
  local out
  out=$("$BIN" "$@" 2>&1)
  local exit_status="$?"
  printf '%s\n' "$out" | sed 's/^/CLI_OUT|/'
  printf 'CASE_END|%s|exit=%s\n' "$case_id" "$exit_status"
}
run_case '01' 'Exact numeric arithmetic' 'expr' '1/2+1/3'
run_case '02' 'Raw symbolic expression' 'expr' '(x+1)^2'
run_case '03' 'Expand binomial' 'expand' '(x+1)^2' '4'
run_case '04' 'Linear equation' 'solve' '2*x+4=10' 'x'
run_case '05' 'Quadratic equation' 'solveq' 'x^2-5*x+6' 'x'
run_case '06' 'Exact irrational roots' 'roots2' 'x^2-2' 'x'
run_case '07' 'Factor quartic' 'factor' 'x^4-1' 'x'
run_case '08' 'Rational simplification and domain' 'rsimplify' '(x^2-1)/(x-1)' 'x'
run_case '09' 'Rational equation with exclusion' 'rsolve' '(x^2-1)/(x-1)=3' 'x'
run_case '10' 'Chain-rule differentiation' 'diffexpr' 'sin(x^2)' 'x'
run_case '11' 'Real derivative with domain' 'diffreal' 'tan(log(x))' 'x'
run_case '12' 'Exact matrix determinant' 'matdet' '2,1;5,3'
run_case '13' 'Solve exact matrix system' 'matsolve' '2,1;5,3' '5,13'
run_case '14' 'Polynomial point evaluation' 'eval' 'x^2+1/2' 'x' '1/2'
run_case '15' 'Excluded rational point' 'reval' '(x^2-1)/(x-1)' 'x' '1'
run_case '16' 'Analytic constant through expr' 'expr' 'sin(0)'
run_case '17' 'General trigonometric equation' 'solve' 'sin(x)=0' 'x'
run_case '18' 'Quadratic inequality' 'solve' 'x^2-4>0' 'x'
run_case '19' 'Absolute-value equation' 'solve' 'abs(x)=3' 'x'
run_case '20' 'Basic trigonometric limit' 'limit' 'sin(x)/x' 'x' '0'
run_case '21' 'Integrate polynomial via CLI' 'integrate' 'x^2' 'x'
run_case '22' 'Integrate product exp*cos' 'integrate' 'exp(x)*cos(x)' 'x'
run_case '23' 'Implicit multiplication' 'expr' '2x+1'
run_case '24' 'Decimal literals' 'expr' '0.25+0.5'
run_case '25' 'General power derivative' 'diffexpr' 'x^x' 'x'
run_case '26' 'Multivariate polynomial via poly command' 'poly' 'x^2+y' 'x'
run_case '27' 'Nonrational function in rational simplify' 'rsimplify' 'sin(x)/x' 'x'
run_case '28' 'Cubic passed to quadratic solver' 'solveq' 'x^3-1' 'x'
run_case '29' 'Higher-degree factorization' 'factor' 'x^8-1' 'x'
run_case '30' 'Partial derivative with second variable' 'diffexpr' 'x*y+(x+1)^3' 'x'
run_case '31' 'Basic distribution' 'expand' '2*(x+3)' '4'
run_case '32' 'Inconsistent linear equation' 'solve' '0*x=1' 'x'
run_case '33' 'Identity linear equation' 'solve' '0*x=0' 'x'
run_case '34' 'Rational equation no roots' 'rsolve' '1/(x-1)=0' 'x'
run_case '35' 'Square-root function syntax' 'expr' 'sqrt(2)'
run_case '36' 'Pi symbol input' 'expr' 'pi'
run_case '37' 'Track cancelled domain' 'domain' '0*x^-1'
run_case '38' 'Factor irreducible quadratic' 'factor' 'x^2+1' 'x'
printf '\nRUNTIME_AUDIT_END\n'
