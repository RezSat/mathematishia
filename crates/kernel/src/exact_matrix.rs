//! Exact linear algebra over Q.
//!
//! Matrices are rectangular, nonempty and bounded in size. Elimination uses
//! rational pivots without floating-point thresholds.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::{EvaluationContext, EvaluationError};

const MAX_MATRIX_DIM: usize = 16;

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn one() -> Rational {
    Rational::from(Integer::one())
}

fn divide(left: &Rational, right: &Rational) -> Rational {
    Rational::new(
        left.numerator() * right.denominator(),
        left.denominator() * right.numerator(),
    )
    .expect("elimination only divides by a nonzero pivot")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixError {
    Empty,
    NonRectangular,
    DimensionLimit,
    DimensionMismatch,
    NotSquare,
    Evaluation(EvaluationError),
}

impl fmt::Display for MatrixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("matrix must have positive dimensions"),
            Self::NonRectangular => f.write_str("matrix rows must have equal lengths"),
            Self::DimensionLimit => f.write_str("matrix dimension exceeds the exact solver limit"),
            Self::DimensionMismatch => f.write_str("matrix dimensions are incompatible"),
            Self::NotSquare => f.write_str("matrix must be square"),
            Self::Evaluation(error) => write!(f, "{error}"),
        }
    }
}
impl Error for MatrixError {}

impl From<EvaluationError> for MatrixError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactMatrix {
    rows: Vec<Vec<Rational>>,
    columns: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReducedMatrix {
    matrix: ExactMatrix,
    pivot_columns: Vec<usize>,
}

impl ReducedMatrix {
    pub fn matrix(&self) -> &ExactMatrix {
        &self.matrix
    }

    pub fn pivot_columns(&self) -> &[usize] {
        &self.pivot_columns
    }

    pub fn rank(&self) -> usize {
        self.pivot_columns.len()
    }
}

/// A system Ax=b has either no solution, one solution, or an affine family.
/// For an affine family, every solution is particular + sum(t_i * basis_i).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinearSystemSolution {
    Inconsistent,
    Unique(Vec<Rational>),
    Infinite {
        particular: Vec<Rational>,
        nullspace_basis: Vec<Vec<Rational>>,
    },
}

impl ExactMatrix {
    pub fn new(
        ctx: &EvaluationContext,
        rows: Vec<Vec<Rational>>,
    ) -> Result<Self, MatrixError> {
        let Some(first) = rows.first() else {
            return Err(MatrixError::Empty);
        };
        let columns = first.len();
        if columns == 0 {
            return Err(MatrixError::Empty);
        }
        if rows.iter().any(|row| row.len() != columns) {
            return Err(MatrixError::NonRectangular);
        }
        if rows.len() > MAX_MATRIX_DIM || columns > MAX_MATRIX_DIM {
            return Err(MatrixError::DimensionLimit);
        }
        for row in &rows {
            for value in row {
                ctx.check_rational(value)?;
            }
        }
        Ok(Self { rows, columns })
    }

    pub fn identity(ctx: &EvaluationContext, size: usize) -> Result<Self, MatrixError> {
        if size == 0 {
            return Err(MatrixError::Empty);
        }
        if size > MAX_MATRIX_DIM {
            return Err(MatrixError::DimensionLimit);
        }
        Self::new(
            ctx,
            (0..size)
                .map(|row| {
                    (0..size)
                        .map(|column| if row == column { one() } else { zero() })
                        .collect()
                })
                .collect(),
        )
    }

