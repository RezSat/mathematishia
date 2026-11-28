//! The initial, deliberately small Mathema textual frontend.
//!
//! Parsing and lowering are frontend responsibilities. The public output uses
//! Mathema's syntax-independent immutable semantic values.

pub mod advanced;
pub mod natural;
pub mod problem;
pub mod school;

use std::error::Error;
use std::fmt;

use mathema::{
    DefinednessError, DomainExpression, Equation, Expr, ExprView, Integer, Rational, ScalarDomain,
    Symbol,
};

const MAX_INPUT_BYTES: usize = 32_768;
const MAX_TOKENS: usize = 4_096;
const MAX_PARSE_DEPTH: usize = 256;
const MAX_DECIMAL_DIGITS: usize = 256;

/// A supported Mathema statement after lowering to semantic values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Expression(Expr),
    Equation(Equation),
}

/// A parse or semantic-lowering failure with a UTF-8 byte offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    offset: usize,
    kind: ParseErrorKind,
}

impl ParseError {
    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn kind(&self) -> &ParseErrorKind {
        &self.kind
    }

    fn at(offset: usize, kind: ParseErrorKind) -> Self {
        Self { offset, kind }
    }
}

/// The first frontend's explicit error categories.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    EmptyInput,
    UnexpectedCharacter(char),
    UnexpectedToken,
    ExpectedClosingParenthesis,
    InvalidInteger,
    InvalidDecimal,
    DecimalDigitLimitExceeded,
    UnsupportedScientificNotation,
    ZeroDenominator,
    ExpectedExpression,
    UnknownFunction(String),
    UnsupportedFunctionDomain,
    InputLimitExceeded,
    TokenLimitExceeded,
    NestingLimitExceeded,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match &self.kind {
            ParseErrorKind::EmptyInput => "empty input".to_owned(),
            ParseErrorKind::UnexpectedCharacter(c) => format!("unexpected character {c:?}"),
            ParseErrorKind::UnexpectedToken => "unexpected token".to_owned(),
            ParseErrorKind::ExpectedClosingParenthesis => "expected ')'".to_owned(),
            ParseErrorKind::InvalidInteger => "invalid integer literal".to_owned(),
            ParseErrorKind::InvalidDecimal => "invalid decimal literal".to_owned(),
            ParseErrorKind::DecimalDigitLimitExceeded => {
                "decimal literal digit limit exceeded".to_owned()
            }
            ParseErrorKind::UnsupportedScientificNotation => {
                "scientific notation is unsupported".to_owned()
            }
            ParseErrorKind::ZeroDenominator => "rational denominator must not be zero".to_owned(),
            ParseErrorKind::ExpectedExpression => {
                "expected an expression, not an equation".to_owned()
            }
            ParseErrorKind::UnknownFunction(name) => format!("unknown function {name:?}"),
            ParseErrorKind::UnsupportedFunctionDomain => {
                "analytic function domain parsing is unsupported".to_owned()
            }
            ParseErrorKind::InputLimitExceeded => "input byte limit exceeded".to_owned(),
            ParseErrorKind::TokenLimitExceeded => "token limit exceeded".to_owned(),
            ParseErrorKind::NestingLimitExceeded => "expression nesting limit exceeded".to_owned(),
        };
        write!(f, "{message} at byte {}", self.offset)
    }
}

impl Error for ParseError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind<'a> {
    Integer(&'a str),
    Decimal(&'a str),
    Identifier(&'a str),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LeftParen,
    RightParen,
    Equal,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    End,
}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    kind: TokenKind<'a>,
    offset: usize,
}

fn tokenize(input: &str) -> Result<Vec<Token<'_>>, ParseError> {
    tokenize_with_comparisons(input, false)
}

