// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! The single fusion entry point (contract D1) and its input and option types (D4, D5, D7, D11, D13).
//!
//! [`fuse`] is the one implementation path: it validates every list (D9, D10), the weights (D13) and the method
//! configuration (D11), computes per-list contributions, accumulates them in a fixed order (list order, then item order,
//! D14), applies `veto` and `force` (D7), orders by the one total order (D5) and then truncates to `top_k`.
//!
//! Rank methods ([`RankMethod`]) take [`Ranked`] lists, score methods ([`ScoreMethod`]) take [`Scored`] lists. The method
//! type is chosen by the list type, so a rank method can never be handed a score it would ignore (D4).
//!
//! ```
//! use sortant::fuse::{fuse, Options, RankMethod, Ranked};
//!
//! let bm25 = Ranked::new(vec!["d1", "d2"]);
//! let dense = Ranked::new(vec!["d2", "d3"]);
//! let fused = fuse(&[bm25, dense], &RankMethod::Rrf { k: 60.0 }, &Options::new()).unwrap();
//! assert_eq!(fused.items[0].0, "d2"); // in both lists
//! ```

use crate::error::{FuseError, WeightProblem};
use crate::input::{check_ranked, check_scored};
use crate::norm::{dbsf, min_max, standardize};
use crate::order::{canonical_score, score_descending};
use crate::sort::stable_sort_by;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// A ranked list, best item first, with no scores (contract D4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked<I> {
    items: Vec<I>,
}

impl<I> Ranked<I> {
    /// Wraps `items`, best first. Duplicates are reported by [`fuse`] as [`FuseError::DuplicateId`].
    #[must_use]
    pub fn new(items: Vec<I>) -> Self {
        Self { items }
    }

    /// The items, best first.
    #[must_use]
    pub fn as_slice(&self) -> &[I] {
        &self.items
    }

    /// Number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when the list has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A scored list: each item has a finite `f64` score, in any order (contract D4, D8).
#[derive(Debug, Clone, PartialEq)]
pub struct Scored<I> {
    items: Vec<(I, f64)>,
}

impl<I> Scored<I> {
    /// Wraps `(id, score)` pairs. Non-finite scores and duplicate ids are reported by [`fuse`].
    #[must_use]
    pub fn new(items: Vec<(I, f64)>) -> Self {
        Self { items }
    }

    /// The `(id, score)` pairs in the order given.
    #[must_use]
    pub fn as_slice(&self) -> &[(I, f64)] {
        &self.items
    }

    /// Number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when the list has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<I: Clone + Eq + Hash> Scored<I> {
    /// Converts to a [`Ranked`] list: validates (D9, D10), then orders by score descending, ties in input order (D5).
    ///
    /// # Errors
    ///
    /// [`FuseError::NonFiniteScore`] or [`FuseError::DuplicateId`], with list index 0.
    pub fn into_ranked(self) -> Result<Ranked<I>, FuseError> {
        let checked = check_scored(0, &self.items)?;
        let sorted = stable_sort_by(checked, |a, b| score_descending(a.1, b.1));
        Ok(Ranked::new(sorted.into_iter().map(|(id, _)| id).collect()))
    }
}

/// Methods that read positions in [`Ranked`] lists.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum RankMethod {
    /// Reciprocal rank fusion: each list adds `weight / (k + rank)` with 1-indexed `rank`. `k` must be finite and
    /// `> 0` ([`FuseError::InvalidConfig`] otherwise); 60 is the usual choice.
    Rrf {
        /// The smoothing constant.
        k: f64,
    },
    /// Borda count: an item at 0-indexed position `p` in a list of `n` items adds `weight * (n - p)` (the first item
    /// gets `n`, the last gets 1). Items absent from a list get 0 from it.
    Borda,
    /// Inverse square-root rank: each list adds `weight / sqrt(k + pos)` with 0-indexed `pos`. This is the formula
    /// rankops 0.3.0 uses; it is not the inverse-square-rank of Mourao et al. `k` must be finite and `> 0`
    /// ([`FuseError::InvalidConfig`] otherwise), which also keeps `pos = 0` from dividing by zero. Uses only `sqrt`,
    /// which IEEE-754 rounds exactly (contract D14).
    Isr {
        /// The smoothing constant.
        k: f64,
    },
}

