// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
// Derived from rankops 0.3.0 (Copyright (c) 2025 Arc, MIT OR Apache-2.0), src/lib.rs lines 1-3346.
// Modified: crate renamed; dropped the rerank/metrics/pipeline/adapt/diagnostics/trec/dp_topk modules, the metrics
// prelude, the serde derives and the unverified OpenSearch benchmark claim; rustfmt (edition 2024) reformatting and two
// clippy `collapsible_if` rewrites (no behaviour change intended). Otherwise unchanged. See PROVENANCE.md.
#![warn(missing_docs)]
//! Rank fusion: combine several ranked lists into one decision.
//!
//! Work in progress. This is the stripped, not yet repaired, fusion core of `rankops` 0.3.0
//! (see `PROVENANCE.md`); behaviour is rankops' until each method is ported under `docs/CONTRACT.md`.
//!
//! ```rust
//! use sortant::rrf;
//!
//! let bm25 = vec![("d1", 12.5), ("d2", 11.0)];
//! let dense = vec![("d2", 0.9), ("d3", 0.8)];
//! let fused = rrf(&bm25, &dense);
//! // d2 ranks highest (appears in both lists)
//! ```
//!
//! # Fusion Algorithms
//!
//! | Function | Uses Scores | Best For |
//! |----------|-------------|----------|
//! | [`rrf`] | No | Incompatible score scales |
//! | [`isr`] | No | When lower ranks matter more |
//! | [`combsum`] | Yes | Similar scales, trust scores |
//! | [`combmnz`] | Yes | Reward overlap between lists |
//! | [`borda`] | No | Simple voting |
//! | [`weighted`] | Yes | Custom retriever weights |
//! | [`dbsf`] | Yes | Different score distributions |
//! | [`condorcet`] | No | Pairwise voting, outlier-robust |
//! | [`copeland`] | No | Net pairwise wins, more discriminative than Condorcet |
//! | [`median_rank`] | No | Median rank across lists, outlier-robust |
//! | [`combmax`] | Yes | At least one retriever likes it |
//! | [`combmin`] | Yes | All retrievers must agree (conservative) |
//! | [`combmed`] | Yes | Median score, robust to outliers |
//!
//! All have `*_multi` variants for 3+ lists.

use std::collections::HashMap;
use std::hash::Hash;

/// Output validators for fusion results.
pub mod validate;

mod fusion_order;

#[cfg(test)]
mod proptests;

// ─────────────────────────────────────────────────────────────────────────────
// Error Types
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during fusion.
///
/// Match this enum with a wildcard arm so new fusion errors can be added.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FusionError {
    /// Weights sum to zero or near-zero.
    ZeroWeights,
    /// Invalid configuration parameter.
    InvalidConfig(String),
}

impl std::fmt::Display for FusionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroWeights => write!(f, "weights sum to zero"),
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
        }
    }
}

impl std::error::Error for FusionError {}

/// Result type for fusion operations.
pub type Result<T> = std::result::Result<T, FusionError>;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration with Builder Pattern
// ─────────────────────────────────────────────────────────────────────────────

/// Threshold for treating weight sum as effectively zero.
///
/// Used in weighted fusion to detect invalid configurations where all weights
/// are zero or near-zero, which would cause division by zero.
const WEIGHT_EPSILON: f32 = 1e-9;

/// Threshold for treating score range as effectively zero (all scores equal).
///
/// Used in min-max normalization to detect degenerate cases where all scores
/// are identical, avoiding division by zero.
const SCORE_RANGE_EPSILON: f32 = 1e-9;

/// RRF configuration.
///
/// # Example
///
/// ```rust
/// use sortant::RrfConfig;
///
/// let config = RrfConfig::default()
///     .with_k(60)
///     .with_top_k(10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RrfConfig {
    /// Smoothing constant (default: 60).
    ///
    /// **Must be >= 1** to avoid division by zero in the RRF formula.
    /// Values < 1 will cause panics during fusion.
    pub k: u32,
    /// Maximum results to return (None = all).
    pub top_k: Option<usize>,
}

impl Default for RrfConfig {
    fn default() -> Self {
        Self { k: 60, top_k: None }
    }
}

impl RrfConfig {
    /// Create config with custom k.
    ///
    /// # Panics
    ///
    /// Panics if `k == 0` (would cause division by zero in RRF formula).
    ///
    /// # Example
    ///
    /// ```rust
    /// use sortant::RrfConfig;
    ///
    /// let config = RrfConfig::new(60);
    /// ```
    #[must_use]
    pub fn new(k: u32) -> Self {
        assert!(
            k >= 1,
            "k must be >= 1 to avoid division by zero in RRF formula"
        );
        Self { k, top_k: None }
    }

    /// Set the k parameter (smoothing constant).
    ///
    /// - `k=60` — Standard RRF, works well for most cases
    /// - `k=1` — Top positions dominate heavily
    /// - `k=100+` — More uniform contribution across ranks
    ///
    /// # Panics
    ///
    /// Panics if `k == 0` (would cause division by zero in RRF formula).
    #[must_use]
    pub fn with_k(mut self, k: u32) -> Self {
        assert!(
            k >= 1,
            "k must be >= 1 to avoid division by zero in RRF formula"
        );
        self.k = k;
        self
    }

    /// Limit output to `top_k` results.
    #[must_use]
    pub const fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

/// Weighted fusion configuration.
///
/// # Example
///
/// ```rust
/// use sortant::WeightedConfig;
///
/// let config = WeightedConfig::default()
///     .with_weights(0.7, 0.3)
///     .with_normalize(true)
///     .with_top_k(10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedConfig {
    /// Weight for first list (default: 0.5).
    pub weight_a: f32,
    /// Weight for second list (default: 0.5).
    pub weight_b: f32,
    /// Normalize scores to `[0,1]` before combining (default: true).
    pub normalize: bool,
    /// Maximum results to return (None = all).
    pub top_k: Option<usize>,
}

impl Default for WeightedConfig {
    fn default() -> Self {
        Self {
            weight_a: 0.5,
            weight_b: 0.5,
            normalize: true,
            top_k: None,
        }
    }
}

impl WeightedConfig {
    /// Create config with custom weights.
    #[must_use]
    pub const fn new(weight_a: f32, weight_b: f32) -> Self {
        Self {
            weight_a,
            weight_b,
            normalize: true,
            top_k: None,
        }
    }

    /// Set weights for the two lists.
    #[must_use]
    pub const fn with_weights(mut self, weight_a: f32, weight_b: f32) -> Self {
        self.weight_a = weight_a;
        self.weight_b = weight_b;
        self
    }

    /// Enable/disable score normalization.
    #[must_use]
    pub const fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    /// Limit output to `top_k` results.
    #[must_use]
    pub const fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

/// Configuration for rank-based fusion (Borda, `CombSUM`, `CombMNZ`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FusionConfig {
    /// Maximum results to return (None = all).
    pub top_k: Option<usize>,
}

impl FusionConfig {
    /// Limit output to `top_k` results.
    #[must_use]
    pub const fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Prelude
// ─────────────────────────────────────────────────────────────────────────────

/// Prelude for common imports.
///
/// ```rust
/// use sortant::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        AdditiveMultiTaskConfig, FusionConfig, FusionError, FusionMethod, Normalization, Result,
        RrfConfig, StandardizedConfig, WeightedConfig,
    };
    pub use crate::{
        additive_multi_task, additive_multi_task_multi, additive_multi_task_with_config, borda,
        combanz, combmax, combmed, combmnz, combsum, condorcet, copeland, dbsf, isr,
        isr_with_config, median_rank, rrf, rrf_with_config, standardized, standardized_multi,
        standardized_with_config, weighted,
    };
}

/// Explainability module for debugging and analysis.
///
/// Provides variants of fusion functions that return full provenance information,
/// showing which retrievers contributed each document and how scores were computed.
pub mod explain {
    pub use crate::{
        ConsensusReport, Explanation, FusedResult, RetrieverId, RetrieverStats, SourceContribution,
        analyze_consensus, attribute_top_k, combmnz_explain, combsum_explain, dbsf_explain,
        rrf_explain,
    };
}

/// Validation module for fusion results.
///
/// Provides utilities to validate fusion results, ensuring they meet expected
/// properties (sorted, no duplicates, finite scores, etc.).
///
/// Re-exports all validation functions from the internal validate module.
pub use validate::{
    ValidationResult, validate, validate_bounds, validate_finite_scores, validate_no_duplicates,
    validate_non_negative_scores, validate_sorted,
};

// ─────────────────────────────────────────────────────────────────────────────
// Unified Fusion Method
// ─────────────────────────────────────────────────────────────────────────────

/// Unified fusion method for dispatching to different algorithms.
///
/// Provides a single entry point for all fusion algorithms with a consistent API.
///
/// # Example
///
/// ```rust
/// use sortant::FusionMethod;
///
/// let sparse = vec![("d1", 10.0), ("d2", 8.0)];
/// let dense = vec![("d2", 0.9), ("d3", 0.7)];
///
/// // Use RRF (rank-based, score-agnostic)
/// let fused = FusionMethod::Rrf { k: 60 }.fuse(&sparse, &dense);
///
/// // Use CombSUM (score-based)
/// let fused = FusionMethod::CombSum.fuse(&sparse, &dense);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FusionMethod {
    /// Reciprocal Rank Fusion (ignores scores, uses rank position).
    Rrf {
        /// Smoothing constant (default: 60).
        k: u32,
    },
    /// Inverse Square Root rank fusion (gentler decay than RRF). Not the
    /// inverse square rank "ISR" of Mourão et al. 2014; see [`isr`].
    Isr {
        /// Smoothing constant (default: 1).
        k: u32,
    },
    /// `CombSUM` — sum of normalized scores.
    CombSum,
    /// `CombMNZ` — sum × overlap count.
    CombMnz,
    /// Borda count — N - rank points.
    Borda,
    /// Condorcet — pairwise majority wins.
    Condorcet,
    /// Copeland — pairwise net wins (wins - losses). More discriminative than Condorcet.
    Copeland,
    /// Median Rank Aggregation — median rank across lists. Outlier-robust.
    MedianRank,
    /// `CombMAX` — maximum score across lists.
    CombMax,
    /// `CombMIN` — minimum score across lists (conservative).
    CombMin,
    /// `CombMED` — median score across lists.
    CombMed,
    /// `CombANZ` — average of non-zero scores.
    CombAnz,
    /// Rank-Biased Centroids with configurable persistence.
    Rbc {
        /// Persistence parameter (default: 0.8). Higher = less top-heavy.
        persistence: f32,
    },
    /// Weighted combination with custom weights.
    Weighted {
        /// Weight for first list.
        weight_a: f32,
        /// Weight for second list.
        weight_b: f32,
        /// Whether to normalize scores before combining.
        normalize: bool,
    },
    /// Distribution-Based Score Fusion (z-score normalization).
    Dbsf,
    /// Standardization-based fusion (z-score).
    ///
    /// Uses z-score normalization (standardization) instead of min-max normalization,
    /// then applies additive fusion. More robust to outliers and different score distributions.
    Standardized {
        /// Clip z-scores to this range (default: [-3.0, 3.0]).
        clip_range: (f32, f32),
    },
    /// Additive multi-task fusion (ResFlow-style).
    ///
    /// Additive fusion of multi-task scores: `α·score_a + β·score_b`.
    /// ResFlow (arXiv:2411.09705) shows additive outperforms multiplicative for e-commerce.
    AdditiveMultiTask {
        /// Weight for first task.
        weight_a: f32,
        /// Weight for second task.
        weight_b: f32,
        /// Normalization method (default: ZScore for robustness).
        normalization: Normalization,
    },
}

impl Default for FusionMethod {
    fn default() -> Self {
        Self::Rrf { k: 60 }
    }
}

impl FusionMethod {
    /// Create RRF method with default k=60.
    #[must_use]
    pub const fn rrf() -> Self {
        Self::Rrf { k: 60 }
    }

    /// Create RRF method with custom k.
    #[must_use]
    pub const fn rrf_with_k(k: u32) -> Self {
        Self::Rrf { k }
    }

    /// Create ISR method with default k=1.
    #[must_use]
    pub const fn isr() -> Self {
        Self::Isr { k: 1 }
    }

