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

```rust
use treeclocks::ItcPair;

let n0 = ItcPair::new();

let n1 = n0.fork();
let n2 = n0.fork();

n0.event();
n1.event();

n2.sync(&n0);
n2.sync(&n1);

n2.event();

n0.sync(&n1);
```

### `ItcIndex`

```rust
index = ItcIndex::new();



diff = my_timestamp - other_timestamp;
let partial = index.get_partial(&diff);
partial.keys() -> Iter<T>

index.sync(&partial_index);

index.insert(IdTree, T);
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
