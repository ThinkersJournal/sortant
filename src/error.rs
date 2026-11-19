// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! The one error type of the SortAnt API (contract D12).

use std::fmt;

/// What is wrong with a weight vector (see [`FuseError::InvalidWeights`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WeightProblem {
    /// The weight at `index` is NaN or infinite.
    NotFinite {
        /// Position of the offending weight.
        index: usize,
    },
    /// The weight at `index` is below zero.
    Negative {
        /// Position of the offending weight.
        index: usize,
    },
    /// All weights are zero, so there is nothing to weigh.
    ZeroSum,
}

/// Why a fusion request was refused. Nothing in SortAnt answers bad input with an empty result.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FuseError {
    /// A score is NaN or infinite (contract D9). `list` and `index` locate it.
    NonFiniteScore {
        /// Which input list.
        list: usize,
        /// Position inside that list.
        index: usize,
    },
    /// An id occurs twice in one list (contract D10). `index` is the position of the second occurrence.
    DuplicateId {
        /// Which input list.
        list: usize,
        /// Position of the repeated id.
        index: usize,
    },
    /// The weights are unusable (contract D13).
    InvalidWeights(WeightProblem),
    /// A configuration value is out of range (contract D11), e.g. an RRF `k` that is not finite and positive.
    InvalidConfig {
        /// Name of the parameter.
        parameter: &'static str,
        /// The value that was refused.
        value: f64,
    },
    /// Two things that must be the same length are not (e.g. weights versus lists).
    LengthMismatch {
        /// The length required.
        expected: usize,
        /// The length given.
        got: usize,
    },
    /// `max - min` of a list overflows to infinity, so it cannot be min-max normalised (contract A3).
    RangeOverflow {
        /// Which input list.
        list: usize,
    },
}

impl fmt::Display for FuseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteScore { list, index } => {
                write!(f, "score {index} of list {list} is NaN or infinite")
            }
            Self::DuplicateId { list, index } => {
                write!(
                    f,
                    "item {index} of list {list} repeats an id already in that list"
                )
            }
            Self::InvalidWeights(WeightProblem::NotFinite { index }) => {
                write!(f, "weight {index} is NaN or infinite")
            }
            Self::InvalidWeights(WeightProblem::Negative { index }) => {
                write!(f, "weight {index} is negative")
            }
            Self::InvalidWeights(WeightProblem::ZeroSum) => write!(f, "all weights are zero"),
            Self::InvalidConfig { parameter, value } => {
                write!(f, "configuration `{parameter}` = {value} is out of range")
            }
            Self::LengthMismatch { expected, got } => {
                write!(f, "expected length {expected}, got {got}")
            }
            Self::RangeOverflow { list } => {
                write!(f, "max - min of list {list} overflows, cannot normalise")
            }
        }
    }
}

impl std::error::Error for FuseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_every_variant_with_its_data() {
        let cases: Vec<(FuseError, &str)> = vec![
            (
                FuseError::NonFiniteScore { list: 1, index: 2 },
                "score 2 of list 1 is NaN or infinite",
            ),
            (
                FuseError::DuplicateId { list: 3, index: 4 },
                "item 4 of list 3 repeats an id already in that list",
            ),
            (
                FuseError::InvalidWeights(WeightProblem::NotFinite { index: 5 }),
                "weight 5 is NaN or infinite",
            ),
            (
                FuseError::InvalidWeights(WeightProblem::Negative { index: 6 }),
                "weight 6 is negative",
            ),
            (
                FuseError::InvalidWeights(WeightProblem::ZeroSum),
                "all weights are zero",
            ),
            (
                FuseError::InvalidConfig {
                    parameter: "k",
                    value: -1.5,
                },
                "configuration `k` = -1.5 is out of range",
            ),
            (
                FuseError::LengthMismatch {
                    expected: 2,
                    got: 3,
                },
                "expected length 2, got 3",
            ),
            (
                FuseError::RangeOverflow { list: 7 },
                "max - min of list 7 overflows, cannot normalise",
            ),
        ];
        for (e, text) in cases {
            assert_eq!(e.to_string(), text);
        }
    }

    #[test]
    fn is_a_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(FuseError::RangeOverflow { list: 0 });
        assert!(e.source().is_none());
    }
}