    /// Create ISR method with custom k.
    #[must_use]
    pub const fn isr_with_k(k: u32) -> Self {
        Self::Isr { k }
    }

    /// Create RBC method with default persistence 0.8.
    #[must_use]
    pub const fn rbc() -> Self {
        Self::Rbc { persistence: 0.8 }
    }

    /// Create RBC method with custom persistence.
    #[must_use]
    pub const fn rbc_with_persistence(persistence: f32) -> Self {
        Self::Rbc { persistence }
    }

    /// Create weighted method with custom weights.
    #[must_use]
    pub const fn weighted(weight_a: f32, weight_b: f32) -> Self {
        Self::Weighted {
            weight_a,
            weight_b,
            normalize: true,
        }
    }

    /// Create standardized fusion method (z-score).
    ///
    /// Uses z-score normalization (standardization) with clipping to prevent outliers.
    /// More robust than min-max when score distributions differ significantly.
    #[must_use]
    pub const fn standardized(clip_range: (f32, f32)) -> Self {
        Self::Standardized { clip_range }
    }

    /// Create standardized fusion method with default clipping [-3.0, 3.0].
    #[must_use]
    pub const fn standardized_default() -> Self {
        Self::Standardized {
            clip_range: (-3.0, 3.0),
        }
    }

    /// Create additive multi-task fusion method (ResFlow-style).
    ///
    /// Additive fusion outperforms multiplicative for e-commerce ranking.
    /// ResFlow's optimal formula: `CTR + CTCVR × 20`.
    #[must_use]
    pub const fn additive_multi_task(weight_a: f32, weight_b: f32) -> Self {
        Self::AdditiveMultiTask {
            weight_a,
            weight_b,
            normalization: Normalization::ZScore,
        }
    }

    /// Create additive multi-task fusion with custom normalization.
    #[must_use]
    pub fn additive_multi_task_with_norm(
        weight_a: f32,
        weight_b: f32,
        normalization: Normalization,
    ) -> Self {
        Self::AdditiveMultiTask {
            weight_a,
            weight_b,
            normalization,
        }
    }

    /// Fuse two ranked lists using this method.
    ///
    /// # Arguments
    /// * `a` - First ranked list (ID, score pairs)
    /// * `b` - Second ranked list (ID, score pairs)
    ///
    /// # Returns
    /// Combined list sorted by fused score (descending)
    #[must_use]
    pub fn fuse<I: Clone + Eq + Hash>(&self, a: &[(I, f32)], b: &[(I, f32)]) -> Vec<(I, f32)> {
        match self {
            Self::Rrf { k } => {
                // Validate k at use time to avoid panics from invalid FusionMethod construction
                if *k == 0 {
                    return Vec::new();
                }
                crate::rrf_multi(&[a, b], RrfConfig::new(*k))
            }
            Self::Isr { k } => {
                if *k == 0 {
                    return Vec::new();
                }
                crate::isr_multi(&[a, b], RrfConfig::new(*k))
            }
            Self::CombSum => crate::combsum(a, b),
            Self::CombMnz => crate::combmnz(a, b),
            Self::Borda => crate::borda(a, b),
            Self::Condorcet => crate::condorcet(a, b),
            Self::Copeland => crate::copeland(a, b),
            Self::MedianRank => crate::median_rank(a, b),
            Self::CombMax => crate::combmax(a, b),
            Self::CombMin => crate::combmin(a, b),
            Self::CombMed => crate::combmed(a, b),
            Self::CombAnz => crate::combanz(a, b),
            Self::Rbc { persistence } => crate::rbc_multi(&[a, b], *persistence),
            Self::Weighted {
                weight_a,
                weight_b,
                normalize,
            } => crate::weighted(
                a,
                b,
                WeightedConfig::new(*weight_a, *weight_b).with_normalize(*normalize),
            ),
            Self::Dbsf => crate::dbsf(a, b),
            Self::Standardized { clip_range } => {
                crate::standardized_with_config(a, b, StandardizedConfig::new(*clip_range))
            }
            Self::AdditiveMultiTask {
                weight_a,
                weight_b,
                normalization,
            } => crate::additive_multi_task_with_config(
                a,
                b,
                AdditiveMultiTaskConfig::new((*weight_a, *weight_b))
                    .with_normalization(*normalization),
            ),
        }
    }