    pub fn rows(&self) -> &[Vec<Rational>] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn multiply(
        &self,
        right: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, MatrixError> {
        if self.columns != right.row_count() {
            return Err(MatrixError::DimensionMismatch);
        }
        let mut product = vec![vec![zero(); right.columns]; self.row_count()];
        for (row, result_row) in product.iter_mut().enumerate() {
            for (column, result) in result_row.iter_mut().enumerate() {
                for middle in 0..self.columns {
                    let term = &self.rows[row][middle] * &right.rows[middle][column];
                    *result = &*result + &term;
                    ctx.check_rational(result)?;
                }
            }
        }
        Self::new(ctx, product)
    }

    pub fn transpose(&self, ctx: &EvaluationContext) -> Result<Self, MatrixError> {
        Self::new(
            ctx,
            (0..self.columns)
                .map(|column| {
                    self.rows
                        .iter()
                        .map(|row| row[column].clone())
                        .collect()
                })
                .collect(),
        )
    }

    /// Row echelon reduction to canonical reduced row echelon form.
    pub fn rref(&self, ctx: &EvaluationContext) -> Result<ReducedMatrix, MatrixError> {
        let mut rows = self.rows.clone();
        let pivots = eliminate(ctx, &mut rows, self.columns)?;
        Ok(ReducedMatrix {
            matrix: Self::new(ctx, rows)?,
            pivot_columns: pivots,
        })
    }

    pub fn rank(&self, ctx: &EvaluationContext) -> Result<usize, MatrixError> {
        Ok(self.rref(ctx)?.rank())
    }

    pub fn determinant(&self, ctx: &EvaluationContext) -> Result<Rational, MatrixError> {
        if self.row_count() != self.columns {
            return Err(MatrixError::NotSquare);
        }
        let n = self.row_count();
        let mut rows = self.rows.clone();
        let mut determinant = one();
        for column in 0..n {
            let Some(pivot) = (column..n)
                .find(|&row| !rows[row][column].numerator().is_zero())
            else {
                return Ok(zero());
            };
            if pivot != column {
                rows.swap(pivot, column);
                determinant = -determinant;
            }
            let pivot_value = rows[column][column].clone();
            determinant = &determinant * &pivot_value;
            ctx.check_rational(&determinant)?;
            let pivot_row = rows[column].clone();
            for row in rows.iter_mut().skip(column + 1) {
                if row[column].numerator().is_zero() {
                    continue;
                }
                let factor = divide(&row[column], &pivot_value);
                ctx.check_rational(&factor)?;
                for (entry, pivot_entry) in row[column..]
                    .iter_mut()
                    .zip(pivot_row[column..].iter())
                {
                    *entry = &*entry - &(&factor * pivot_entry);
                    ctx.check_rational(entry)?;
                }
            }
        }
        Ok(determinant)
    }

    /// Returns None for a singular matrix; verifies both inverse identities
    /// by independent exact multiplication before returning a candidate.
    pub fn inverse(&self, ctx: &EvaluationContext) -> Result<Option<Self>, MatrixError> {
        if self.row_count() != self.columns {
            return Err(MatrixError::NotSquare);
        }
        let n = self.columns;
        let mut augmented = self.rows.clone();
        for (i, row) in augmented.iter_mut().enumerate() {
            row.extend((0..n).map(|j| if i == j { one() } else { zero() }));
        }
        let pivots = eliminate(ctx, &mut augmented, n)?;
        if pivots.len() != n {
            return Ok(None);
        }
        let inverse = Self::new(
            ctx,
            augmented
                .into_iter()
                .map(|row| row[n..].to_vec())
                .collect(),
        )?;
        if !verify_matrix_inverse(ctx, self, &inverse)? {
            return Ok(None);
        }
        Ok(Some(inverse))
    }

    pub fn solve(
        &self,
        ctx: &EvaluationContext,
        rhs: &[Rational],
    ) -> Result<LinearSystemSolution, MatrixError> {
        if rhs.len() != self.row_count() {
            return Err(MatrixError::DimensionMismatch);
        }
        let mut augmented = self.rows.clone();
        for (row, value) in augmented.iter_mut().zip(rhs) {
            ctx.check_rational(value)?;
            row.push(value.clone());
        }
        let pivots = eliminate(ctx, &mut augmented, self.columns)?;
        if augmented.iter().any(|row| {
            row[..self.columns]
                .iter()
                .all(|value| value.numerator().is_zero())
                && !row[self.columns].numerator().is_zero()
        }) {
            return Ok(LinearSystemSolution::Inconsistent);
        }

        let mut particular = vec![zero(); self.columns];
        for (row, &column) in pivots.iter().enumerate() {
            particular[column] = augmented[row][self.columns].clone();
        }
        if pivots.len() == self.columns {
            return Ok(LinearSystemSolution::Unique(particular));
        }
        let mut basis = Vec::new();
        for free in (0..self.columns).filter(|index| !pivots.contains(index)) {
            let mut vector = vec![zero(); self.columns];
            vector[free] = one();
            for (row, &pivot) in pivots.iter().enumerate() {
                vector[pivot] = -&augmented[row][free];
            }
            basis.push(vector);
        }
        Ok(LinearSystemSolution::Infinite {
            particular,
            nullspace_basis: basis,
        })
    }
}

fn eliminate(
    ctx: &EvaluationContext,
    rows: &mut [Vec<Rational>],
    pivot_limit: usize,
) -> Result<Vec<usize>, MatrixError> {
    let mut pivots = Vec::new();
    let mut next_row = 0;
    for column in 0..pivot_limit {
        let Some(pivot) = (next_row..rows.len())
            .find(|&row| !rows[row][column].numerator().is_zero())
        else {
            continue;
        };
        rows.swap(next_row, pivot);
        let factor = divide(&one(), &rows[next_row][column]);
        ctx.check_rational(&factor)?;
        for value in &mut rows[next_row] {
            *value = &*value * &factor;
            ctx.check_rational(value)?;
        }
        let pivot_row = rows[next_row].clone();
        for (row_index, row) in rows.iter_mut().enumerate() {
            if row_index == next_row || row[column].numerator().is_zero() {
                continue;
            }
            let multiplier = row[column].clone();
            for (value, pivot_value) in row.iter_mut().zip(&pivot_row) {
                *value = &*value - &(&multiplier * pivot_value);
                ctx.check_rational(value)?;
            }
        }
        pivots.push(column);
        next_row += 1;
        if next_row == rows.len() {
            break;
        }
    }
    Ok(pivots)
}

/// Verify both A*A^-1=I and A^-1*A=I using exact matrix multiplication.
pub fn verify_matrix_inverse(
    ctx: &EvaluationContext,
    matrix: &ExactMatrix,
    candidate: &ExactMatrix,
) -> Result<bool, MatrixError> {
    if matrix.row_count() != matrix.columns
        || candidate.row_count() != matrix.columns
        || candidate.columns != matrix.columns
    {
        return Ok(false);
    }
    let identity = ExactMatrix::identity(ctx, matrix.columns)?;
    Ok(matrix.multiply(candidate, ctx)? == identity
        && candidate.multiply(matrix, ctx)? == identity)
}

/// Verifies the supplied vector satisfies Ax=b.
pub fn verify_linear_system_vector(
    ctx: &EvaluationContext,
    matrix: &ExactMatrix,
    rhs: &[Rational],
    vector: &[Rational],
) -> Result<bool, MatrixError> {
    if vector.len() != matrix.columns || rhs.len() != matrix.row_count() {
        return Ok(false);
    }
    for (row, target) in matrix.rows.iter().zip(rhs) {
        let mut sum = zero();
        for (entry, value) in row.iter().zip(vector) {
            sum = &sum + &(entry * value);
            ctx.check_rational(&sum)?;
        }
        if &sum != target {
            return Ok(false);
        }
    }
    Ok(true)
}