/// Methods that read scores in [`Scored`] lists. Every method but `Dbsf`, `Standardized` and the raw `CombMax/Min/Med/Anz` min-max normalises each list first (contract D16, D17:
/// a constant list normalises to 0.5, a range that overflows is [`FuseError::RangeOverflow`]); absent items get 0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ScoreMethod {
    /// Weighted sum of normalised scores with weights scaled to sum to 1 (equal weights when none are given).
    Weighted,
    /// Sum of normalised scores, each list scaled by its weight as given (1 when none are given).
    CombSum,
    /// `CombSum` multiplied by the number of lists that contain the item (the lists, because a duplicate id inside one
    /// list is an error, contract D10).
    CombMnz,
    /// Distribution-based score fusion: each list is normalised by its own mean and population standard deviation
    /// (`z = (s - mean) / std` clamped to `[-3, 3]`, then `z / 6 + 0.5`; see [`crate::norm::dbsf`]) instead of
    /// min-max, then summed with the weights as given. A list with `std <= 1e-9` normalises to 0.5.
    Dbsf,
    /// Standardised score fusion: each list becomes z-scores (mean, population standard deviation) clamped to
    /// `[clip_min, clip_max]`, summed with the weights as given. Absent items add 0, which is a mid-range z-score;
    /// a constant list (`std <= 1e-9`) gives 0 for every item. `clip_min` must be finite and `<= clip_max`, which must
    /// be finite ([`FuseError::InvalidConfig`] names the offending parameter); this fixes rankops' `clamp` panic.
    Standardized {
        /// Lower clip bound.
        clip_min: f64,
        /// Upper clip bound.
        clip_max: f64,
    },
    /// Highest score an item has in any list that contains it. Raw scores, no normalisation (as in rankops), each
    /// multiplied by its list's weight.
    CombMax,
    /// Lowest score an item has in any list that contains it. Raw scores times the list weight.
    CombMin,
    /// Median of an item's scores over the lists that contain it; an even count takes the midpoint of the two middle
    /// values. Raw scores times the list weight.
    CombMed,
    /// Mean of an item's non-zero scores over the lists that contain it (the published CombANZ; rankops averages all
    /// present scores, contract section 6). An item whose scores are all zero gets 0. Raw scores times the list weight.
    CombAnz,
}

mod sealed {
    pub trait Sealed {}
}

/// A list type [`fuse`] accepts: [`Ranked`] or [`Scored`]. Sealed; its methods are not public API.
pub trait Source<I>: sealed::Sealed + Sized {
    /// The method type that goes with this list type.
    type Method;

    #[doc(hidden)]
    fn contributions(
        lists: &[Self],
        method: &Self::Method,
        weights: &[f64],
    ) -> Result<Vec<Vec<(I, f64)>>, FuseError>;

    #[doc(hidden)]
    /// Combines one id's contributions, in list order, into its fused score.
    fn finish(method: &Self::Method, contributions: &[f64]) -> f64;

    #[doc(hidden)]
    fn normalise_weights(method: &Self::Method) -> bool;
}

impl<I> sealed::Sealed for Ranked<I> {}
impl<I> sealed::Sealed for Scored<I> {}

impl<I: Clone + Eq + Hash> Source<I> for Ranked<I> {
    type Method = RankMethod;