    /// Fuse multiple ranked lists using this method.
    ///
    /// # Arguments
    /// * `lists` - Slice of ranked lists
    ///
    /// # Returns
    /// Combined list sorted by fused score (descending)
    #[must_use]
    pub fn fuse_multi<I, L>(&self, lists: &[L]) -> Vec<(I, f32)>
    where
        I: Clone + Eq + Hash,
        L: AsRef<[(I, f32)]>,
    {
        match self {
            Self::Rrf { k } => {
                if *k == 0 {
                    return Vec::new();
                }
                crate::rrf_multi(lists, RrfConfig::new(*k))
            }
            Self::Isr { k } => {
                if *k == 0 {
                    return Vec::new();
                }
                crate::isr_multi(lists, RrfConfig::new(*k))
            }
            Self::CombSum => crate::combsum_multi(lists, FusionConfig::default()),
            Self::CombMnz => crate::combmnz_multi(lists, FusionConfig::default()),
            Self::Borda => crate::borda_multi(lists, FusionConfig::default()),
            Self::Condorcet => crate::condorcet_multi(lists, FusionConfig::default()),
            Self::Copeland => crate::copeland_multi(lists, FusionConfig::default()),
            Self::MedianRank => crate::median_rank_multi(lists, FusionConfig::default()),
            Self::CombMax => crate::combmax_multi(lists, FusionConfig::default()),
            Self::CombMin => crate::combmin_multi(lists, FusionConfig::default()),
            Self::CombMed => crate::combmed_multi(lists, FusionConfig::default()),
            Self::CombAnz => crate::combanz_multi(lists, FusionConfig::default()),
            Self::Rbc { persistence } => crate::rbc_multi(lists, *persistence),
            Self::Weighted { normalize, .. } => {
                if lists.len() == 2 {
                    self.fuse(lists[0].as_ref(), lists[1].as_ref())
                } else {
                    // Equal weights for 3+ lists
                    let weighted_lists: Vec<_> = lists
                        .iter()
                        .map(|l| (l.as_ref(), 1.0 / lists.len() as f32))
                        .collect();
                    crate::weighted_multi(&weighted_lists, *normalize, None).unwrap_or_default()
                }
            }
            Self::Dbsf => crate::dbsf_multi(lists, FusionConfig::default()),
            Self::Standardized { clip_range } => {
                crate::standardized_multi(lists, StandardizedConfig::new(*clip_range))
            }
            Self::AdditiveMultiTask {
                weight_a,
                weight_b,
                normalization,
            } => {
                // For multi-list, use equal weights (users should use additive_multi_task_multi directly)
                if lists.len() == 2 {
                    self.fuse(lists[0].as_ref(), lists[1].as_ref())
                } else {
                    // For 3+ lists, convert to weighted lists format
                    let weighted_lists: Vec<_> = lists
                        .iter()
                        .map(|l| (l.as_ref(), 1.0 / lists.len() as f32))
                        .collect();
                    crate::additive_multi_task_multi(
                        &weighted_lists,
                        AdditiveMultiTaskConfig::new((*weight_a, *weight_b))
                            .with_normalization(*normalization),
                    )
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RRF (Reciprocal Rank Fusion)
// ─────────────────────────────────────────────────────────────────────────────

#[must_use]
/// Reciprocal Rank Fusion (RRF) with default configuration (k=60).
///
/// RRF is the recommended fusion method when combining rankings with incompatible
/// score scales. It uses only rank positions, ignoring score magnitudes entirely.
///
/// # Formula
///
/// `RRF(d) = Σ 1/(k + rank_r(d))` where:
/// - `k` = smoothing constant (default: 60)
/// - `rank_r(d)` = position of document d in ranking r (1-indexed: the top
///   document has rank 1, as in Cormack, Clarke & Büttcher 2009)
///
/// # Why RRF?
///
/// - **Score Scale Independent**: Works with any scoring system (BM25: 0-100, embeddings: 0-1)
/// - **Robust**: Handles missing documents gracefully (documents not in a list contribute 0)
/// - **Effective**: Proven to outperform individual rankers in hybrid search
/// - **Fast**: O(n log n) complexity where n = total unique documents
///
/// # Arguments
///
/// * `results_a` - First ranked list: `Vec<(document_id, score)>`
/// * `results_b` - Second ranked list: `Vec<(document_id, score)>`
///
/// Note: Scores are ignored; only rank positions matter.
///
/// # Returns
///
/// Fused ranking sorted by RRF score (descending). Documents appearing in both
/// lists rank higher than those appearing in only one list.
///
/// # Example
///
/// ```rust
/// use sortant::rrf;
///
/// // BM25 results (high scores = better)
/// let bm25 = vec![
///     ("doc1", 12.5),
///     ("doc2", 11.0),
///     ("doc3", 10.0),
/// ];
///
/// // Dense embedding results (different scale: 0-1)
/// let dense = vec![
///     ("doc2", 0.9),
///     ("doc3", 0.8),
///     ("doc1", 0.7),
/// ];
///
/// // RRF ignores scores, uses only rank positions
/// let fused = rrf(&bm25, &dense);
/// // doc2 ranks highest (appears in both lists at high positions)
/// // doc1 and doc3 follow (appear in both lists but at different positions)
/// ```
///
/// # Performance
///
/// Time complexity: O(n log n) where n = |results_a| + |results_b| (dominated by final sort).
/// For typical workloads (100-1000 items per list), fusion completes in <1ms.
///
/// # When to Use
///
/// - ✅ Combining BM25 + dense embeddings (different scales)
/// - ✅ Combining sparse + dense retrieval
/// - ✅ Unknown or incompatible score scales
/// - ✅ Need robust fusion without normalization tuning
///
/// # When NOT to Use
///
/// - ❌ Scores are already normalized and comparable (use `combsum` or `weighted`)
/// - ❌ You trust score magnitudes (use score-based fusion)
/// - ❌ Need fine-grained control over retriever importance (use `rrf_weighted`)
pub fn rrf<I: Clone + Eq + Hash>(results_a: &[(I, f32)], results_b: &[(I, f32)]) -> Vec<(I, f32)> {
    rrf_with_config(results_a, results_b, RrfConfig::default())
}

/// RRF with custom configuration.
///
/// Use this when you need to tune the k parameter:
/// - **k=20-40**: Top positions dominate more. Use when top retrievers are highly reliable.
/// - **k=60**: Default (empirically chosen by Cormack et al., 2009). Balanced for most scenarios.
/// - **k=100+**: More uniform contribution. Use when lower-ranked items are still valuable.
///
/// **Sensitivity**: k=10 gives 1.5x ratio (rank 0 vs rank 5), k=60 gives 1.1x, k=100 gives 1.05x.
///
/// # Example
///
/// ```rust
/// use sortant::{rrf_with_config, RrfConfig};
///
/// let a = vec![("d1", 0.9), ("d2", 0.5)];
/// let b = vec![("d2", 0.8), ("d3", 0.3)];
///
/// // k=20: emphasize top positions (strong consensus required)
/// let fused = rrf_with_config(&a, &b, RrfConfig::new(20));
/// ```
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn rrf_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: RrfConfig,
) -> Vec<(I, f32)> {
    // Validate k >= 1 to avoid division by zero (k=0 would cause 1/0 for rank 0)
    if config.k == 0 {
        return Vec::new();
    }
    let k = config.k as f32;
    // Pre-allocate capacity to avoid reallocations during insertion
    let estimated_size = results_a.len() + results_b.len();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    // Use get_mut + insert pattern to avoid cloning IDs when entry already exists
    for (rank, (id, _)) in results_a.iter().enumerate() {
        let contribution = 1.0 / (k + (rank + 1) as f32);
        if let Some(score) = scores.get_mut(id) {
            *score += contribution;
        } else {
            scores.insert(id.clone(), contribution);
        }
    }
    for (rank, (id, _)) in results_b.iter().enumerate() {
        let contribution = 1.0 / (k + (rank + 1) as f32);
        if let Some(score) = scores.get_mut(id) {
            *score += contribution;
        } else {
            scores.insert(id.clone(), contribution);
        }
    }

    finalize(scores, config.top_k)
}

/// RRF for 3+ result lists.
///
/// # Empty Lists
///
/// If `lists` is empty, returns an empty result. If some lists are empty,
/// they contribute zero scores (documents not appearing in those lists
/// receive no contribution from them).
///
/// # Complexity
///
/// O(L×N + U×log U) where L = number of lists, N = average list size,
/// U = number of unique document IDs across all lists.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn rrf_multi<I, L>(lists: &[L], config: RrfConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    // Validate k >= 1 to avoid division by zero
    if config.k == 0 {
        return Vec::new();
    }
    let k = config.k as f32;
    // Estimate capacity: sum of all list sizes (may overestimate due to duplicates)
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    // Use get_mut + insert pattern to avoid cloning IDs when entry already exists
    for list in lists {
        for (rank, (id, _)) in list.as_ref().iter().enumerate() {
            let contribution = 1.0 / (k + (rank + 1) as f32);
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    finalize(scores, config.top_k)
}

/// Weighted RRF: per-retriever weights applied to rank-based scores.
///
/// Unlike standard RRF which treats all lists equally, weighted RRF allows
/// assigning different importance to different retrievers based on domain
/// knowledge or tuning.
///
/// Formula: `score(d) = Σ w_i / (k + rank_i(d))`, with 1-indexed ranks as in [`rrf`].
///
/// # Example
///
/// ```rust
/// use sortant::{rrf_weighted, RrfConfig};
///
/// let bm25 = vec![("d1", 0.0), ("d2", 0.0)];   // scores ignored
/// let dense = vec![("d2", 0.0), ("d3", 0.0)];
///
/// // Trust dense retriever 2x more than BM25
/// let weights = [0.33, 0.67];
/// let fused = rrf_weighted(&[&bm25[..], &dense[..]], &weights, RrfConfig::default());
/// ```
///
/// # Errors
///
/// - Returns [`FusionError::ZeroWeights`] if weights sum to zero.
/// - Returns [`FusionError::InvalidConfig`] if `lists.len() != weights.len()`.
#[allow(clippy::cast_precision_loss)]
pub fn rrf_weighted<I, L>(lists: &[L], weights: &[f32], config: RrfConfig) -> Result<Vec<(I, f32)>>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.len() != weights.len() {
        return Err(FusionError::InvalidConfig(format!(
            "lists.len() ({}) != weights.len() ({}). Each list must have a corresponding weight.",
            lists.len(),
            weights.len()
        )));
    }
    let weight_sum: f32 = weights.iter().sum();
    if weight_sum.abs() < WEIGHT_EPSILON {
        return Err(FusionError::ZeroWeights);
    }

    let k = config.k as f32;
    // Pre-allocate capacity
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for (list, &weight) in lists.iter().zip(weights.iter()) {
        let normalized_weight = weight / weight_sum;
        for (rank, (id, _)) in list.as_ref().iter().enumerate() {
            let contribution = normalized_weight / (k + (rank + 1) as f32);
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    Ok(finalize(scores, config.top_k))
}

// ─────────────────────────────────────────────────────────────────────────────
// ISR (Inverse Square Root Rank)
// ─────────────────────────────────────────────────────────────────────────────

/// Inverse Square Root rank fusion with default config (k=1).
///
/// ISR uses a gentler decay than RRF, giving lower-ranked documents more
/// relative contribution compared to top positions. This makes it useful when
/// you believe relevant documents may appear deeper in the ranking lists.
///
/// # Formula
///
/// `score(d) = Σ 1/sqrt(k + rank)` where rank is 0-indexed.
///
/// Naming: here "ISR" means inverse square *root*. It is not the "ISR"
/// (inverse square *rank*, `|M_d| · Σ 1/rank²`) of Mourão et al. 2014, which
/// decays faster than RRF rather than slower.
///
/// Compared to RRF's `1/(k + rank)`, ISR's `1/sqrt(k + rank)` decays more slowly,
/// meaning rank 10 vs rank 20 has a smaller relative difference than in RRF.
///
/// # Arguments
///
/// * `results_a` - First ranked list (scores ignored, only positions matter)
/// * `results_b` - Second ranked list (scores ignored, only positions matter)
///
/// # Returns
///
/// Fused ranking sorted by ISR score (descending). Documents appearing in both
/// lists rank higher, with less emphasis on exact position than RRF.
///
/// # Example
///
/// ```rust
/// use sortant::isr;
///
/// let sparse = vec![("d1", 0.9), ("d2", 0.5), ("d3", 0.3)];
/// let dense = vec![("d2", 0.8), ("d3", 0.7), ("d4", 0.2)];
///
/// let fused = isr(&sparse, &dense);
/// // d2 and d3 appear in both lists, so they rank highest
/// // ISR gives more weight to lower positions than RRF
/// ```
///
/// # Performance
///
/// Time complexity: O(n log n) where n = |results_a| + |results_b| (dominated by final sort).
/// For typical workloads (100-1000 items per list), fusion completes in <1ms.
///
/// # When to Use
///
/// - ✅ Relevant documents may appear deeper in lists (rank 20-50 still valuable)
/// - ✅ Want gentler position-based decay than RRF
/// - ✅ Combining retrievers where position uncertainty is high
///
/// # When NOT to Use
///
/// - ❌ Top positions are highly reliable (use RRF with k=20-40)
/// - ❌ Need score-based fusion (use `combsum` or `weighted`)
/// - ❌ Unknown score scales but want standard decay (use RRF)
///
/// # Trade-offs vs RRF
///
/// - **Decay**: Gentler (lower ranks contribute more)
/// - **Top emphasis**: Less emphasis on exact top positions
/// - **Use case**: Better for noisy retrievers or when depth matters
#[must_use]
pub fn isr<I: Clone + Eq + Hash>(results_a: &[(I, f32)], results_b: &[(I, f32)]) -> Vec<(I, f32)> {
    isr_with_config(results_a, results_b, RrfConfig::new(1))
}

/// ISR with custom configuration.
///
/// The k parameter controls decay steepness:
/// - Lower k (e.g., 1): Top positions dominate more
/// - Higher k (e.g., 10): More uniform contribution across positions
///
/// # Example
///
/// ```rust
/// use sortant::{isr_with_config, RrfConfig};
///
/// let a = vec![("d1", 0.9), ("d2", 0.5)];
/// let b = vec![("d2", 0.8), ("d3", 0.3)];
///
/// let fused = isr_with_config(&a, &b, RrfConfig::new(1));
/// ```
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn isr_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: RrfConfig,
) -> Vec<(I, f32)> {
    // Validate k >= 1 to avoid division by zero (k=0 would cause 1/0 for rank 0)
    if config.k == 0 {
        return Vec::new();
    }
    let k = config.k as f32;
    let estimated_size = results_a.len() + results_b.len();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    // Use get_mut + insert pattern to avoid cloning IDs when entry already exists
    for (rank, (id, _)) in results_a.iter().enumerate() {
        let contribution = 1.0 / (k + rank as f32).sqrt();
        if let Some(score) = scores.get_mut(id) {
            *score += contribution;
        } else {
            scores.insert(id.clone(), contribution);
        }
    }
    for (rank, (id, _)) in results_b.iter().enumerate() {
        let contribution = 1.0 / (k + rank as f32).sqrt();
        if let Some(score) = scores.get_mut(id) {
            *score += contribution;
        } else {
            scores.insert(id.clone(), contribution);
        }
    }

    finalize(scores, config.top_k)
}

/// ISR for 3+ result lists.
///
/// # Invalid Configuration
///
/// If `config.k == 0`, returns an empty result to avoid division by zero.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn isr_multi<I, L>(lists: &[L], config: RrfConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    // Validate k >= 1 to avoid division by zero
    if config.k == 0 {
        return Vec::new();
    }
    let k = config.k as f32;
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for list in lists {
        // Use get_mut + insert pattern to avoid cloning IDs when entry already exists
        for (rank, (id, _)) in list.as_ref().iter().enumerate() {
            let contribution = 1.0 / (k + rank as f32).sqrt();
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    finalize(scores, config.top_k)
}

// ─────────────────────────────────────────────────────────────────────────────
// Score-based Fusion
// ─────────────────────────────────────────────────────────────────────────────

/// Weighted score fusion with configurable retriever trust.
///
/// Formula: `score(d) = w_a × norm(s_a) + w_b × norm(s_b)`
///
/// Use when you know one retriever is more reliable for your domain.
/// Weights are normalized to sum to 1.
///
/// # Complexity
///
/// O(n log n) where n = total items across all lists.
#[must_use]
pub fn weighted<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: WeightedConfig,
) -> Vec<(I, f32)> {
    weighted_impl(
        &[(results_a, config.weight_a), (results_b, config.weight_b)],
        config.normalize,
        config.top_k,
    )
}

/// Weighted fusion for 3+ result lists.
///
/// Each list is paired with its weight. Weights are normalized to sum to 1.
///
/// # Errors
///
/// Returns `Err(FusionError::ZeroWeights)` if weights sum to zero.
pub fn weighted_multi<I, L>(
    lists: &[(L, f32)],
    normalize: bool,
    top_k: Option<usize>,
) -> Result<Vec<(I, f32)>>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    let total_weight: f32 = lists.iter().map(|(_, w)| w).sum();
    if total_weight.abs() < WEIGHT_EPSILON {
        return Err(FusionError::ZeroWeights);
    }

    let estimated_size: usize = lists.iter().map(|(l, _)| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for (list, weight) in lists {
        let items = list.as_ref();
        let w = weight / total_weight;
        let (norm, off) = if normalize {
            min_max_params(items)
        } else {
            (1.0, 0.0)
        };
        for (id, s) in items {
            let contribution = w * (s - off) * norm;
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    Ok(finalize(scores, top_k))
}

/// Internal weighted implementation (infallible for two-list case).
fn weighted_impl<I, L>(lists: &[(L, f32)], normalize: bool, top_k: Option<usize>) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    let total_weight: f32 = lists.iter().map(|(_, w)| w).sum();
    if total_weight.abs() < WEIGHT_EPSILON {
        return Vec::new();
    }

    let estimated_size: usize = lists.iter().map(|(l, _)| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for (list, weight) in lists {
        let items = list.as_ref();
        let w = weight / total_weight;
        let (norm, off) = if normalize {
            min_max_params(items)
        } else {
            (1.0, 0.0)
        };
        for (id, s) in items {
            let contribution = w * (s - off) * norm;
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    finalize(scores, top_k)
}

/// Sum of min-max normalized scores (CombSUM).
///
/// CombSUM normalizes each list to [0, 1] using min-max normalization, then sums
/// the normalized scores. This preserves score magnitudes while handling different scales.
///
/// # Formula
///
/// For each list: `normalized = (score - min) / (max - min)`
/// Final score: `score(d) = Σ normalized_scores(d)`
///
/// # Arguments
///
/// * `results_a` - First ranked list: `Vec<(document_id, score)>`
/// * `results_b` - Second ranked list: `Vec<(document_id, score)>`
///
/// # Returns
///
/// Fused ranking sorted by combined score (descending). Documents with higher
/// normalized scores across lists rank higher.
///
/// # Example
///
/// ```rust
/// use sortant::combsum;
///
/// // Both lists use cosine similarity (0-1 scale)
/// let sparse = vec![
///     ("doc1", 0.9),
///     ("doc2", 0.8),
///     ("doc3", 0.7),
/// ];
///
/// let dense = vec![
///     ("doc2", 0.95),
///     ("doc1", 0.85),
///     ("doc3", 0.75),
/// ];
///
/// let fused = combsum(&sparse, &dense);
/// // doc2 ranks highest (0.8 + 0.95 = 1.75 after normalization)
/// ```
///
/// # Performance
///
/// Time complexity: O(n log n) where n = total items across all lists.
/// For typical workloads (100-1000 items per list), fusion completes in <1ms.
///
/// # When to Use
///
/// - ✅ Scores are on similar scales (e.g., all cosine similarities 0-1)
/// - ✅ You trust score magnitudes (scores represent true relevance)
/// - ✅ Need better accuracy than RRF (CombSUM typically 3-4% higher NDCG)
///
/// # When NOT to Use
///
/// - ❌ Incompatible score scales (BM25: 0-100 vs embeddings: 0-1) - use RRF
/// - ❌ Score distributions differ significantly - use `standardized` or `dbsf`
/// - ❌ Unknown score scales - use RRF
///
/// # Trade-offs
///
/// - **Accuracy**: Typically 3-4% higher NDCG than RRF (OpenSearch benchmarks)
/// - **Robustness**: Less robust to outliers than RRF (min-max is sensitive)
/// - **Speed**: Similar to RRF (~1-2% faster due to simpler computation)
#[must_use]
pub fn combsum<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    combsum_with_config(results_a, results_b, FusionConfig::default())
}

/// `CombSUM` with configuration.
#[must_use]
pub fn combsum_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: FusionConfig,
) -> Vec<(I, f32)> {
    combsum_multi(&[results_a, results_b], config)
}

/// `CombSUM` for 3+ result lists.
///
/// # Empty Lists
///
/// If `lists` is empty, returns an empty result. Empty lists within the slice
/// contribute zero scores (documents not appearing in those lists receive
/// no contribution from them).
#[must_use]
pub fn combsum_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for list in lists {
        let items = list.as_ref();
        let (norm, off) = min_max_params(items);
        for (id, s) in items {
            let contribution = (s - off) * norm;
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    finalize(scores, config.top_k)
}

/// Normalized sum × overlap count (CombMNZ).
///
/// CombMNZ multiplies the CombSUM score by the number of lists containing the document,
/// rewarding documents that appear in multiple retrievers (consensus signal).
///
/// # Formula
///
/// `score(d) = CombSUM(d) × |{lists containing d}|`
///
/// Where CombSUM(d) is the sum of min-max normalized scores, and the multiplier
/// is the number of lists that contain document d.
///
/// # Arguments
///
/// * `results_a` - First ranked list: `Vec<(document_id, score)>`
/// * `results_b` - Second ranked list: `Vec<(document_id, score)>`
///
/// # Returns
///
/// Fused ranking sorted by CombMNZ score (descending). Documents appearing in
/// both lists get a 2x multiplier, significantly boosting their scores.
///
/// # Example
///
/// ```rust
/// use sortant::combmnz;
///
/// let sparse = vec![("doc1", 0.9), ("doc2", 0.8)];
/// let dense = vec![("doc2", 0.95), ("doc3", 0.7)];
///
/// let fused = combmnz(&sparse, &dense);
/// // doc2 ranks highest: (0.8 + 0.95) × 2 = 3.5 (appears in both lists)
/// // doc1: 0.9 × 1 = 0.9 (only in sparse)
/// // doc3: 0.7 × 1 = 0.7 (only in dense)
/// ```
///
/// # Performance
///
/// Time complexity: O(n log n) where n = total items across all lists.
/// For typical workloads (100-1000 items per list), fusion completes in <1ms.
///
/// # When to Use
///
/// - ✅ Overlap between retrievers signals higher relevance
/// - ✅ Want to strongly favor documents found by multiple retrievers
/// - ✅ Combining complementary retrievers (e.g., keyword + semantic)
///
/// # When NOT to Use
///
/// - ❌ Retrievers are highly correlated (overlap doesn't add information)
/// - ❌ Single-source documents are still valuable (CombMNZ penalizes them)
/// - ❌ Need fine-grained control (use `weighted` with custom weights)
///
/// # Trade-offs
///
/// - **Consensus**: Strongly favors documents in multiple lists
/// - **Diversity**: May reduce diversity (single-source documents rank lower)
/// - **Accuracy**: Typically similar to CombSUM, better when overlap is informative
#[must_use]
pub fn combmnz<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    combmnz_with_config(results_a, results_b, FusionConfig::default())
}

/// `CombMNZ` with configuration.
#[must_use]
pub fn combmnz_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: FusionConfig,
) -> Vec<(I, f32)> {
    combmnz_multi(&[results_a, results_b], config)
}

/// `CombMNZ` for 3+ result lists.
///
/// # Empty Lists
///
/// If `lists` is empty, returns an empty result. Empty lists within the slice
/// contribute zero scores and don't affect overlap counts.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn combmnz_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, (f32, u32)> = HashMap::with_capacity(estimated_size);

    for list in lists {
        let items = list.as_ref();
        let (norm, off) = min_max_params(items);
        for (id, s) in items {
            // Use get_mut + insert pattern to avoid cloning IDs when entry already exists
            let contribution = (s - off) * norm;
            if let Some(entry) = scores.get_mut(id) {
                entry.0 += contribution;
                entry.1 += 1;
            } else {
                scores.insert(id.clone(), (contribution, 1));
            }
        }
    }

    let mut results: Vec<_> = scores
        .into_iter()
        .map(|(id, (sum, n))| (id, sum * n as f32))
        .collect();
    sort_scored_desc(&mut results);
    if let Some(top_k) = config.top_k {
        results.truncate(top_k);
    }
    results
}

// ─────────────────────────────────────────────────────────────────────────────
// Rank-based Fusion
// ─────────────────────────────────────────────────────────────────────────────

/// Borda count voting — position-based scoring.
///
/// Borda count assigns points based on position: the first item gets N points,
/// the second gets N-1 points, etc., where N is the list length. Simple and
/// robust when you don't trust score magnitudes.
///
/// # Formula
///
/// `score(d) = Σ (N - rank)` where:
/// - N = list length
/// - rank = 0-indexed position in the list
///
/// Each list contributes independently, and scores are summed across lists.
///
/// # Arguments
///
/// * `results_a` - First ranked list (scores ignored, only positions matter)
/// * `results_b` - Second ranked list (scores ignored, only positions matter)
///
/// # Returns
///
/// Fused ranking sorted by Borda score (descending). Documents appearing in
/// both lists at high positions rank highest.
///
/// # Example
///
/// ```rust
/// use sortant::borda;
///
/// // List 1: 3 items (positions 0, 1, 2 → scores 3, 2, 1)
/// let list1 = vec![("d1", 0.9), ("d2", 0.5), ("d3", 0.3)];
///
/// // List 2: 2 items (positions 0, 1 → scores 2, 1)
/// let list2 = vec![("d2", 0.8), ("d4", 0.7)];
///
/// let fused = borda(&list1, &list2);
/// // d2: 2 (from list1) + 2 (from list2) = 4 points
/// // d1: 3 (from list1) = 3 points
/// // d3: 1 (from list1) = 1 point
/// // d4: 1 (from list2) = 1 point
/// ```
///
/// # Performance
///
/// Time complexity: O(n log n) where n = total items across all lists.
/// For typical workloads (100-1000 items per list), fusion completes in <1ms.
///
/// # When to Use
///
/// - ✅ Simple voting-based fusion needed
/// - ✅ Don't trust score magnitudes (only positions matter)
/// - ✅ Need interpretable scoring (easy to explain: "sum of position points")
///
/// # When NOT to Use
///
/// - ❌ Lists have very different lengths (longer lists dominate)
/// - ❌ Need position decay (use RRF or ISR for exponential/square-root decay)
/// - ❌ Score magnitudes are reliable (use `combsum` or `weighted`)
///
/// # Trade-offs
///
/// - **Simplicity**: Very simple to understand and implement
/// - **Fairness**: Treats all positions linearly (no decay)
/// - **Length bias**: Longer lists contribute more total points
#[must_use]
pub fn borda<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    borda_with_config(results_a, results_b, FusionConfig::default())
}

/// Borda count with configuration.
#[must_use]
pub fn borda_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: FusionConfig,
) -> Vec<(I, f32)> {
    borda_multi(&[results_a, results_b], config)
}

/// Borda count for 3+ result lists.
///
/// # Empty Lists
///
/// If `lists` is empty, returns an empty result. Empty lists within the slice
/// contribute zero scores (documents not appearing in those lists receive
/// no Borda points from them).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn borda_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for list in lists {
        let items = list.as_ref();
        let n = items.len() as f32;
        for (rank, (id, _)) in items.iter().enumerate() {
            let contribution = n - rank as f32;
            if let Some(score) = scores.get_mut(id) {
                *score += contribution;
            } else {
                scores.insert(id.clone(), contribution);
            }
        }
    }

    finalize(scores, config.top_k)
}

// ─────────────────────────────────────────────────────────────────────────────
// Distribution-Based Score Fusion (DBSF)
// ─────────────────────────────────────────────────────────────────────────────

/// Distribution-Based Score Fusion (DBSF).
///
/// DBSF uses z-score normalization (standardization) with mean ± 3σ clipping,
/// then sums the normalized scores. More robust than min-max normalization
/// when score distributions differ significantly or contain outliers.
///
/// # Algorithm
///
/// For each list:
/// 1. Compute mean (μ) and standard deviation (σ)
/// 2. Normalize: `(score - (μ - 3σ)) / 6σ`, clipped to [0, 1] (the z-score
///    clipped to [-3, 3] and shifted so the list's floor maps to 0)
/// 3. Sum normalized scores across lists; a document absent from a list gets 0
///
/// The ±3σ clipping prevents extreme outliers from dominating the fusion.
///
/// # Arguments
///
/// * `results_a` - First ranked list with scores
/// * `results_b` - Second ranked list with scores
///
/// # Returns
///
/// Fused ranking sorted by combined normalized score (descending). Because an
/// absent document contributes 0, appearing in more lists raises the score.
///
/// # Example
///
/// ```rust
/// use sortant::dbsf;
///
/// // BM25 scores (high variance, different scale)
/// let bm25 = vec![("d1", 15.0), ("d2", 12.0), ("d3", 8.0)];
///
/// // Dense embedding scores (low variance, different scale)
/// let dense = vec![("d2", 0.9), ("d3", 0.7), ("d4", 0.5)];
///
/// let fused = dbsf(&bm25, &dense);
/// // d2 and d3 appear in both lists, so they rank highest
/// let order: Vec<&str> = fused.iter().map(|(d, _)| *d).collect();
/// assert_eq!(order, ["d2", "d3", "d1", "d4"]);
/// ```
///
/// # Performance
///
/// Time complexity: O(n log n) where n = total items across all lists.
/// Requires computing mean and std for each list (O(n) per list).
/// For typical workloads (100-1000 items per list), fusion completes in <1ms.
///
/// # When to Use
///
/// - ✅ Score distributions differ significantly (BM25: 0-100, embeddings: 0-1)
/// - ✅ Outliers are present (z-score clipping handles them)
/// - ✅ Need robust normalization (more robust than min-max)
///
/// # When NOT to Use
///
/// - ❌ Score scales are similar (use `combsum` for simplicity)
/// - ❌ Need configurable clipping (use `standardized` with custom range)
/// - ❌ Unknown score scales (use RRF to avoid normalization)
///
/// # Trade-offs vs CombSUM
///
/// - **Robustness**: More robust to outliers (z-score vs min-max)
/// - **Complexity**: Slightly more complex (requires mean/std computation)
/// - **Clipping**: Fixed [-3, 3] range (use `standardized` for custom range)
///
/// # Differences from Standardized
///
/// - DBSF uses fixed [-3, 3] clipping
/// - Standardized allows configurable clipping range
/// - Standardized sums raw z-scores, so an absent document counts as the
///   list mean; DBSF shifts to [0, 1], so an absent document counts as the floor
#[must_use]
pub fn dbsf<I: Clone + Eq + Hash>(results_a: &[(I, f32)], results_b: &[(I, f32)]) -> Vec<(I, f32)> {
    dbsf_with_config(results_a, results_b, FusionConfig::default())
}

/// DBSF with configuration.
#[must_use]
pub fn dbsf_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: FusionConfig,
) -> Vec<(I, f32)> {
    dbsf_multi(&[results_a, results_b], config)
}