fn tokenize_with_comparisons(input: &str, comparisons: bool) -> Result<Vec<Token<'_>>, ParseError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ParseError::at(0, ParseErrorKind::InputLimitExceeded));
    }
    let mut result = Vec::new();
    let mut offset = 0;

    while offset < input.len() {
        let start = offset;
        let c = input[offset..].chars().next().expect("nonempty suffix");
        if c.is_whitespace() {
            offset += c.len_utf8();
            continue;
        }
        let kind = if c.is_ascii_digit() || c == '.' {
            while offset < input.len() && input.as_bytes()[offset].is_ascii_digit() {
                offset += 1;
            }
            let decimal = if input.as_bytes().get(offset) == Some(&b'.') {
                let point_offset = offset;
                offset += 1;
                let fraction_start = offset;
                while offset < input.len() && input.as_bytes()[offset].is_ascii_digit() {
                    offset += 1;
                }
                if offset == fraction_start {
                    return Err(ParseError::at(point_offset, ParseErrorKind::InvalidDecimal));
                }
                if offset - start - 1 > MAX_DECIMAL_DIGITS {
                    return Err(ParseError::at(
                        start,
                        ParseErrorKind::DecimalDigitLimitExceeded,
                    ));
                }
                true
            } else {
                false
            };
            if matches!(input.as_bytes().get(offset).copied(), Some(b'e' | b'E')) {
                let exponent_start = offset + 1;
                let digit_offset = if matches!(
                    input.as_bytes().get(exponent_start).copied(),
                    Some(b'+' | b'-')
                ) {
                    exponent_start + 1
                } else {
                    exponent_start
                };
                if input
                    .as_bytes()
                    .get(digit_offset)
                    .is_some_and(u8::is_ascii_digit)
                {
                    return Err(ParseError::at(
                        offset,
                        ParseErrorKind::UnsupportedScientificNotation,
                    ));
                }
            }
            if decimal {
                TokenKind::Decimal(&input[start..offset])
            } else {
                TokenKind::Integer(&input[start..offset])
            }
        } else if c.is_alphabetic() || c == '_' {
            offset += c.len_utf8();
            while offset < input.len() {
                let next = input[offset..].chars().next().expect("nonempty suffix");
                if next.is_alphanumeric() || next == '_' {
                    offset += next.len_utf8();
                } else {
                    break;
                }
            }
            TokenKind::Identifier(&input[start..offset])
        } else {
            offset += c.len_utf8();
            match c {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '^' => TokenKind::Caret,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '=' => TokenKind::Equal,
                '<' if comparisons => {
                    if input.as_bytes().get(offset) == Some(&b'=') {
                        offset += 1;
                        TokenKind::LessOrEqual
                    } else {
                        TokenKind::Less
                    }
                }
                '>' if comparisons => {
                    if input.as_bytes().get(offset) == Some(&b'=') {
                        offset += 1;
                        TokenKind::GreaterOrEqual
                    } else {
                        TokenKind::Greater
                    }
                }
                '≤' if comparisons => TokenKind::LessOrEqual,
                '≥' if comparisons => TokenKind::GreaterOrEqual,
                _ => {
                    return Err(ParseError::at(
                        start,
                        ParseErrorKind::UnexpectedCharacter(c),
                    ));
                }
            }
        };
        if result.len() == MAX_TOKENS {
            return Err(ParseError::at(start, ParseErrorKind::TokenLimitExceeded));
        }
        result.push(Token {
            kind,
            offset: start,
        });
    }
    result.push(Token {
        kind: TokenKind::End,
        offset: input.len(),
    });
    Ok(result)
}