    fn contributions(
        lists: &[Self],
        method: &RankMethod,
        weights: &[f64],
    ) -> Result<Vec<Vec<(I, f64)>>, FuseError> {
        if let RankMethod::Rrf { k } | RankMethod::Isr { k } = *method
            && !(k.is_finite() && k > 0.0)
        {
            return Err(FuseError::InvalidConfig {
                parameter: "k",
                value: k,
            });
        }
        let mut out = Vec::with_capacity(lists.len());
        for (list, ranked) in lists.iter().enumerate() {
            check_ranked(list, &ranked.items)?;
            let w = weights[list];
            let n = ranked.items.len();
            let row = ranked
                .items
                .iter()
                .enumerate()
                .map(|(pos, id)| {
                    let c = match *method {
                        // 1-indexed rank = pos + 1; usize -> f64 is exact below 2^53 (D15).
                        RankMethod::Rrf { k } => w / (k + (pos + 1) as f64),
                        RankMethod::Borda => w * (n - pos) as f64,
                        RankMethod::Isr { k } => w / (k + pos as f64).sqrt(),
                    };
                    (id.clone(), c)
                })
                .collect();
            out.push(row);
        }
        Ok(out)
    }

    fn finish(_: &RankMethod, contributions: &[f64]) -> f64 {
        sum_in_order(contributions)
    }

    fn normalise_weights(_: &RankMethod) -> bool {
        false
    }
}

impl<I: Clone + Eq + Hash> Source<I> for Scored<I> {
    type Method = ScoreMethod;

    fn contributions(
        lists: &[Self],
        method: &ScoreMethod,
        weights: &[f64],
    ) -> Result<Vec<Vec<(I, f64)>>, FuseError> {
        if let ScoreMethod::Standardized { clip_min, clip_max } = *method {
            for (parameter, value) in [("clip_min", clip_min), ("clip_max", clip_max)] {
                if !value.is_finite() {
                    return Err(FuseError::InvalidConfig { parameter, value });
                }
            }
            if clip_min > clip_max {
                return Err(FuseError::InvalidConfig {
                    parameter: "clip_min",
                    value: clip_min,
                });
            }
        }
        let mut out = Vec::with_capacity(lists.len());
        for (list, scored) in lists.iter().enumerate() {
            let checked = check_scored(list, &scored.items)?;
            let scores: Vec<f64> = checked.iter().map(|x| x.1).collect();
            let norm = match method {
                ScoreMethod::CombMax
                | ScoreMethod::CombMin
                | ScoreMethod::CombMed
                | ScoreMethod::CombAnz => scores.clone(),
                ScoreMethod::Dbsf => dbsf(list, &scores)?,
                ScoreMethod::Standardized { clip_min, clip_max } => {
                    standardize(list, &scores, *clip_min, *clip_max)?
                }
                _ => min_max(list, &scores)?,
            };
            let w = weights[list];
            out.push(
                checked
                    .into_iter()
                    .zip(norm)
                    .map(|((id, _), n)| (id, w * n))
                    .collect(),
            );
        }
        Ok(out)
    }

    fn finish(method: &ScoreMethod, contributions: &[f64]) -> f64 {
        match method {
            ScoreMethod::CombMnz => sum_in_order(contributions) * contributions.len() as f64,
            ScoreMethod::CombMax => contributions
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
            ScoreMethod::CombMin => contributions.iter().copied().fold(f64::INFINITY, f64::min),
            ScoreMethod::CombMed => median(contributions),
            ScoreMethod::CombAnz => {
                let nonzero: Vec<f64> = contributions
                    .iter()
                    .copied()
                    .filter(|&c| c != 0.0)
                    .collect();
                if nonzero.is_empty() {
                    0.0
                } else {
                    sum_in_order(&nonzero) / nonzero.len() as f64
                }
            }
            _ => sum_in_order(contributions),
        }
    }

    fn normalise_weights(method: &ScoreMethod) -> bool {
        matches!(method, ScoreMethod::Weighted)
    }
}

/// Sums in the given order, starting from the first value (not from 0.0), as the pre-vector accumulator did.
fn sum_in_order(values: &[f64]) -> f64 {
    let mut it = values.iter().copied();
    let first = it.next().unwrap_or(0.0);
    it.fold(first, |acc, v| acc + v)
}