/// DBSF for 3+ result lists.
#[must_use]
pub fn dbsf_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);
    for list in lists {
        let items = list.as_ref();
        let (mean, std) = zscore_params(items);
        for (id, s) in items {
            *scores.entry(id.clone()).or_insert(0.0) += dbsf_normalize(*s, mean, std);
        }
    }
    finalize(scores, config.top_k)
}

/// Map a score into [0, 1] by its list's `[μ - 3σ, μ + 3σ]` range.
///
/// A constant list (σ = 0) maps to 0.5, the midpoint, so presence still
/// counts for more than absence (0).
#[inline]
fn dbsf_normalize(score: f32, mean: f32, std: f32) -> f32 {
    if std > SCORE_RANGE_EPSILON {
        ((score - mean) / std).clamp(-3.0, 3.0) / 6.0 + 0.5
    } else {
        0.5
    }
}

/// Compute mean and standard deviation for z-score normalization.
#[inline(always)]
fn zscore_params<I>(results: &[(I, f32)]) -> (f32, f32) {
    if results.is_empty() {
        return (0.0, 1.0);
    }

    let n = results.len() as f32;
    let mean = results.iter().map(|(_, s)| s).sum::<f32>() / n;
    let variance = results.iter().map(|(_, s)| (s - mean).powi(2)).sum::<f32>() / n;
    let std = variance.sqrt();

    (mean, std)
}

