// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
#![warn(missing_docs)]
//! Rank fusion: combine several ranked or scored lists into one ordered list, deterministically.
//!
//! One entry point, [`fuse::fuse`], takes a method and [`fuse::Options`]; [`fuse::rrf`], [`fuse::borda`],
//! [`fuse::weighted`], [`fuse::combsum`] and [`fuse::combmnz`] are logic-free two-list wrappers. See
//! [`fuse::RankMethod`] and [`fuse::ScoreMethod`] for the methods, and `docs/CONTRACT.md` for the decisions behind them.
//!
//! ```rust
//! use sortant::fuse::{Ranked, rrf};
//!
//! let keyword = Ranked::new(vec!["d1", "d2"]);
//! let dense = Ranked::new(vec!["d2", "d3"]);
//! let fused = rrf(&keyword, &dense).unwrap();
//! assert_eq!(fused.items[0].0, "d2"); // in both lists
//! ```

/// Compiles and runs the README's code blocks as doctests, so the README cannot drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub mod error;
pub mod fuse;
pub mod input;
pub mod norm;
pub mod order;
mod pairwise;
pub mod sort;

#[cfg(test)]
mod fuse_tests;