// The syntax tree never enters the semantic crate. Lowering takes place only
// after a complete statement has been parsed successfully.
enum Syntax<'a> {
    Integer(&'a str, usize),
    Decimal(&'a str, usize),
    Name(&'a str),
    Function(&'a str, Box<Self>),
    Sum(Box<Self>, Box<Self>),
    Difference(Box<Self>, Box<Self>),
    Product(Box<Self>, Box<Self>),
    Quotient(Box<Self>, Box<Self>, usize),
    Power(Box<Self>, Box<Self>),
    Negate(Box<Self>),
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    cursor: usize,
    allow_real_abs: bool,
}

impl<'a> Parser<'a> {
    fn current(&self) -> TokenKind<'a> {
        self.tokens[self.cursor].kind
    }

    fn offset(&self) -> usize {
        self.tokens[self.cursor].offset
    }

    fn advance(&mut self) {
        if self.current() != TokenKind::End {
            self.cursor += 1;
        }
    }

    fn consume(&mut self, kind: TokenKind<'a>) -> bool {
        if self.current() == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn has_implicit_product(&self) -> bool {
        let previous = self.tokens[self.cursor - 1].kind;
        matches!(
            (previous, self.current()),
            (
                TokenKind::Integer(_) | TokenKind::Decimal(_),
                TokenKind::Identifier(_) | TokenKind::LeftParen
            ) | (TokenKind::RightParen, TokenKind::LeftParen)
        )
    }

    fn check_depth(&self, depth: usize) -> Result<(), ParseError> {
        if depth > MAX_PARSE_DEPTH {
            Err(ParseError::at(
                self.offset(),
                ParseErrorKind::NestingLimitExceeded,
            ))
        } else {
            Ok(())
        }
    }

    fn sum(&mut self, depth: usize) -> Result<Syntax<'a>, ParseError> {
        self.check_depth(depth)?;
        let mut left = self.product(depth + 1)?;
        loop {
            if self.consume(TokenKind::Plus) {
                left = Syntax::Sum(Box::new(left), Box::new(self.product(depth + 1)?));
            } else if self.consume(TokenKind::Minus) {
                left = Syntax::Difference(Box::new(left), Box::new(self.product(depth + 1)?));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn product(&mut self, depth: usize) -> Result<Syntax<'a>, ParseError> {
        self.check_depth(depth)?;
        let mut left = self.signed(depth + 1)?;
        loop {
            if self.consume(TokenKind::Star) {
                left = Syntax::Product(Box::new(left), Box::new(self.signed(depth + 1)?));
            } else if self.current() == TokenKind::Slash {
                let offset = self.offset();
                self.advance();
                left = Syntax::Quotient(Box::new(left), Box::new(self.signed(depth + 1)?), offset);
            } else if self.has_implicit_product() {
                left = Syntax::Product(Box::new(left), Box::new(self.signed(depth + 1)?));
            } else {
                break;
            }
        }
        Ok(left)
    }

    // A leading negative has lower precedence than exponentiation.
    fn signed(&mut self, depth: usize) -> Result<Syntax<'a>, ParseError> {
        self.check_depth(depth)?;
        if self.consume(TokenKind::Plus) {
            self.signed(depth + 1)
        } else if self.consume(TokenKind::Minus) {
            Ok(Syntax::Negate(Box::new(self.signed(depth + 1)?)))
        } else {
            self.power(depth + 1)
        }
    }

    // Recursive signed RHS makes powers right-associative and accepts x^-2.
    fn power(&mut self, depth: usize) -> Result<Syntax<'a>, ParseError> {
        self.check_depth(depth)?;
        let base = self.atom(depth + 1)?;
        if self.consume(TokenKind::Caret) {
            Ok(Syntax::Power(
                Box::new(base),
                Box::new(self.signed(depth + 1)?),
            ))
        } else {
            Ok(base)
        }
    }

    fn atom(&mut self, depth: usize) -> Result<Syntax<'a>, ParseError> {
        self.check_depth(depth)?;
        let offset = self.offset();
        match self.current() {
            TokenKind::Integer(value) => {
                self.advance();
                Ok(Syntax::Integer(value, offset))
            }
            TokenKind::Decimal(value) => {
                self.advance();
                Ok(Syntax::Decimal(value, offset))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                if self.consume(TokenKind::LeftParen) {
                    if !(matches!(name, "sin" | "cos" | "tan" | "exp" | "log")
                        || self.allow_real_abs && name == "abs")
                    {
                        return Err(ParseError::at(
                            offset,
                            ParseErrorKind::UnknownFunction(name.to_owned()),
                        ));
                    }
                    let argument = self.sum(depth + 1)?;
                    if !self.consume(TokenKind::RightParen) {
                        return Err(ParseError::at(
                            self.offset(),
                            ParseErrorKind::ExpectedClosingParenthesis,
                        ));
                    }
                    Ok(Syntax::Function(name, Box::new(argument)))
                } else {
                    Ok(Syntax::Name(name))
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let expression = self.sum(depth + 1)?;
                if !self.consume(TokenKind::RightParen) {
                    return Err(ParseError::at(
                        self.offset(),
                        ParseErrorKind::ExpectedClosingParenthesis,
                    ));
                }
                Ok(expression)
            }
            _ => Err(ParseError::at(offset, ParseErrorKind::UnexpectedToken)),
        }
    }
}

fn lower(node: Syntax<'_>) -> Result<Expr, ParseError> {
    match node {
        Syntax::Integer(digits, offset) => digits
            .parse::<Integer>()
            .map(Expr::from)
            .map_err(|_| ParseError::at(offset, ParseErrorKind::InvalidInteger)),
        Syntax::Decimal(digits, offset) => {
            let (whole, fractional) = digits
                .split_once('.')
                .expect("decimal tokens contain a point");
            let numerator = format!("{whole}{fractional}")
                .parse::<Integer>()
                .map_err(|_| ParseError::at(offset, ParseErrorKind::InvalidDecimal))?;
            let denominator = format!("1{}", "0".repeat(fractional.len()))
                .parse::<Integer>()
                .map_err(|_| ParseError::at(offset, ParseErrorKind::InvalidDecimal))?;
            Ok(Expr::from(
                Rational::new(numerator, denominator)
                    .expect("a decimal power-of-ten denominator is nonzero"),
            ))
        }
        Syntax::Name(name) => Ok(Symbol::new(name)
            .expect("lexer only produces nonempty identifiers")
            .into()),
        Syntax::Function(name, argument) => {
            let value = lower(*argument)?;
            Ok(match name {
                "sin" => Expr::sin(value),
                "cos" => Expr::cos(value),
                "tan" => Expr::tan(value),
                "exp" => Expr::exp(value),
                "log" => Expr::log(value),
                _ => unreachable!("parser admits only built-in functions"),
            })
        }
        Syntax::Sum(left, right) => Ok(Expr::add([lower(*left)?, lower(*right)?])),
        Syntax::Difference(left, right) => Ok(Expr::add([
            lower(*left)?,
            Expr::mul([(-1).into(), lower(*right)?]),
        ])),
        Syntax::Product(left, right) => Ok(Expr::mul([lower(*left)?, lower(*right)?])),
        Syntax::Quotient(left, right, offset) => {
            let numerator = lower(*left)?;
            let denominator = lower(*right)?;
            let rational = match denominator.view() {
                ExprView::Integer(value) => Some(Rational::from(value.clone())),
                ExprView::Rational(value) => Some(value.clone()),
                _ => None,
            };
            if let Some(value) = rational {
                let inverse = Rational::new(value.denominator().clone(), value.numerator().clone())
                    .map_err(|_| ParseError::at(offset, ParseErrorKind::ZeroDenominator))?;
                Ok(Expr::mul([numerator, Expr::from(inverse)]))
            } else {
                Ok(Expr::mul([
                    numerator,
                    Expr::pow(denominator, Expr::from(-1)),
                ]))
            }
        }
        Syntax::Power(left, right) => Ok(Expr::pow(lower(*left)?, lower(*right)?)),
        Syntax::Negate(node) => Ok(Expr::mul([(-1).into(), lower(*node)?])),
    }
}

/// Domain-preserving parsed values, including restrictions removed by
/// structural normalization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainStatement {
    Expression(DomainExpression),
    Equation {
        left: DomainExpression,
        right: DomainExpression,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainParseError {
    Syntax(ParseError),
    Domain(DefinednessError),
}

impl fmt::Display for DomainParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => write!(f, "{error}"),
            Self::Domain(error) => write!(f, "{error}"),
        }
    }
}