// ─────────────────────────────────────────────────────────────────────────────
// Standardization-Based Fusion (z-score)
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for standardization-based fusion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandardizedConfig {
    /// Clip z-scores to this range (default: [-3.0, 3.0]).
    pub clip_range: (f32, f32),
    /// Maximum results to return (None = all).
    pub top_k: Option<usize>,
}

impl Default for StandardizedConfig {
    fn default() -> Self {
        Self {
            clip_range: (-3.0, 3.0),
            top_k: None,
        }
    }
}

impl StandardizedConfig {
    /// Create new config with custom clipping range.
    #[must_use]
    pub const fn new(clip_range: (f32, f32)) -> Self {
        Self {
            clip_range,
            top_k: None,
        }
    }

    /// DBSF preset: z-score normalization with [-3, 3] clipping.
    ///
    /// Equivalent to `StandardizedConfig::default()`: the 3-sigma clip range that
    /// DBSF (Distribution-Based Score Fusion) uses. [`dbsf`] additionally shifts
    /// clipped z-scores into [0, 1] so absent documents score lowest.
    #[must_use]
    pub const fn dbsf() -> Self {
        Self {
            clip_range: (-3.0, 3.0),
            top_k: None,
        }
    }

    /// Limit output to `top_k` results.
    #[must_use]
    pub const fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

/// Standardization-based fusion (z-score).
///
/// Uses z-score normalization (standardization) instead of min-max normalization,
/// then applies additive fusion. More robust to outliers and different score distributions.
///
/// # Algorithm
///
/// For each list:
/// 1. Compute mean (μ) and standard deviation (σ)
/// 2. Normalize: `z = (score - μ) / σ`, clipped to `clip_range`
/// 3. Sum normalized scores across lists
///
/// # Differences from DBSF
///
/// - DBSF uses fixed [-3, 3] clipping
/// - Standardized allows configurable clipping range
/// - Both use the same z-score approach, but standardized is more flexible
///
/// # Example
///
/// ```rust
/// use sortant::standardized;
///
/// let bm25 = vec![("d1", 15.0), ("d2", 12.0), ("d3", 8.0)];
/// let dense = vec![("d2", 0.9), ("d3", 0.7), ("d4", 0.5)];
/// let fused = standardized(&bm25, &dense);
/// ```
#[must_use]
pub fn standardized<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    standardized_with_config(results_a, results_b, StandardizedConfig::default())
}

/// Standardized fusion with configuration.
#[must_use]
pub fn standardized_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: StandardizedConfig,
) -> Vec<(I, f32)> {
    standardized_multi(&[results_a, results_b], config)
}

/// Standardized fusion for 3+ result lists.
///
/// # Empty Lists
///
/// If `lists` is empty, returns an empty result. Empty lists within the slice
/// contribute zero scores (documents not appearing in those lists receive
/// no z-score contribution from them).
///
/// # Degenerate Cases
///
/// If all scores in a list are equal (zero variance), that list contributes
/// z-score=0.0 for all documents, which is mathematically correct but
/// effectively ignores that list's contribution.
#[must_use]
pub fn standardized_multi<I, L>(lists: &[L], config: StandardizedConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let estimated_size: usize = lists.iter().map(|l| l.as_ref().len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);
    let (clip_min, clip_max) = config.clip_range;

    for list in lists {
        let items = list.as_ref();
        let (mean, std) = zscore_params(items);

        for (id, s) in items {
            // Z-score normalize and clip to configurable range
            let z = if std > SCORE_RANGE_EPSILON {
                ((s - mean) / std).clamp(clip_min, clip_max)
            } else {
                0.0 // All scores equal
            };
            if let Some(score) = scores.get_mut(id) {
                *score += z;
            } else {
                scores.insert(id.clone(), z);
            }
        }
    }

    finalize(scores, config.top_k)
}

// ─────────────────────────────────────────────────────────────────────────────
// Additive Multi-Task Fusion (ResFlow-style)
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for additive multi-task fusion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdditiveMultiTaskConfig {
    /// Weights for each task: (weight_a, weight_b).
    pub weights: (f32, f32),
    /// Normalization method (default: ZScore for robustness).
    pub normalization: Normalization,
    /// Maximum results to return (None = all).
    pub top_k: Option<usize>,
}

impl Default for AdditiveMultiTaskConfig {
    fn default() -> Self {
        Self {
            weights: (1.0, 1.0),
            normalization: Normalization::ZScore,
            top_k: None,
        }
    }
}

impl AdditiveMultiTaskConfig {
    /// Create new config with custom weights.
    ///
    /// ResFlow's optimal formula for e-commerce: `CTR + CTCVR × 20`.
    /// This would be `AdditiveMultiTaskConfig::new((1.0, 20.0))`.
    #[must_use]
    pub const fn new(weights: (f32, f32)) -> Self {
        Self {
            weights,
            normalization: Normalization::ZScore,
            top_k: None,
        }
    }

    /// Set normalization method.
    #[must_use]
    pub const fn with_normalization(mut self, normalization: Normalization) -> Self {
        self.normalization = normalization;
        self
    }

    /// Limit output to `top_k` results.
    #[must_use]
    pub const fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

/// Additive multi-task fusion (ResFlow-style).
///
/// Additive fusion of multi-task scores: `α·score_a + β·score_b`.
///
/// ResFlow (arXiv:2411.09705) shows additive outperforms multiplicative for e-commerce
/// ranking tasks. The optimal formula is typically `CTR + CTCVR × 20`, where CTR and
/// CTCVR are normalized scores from different tasks.
///
/// # Algorithm
///
/// 1. Normalize each list using the specified normalization method
/// 2. Compute weighted sum: `α·norm_a + β·norm_b`
/// 3. Sort by combined score (descending)
///
/// # Example
///
/// ```rust
/// use sortant::{additive_multi_task, AdditiveMultiTaskConfig};
///
/// let ctr_scores = vec![("item1", 0.05), ("item2", 0.03), ("item3", 0.01)];
/// let ctcvr_scores = vec![("item1", 0.02), ("item2", 0.01), ("item3", 0.005)];
///
/// // ResFlow optimal: CTR + CTCVR × 20
/// let config = AdditiveMultiTaskConfig::new((1.0, 20.0));
/// let fused = additive_multi_task(&ctr_scores, &ctcvr_scores, config);
/// ```
#[must_use]
pub fn additive_multi_task<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: AdditiveMultiTaskConfig,
) -> Vec<(I, f32)> {
    additive_multi_task_with_config(results_a, results_b, config)
}

/// Additive multi-task fusion with configuration.
#[must_use]
pub fn additive_multi_task_with_config<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
    config: AdditiveMultiTaskConfig,
) -> Vec<(I, f32)> {
    let weighted_lists = vec![(results_a, config.weights.0), (results_b, config.weights.1)];
    additive_multi_task_multi(&weighted_lists, config)
}

/// Additive multi-task fusion for 3+ weighted lists.
///
/// # Arguments
///
/// * `weighted_lists` - Slice of (list, weight) pairs. Each list is normalized independently,
///   then combined using weighted sum.
///
/// # Example
///
/// ```rust
/// use sortant::{additive_multi_task_multi, AdditiveMultiTaskConfig};
///
/// let task1 = vec![("d1", 0.9), ("d2", 0.7)];
/// let task2 = vec![("d1", 0.8), ("d2", 0.6)];
/// let task3 = vec![("d1", 0.5), ("d2", 0.4)];
///
/// let weighted = vec![
///     (&task1[..], 1.0),
///     (&task2[..], 2.0),
///     (&task3[..], 0.5),
/// ];
///
/// let config = AdditiveMultiTaskConfig::default();
/// let fused = additive_multi_task_multi(&weighted, config);
/// ```
#[must_use]
pub fn additive_multi_task_multi<I, L>(
    weighted_lists: &[(L, f32)],
    config: AdditiveMultiTaskConfig,
) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if weighted_lists.is_empty() {
        return Vec::new();
    }

    // Normalize each list independently
    let normalized: Vec<_> = weighted_lists
        .iter()
        .map(|(list, _)| normalize_scores(list.as_ref(), config.normalization))
        .collect();

    // Compute weighted sum
    let estimated_size: usize = normalized.iter().map(|n| n.len()).sum();
    let mut scores: HashMap<I, f32> = HashMap::with_capacity(estimated_size);

    for (normalized_list, (_, weight)) in normalized.iter().zip(weighted_lists.iter()) {
        for (id, norm_score) in normalized_list {
            if let Some(score) = scores.get_mut(id) {
                *score += weight * norm_score;
            } else {
                scores.insert(id.clone(), weight * norm_score);
            }
        }
    }

    finalize(scores, config.top_k)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Sort scores descending and optionally truncate.
///
/// Uses `total_cmp` plus an identifier-hash fallback for reproducible tie ordering.
#[inline]
fn finalize<I: Hash>(scores: HashMap<I, f32>, top_k: Option<usize>) -> Vec<(I, f32)> {
    let capacity = top_k.map(|k| k.min(scores.len())).unwrap_or(scores.len());
    let mut results = Vec::with_capacity(capacity);
    results.extend(scores);
    sort_scored_desc(&mut results);
    if let Some(k) = top_k {
        results.truncate(k);
    }
    results
}

/// Sort scored results in descending order.
///
/// Uses `f32::total_cmp` and an identifier-hash fallback for tie ordering.
#[inline]
fn sort_scored_desc<I: Hash>(results: &mut [(I, f32)]) {
    results.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| stable_hash(&a.0).cmp(&stable_hash(&b.0)))
    });
}

#[inline]
fn stable_hash<T: Hash>(value: &T) -> u64 {
    use std::hash::Hasher;

    // FNV-1a 64. std's `DefaultHasher` algorithm is unspecified and may change
    // between Rust releases, which would reorder tied results.
    struct Fnv1a(u64);
    impl Hasher for Fnv1a {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 ^= u64::from(b);
                self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }

    let mut hasher = Fnv1a(0xcbf2_9ce4_8422_2325);
    value.hash(&mut hasher);
    hasher.finish()
}

/// Score normalization methods.
///
/// Different retrievers produce scores on different scales. Normalization
/// puts them on a common scale before combining.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Normalization {
    /// Min-max normalization: `(score - min) / (max - min)` → [0, 1]
    ///
    /// Best when score distributions are similar. Sensitive to outliers.
    #[default]
    MinMax,
    /// Z-score normalization: `(score - mean) / std`, clipped to [-3, 3]
    ///
    /// More robust to outliers. Better when distributions differ.
    ZScore,
    /// Sum normalization: `score / sum(scores)`
    ///
    /// Preserves relative magnitudes. Useful when scores represent probabilities.
    Sum,
    /// Rank-based: convert scores to ranks, then normalize
    ///
    /// Sorts input by score (descending), assigns ranks 0..n-1, then normalizes
    /// to [0, 1] range where rank 0 (best) → 1.0, rank n-1 (worst) → 1/n.
    /// Ignores score magnitudes entirely. Most robust but loses information.
    Rank,
    /// Quantile normalization: maps scores to their percentile rank in [0, 1].
    ///
    /// More robust than min-max for non-Gaussian score distributions.
    /// Each score becomes `rank_among_scores / (n - 1)`.
    /// Referenced as an alternative to 3-sigma DBSF normalization
    /// when cosine similarity scores are not normally distributed.
    Quantile,
    /// Sigmoid normalization: `1 / (1 + exp(-score))` → (0, 1).
    ///
    /// Squashes unbounded scores to (0, 1) while preserving relative ordering.
    /// Useful for cross-encoder logits or other unbounded score ranges.
    Sigmoid,
    /// No normalization: use raw scores
    ///
    /// Only use when all retrievers use the same scale.
    None,
}