/// Median of finite values; an even count takes the midpoint of the two middle values without overflowing.
fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        return v[n / 2];
    }
    let (a, b) = (v[n / 2 - 1], v[n / 2]);
    if a <= 0.0 && b >= 0.0 {
        // opposite signs (or zero): the sum cannot overflow
        (a + b) / 2.0
    } else {
        // same sign: b - a cannot overflow
        a + (b - a) / 2.0
    }
}

type Comparator<I> = Box<dyn Fn(&I, &I) -> Ordering>;

/// How equal fused scores are broken (contract D5). Without one, ties keep input order.
enum TieBreak<I> {
    Ranker(HashMap<I, usize>),
    Comparator(Comparator<I>),
}

/// Options for [`fuse`]. Build with [`Options::new`] and the chained setters.
pub struct Options<I> {
    top_k: Option<usize>,
    weights: Option<Vec<f64>>,
    tie_break: Option<TieBreak<I>>,
    veto: HashSet<I>,
    force: HashSet<I>,
}

impl<I> Default for Options<I> {
    fn default() -> Self {
        Self {
            top_k: None,
            weights: None,
            tie_break: None,
            veto: HashSet::new(),
            force: HashSet::new(),
        }
    }
}

impl<I> Options<I> {
    /// Defaults: no `top_k`, one weight of 1 per list, ties in input order, no veto, no force.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep at most `n` items, after veto and force. `0` gives an empty result, not an error (contract D11).
    #[must_use]
    pub fn top_k(mut self, n: usize) -> Self {
        self.top_k = Some(n);
        self
    }

    /// One weight per list, each finite and `>= 0`, the sum `> 0`, the count equal to the list count; otherwise
    /// [`FuseError::InvalidWeights`] or [`FuseError::LengthMismatch`]. A weight is never replaced or ignored (D13).
    #[must_use]
    pub fn weights(mut self, weights: Vec<f64>) -> Self {
        self.weights = Some(weights);
        self
    }

    /// Break equal scores by position in `ranker` (best first); items the ranker lacks come last; if the ranker repeats
    /// an id its first position counts.
    #[must_use]
    pub fn tie_break_ranker(mut self, ranker: &Ranked<I>) -> Self
    where
        I: Clone + Eq + Hash,
    {
        let mut pos = HashMap::with_capacity(ranker.items.len());
        for (i, id) in ranker.items.iter().enumerate() {
            pos.entry(id.clone()).or_insert(i);
        }
        self.tie_break = Some(TieBreak::Ranker(pos));
        self
    }

    /// Break equal scores with `cmp` (`Less` puts the first argument earlier). It is run by SortAnt's own safe sort,
    /// so an inconsistent comparator cannot panic (its order is then unspecified but deterministic); a closure that
    /// itself panics propagates.
    #[must_use]
    pub fn tie_break_by(mut self, cmp: impl Fn(&I, &I) -> Ordering + 'static) -> Self {
        self.tie_break = Some(TieBreak::Comparator(Box::new(cmp)));
        self
    }

    /// Items that must not appear in the output (contract D7). Veto beats force.
    #[must_use]
    pub fn veto(mut self, ids: impl IntoIterator<Item = I>) -> Self
    where
        I: Eq + Hash,
    {
        self.veto.extend(ids);
        self
    }

    /// Items placed before all unforced items, ordered among themselves by the usual order (D7). They count toward
    /// `top_k`.
    #[must_use]
    pub fn force(mut self, ids: impl IntoIterator<Item = I>) -> Self
    where
        I: Eq + Hash,
    {
        self.force.extend(ids);
        self
    }
}

/// The result of [`fuse`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Fused<I> {
    /// `(id, fused score)`, best first. The score is never `-0.0` (amendment A2).
    pub items: Vec<(I, f64)>,
    /// Ids that were in both `veto` and `force` and appear in the input, in first-appearance order. They are vetoed.
    pub conflicts: Vec<I>,
}