impl Error for DomainParseError {}

impl From<ParseError> for DomainParseError {
    fn from(error: ParseError) -> Self {
        Self::Syntax(error)
    }
}

impl From<DefinednessError> for DomainParseError {
    fn from(error: DefinednessError) -> Self {
        Self::Domain(error)
    }
}

enum SyntaxStatement<'a> {
    Expression(Syntax<'a>),
    Equation(Syntax<'a>, Syntax<'a>),
}

fn parse_syntax(input: &str) -> Result<SyntaxStatement<'_>, ParseError> {
    parse_syntax_mode(input, false)
}

fn parse_syntax_mode(input: &str, allow_real_abs: bool) -> Result<SyntaxStatement<'_>, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        allow_real_abs,
    };
    if parser.current() == TokenKind::End {
        return Err(ParseError::at(0, ParseErrorKind::EmptyInput));
    }
    let left = parser.sum(0)?;
    let statement = if parser.consume(TokenKind::Equal) {
        let right = parser.sum(0)?;
        SyntaxStatement::Equation(left, right)
    } else {
        SyntaxStatement::Expression(left)
    };
    if parser.current() != TokenKind::End {
        return Err(ParseError::at(
            parser.offset(),
            ParseErrorKind::UnexpectedToken,
        ));
    }
    Ok(statement)
}