/// Normalize a list of scores using the specified method.
///
/// Returns a vector of (id, normalized_score) pairs.
pub fn normalize_scores<I: Clone>(results: &[(I, f32)], method: Normalization) -> Vec<(I, f32)> {
    if results.is_empty() {
        return Vec::new();
    }

    match method {
        Normalization::MinMax => {
            let (norm, off) = min_max_params(results);
            results
                .iter()
                .map(|(id, s)| (id.clone(), (s - off) * norm))
                .collect()
        }
        Normalization::ZScore => {
            let (mean, std) = zscore_params(results);
            results
                .iter()
                .map(|(id, s)| {
                    let z = if std > SCORE_RANGE_EPSILON {
                        ((s - mean) / std).clamp(-3.0, 3.0)
                    } else {
                        0.0
                    };
                    (id.clone(), z)
                })
                .collect()
        }
        Normalization::Sum => {
            let sum: f32 = results.iter().map(|(_, s)| s).sum();
            if sum.abs() < SCORE_RANGE_EPSILON {
                return results.to_vec();
            }
            results
                .iter()
                .map(|(id, s)| (id.clone(), s / sum))
                .collect()
        }
        Normalization::Rank => {
            // Sort by score (descending) first, then assign ranks
            // This ensures higher scores get better (lower) ranks
            // NaN/Inf values are treated as worst (sorted to end) so valid scores rank first
            let mut sorted: Vec<_> = results.to_vec();
            sorted.sort_by(|a, b| {
                // Custom comparison: finite values sort normally (descending),
                // non-finite values (NaN, Inf) sort to the end
                match (a.1.is_finite(), b.1.is_finite()) {
                    (true, true) => b.1.total_cmp(&a.1), // Both finite: descending
                    (true, false) => std::cmp::Ordering::Less, // a is finite, b is not: a first
                    (false, true) => std::cmp::Ordering::Greater, // b is finite, a is not: b first
                    (false, false) => std::cmp::Ordering::Equal, // Both non-finite: equal
                }
            });

            let n = sorted.len() as f32;
            sorted
                .iter()
                .enumerate()
                .map(|(rank, (id, _))| (id.clone(), 1.0 - (rank as f32 / n)))
                .collect()
        }
        Normalization::Quantile => {
            // Sort scores to assign percentile ranks
            let mut indexed: Vec<(usize, f32)> = results
                .iter()
                .enumerate()
                .map(|(i, (_, s))| (i, *s))
                .collect();
            indexed.sort_by(|a, b| a.1.total_cmp(&b.1));

            let n = indexed.len();
            let mut quantiles = vec![0.0f32; n];
            if n == 1 {
                quantiles[0] = 0.5; // Single item gets middle quantile
            } else {
                for (rank, &(orig_idx, _)) in indexed.iter().enumerate() {
                    quantiles[orig_idx] = rank as f32 / (n - 1) as f32;
                }
            }

            results
                .iter()
                .enumerate()
                .map(|(i, (id, _))| (id.clone(), quantiles[i]))
                .collect()
        }
        Normalization::Sigmoid => results
            .iter()
            .map(|(id, s)| (id.clone(), 1.0 / (1.0 + (-s).exp())))
            .collect(),
        Normalization::None => results.to_vec(),
    }
}

/// Returns `(norm_factor, offset)` for min-max normalization.
///
/// Normalized score = `(score - offset) * norm_factor`
///
/// For single-element lists or lists where all scores are equal,
/// returns `(0.0, 0.0)` so each element contributes its raw score.
#[inline(always)]
fn min_max_params<I>(results: &[(I, f32)]) -> (f32, f32) {
    if results.is_empty() {
        return (1.0, 0.0);
    }
    let (min, max) = results
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), (_, s)| {
            (lo.min(*s), hi.max(*s))
        });
    let range = max - min;
    if range < SCORE_RANGE_EPSILON {
        // All scores equal: just pass through the score (norm=1, offset=0)
        (1.0, 0.0)
    } else {
        (1.0 / range, min)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Explainability
// ─────────────────────────────────────────────────────────────────────────────

/// A fused result with full provenance information for debugging and analysis.
///
/// Unlike the simple `Vec<(DocId, f32)>` returned by standard fusion functions,
/// `FusedResult` preserves which retrievers contributed each document, their
/// original ranks and scores, and how much each source contributed to the final score.
///
/// # Example
///
/// ```rust
/// use sortant::explain::{rrf_explain, RetrieverId};
///
/// let bm25 = vec![("d1", 12.5), ("d2", 11.0)];
/// let dense = vec![("d2", 0.9), ("d3", 0.8)];
///
/// let retrievers = vec![
///     RetrieverId::new("bm25"),
///     RetrieverId::new("dense"),
/// ];
///
/// let explained = rrf_explain(
///     &[&bm25[..], &dense[..]],
///     &retrievers,
///     sortant::RrfConfig::default(),
/// );
///
/// // d2 appears in both lists, so it has 2 source contributions
/// let d2 = explained.iter().find(|r| r.id == "d2").unwrap();
/// assert_eq!(d2.explanation.sources.len(), 2);
/// assert_eq!(d2.explanation.consensus_score, 1.0); // 2/2 lists
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FusedResult<K> {
    /// Document identifier.
    pub id: K,
    /// Final fused score.
    pub score: f32,
    /// Final rank position (0-indexed, highest score = rank 0).
    pub rank: usize,
    /// Explanation of how this score was computed.
    pub explanation: Explanation,
}

/// Explanation of how a fused score was computed.
#[derive(Debug, Clone, PartialEq)]
pub struct Explanation {
    /// Contributions from each retriever that contained this document.
    pub sources: Vec<SourceContribution>,
    /// Fusion method used (e.g., "rrf", "combsum").
    pub method: &'static str,
    /// Consensus score: fraction of retrievers that contained this document (0.0-1.0).
    ///
    /// - 1.0 = document appeared in all retrievers (strong consensus)
    /// - 0.5 = document appeared in half of retrievers
    /// - < 0.3 = document appeared in few retrievers (outlier)
    pub consensus_score: f32,
}

/// Contribution from a single retriever to a document's final score.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceContribution {
    /// Identifier for this retriever (e.g., "bm25", "dense_vector").
    pub retriever_id: String,
    /// Original rank in this retriever's list (0-indexed, None if not present).
    pub original_rank: Option<usize>,
    /// Original score from this retriever (None for rank-based methods or if not present).
    pub original_score: Option<f32>,
    /// Normalized score (for score-based methods, None for rank-based).
    pub normalized_score: Option<f32>,
    /// How much this source contributed to the final fused score.
    ///
    /// For RRF: `1/(k + rank)` or `weight / (k + rank)` for weighted, with
    /// `rank` 1-indexed (`original_rank + 1`).
    /// For CombSUM: normalized score.
    /// For CombMNZ: normalized score × overlap count.
    pub contribution: f32,
}

/// Retriever identifier for explainability.
///
/// Used to label which retriever each list comes from when calling explain variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RetrieverId {
    id: String,
}

impl RetrieverId {
    /// Create a new retriever identifier.
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self { id: id.into() }
    }

    /// Get the identifier string.
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl From<&str> for RetrieverId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<String> for RetrieverId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

/// RRF with explainability: returns full provenance for each result.
///
/// This variant preserves which retrievers contributed each document, their
/// original ranks, and how much each source contributed to the final RRF score.
///
/// # Arguments
///
/// * `lists` - Ranked lists from each retriever
/// * `retriever_ids` - Identifiers for each retriever (must match `lists.len()`)
/// * `config` - RRF configuration
///
/// # Returns
///
/// Results sorted by fused score (descending), with full explanation metadata.
///
/// # Example
///
/// ```rust
/// use sortant::explain::{rrf_explain, RetrieverId};
/// use sortant::RrfConfig;
///
/// let bm25 = vec![("d1", 12.5), ("d2", 11.0)];
/// let dense = vec![("d2", 0.9), ("d3", 0.8)];
///
/// let retrievers = vec![
///     RetrieverId::new("bm25"),
///     RetrieverId::new("dense"),
/// ];
///
/// let explained = rrf_explain(
///     &[&bm25[..], &dense[..]],
///     &retrievers,
///     RrfConfig::default(),
/// );
///
/// // d2 appears in both lists at rank 1 and 0 respectively
/// let d2 = explained.iter().find(|r| r.id == "d2").unwrap();
/// assert_eq!(d2.explanation.sources.len(), 2);
/// assert_eq!(d2.explanation.consensus_score, 1.0); // in both lists
///
/// // Check contributions
/// let bm25_contrib = d2.explanation.sources.iter()
///     .find(|s| s.retriever_id == "bm25")
///     .unwrap();
/// assert_eq!(bm25_contrib.original_rank, Some(1));
/// assert!(bm25_contrib.contribution > 0.0);
/// ```
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn rrf_explain<I, L>(
    lists: &[L],
    retriever_ids: &[RetrieverId],
    config: RrfConfig,
) -> Vec<FusedResult<I>>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() || lists.len() != retriever_ids.len() {
        return Vec::new();
    }

    let k = config.k as f32;
    let num_retrievers = lists.len() as f32;

    // Track scores and provenance
    let mut scores: HashMap<I, f32> = HashMap::new();
    let mut provenance: HashMap<I, Vec<SourceContribution>> = HashMap::new();

    for (list, retriever_id) in lists.iter().zip(retriever_ids.iter()) {
        for (rank, (id, original_score)) in list.as_ref().iter().enumerate() {
            let contribution = 1.0 / (k + (rank + 1) as f32);

            // Update score
            *scores.entry(id.clone()).or_insert(0.0) += contribution;

            // Track provenance
            provenance
                .entry(id.clone())
                .or_default()
                .push(SourceContribution {
                    retriever_id: retriever_id.id.clone(),
                    original_rank: Some(rank),
                    original_score: Some(*original_score),
                    normalized_score: None, // RRF doesn't normalize
                    contribution,
                });
        }
    }

    // Build results with explanations
    let mut results: Vec<FusedResult<I>> = scores
        .into_iter()
        .map(|(id, score)| {
            let sources = provenance.remove(&id).unwrap_or_default();
            let consensus_score = sources.len() as f32 / num_retrievers;

            FusedResult {
                id,
                score,
                rank: 0, // Will be set after sorting
                explanation: Explanation {
                    sources,
                    method: "rrf",
                    consensus_score,
                },
            }
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.total_cmp(&a.score));

    // Set ranks
    for (rank, result) in results.iter_mut().enumerate() {
        result.rank = rank;
    }

    // Apply top_k
    if let Some(top_k) = config.top_k {
        results.truncate(top_k);
    }

    results
}

/// Analyze consensus patterns across retrievers.
///
/// Returns statistics about how retrievers agree or disagree on document relevance.
///
/// # Example
///
/// ```rust
/// use sortant::explain::{rrf_explain, analyze_consensus, RetrieverId};
/// use sortant::RrfConfig;
///
/// let bm25 = vec![("d1", 12.5), ("d2", 11.0)];
/// let dense = vec![("d2", 0.9), ("d3", 0.8)];
///
/// let explained = rrf_explain(
///     &[&bm25[..], &dense[..]],
///     &[RetrieverId::new("bm25"), RetrieverId::new("dense")],
///     RrfConfig::default(),
/// );
///
/// let consensus = analyze_consensus(&explained);
/// // consensus.high_consensus contains documents in all retrievers
/// // consensus.single_source contains documents only in one retriever
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConsensusReport<K> {
    /// Documents that appeared in all retrievers (consensus_score == 1.0).
    pub high_consensus: Vec<K>,
    /// Documents that appeared in only one retriever (consensus_score < 0.5).
    pub single_source: Vec<K>,
    /// Documents with large rank disagreements across retrievers.
    ///
    /// A document might appear at rank 0 in one retriever but rank 50 in another,
    /// indicating retriever disagreement.
    pub rank_disagreement: Vec<(K, Vec<(String, usize)>)>,
}