fn check_weights(weights: Option<&[f64]>, lists: usize) -> Result<Vec<f64>, FuseError> {
    let Some(w) = weights else {
        return Ok(vec![1.0; lists]);
    };
    if w.len() != lists {
        return Err(FuseError::LengthMismatch {
            expected: lists,
            got: w.len(),
        });
    }
    for (index, &x) in w.iter().enumerate() {
        if !x.is_finite() {
            return Err(FuseError::InvalidWeights(WeightProblem::NotFinite {
                index,
            }));
        }
        if x < 0.0 {
            return Err(FuseError::InvalidWeights(WeightProblem::Negative { index }));
        }
    }
    let sum: f64 = w.iter().sum();
    if sum == 0.0 {
        return Err(FuseError::InvalidWeights(WeightProblem::ZeroSum));
    }
    if !sum.is_finite() {
        return Err(FuseError::InvalidConfig {
            parameter: "sum of weights",
            value: sum,
        });
    }
    Ok(w.to_vec())
}

/// Fuses `lists` with `method` under `options`.
///
/// Output order (contract D5): forced items first; then fused score descending; then the tie-break ranker or
/// comparator; then first appearance (lists in the order given, items in list order). Accumulation is in the same
/// fixed order, so the result is bit-identical on every platform (D14).
///
/// No list at all is `Ok` with an empty result.
///
/// # Errors
///
/// [`FuseError::NonFiniteScore`], [`FuseError::DuplicateId`], [`FuseError::InvalidWeights`],
/// [`FuseError::LengthMismatch`], [`FuseError::InvalidConfig`], [`FuseError::RangeOverflow`]. Validation covers every
/// list, including items that are later vetoed. Nothing panics except a tie-break closure that itself panics.
pub fn fuse<I, L>(
    lists: &[L],
    method: &L::Method,
    options: &Options<I>,
) -> Result<Fused<I>, FuseError>
where
    I: Clone + Eq + Hash,
    L: Source<I>,
{
    let mut weights = check_weights(options.weights.as_deref(), lists.len())?;
    if L::normalise_weights(method) {
        // check_weights guarantees a finite sum > 0 whenever there is a list; with no lists the loop body never runs.
        let total: f64 = weights.iter().sum();
        for w in &mut weights {
            *w /= total;
        }
    }
    let rows = L::contributions(lists, method, &weights)?;

    // Accumulate in list order, then item order; the map indexes, it never orders (D14).
    let mut index: HashMap<I, usize> = HashMap::new();
    let mut acc: Vec<(I, Vec<f64>)> = Vec::new();
    for row in rows {
        for (id, c) in row {
            match index.get(&id) {
                Some(&i) => acc[i].1.push(c),
                None => {
                    index.insert(id.clone(), acc.len());
                    acc.push((id, vec![c]));
                }
            }
        }
    }

    let conflicts: Vec<I> = acc
        .iter()
        .filter(|(id, _)| options.veto.contains(id) && options.force.contains(id))
        .map(|(id, _)| id.clone())
        .collect();

    let entries: Vec<(I, f64, bool)> = acc
        .into_iter()
        .filter(|(id, _)| !options.veto.contains(id))
        .map(|(id, contributions)| {
            let score = canonical_score(L::finish(method, &contributions));
            let forced = options.force.contains(&id);
            (id, score, forced)
        })
        .collect();

    let sorted = stable_sort_by(entries, |a, b| {
        b.2.cmp(&a.2)
            .then_with(|| score_descending(a.1, b.1))
            .then_with(|| match &options.tie_break {
                None => Ordering::Equal,
                Some(TieBreak::Comparator(f)) => f(&a.0, &b.0),
                Some(TieBreak::Ranker(pos)) => {
                    let p = |id: &I| pos.get(id).copied().unwrap_or(usize::MAX);
                    p(&a.0).cmp(&p(&b.0))
                }
            })
    });

    let limit = options.top_k.unwrap_or(usize::MAX);
    let items = sorted
        .into_iter()
        .take(limit)
        .map(|(id, score, _)| (id, score))
        .collect();
    Ok(Fused { items, conflicts })
}