fn lower_domain(
    node: Syntax<'_>,
    domain: ScalarDomain,
) -> Result<DomainExpression, DomainParseError> {
    lower_domain_mode(node, domain, false)
}

fn lower_domain_mode(
    node: Syntax<'_>,
    domain: ScalarDomain,
    bound: bool,
) -> Result<DomainExpression, DomainParseError> {
    const MAX_CONDITIONS: usize = 256;
    match node {
        Syntax::Function(name, argument) => Ok(DomainExpression::function(
            Symbol::new(name).expect("parser accepts nonempty built-in names"),
            lower_domain_mode(*argument, domain, bound)?,
            domain,
            MAX_CONDITIONS,
        )?),
        leaf @ (Syntax::Integer(..) | Syntax::Decimal(..) | Syntax::Name(..)) => {
            if bound {
                Ok(DomainExpression::from_expr_in_domain(
                    lower(leaf)?,
                    domain,
                    MAX_TOKENS,
                    MAX_CONDITIONS,
                )?)
            } else {
                Ok(DomainExpression::from_expr(lower(leaf)?, MAX_TOKENS)?)
            }
        }
        Syntax::Sum(left, right) => Ok(DomainExpression::add(
            [
                lower_domain_mode(*left, domain, bound)?,
                lower_domain_mode(*right, domain, bound)?,
            ],
            MAX_CONDITIONS,
        )?),
        Syntax::Difference(left, right) => {
            let negative_right = DomainExpression::multiply(
                [
                    DomainExpression::from_expr(Expr::from(-1), MAX_TOKENS)?,
                    lower_domain_mode(*right, domain, bound)?,
                ],
                MAX_CONDITIONS,
            )?;
            Ok(DomainExpression::add(
                [lower_domain_mode(*left, domain, bound)?, negative_right],
                MAX_CONDITIONS,
            )?)
        }
        Syntax::Product(left, right) => Ok(DomainExpression::multiply(
            [
                lower_domain_mode(*left, domain, bound)?,
                lower_domain_mode(*right, domain, bound)?,
            ],
            MAX_CONDITIONS,
        )?),
        Syntax::Quotient(left, right, _) => Ok(lower_domain_mode(*left, domain, bound)?
            .divide(lower_domain_mode(*right, domain, bound)?, MAX_CONDITIONS)?),
        Syntax::Power(left, right) => Ok(lower_domain_mode(*left, domain, bound)?
            .pow(lower_domain_mode(*right, domain, bound)?, MAX_CONDITIONS)?),
        Syntax::Negate(node) => Ok(DomainExpression::multiply(
            [
                DomainExpression::from_expr(Expr::from(-1), MAX_TOKENS)?,
                lower_domain_mode(*node, domain, bound)?,
            ],
            MAX_CONDITIONS,
        )?),
    }
}