/// Analyze consensus across fused results, identifying high-agreement and single-source items.
pub fn analyze_consensus<K: Clone + Eq + Hash>(results: &[FusedResult<K>]) -> ConsensusReport<K> {
    let mut high_consensus = Vec::new();
    let mut single_source = Vec::new();
    let mut rank_disagreement = Vec::new();

    for result in results {
        // High consensus: in all retrievers
        if result.explanation.consensus_score >= 1.0 - 1e-6 {
            high_consensus.push(result.id.clone());
        }

        // Single source: in only one retriever
        if result.explanation.sources.len() == 1 {
            single_source.push(result.id.clone());
        }

        // Rank disagreement: large spread in ranks
        if result.explanation.sources.len() > 1 {
            let ranks: Vec<usize> = result
                .explanation
                .sources
                .iter()
                .filter_map(|s| s.original_rank)
                .collect();
            if let (Some(&min_rank), Some(&max_rank)) = (ranks.iter().min(), ranks.iter().max())
                && max_rank - min_rank > 10
            {
                // Large disagreement threshold
                let rank_info: Vec<(String, usize)> = result
                    .explanation
                    .sources
                    .iter()
                    .filter_map(|s| s.original_rank.map(|r| (s.retriever_id.clone(), r)))
                    .collect();
                rank_disagreement.push((result.id.clone(), rank_info));
            }
        }
    }

    ConsensusReport {
        high_consensus,
        single_source,
        rank_disagreement,
    }
}

/// Attribution statistics for each retriever.
///
/// Shows how much each retriever contributed to the top-k results.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrieverStats {
    /// Number of top-k documents this retriever contributed.
    pub top_k_count: usize,
    /// Average contribution strength for documents in top-k.
    pub avg_contribution: f32,
    /// Documents that only this retriever found (unique to this retriever).
    pub unique_docs: usize,
}

/// Attribute top-k results to retrievers.
///
/// Returns statistics showing which retrievers contributed most to the top-k results.
///
/// # Example
///
/// ```rust
/// use sortant::explain::{rrf_explain, attribute_top_k, RetrieverId};
/// use sortant::RrfConfig;
///
/// let bm25 = vec![("d1", 12.5), ("d2", 11.0)];
/// let dense = vec![("d2", 0.9), ("d3", 0.8)];
///
/// let explained = rrf_explain(
///     &[&bm25[..], &dense[..]],
///     &[RetrieverId::new("bm25"), RetrieverId::new("dense")],
///     RrfConfig::default(),
/// );
///
/// let attribution = attribute_top_k(&explained, 5);
/// // attribution["bm25"].top_k_count shows how many top-5 docs came from BM25
/// ```
pub fn attribute_top_k<K: Clone + Eq + Hash>(
    results: &[FusedResult<K>],
    k: usize,
) -> std::collections::HashMap<String, RetrieverStats> {
    let top_k = results.iter().take(k);
    let mut stats: std::collections::HashMap<String, RetrieverStats> =
        std::collections::HashMap::new();

    // Track which documents each retriever found
    let mut retriever_docs: std::collections::HashMap<String, std::collections::HashSet<K>> =
        std::collections::HashMap::new();

    for result in top_k {
        for source in &result.explanation.sources {
            let entry =
                stats
                    .entry(source.retriever_id.clone())
                    .or_insert_with(|| RetrieverStats {
                        top_k_count: 0,
                        avg_contribution: 0.0,
                        unique_docs: 0,
                    });

            entry.top_k_count += 1;
            entry.avg_contribution += source.contribution;

            retriever_docs
                .entry(source.retriever_id.clone())
                .or_default()
                .insert(result.id.clone());
        }
    }

    // Calculate averages and unique counts
    for (retriever_id, stat) in &mut stats {
        if stat.top_k_count > 0 {
            stat.avg_contribution /= stat.top_k_count as f32;
        }

        // Count unique documents (only in this retriever)
        let this_retriever_docs = retriever_docs
            .get(retriever_id)
            .cloned()
            .unwrap_or_default();
        let other_retriever_docs: std::collections::HashSet<K> = retriever_docs
            .iter()
            .filter(|(id, _)| *id != retriever_id)
            .flat_map(|(_, docs)| docs.iter().cloned())
            .collect();

        stat.unique_docs = this_retriever_docs
            .difference(&other_retriever_docs)
            .count();
    }

    stats
}

/// CombSUM with explainability.
#[must_use]
pub fn combsum_explain<I, L>(
    lists: &[L],
    retriever_ids: &[RetrieverId],
    config: FusionConfig,
) -> Vec<FusedResult<I>>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() || lists.len() != retriever_ids.len() {
        return Vec::new();
    }

    let num_retrievers = lists.len() as f32;
    let mut scores: HashMap<I, f32> = HashMap::new();
    let mut provenance: HashMap<I, Vec<SourceContribution>> = HashMap::new();

    for (list, retriever_id) in lists.iter().zip(retriever_ids.iter()) {
        let items = list.as_ref();
        let (norm, off) = min_max_params(items);
        for (rank, (id, original_score)) in items.iter().enumerate() {
            let normalized_score = (original_score - off) * norm;
            let contribution = normalized_score;

            *scores.entry(id.clone()).or_insert(0.0) += contribution;

            provenance
                .entry(id.clone())
                .or_default()
                .push(SourceContribution {
                    retriever_id: retriever_id.id.clone(),
                    original_rank: Some(rank),
                    original_score: Some(*original_score),
                    normalized_score: Some(normalized_score),
                    contribution,
                });
        }
    }

    build_explained_results(scores, provenance, num_retrievers, "combsum", config.top_k)
}

/// CombMNZ with explainability.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn combmnz_explain<I, L>(
    lists: &[L],
    retriever_ids: &[RetrieverId],
    config: FusionConfig,
) -> Vec<FusedResult<I>>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() || lists.len() != retriever_ids.len() {
        return Vec::new();
    }

    let num_retrievers = lists.len() as f32;
    let mut scores: HashMap<I, (f32, u32)> = HashMap::new();
    let mut provenance: HashMap<I, Vec<SourceContribution>> = HashMap::new();

    for (list, retriever_id) in lists.iter().zip(retriever_ids.iter()) {
        let items = list.as_ref();
        let (norm, off) = min_max_params(items);
        for (rank, (id, original_score)) in items.iter().enumerate() {
            let normalized_score = (original_score - off) * norm;
            let contribution = normalized_score;

            let entry = scores.entry(id.clone()).or_insert((0.0, 0));
            entry.0 += contribution;
            entry.1 += 1;

            provenance
                .entry(id.clone())
                .or_default()
                .push(SourceContribution {
                    retriever_id: retriever_id.id.clone(),
                    original_rank: Some(rank),
                    original_score: Some(*original_score),
                    normalized_score: Some(normalized_score),
                    contribution,
                });
        }
    }

    // Apply CombMNZ multiplier (overlap count)
    let mut final_scores: HashMap<I, f32> = HashMap::new();
    let mut final_provenance: HashMap<I, Vec<SourceContribution>> = HashMap::new();

    for (id, (sum, overlap_count)) in scores {
        let final_score = sum * overlap_count as f32;
        final_scores.insert(id.clone(), final_score);

        // Update contributions to reflect multiplier
        if let Some(mut sources) = provenance.remove(&id) {
            for source in &mut sources {
                source.contribution *= overlap_count as f32;
            }
            final_provenance.insert(id, sources);
        }
    }

    build_explained_results(
        final_scores,
        final_provenance,
        num_retrievers,
        "combmnz",
        config.top_k,
    )
}

/// DBSF with explainability.
#[must_use]
pub fn dbsf_explain<I, L>(
    lists: &[L],
    retriever_ids: &[RetrieverId],
    config: FusionConfig,
) -> Vec<FusedResult<I>>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() || lists.len() != retriever_ids.len() {
        return Vec::new();
    }

    let num_retrievers = lists.len() as f32;
    let mut scores: HashMap<I, f32> = HashMap::new();
    let mut provenance: HashMap<I, Vec<SourceContribution>> = HashMap::new();

    for (list, retriever_id) in lists.iter().zip(retriever_ids.iter()) {
        let items = list.as_ref();
        let (mean, std) = zscore_params(items);

        for (rank, (id, original_score)) in items.iter().enumerate() {
            let z = dbsf_normalize(*original_score, mean, std);
            let contribution = z;

            *scores.entry(id.clone()).or_insert(0.0) += contribution;

            provenance
                .entry(id.clone())
                .or_default()
                .push(SourceContribution {
                    retriever_id: retriever_id.id.clone(),
                    original_rank: Some(rank),
                    original_score: Some(*original_score),
                    normalized_score: Some(z),
                    contribution,
                });
        }
    }

    build_explained_results(scores, provenance, num_retrievers, "dbsf", config.top_k)
}

/// Helper to build explained results from scores and provenance.
fn build_explained_results<I: Clone + Eq + Hash>(
    scores: HashMap<I, f32>,
    mut provenance: HashMap<I, Vec<SourceContribution>>,
    num_retrievers: f32,
    method: &'static str,
    top_k: Option<usize>,
) -> Vec<FusedResult<I>> {
    let mut results: Vec<FusedResult<I>> = scores
        .into_iter()
        .map(|(id, score)| {
            let sources = provenance.remove(&id).unwrap_or_default();
            let consensus_score = sources.len() as f32 / num_retrievers;

            FusedResult {
                id,
                score,
                rank: 0, // Will be set after sorting
                explanation: Explanation {
                    sources,
                    method,
                    consensus_score,
                },
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.total_cmp(&a.score));

    for (rank, result) in results.iter_mut().enumerate() {
        result.rank = rank;
    }

    if let Some(k) = top_k {
        results.truncate(k);
    }

    results
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional Algorithms
// ─────────────────────────────────────────────────────────────────────────────

/// CombMAX: maximum score across all lists.
///
/// Formula: `score(d) = max(s_r(d))` for all retrievers r containing d.
///
/// Use as a baseline or when you want to favor documents that score highly
/// in at least one retriever.
#[must_use]
pub fn combmax<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    combmax_multi(&[results_a, results_b], FusionConfig::default())
}

/// CombMAX for 3+ result lists.
#[must_use]
pub fn combmax_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let mut scores: HashMap<I, f32> = HashMap::new();

    for list in lists {
        for (id, s) in list.as_ref() {
            scores
                .entry(id.clone())
                .and_modify(|max_score| *max_score = max_score.max(*s))
                .or_insert(*s);
        }
    }

    finalize(scores, config.top_k)
}

/// CombMIN: minimum score across all lists.
///
/// Formula: `score(d) = min(s_r(d))` for all retrievers r containing d.
///
/// # Historical Context
///
/// CombMIN emerged from the information retrieval meta-search literature of
/// the late 1990s alongside CombSUM, CombMAX, and CombMNZ. The "Comb" family
/// was systematically studied by Fox & Shaw (1994) and later by Lee (1997).
///
/// | Method | Formula | Intuition |
/// |--------|---------|-----------|
/// | CombSUM | Σ s_r(d) | Agreement across all retrievers |
/// | CombMAX | max s_r(d) | At least one retriever likes it |
/// | CombMIN | min s_r(d) | All retrievers agree (conservative) |
/// | CombMNZ | Σ s_r(d) × count | Reward overlap explicitly |
///
/// # When to Use CombMIN
///
/// - **High-precision requirements**: When false positives are costly
/// - **Consensus retrieval**: Only surface documents all systems agree on
/// - **Spam filtering**: A document must pass multiple filters
///
/// CombMIN is inherently **conservative**: a document with scores [0.9, 0.1]
/// gets score 0.1, while CombMAX would give 0.9.
///
/// # Caution
///
/// Documents appearing in only one list will have that single score as their
/// CombMIN. To require presence in multiple lists, combine with a threshold
/// on occurrence count.
///
/// # Reference
///
/// Fox & Shaw, "Combination of Multiple Searches", NIST TREC 1994.
/// Lee, "Analyses of Multiple Evidence Combination", SIGIR 1997.
#[must_use]
pub fn combmin<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    combmin_multi(&[results_a, results_b], FusionConfig::default())
}

