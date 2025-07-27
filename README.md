# Tree Clocks

Various Tree Clock data structures including the original Interval Tree Clock data structure from the 2008 paper "Interval Tree Clocks: A Logical Clock for Dynamic Systems"[[pdf](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf)] as well as extensions on the idea.


## Features

- Implementation of the `IdTree` and `EventTree` from the original paper
- A higher-level `ItcPair` abstraction for ease of use
- A new `ItcIndex` to go from `EventTree` to `Set<IdTree>`

## Usage

Add to your `Cargo.toml`:

`$ cargo add treeclocks`

### `ItcPair`

The `ItcPair` provides a higher-level abstraction around the `IdTree` and `EventTree` data-structures.

```rust,no_run
use treeclocks::ItcPair;

let mut n0 = ItcPair::new();
let mut n1 = n0.fork();
let mut n2 = n1.fork();

n0.event();
n0.event();
n2.event();

// Sync to update the `EventTree` without merging `IdTree`s
n1.sync(&n2.timestamp);

// Join to merge both `EventTree` and `IdTree`s
n0.join(n2);

assert!(n0.timestamp > n1.timestamp);
```

### `ItcMap`

The `ItcMap` provides an in-memory key-value store with operations to keep the map in sync with minimal overhead in a distributed system.

```rust
use treeclocks::{IdTree, ItcMap};

let mut map_a = ItcMap::new();
let mut map_b = ItcMap::new();

let id_a = IdTree::one();
let (id_a, id_b) = id_a.fork();

map_a.insert(id_a.clone(), 207);
map_b.insert(id_b.clone(), 324);

let patch_for_b = map_a.diff(map_b.timestamp());
let patch_for_a = map_b.diff(map_a.timestamp());

map_a.apply(patch_for_a);
map_b.apply(patch_for_b);

assert_eq!(map_a, map_b)
```

## License

Licensed under either of:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
