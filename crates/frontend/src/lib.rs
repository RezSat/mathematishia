//! The initial, deliberately small Mathema textual frontend.
//!
//! Parsing and lowering are frontend responsibilities. The public output uses
//! Mathema's syntax-independent immutable semantic values.

use std::error::Error;
use std::fmt;

use mathema::{DefinednessError, DomainExpression, Equation, Expr, Integer, Rational, Symbol};

const MAX_INPUT_BYTES: usize = 32_768;
const MAX_TOKENS: usize = 4_096;
const MAX_PARSE_DEPTH: usize = 256;

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
    ZeroDenominator,
    ExpectedExpression,
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
            ParseErrorKind::ZeroDenominator => "rational denominator must not be zero".to_owned(),
            ParseErrorKind::ExpectedExpression => {
                "expected an expression, not an equation".to_owned()
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
    Identifier(&'a str),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LeftParen,
    RightParen,
    Equal,
    End,
}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    kind: TokenKind<'a>,
    offset: usize,
}

fn tokenize(input: &str) -> Result<Vec<Token<'_>>, ParseError> {
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
        let kind = if c.is_ascii_digit() {
            offset += 1;
            while offset < input.len() && input.as_bytes()[offset].is_ascii_digit() {
                offset += 1;
            }
            TokenKind::Integer(&input[start..offset])
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
    Fraction(&'a str, &'a str, usize),
    Name(&'a str),
    Sum(Box<Self>, Box<Self>),
    Difference(Box<Self>, Box<Self>),
    Product(Box<Self>, Box<Self>),
    Power(Box<Self>, Box<Self>),
    Negate(Box<Self>),
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    cursor: usize,
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
        while self.consume(TokenKind::Star) {
            left = Syntax::Product(Box::new(left), Box::new(self.signed(depth + 1)?));
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
            TokenKind::Integer(numerator) => {
                self.advance();
                if self.consume(TokenKind::Slash) {
                    let TokenKind::Integer(denominator) = self.current() else {
                        return Err(ParseError::at(
                            self.offset(),
                            ParseErrorKind::UnexpectedToken,
                        ));
                    };
                    self.advance();
                    Ok(Syntax::Fraction(numerator, denominator, offset))
                } else {
                    Ok(Syntax::Integer(numerator, offset))
                }
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Syntax::Name(name))
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
        Syntax::Fraction(numerator, denominator, offset) => {
            let n = numerator
                .parse::<Integer>()
                .map_err(|_| ParseError::at(offset, ParseErrorKind::InvalidInteger))?;
            let d = denominator
                .parse::<Integer>()
                .map_err(|_| ParseError::at(offset, ParseErrorKind::InvalidInteger))?;
            let value = Rational::new(n, d)
                .map_err(|_| ParseError::at(offset, ParseErrorKind::ZeroDenominator))?;
            Ok(value.into())
        }
        Syntax::Name(name) => Ok(Symbol::new(name)
            .expect("lexer only produces nonempty identifiers")
            .into()),
        Syntax::Sum(left, right) => Ok(Expr::add([lower(*left)?, lower(*right)?])),
        Syntax::Difference(left, right) => Ok(Expr::add([
            lower(*left)?,
            Expr::mul([(-1).into(), lower(*right)?]),
        ])),
        Syntax::Product(left, right) => Ok(Expr::mul([lower(*left)?, lower(*right)?])),
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
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, cursor: 0 };
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

fn lower_domain(node: Syntax<'_>) -> Result<DomainExpression, DomainParseError> {
    const MAX_CONDITIONS: usize = 256;
    match node {
        leaf @ (Syntax::Integer(..) | Syntax::Fraction(..) | Syntax::Name(..)) => {
            Ok(DomainExpression::from_expr(lower(leaf)?, MAX_TOKENS)?)
        }
        Syntax::Sum(left, right) => Ok(DomainExpression::add(
            [lower_domain(*left)?, lower_domain(*right)?],
            MAX_CONDITIONS,
        )?),
        Syntax::Difference(left, right) => {
            let negative_right = DomainExpression::multiply(
                [
                    DomainExpression::from_expr(Expr::from(-1), MAX_TOKENS)?,
                    lower_domain(*right)?,
                ],
                MAX_CONDITIONS,
            )?;
            Ok(DomainExpression::add(
                [lower_domain(*left)?, negative_right],
                MAX_CONDITIONS,
            )?)
        }
        Syntax::Product(left, right) => Ok(DomainExpression::multiply(
            [lower_domain(*left)?, lower_domain(*right)?],
            MAX_CONDITIONS,
        )?),
        Syntax::Power(left, right) => {
            Ok(lower_domain(*left)?.pow(lower_domain(*right)?, MAX_CONDITIONS)?)
        }
        Syntax::Negate(node) => Ok(DomainExpression::multiply(
            [
                DomainExpression::from_expr(Expr::from(-1), MAX_TOKENS)?,
                lower_domain(*node)?,
            ],
            MAX_CONDITIONS,
        )?),
    }
}

/// Parses without discarding restrictions during expression normalization.
///
/// The supported definedness domain includes integer powers over complex
/// scalars. Symbolic and noninteger exponents return a domain error.
pub fn parse_domain_statement(input: &str) -> Result<DomainStatement, DomainParseError> {
    match parse_syntax(input)? {
        SyntaxStatement::Expression(node) => Ok(DomainStatement::Expression(lower_domain(node)?)),
        SyntaxStatement::Equation(left, right) => Ok(DomainStatement::Equation {
            left: lower_domain(left)?,
            right: lower_domain(right)?,
        }),
    }
}

pub fn parse_domain_expression(input: &str) -> Result<DomainExpression, DomainParseError> {
    match parse_domain_statement(input)? {
        DomainStatement::Expression(expr) => Ok(expr),
        DomainStatement::Equation { .. } => {
            Err(ParseError::at(0, ParseErrorKind::ExpectedExpression).into())
        }
    }
}

/// Parses a single Mathema expression or equation.
///
/// Initial grammar: ASCII decimal integers, exact rational literals n/d,
/// Unicode identifiers, parentheses, +, -, *, ^ and a single top-level =.
/// Operators are explicit (no implicit multiplication or general division).
/// Exponentiation associates right, and unary minus binds less tightly than ^.
/// There are no function calls, assignments, floating point values or statements
/// separated by semicolons. A syntax tree is parsed before semantic lowering.
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