/// CombMIN for 3+ result lists.
#[must_use]
pub fn combmin_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let mut scores: HashMap<I, f32> = HashMap::new();

    for list in lists {
        for (id, s) in list.as_ref() {
            scores
                .entry(id.clone())
                .and_modify(|min_score| *min_score = min_score.min(*s))
                .or_insert(*s);
        }
    }

    finalize(scores, config.top_k)
}

/// CombMED: median score across all lists.
///
/// Formula: `score(d) = median(s_r(d))` for all retrievers r containing d.
///
/// More robust to outliers than CombMAX or CombSUM.
#[must_use]
pub fn combmed<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    combmed_multi(&[results_a, results_b], FusionConfig::default())
}

/// CombMED for 3+ result lists.
#[must_use]
pub fn combmed_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let mut score_lists: HashMap<I, Vec<f32>> = HashMap::new();

    for list in lists {
        for (id, s) in list.as_ref() {
            score_lists.entry(id.clone()).or_default().push(*s);
        }
    }

    let mut scores: HashMap<I, f32> = HashMap::new();
    for (id, mut score_vec) in score_lists {
        score_vec.sort_by(|a, b| a.total_cmp(b));
        let median = if score_vec.len() % 2 == 0 {
            let mid = score_vec.len() / 2;
            (score_vec[mid - 1] + score_vec[mid]) / 2.0
        } else {
            score_vec[score_vec.len() / 2]
        };
        scores.insert(id, median);
    }

    finalize(scores, config.top_k)
}

/// CombANZ: average of non-zero scores.
///
/// Formula: `score(d) = mean(s_r(d))` for all retrievers r containing d.
///
/// Similar to CombSUM but divides by count (average instead of sum).
#[must_use]
pub fn combanz<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    combanz_multi(&[results_a, results_b], FusionConfig::default())
}

/// CombANZ for 3+ result lists.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn combanz_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }
    let mut scores: HashMap<I, (f32, usize)> = HashMap::new();

    for list in lists {
        for (id, s) in list.as_ref() {
            let entry = scores.entry(id.clone()).or_insert((0.0, 0));
            entry.0 += s;
            entry.1 += 1;
        }
    }

    let mut results: Vec<_> = scores
        .into_iter()
        .map(|(id, (sum, count))| {
            // count is always >= 1 because we only add entries when we see items
            debug_assert!(count > 0, "Count should always be > 0 for CombANZ");
            (id, sum / count as f32)
        })
        .collect();
    sort_scored_desc(&mut results);
    if let Some(top_k) = config.top_k {
        results.truncate(top_k);
    }
    results
}

/// Rank-Biased Centroids (RBC) fusion.
///
/// Weights ranks with a geometric series that does not depend on list length,
/// so lists of different lengths contribute comparably.
///
/// Formula: `score(d) = Σ (1 - p) * p^rank` where:
/// - `p` is the persistence parameter (default 0.8; higher = less top-heavy,
///   expected inspection depth is `1 / (1 - p)`)
/// - `rank` is 0-indexed
///
/// From Bailey et al. (2017), weight `(1 - φ) φ^(r-1)` at 1-indexed rank `r`.
#[must_use]
pub fn rbc<I: Clone + Eq + Hash>(results_a: &[(I, f32)], results_b: &[(I, f32)]) -> Vec<(I, f32)> {
    rbc_multi(&[results_a, results_b], 0.8)
}

/// RBC for 3+ result lists with custom persistence.
///
/// # Arguments
/// * `lists` - Ranked lists to fuse
/// * `persistence` - Persistence parameter (0.0-1.0), default 0.8. Higher = less top-heavy.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn rbc_multi<I, L>(lists: &[L], persistence: f32) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }

    let p = persistence.clamp(0.0, 1.0);
    let mut scores: HashMap<I, f32> = HashMap::new();

    for list in lists {
        let items = list.as_ref();
        for (rank, (id, _)) in items.iter().enumerate() {
            let contribution = (1.0 - p) * p.powi(rank as i32);

            *scores.entry(id.clone()).or_insert(0.0) += contribution;
        }
    }

    finalize(scores, None)
}

/// Condorcet fusion (pairwise comparison voting).
///
/// For each pair of documents, counts how many retrievers prefer one over the other.
/// Documents that beat all others in pairwise comparisons win.
///
/// This is a simplified Condorcet method. Full Condorcet (Kemeny optimal) is NP-hard.
///
/// # Algorithm
///
/// 1. For each document pair (d1, d2), count retrievers where d1 ranks higher than d2
/// 2. Document d1 "beats" d2 if majority of retrievers prefer d1
/// 3. Score = number of documents that this document beats
///
/// More robust to outliers than score-based methods.
#[must_use]
pub fn condorcet<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    condorcet_multi(&[results_a, results_b], FusionConfig::default())
}

/// Condorcet for 3+ result lists.
#[must_use]
pub fn condorcet_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }

    // Build rank maps: doc_id -> rank in each list
    let mut doc_ranks: HashMap<I, Vec<Option<usize>>> = HashMap::new();
    let mut all_docs: std::collections::HashSet<I> = std::collections::HashSet::new();

    for list in lists {
        let items = list.as_ref();
        for doc_id in items.iter().map(|(id, _)| id) {
            all_docs.insert(doc_id.clone());
        }
    }

    // Initialize all docs with None ranks
    for doc_id in &all_docs {
        doc_ranks.insert(doc_id.clone(), vec![None; lists.len()]);
    }

    // Fill in actual ranks
    for (list_idx, list) in lists.iter().enumerate() {
        for (rank, (id, _)) in list.as_ref().iter().enumerate() {
            if let Some(ranks) = doc_ranks.get_mut(id) {
                ranks[list_idx] = Some(rank);
            }
        }
    }

    // For each document, count how many others it beats
    let mut scores: HashMap<I, f32> = HashMap::new();
    let doc_vec: Vec<I> = all_docs.into_iter().collect();

    for (i, d1) in doc_vec.iter().enumerate() {
        let mut wins = 0;

        for (j, d2) in doc_vec.iter().enumerate() {
            if i == j {
                continue;
            }

            // Count lists where d1 ranks better than d2
            let d1_ranks = &doc_ranks[d1];
            let d2_ranks = &doc_ranks[d2];

            let mut d1_wins = 0;
            for (r1, r2) in d1_ranks.iter().zip(d2_ranks.iter()) {
                match (r1, r2) {
                    (Some(rank1), Some(rank2)) if rank1 < rank2 => d1_wins += 1,
                    (Some(_), None) => d1_wins += 1, // d1 present, d2 not
                    _ => {}
                }
            }

            // Majority wins
            if d1_wins > lists.len() / 2 {
                wins += 1;
            }
        }

        scores.insert(d1.clone(), wins as f32);
    }

    finalize(scores, config.top_k)
}

/// Copeland fusion -- pairwise net wins across ranked lists.
///
/// For each document pair (d1, d2), counts how many input lists rank d1 above d2.
/// Score = (pairwise wins) - (pairwise losses). This provides a complete ranking
/// where Condorcet only counts wins.
///
/// Copeland is more discriminative than Condorcet and Borda, and satisfies the
/// Condorcet winner criterion (a document beating all others pairwise always ranks first).
///
/// # Reference
///
/// Tyomkin & Kurland, "Analyzing Fusion Methods Using the Condorcet Rule," SIGIR 2024.
/// Shows Copeland beats both CondorcetFuse and Borda on TREC tracks.
///
/// # Complexity
///
/// O(n^2 * m) where n = total documents, m = number of input lists.
#[must_use]
pub fn copeland<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    copeland_multi(&[results_a, results_b], FusionConfig::default())
}

/// Copeland fusion for 3+ result lists.
#[must_use]
pub fn copeland_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }

    // Build rank maps: doc_id -> rank in each list (None = absent)
    let mut doc_ranks: HashMap<I, Vec<Option<usize>>> = HashMap::new();
    let mut all_docs: std::collections::HashSet<I> = std::collections::HashSet::new();

    for list in lists {
        for (id, _) in list.as_ref() {
            all_docs.insert(id.clone());
        }
    }

    for doc_id in &all_docs {
        doc_ranks.insert(doc_id.clone(), vec![None; lists.len()]);
    }

    for (list_idx, list) in lists.iter().enumerate() {
        for (rank, (id, _)) in list.as_ref().iter().enumerate() {
            if let Some(ranks) = doc_ranks.get_mut(id) {
                ranks[list_idx] = Some(rank);
            }
        }
    }

    // Copeland: score = wins - losses (net pairwise preference)
    let mut scores: HashMap<I, f32> = HashMap::new();
    let doc_vec: Vec<I> = all_docs.into_iter().collect();

    for (i, d1) in doc_vec.iter().enumerate() {
        let mut net = 0i32;

        for (j, d2) in doc_vec.iter().enumerate() {
            if i == j {
                continue;
            }

            let d1_ranks = &doc_ranks[d1];
            let d2_ranks = &doc_ranks[d2];

            let mut d1_preferred = 0;
            let mut d2_preferred = 0;

            for (r1, r2) in d1_ranks.iter().zip(d2_ranks.iter()) {
                match (r1, r2) {
                    (Some(rank1), Some(rank2)) => {
                        if rank1 < rank2 {
                            d1_preferred += 1;
                        } else if rank2 < rank1 {
                            d2_preferred += 1;
                        }
                    }
                    (Some(_), None) => d1_preferred += 1, // present beats absent
                    (None, Some(_)) => d2_preferred += 1,
                    (None, None) => {}
                }
            }

            // Majority rule
            if d1_preferred > d2_preferred {
                net += 1; // win
            } else if d2_preferred > d1_preferred {
                net -= 1; // loss
            }
            // tie: net unchanged
        }

        scores.insert(d1.clone(), net as f32);
    }

    finalize(scores, config.top_k)
}

/// Median Rank Aggregation.
///
/// Scores each document by the median of its ranks across all input lists.
/// Documents not in a list receive a penalty rank of `max_rank + 1`.
///
/// Lower median rank = higher fusion score. The output is normalized to
/// descending scores (higher = better) for consistency with other fusion methods.
///
/// Outlier-robust: a single bad retriever rank has minimal effect on the median.
#[must_use]
pub fn median_rank<I: Clone + Eq + Hash>(
    results_a: &[(I, f32)],
    results_b: &[(I, f32)],
) -> Vec<(I, f32)> {
    median_rank_multi(&[results_a, results_b], FusionConfig::default())
}

/// Median Rank Aggregation for 3+ result lists.
#[must_use]
pub fn median_rank_multi<I, L>(lists: &[L], config: FusionConfig) -> Vec<(I, f32)>
where
    I: Clone + Eq + Hash,
    L: AsRef<[(I, f32)]>,
{
    if lists.is_empty() {
        return Vec::new();
    }

    // Find penalty rank: max list length + 1
    let max_len = lists.iter().map(|l| l.as_ref().len()).max().unwrap_or(0);
    let penalty_rank = max_len + 1;

    // Collect all docs and their ranks
    let mut doc_ranks: HashMap<I, Vec<usize>> = HashMap::new();

    for list in lists {
        for (rank, (id, _)) in list.as_ref().iter().enumerate() {
            doc_ranks.entry(id.clone()).or_default().push(rank);
        }
    }

    // Compute median rank for each doc, converting to a descending score
    let mut scores: HashMap<I, f32> = HashMap::new();

    for (id, mut ranks) in doc_ranks {
        // Pad with penalty_rank for lists where doc is absent
        while ranks.len() < lists.len() {
            ranks.push(penalty_rank);
        }
        ranks.sort_unstable();

        let median = if ranks.len() % 2 == 1 {
            ranks[ranks.len() / 2] as f32
        } else {
            (ranks[ranks.len() / 2 - 1] + ranks[ranks.len() / 2]) as f32 / 2.0
        };

        // Invert: lower median rank -> higher score
        // Use 1/(1+median) so scores are in (0, 1] and descending
        scores.insert(id, 1.0 / (1.0 + median));
    }

    finalize(scores, config.top_k)
}