/// One real comparison between two domain-aware scalar expressions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealComparisonOperator {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealComparison {
    pub left: DomainExpression,
    pub operator: RealComparisonOperator,
    pub right: DomainExpression,
}

/// Parses exactly one comparison, preserving each operand's original domain.
/// Chained comparisons and comparisons nested inside operands are rejected.
pub fn parse_real_comparison(input: &str) -> Result<RealComparison, DomainParseError> {
    parse_real_comparison_mode(input, false)
}

pub fn parse_bound_real_comparison(input: &str) -> Result<RealComparison, DomainParseError> {
    parse_real_comparison_mode(input, true)
}

fn parse_real_comparison_mode(
    input: &str,
    bound: bool,
) -> Result<RealComparison, DomainParseError> {
    let tokens = tokenize_with_comparisons(input, true)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        allow_real_abs: false,
    };
    let left = parser.sum(0)?;
    let operator = match parser.current() {
        TokenKind::Less => RealComparisonOperator::Less,
        TokenKind::LessOrEqual => RealComparisonOperator::LessOrEqual,
        TokenKind::Greater => RealComparisonOperator::Greater,
        TokenKind::GreaterOrEqual => RealComparisonOperator::GreaterOrEqual,
        _ => {
            return Err(ParseError::at(parser.offset(), ParseErrorKind::UnexpectedToken).into());
        }
    };
    parser.advance();
    let right = parser.sum(0)?;
    if parser.current() != TokenKind::End {
        return Err(ParseError::at(parser.offset(), ParseErrorKind::UnexpectedToken).into());
    }
    Ok(RealComparison {
        left: lower_domain_mode(left, ScalarDomain::Real, bound)?,
        operator,
        right: lower_domain_mode(right, ScalarDomain::Real, bound)?,
    })
}

/// Parses without discarding restrictions during expression normalization.
///
/// The supported definedness domain includes integer powers over complex
/// scalars. Symbolic and noninteger exponents return a domain error. Division
/// preserves numerator and denominator restrictions.
pub fn parse_domain_statement(input: &str) -> Result<DomainStatement, DomainParseError> {
    parse_domain_statement_in_domain(input, ScalarDomain::Complex)
}

/// Domain-preserving parser for either real or complex scalar expressions.
/// Conditions are symbolic and are not presumed to hold.
pub fn parse_domain_statement_in_domain(
    input: &str,
    domain: ScalarDomain,
) -> Result<DomainStatement, DomainParseError> {
    match parse_syntax(input)? {
        SyntaxStatement::Expression(node) => {
            Ok(DomainStatement::Expression(lower_domain(node, domain)?))
        }
        SyntaxStatement::Equation(left, right) => Ok(DomainStatement::Equation {
            left: lower_domain(left, domain)?,
            right: lower_domain(right, domain)?,
        }),
    }
}

/// Parse source with the domain fixed at every leaf, before normalization.
pub fn parse_bound_domain_statement(
    input: &str,
    domain: ScalarDomain,
) -> Result<DomainStatement, DomainParseError> {
    match parse_syntax_mode(input, domain == ScalarDomain::Real)? {
        SyntaxStatement::Expression(node) => Ok(DomainStatement::Expression(lower_domain_mode(
            node, domain, true,
        )?)),
        SyntaxStatement::Equation(left, right) => Ok(DomainStatement::Equation {
            left: lower_domain_mode(left, domain, true)?,
            right: lower_domain_mode(right, domain, true)?,
        }),
    }
}

