// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! A safe, stable merge sort for caller-supplied comparators (contract D6, amendment A1).
//!
//! `slice::sort_by` may panic when the comparator is not a total order. This sort never does: an inconsistent comparator
//! gives an unspecified but deterministic order, and the result is always a permutation of the input.

use std::cmp::Ordering;

/// Sorts `items` stably by `cmp` (equal items keep their input order).
///
/// Never panics on an inconsistent comparator (intransitive, non-reflexive, ...): the output is then in an unspecified
/// but deterministic order, and is always a permutation of the input. A comparator closure that itself panics
/// propagates the panic. Runs in O(n log n) comparisons and O(n) extra space; contains no `unsafe`.
pub fn stable_sort_by<T, F>(items: Vec<T>, mut cmp: F) -> Vec<T>
where
    F: FnMut(&T, &T) -> Ordering,
{
    let n = items.len();
    let mut src: Vec<usize> = (0..n).collect();
    let mut dst: Vec<usize> = vec![0; n];
    let mut width = 1;
    while width < n {
        let mut start = 0;
        while start < n {
            let mid = (start + width).min(n);
            let end = (start + 2 * width).min(n);
            let (mut l, mut r, mut o) = (start, mid, start);
            while l < mid && r < end {
                // Take the right element only when it is strictly smaller: that keeps equal items in input order.
                if cmp(&items[src[r]], &items[src[l]]) == Ordering::Less {
                    dst[o] = src[r];
                    r += 1;
                } else {
                    dst[o] = src[l];
                    l += 1;
                }
                o += 1;
            }
            while l < mid {
                dst[o] = src[l];
                l += 1;
                o += 1;
            }
            while r < end {
                dst[o] = src[r];
                r += 1;
                o += 1;
            }
            start += 2 * width;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();
    src.into_iter().filter_map(|i| slots[i].take()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random numbers (SplitMix64), so the tests need no dependency and never flake.
    fn rng(seed: u64) -> impl FnMut() -> u64 {
        let mut s = seed;
        move || {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(
            stable_sort_by(Vec::<u8>::new(), |a, b| a.cmp(b)),
            Vec::<u8>::new()
        );
        assert_eq!(stable_sort_by(vec![7], |a, b| a.cmp(b)), vec![7]);
    }

    #[test]
    fn sorts_small_exact() {
        assert_eq!(
            stable_sort_by(vec![3, 1, 2], |a, b| a.cmp(b)),
            vec![1, 2, 3]
        );
        assert_eq!(stable_sort_by(vec![2, 1], |a, b| a.cmp(b)), vec![1, 2]);
        assert_eq!(
            stable_sort_by(vec![5, 4, 3, 2, 1], |a, b| a.cmp(b)),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            stable_sort_by(vec![1, 2, 3, 4, 5, 6, 7], |a, b| b.cmp(a)),
            vec![7, 6, 5, 4, 3, 2, 1]
        );
    }

    #[test]
    fn is_stable_on_equal_keys() {
        // Sort by the first field only; the second records input order.
        let items: Vec<(u8, usize)> = [2, 1, 2, 1, 2, 0, 1, 0]
            .iter()
            .copied()
            .enumerate()
            .map(|(i, k)| (k, i))
            .collect();
        let sorted = stable_sort_by(items, |a, b| a.0.cmp(&b.0));
        assert_eq!(
            sorted,
            vec![
                (0, 5),
                (0, 7),
                (1, 1),
                (1, 3),
                (1, 6),
                (2, 0),
                (2, 2),
                (2, 4)
            ]
        );
    }

    #[test]
    fn matches_std_sort_on_a_total_order_for_thousands_of_items() {
        let mut next = rng(42);
        for &n in &[0usize, 1, 2, 3, 7, 8, 9, 100, 1000, 4097] {
            // Few distinct keys, so stability is exercised heavily; the index makes equal keys distinguishable.
            let items: Vec<(u64, usize)> = (0..n).map(|i| (next() % 13, i)).collect();
            let mut oracle = items.clone();
            oracle.sort_by_key(|a| a.0); // std sort is stable and the order is total
            assert_eq!(
                stable_sort_by(items, |a, b| a.0.cmp(&b.0)),
                oracle,
                "n = {n}"
            );
        }
    }

    fn assert_permutation(original: &[u32], sorted: &[u32]) {
        let (mut a, mut b) = (original.to_vec(), sorted.to_vec());
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
    }

    #[test]
    fn intransitive_comparator_terminates_deterministically_as_a_permutation() {
        // Rock-paper-scissors: 0 < 1 < 2 < 0.
        let cmp = |a: &u32, b: &u32| match (a, b) {
            (x, y) if x == y => Ordering::Equal,
            (0, 1) | (1, 2) | (2, 0) => Ordering::Less,
            _ => Ordering::Greater,
        };
        let mut next = rng(7);
        for n in [0usize, 1, 2, 3, 10, 500] {
            let items: Vec<u32> = (0..n).map(|_| (next() % 3) as u32).collect();
            let first = stable_sort_by(items.clone(), cmp);
            assert_permutation(&items, &first);
            assert_eq!(
                first,
                stable_sort_by(items.clone(), cmp),
                "same output on every run, n = {n}"
            );
        }
    }

    #[test]
    fn non_reflexive_and_random_comparators_do_not_panic() {
        let items: Vec<u32> = (0..300).collect();
        // Always Less, even against itself.
        let a = stable_sort_by(items.clone(), |_, _| Ordering::Less);
        assert_permutation(&items, &a);
        // Always Greater.
        let b = stable_sort_by(items.clone(), |_, _| Ordering::Greater);
        assert_eq!(b, items, "never Less means nothing moves");
        // A comparator driven by a counter, different on every call but deterministic across runs.
        let run = || {
            let mut c = 0u32;
            stable_sort_by(items.clone(), move |_, _| {
                c = c.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                match c >> 30 {
                    0 => Ordering::Less,
                    1 => Ordering::Equal,
                    _ => Ordering::Greater,
                }
            })
        };
        let first = run();
        assert_permutation(&items, &first);
        assert_eq!(first, run());
    }

    #[test]
    fn always_less_reverses_a_two_item_input() {
        // Pins the merge direction: the right element is taken when strictly smaller.
        assert_eq!(
            stable_sort_by(vec![1, 2], |_, _| Ordering::Less),
            vec![2, 1]
        );
        assert_eq!(
            stable_sort_by(vec![1, 2], |_, _| Ordering::Equal),
            vec![1, 2]
        );
    }

    #[test]
    fn a_panicking_comparator_propagates() {
        let r = std::panic::catch_unwind(|| {
            stable_sort_by(vec![1, 2, 3], |_: &i32, _: &i32| panic!("caller closure"))
        });
        assert!(r.is_err());
    }
}
