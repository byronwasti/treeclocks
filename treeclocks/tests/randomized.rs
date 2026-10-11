//! Randomized invariant checks over the public API.
//!
//! These complement the unit tests: rather than pinning specific values, each
//! test drives random operation schedules and asserts a property that must
//! hold for every one of them. Everything is seeded from a fixed-parameter
//! LCG, so a failure reproduces exactly and no dev-dependency is needed.

use treeclocks::{EventTree, IdTree, ItcMap, ItcPair};

/// Deterministic LCG (the constants are Knuth's MMIX). Only the high bits are
/// used, since the low bits of an LCG cycle far too regularly.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }

    /// A random tree up to `depth` levels deep, with small values so that
    /// distinct trees collide often enough to exercise the equality paths.
    fn event_tree(&mut self, depth: u32) -> EventTree {
        if depth == 0 || self.below(3) == 0 {
            EventTree::Leaf(self.below(4) as u64)
        } else {
            EventTree::subtree(
                self.below(3) as u64,
                self.event_tree(depth - 1),
                self.event_tree(depth - 1),
            )
        }
    }
}

/// `event` must always advance the clock, and `sync` must be monotonic and
/// absorb the peer's timestamp, under any interleaving of fork/join/sync.
#[test]
fn pair_event_advances_and_sync_absorbs() {
    for seed in 0..200u64 {
        let mut rng = Rng(seed);
        let mut pairs = vec![ItcPair::new()];

        for _ in 0..60 {
            let i = rng.below(pairs.len());

            match rng.below(4) {
                0 if pairs.len() < 8 => {
                    let p = pairs[i].fork();
                    pairs.push(p);
                }
                1 if pairs.len() > 1 => {
                    let j = rng.below(pairs.len());
                    if i != j {
                        let other = pairs.remove(j);
                        let i = if j < i { i - 1 } else { i };
                        pairs[i].join(other);
                    }
                }
                2 => {
                    let j = rng.below(pairs.len());
                    let peer = pairs[j].timestamp.clone();
                    let before = pairs[i].timestamp.clone();

                    pairs[i].sync(&peer);

                    assert!(pairs[i].timestamp >= before, "seed {seed}: sync regressed");
                    assert!(
                        pairs[i].timestamp >= peer,
                        "seed {seed}: sync did not absorb the peer"
                    );
                }
                _ => {
                    let before = pairs[i].timestamp.clone();

                    pairs[i].event();

                    assert!(
                        pairs[i].timestamp > before,
                        "seed {seed}: event did not advance the clock: {before} -> {}",
                        pairs[i].timestamp
                    );
                }
            }
        }
    }
}

/// Replicas exchanging patches in a random order must end up identical once
/// the gossip settles, no matter how the writes interleaved.
#[test]
fn maps_converge_under_random_gossip() {
    const REPLICAS: usize = 4;

    for seed in 0..200u64 {
        let mut rng = Rng(seed);
        let ids = IdTree::one().fork_many(REPLICAS);
        let mut maps: Vec<ItcMap<u64>> = (0..REPLICAS).map(|_| ItcMap::new()).collect();

        for step in 0..80u64 {
            let i = rng.below(REPLICAS);

            if rng.below(3) == 0 {
                let j = rng.below(REPLICAS);
                if i != j
                    && let Some(patch) = maps[j].diff(maps[i].timestamp())
                {
                    maps[i].apply(patch);
                }
            } else {
                maps[i].insert(ids[i].clone(), step);
            }
        }

        // Gossip to quiescence.
        for _ in 0..40 {
            for i in 0..REPLICAS {
                for j in 0..REPLICAS {
                    if i != j
                        && let Some(patch) = maps[j].diff(maps[i].timestamp())
                    {
                        maps[i].apply(patch);
                    }
                }
            }
        }

        for i in 1..REPLICAS {
            assert_eq!(maps[0], maps[i], "seed {seed}: replicas 0 and {i} diverged");
        }
    }
}

/// `PartialEq`, `PartialOrd` and `Hash` must agree with each other, including
/// on the non-normalized shapes that `subtree` and the parser can build.
#[test]
fn event_tree_eq_ord_and_hash_agree() {
    use std::cmp::Ordering;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash(e: &EventTree) -> u64 {
        let mut state = DefaultHasher::new();
        e.hash(&mut state);
        state.finish()
    }

    let mut rng = Rng(7);

    for _ in 0..20_000 {
        let a = rng.event_tree(3);
        let b = rng.event_tree(3);

        assert_eq!(
            a.partial_cmp(&b) == Some(Ordering::Equal),
            a == b,
            "eq and partial_cmp disagree on {a} vs {b}"
        );

        if a == b {
            assert_eq!(
                hash(&a),
                hash(&b),
                "equal trees hash differently: {a} / {b}"
            );
        }

        assert_eq!(a.partial_cmp(&a), Some(Ordering::Equal), "{a} != itself");

        if let (Some(x), Some(y)) = (a.partial_cmp(&b), b.partial_cmp(&a)) {
            assert_eq!(x, y.reverse(), "asymmetric ordering on {a} vs {b}");
        }
    }
}

/// `contains` must agree with `get`, which answers the same question by an
/// independent route: an id owns events exactly when the slice of the tree it
/// owns is non-zero.
#[test]
fn contains_agrees_with_get() {
    fn ids(rng: &mut Rng, depth: u32) -> IdTree {
        if depth == 0 || rng.below(3) == 0 {
            if rng.below(2) == 0 {
                IdTree::zero()
            } else {
                IdTree::one()
            }
        } else {
            IdTree::subtree(ids(rng, depth - 1), ids(rng, depth - 1))
        }
    }

    let mut rng = Rng(11);

    for _ in 0..20_000 {
        let e = rng.event_tree(3);
        let id = ids(&mut rng, 3);

        assert_eq!(
            e.contains(&id),
            e.clone().get(&id) != EventTree::Leaf(0),
            "contains and get disagree for tree {e} and id {id}"
        );
    }
}

/// `join` is an upper bound on both of its arguments, and `diff` is empty
/// exactly when the left side is already covered by the right.
#[test]
fn join_bounds_and_diff_matches_ordering() {
    let mut rng = Rng(13);

    for _ in 0..20_000 {
        let a = rng.event_tree(3);
        let b = rng.event_tree(3);

        let joined = a.clone().join(b.clone());
        assert!(joined >= a, "join dropped below its left argument: {a}");
        assert!(joined >= b, "join dropped below its right argument: {b}");

        assert_eq!(
            a.clone().diff(&b) == EventTree::Leaf(0),
            a <= b,
            "diff and ordering disagree on {a} vs {b}"
        );
    }
}