pub fn parse_bound_domain_expression(
    input: &str,
    domain: ScalarDomain,
) -> Result<DomainExpression, DomainParseError> {
    match parse_bound_domain_statement(input, domain)? {
        DomainStatement::Expression(value) => Ok(value),
        DomainStatement::Equation { .. } => {
            Err(ParseError::at(0, ParseErrorKind::ExpectedExpression).into())
        }
    }
}

pub fn parse_domain_expression(input: &str) -> Result<DomainExpression, DomainParseError> {
    parse_domain_expression_in_domain(input, ScalarDomain::Complex)
}

pub fn parse_domain_expression_in_domain(
    input: &str,
    domain: ScalarDomain,
) -> Result<DomainExpression, DomainParseError> {
    match parse_domain_statement_in_domain(input, domain)? {
        DomainStatement::Expression(expr) => Ok(expr),
        DomainStatement::Equation { .. } => {
            Err(ParseError::at(0, ParseErrorKind::ExpectedExpression).into())
        }
    }
}

/// Parses a single Mathema expression or equation.
///
/// Grammar: ASCII integers and finite decimals (including .5), exact rational
/// literals n/d, Unicode identifiers, parentheses, +, -, *, /, ^ and one =.
/// A decimal has digits after its point, at most 256 total digits, and lowers
/// exactly to a rational. Scientific notation and trailing points are rejected.
/// Multiplication may be written as 2x, 2(x+1), or (x+1)(x-1), including
/// whitespace between factors. Only numeric-to-name, numeric-to-parenthesis,
/// and closing-to-opening parenthesis juxtaposition is implicit; other
/// adjacency remains unsupported. Implicit multiplication shares * and /
/// precedence and associates left to right, so 1/2x means (1/2)*x.
/// Exponentiation associates right and binds tighter than unary minus.
/// sin, cos, tan, exp, and natural log require name(expr) calls. Other
/// name(expr) calls are rejected rather than interpreted as multiplication.
/// No inequalities, assignments or statement separators are accepted.
/// A syntax tree is parsed before semantic lowering. Domain-aware entrypoints
/// retain restrictions that ordinary Expr normalization can erase.
pub fn parse_statement(input: &str) -> Result<Statement, ParseError> {
    match parse_syntax(input)? {
        SyntaxStatement::Expression(node) => Ok(Statement::Expression(lower(node)?)),
        SyntaxStatement::Equation(left, right) => Ok(Statement::Equation(Equation::new(
            lower(left)?,
            lower(right)?,
        ))),
    }
}

/// Parses a scalar expression and rejects equations.
pub fn parse_expression(input: &str) -> Result<Expr, ParseError> {
    match parse_statement(input)? {
        Statement::Expression(expr) => Ok(expr),
        Statement::Equation(_) => Err(ParseError::at(0, ParseErrorKind::ExpectedExpression)),
    }
}

#[cfg(test)]
mod analytic_tests {
    use super::*;

    fn parsed(input: &str) -> Expr {
        parse_expression(input).unwrap()
    }