// ---- wrappers (contract D1) ---------------------------------------------------------------------------------------
//
// Each wrapper is one call to `fuse` with default options and no logic of its own. They live in this module, not at the
// crate root, while the old f32 functions of the same names still exist there.

/// Fuses two ranked lists with reciprocal rank fusion at `k = 60`. Same as [`fuse`] with [`RankMethod::Rrf`] and default
/// [`Options`].
///
/// ```
/// use sortant::fuse::{rrf, Ranked};
///
/// let fused = rrf(&Ranked::new(vec!["a", "b"]), &Ranked::new(vec!["b", "c"])).unwrap();
/// assert_eq!(fused.items[0].0, "b");
/// ```
///
/// # Errors
///
/// [`FuseError::DuplicateId`] if an id repeats within one list.
pub fn rrf<I: Clone + Eq + Hash>(a: &Ranked<I>, b: &Ranked<I>) -> Result<Fused<I>, FuseError> {
    fuse(
        &[a.clone(), b.clone()],
        &RankMethod::Rrf { k: 60.0 },
        &Options::new(),
    )
}

/// Fuses two ranked lists with a Borda count. Same as [`fuse`] with [`RankMethod::Borda`] and default [`Options`].
///
/// # Errors
///
/// [`FuseError::DuplicateId`] if an id repeats within one list.
pub fn borda<I: Clone + Eq + Hash>(a: &Ranked<I>, b: &Ranked<I>) -> Result<Fused<I>, FuseError> {
    fuse(&[a.clone(), b.clone()], &RankMethod::Borda, &Options::new())
}

/// Fuses two scored lists by the equal-weight sum of min-max normalised scores. Same as [`fuse`] with
/// [`ScoreMethod::Weighted`] and default [`Options`].
///
/// ```
/// use sortant::fuse::{weighted, Scored};
///
/// let a = Scored::new(vec![("a", 3.0), ("b", 2.0), ("c", 1.0)]);
/// let b = Scored::new(vec![("b", 9.0), ("c", 3.0)]);
/// let fused = weighted(&a, &b).unwrap();
/// assert_eq!(fused.items[0].0, "b");
/// ```
///
/// # Errors
///
/// [`FuseError::NonFiniteScore`], [`FuseError::DuplicateId`] or [`FuseError::RangeOverflow`] from the lists.
pub fn weighted<I: Clone + Eq + Hash>(a: &Scored<I>, b: &Scored<I>) -> Result<Fused<I>, FuseError> {
    fuse(
        &[a.clone(), b.clone()],
        &ScoreMethod::Weighted,
        &Options::new(),
    )
}

/// Fuses two scored lists with CombSUM. Same as [`fuse`] with [`ScoreMethod::CombSum`] and default [`Options`].
///
/// # Errors
///
/// As [`weighted`].
pub fn combsum<I: Clone + Eq + Hash>(a: &Scored<I>, b: &Scored<I>) -> Result<Fused<I>, FuseError> {
    fuse(
        &[a.clone(), b.clone()],
        &ScoreMethod::CombSum,
        &Options::new(),
    )
}

/// Fuses two scored lists with CombMNZ. Same as [`fuse`] with [`ScoreMethod::CombMnz`] and default [`Options`].
///
/// # Errors
///
/// As [`weighted`].
pub fn combmnz<I: Clone + Eq + Hash>(a: &Scored<I>, b: &Scored<I>) -> Result<Fused<I>, FuseError> {
    fuse(
        &[a.clone(), b.clone()],
        &ScoreMethod::CombMnz,
        &Options::new(),
    )
}