    #[test]
    fn functions_parse_nested_and_preserve_identity() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let cases = [
            ("sin(x)", Expr::sin(x.clone())),
            ("cos(x)", Expr::cos(x.clone())),
            ("tan(x)", Expr::tan(x.clone())),
            ("exp(x)", Expr::exp(x.clone())),
            ("log(x)", Expr::log(x.clone())),
            ("sin(log(x))", Expr::sin(Expr::log(x.clone()))),
            (
                "log(tan(1 + x))",
                Expr::log(Expr::tan(Expr::add([1.into(), x.clone()]))),
            ),
            ("sin(x)^2", Expr::pow(Expr::sin(x.clone()), 2.into())),
            ("2*sin(x)", Expr::mul([2.into(), Expr::sin(x.clone())])),
            ("-sin(x)", Expr::mul([(-1).into(), Expr::sin(x.clone())])),
        ];
        for (source, expected) in cases {
            assert_eq!(parsed(source), expected, "{source}");
            assert_eq!(parsed(&expected.to_string()), expected, "{source}");
        }
        assert_eq!(
            parse_statement("exp(x)=log(x)").unwrap(),
            Statement::Equation(Equation::new(Expr::exp(x.clone()), Expr::log(x)))
        );
    }

    #[test]
    fn old_names_and_explicit_product_grammar_are_unchanged() {
        for name in ["sin", "cos", "tan", "exp", "log", "arcsin", "sin_x", "α"] {
            assert_eq!(parsed(name), Expr::from(Symbol::new(name).unwrap()));
        }
        assert_eq!(
            parsed("sin*(x + 1)"),
            Expr::mul([
                Expr::from(Symbol::new("sin").unwrap()),
                Expr::add([1.into(), Expr::from(Symbol::new("x").unwrap())])
            ])
        );
        assert_eq!(
            parsed("2^3^2"),
            Expr::pow(2.into(), Expr::pow(3.into(), 2.into()))
        );
        assert_eq!(
            parsed("-x^2"),
            Expr::mul([
                (-1).into(),
                Expr::pow(Expr::from(Symbol::new("x").unwrap()), 2.into())
            ])
        );
        assert_eq!(
            parsed("3/2 + x"),
            Expr::add([
                Expr::from(Rational::new(3.into(), 2.into()).unwrap()),
                Expr::from(Symbol::new("x").unwrap())
            ])
        );
    }

    #[test]
    fn invalid_calls_are_explicit_errors() {
        for (source, name) in [
            ("foo(x)", "foo"),
            ("Sin(x)", "Sin"),
            ("arcsin(x)", "arcsin"),
            ("sin_1(x)", "sin_1"),
        ] {
            assert_eq!(
                parse_expression(source).unwrap_err().kind(),
                &ParseErrorKind::UnknownFunction(name.to_owned())
            );
        }
        for source in [
            "sin()",
            "sin(",
            "log(,)",
            "tan(x, y)",
            "cos(x",
            "sin(x))",
            "sin(x)y",
        ] {
            assert!(parse_expression(source).is_err(), "{source}");
        }
        assert_eq!(
            parse_expression("log(x)=0").unwrap_err().kind(),
            &ParseErrorKind::ExpectedExpression
        );
    }

    #[test]
    fn domain_parser_does_not_discard_function_obligations() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let complex = parse_domain_expression("sin(log(x))").unwrap();
        assert_eq!(
            complex.conditions(),
            &[mathema::Condition::NonZero(x.clone())]
        );

        let real =
            parse_domain_expression_in_domain("0 * tan(log(x))", ScalarDomain::Real).unwrap();
        assert_eq!(real.expression(), &Expr::from(0));
        assert_eq!(
            real.conditions(),
            &[
                mathema::Condition::Positive(x.clone()),
                mathema::Condition::NonZero(Expr::cos(Expr::log(x.clone()))),
            ]
        );
        let tan = parse_domain_expression("tan(x)").unwrap();
        assert_eq!(
            tan.conditions(),
            &[mathema::Condition::NonZero(Expr::cos(x.clone()))],
        );
        let no_conditions = parse_domain_expression("sin(exp(x))").unwrap();
        assert!(no_conditions.conditions().is_empty());
        assert_eq!(
            parse_domain_expression("1/x").unwrap().conditions().len(),
            1
        );
    }

    #[test]
    fn function_parsing_honors_existing_resource_limits() {
        let depth = format!("{}x{}", "sin(".repeat(300), ")".repeat(300));
        assert_eq!(
            parse_expression(&depth).unwrap_err().kind(),
            &ParseErrorKind::NestingLimitExceeded
        );
        let input = "x".repeat(MAX_INPUT_BYTES + 1);
        assert_eq!(
            parse_expression(&input).unwrap_err().kind(),
            &ParseErrorKind::InputLimitExceeded
        );
        let input = format!("sin({})", "x+".repeat(MAX_TOKENS));
        assert_eq!(
            parse_expression(&input).unwrap_err().kind(),
            &ParseErrorKind::TokenLimitExceeded
        );
    }
}
