# Tree Clocks

Various Tree Clock data structures including the original Interval Tree Clock data structure from the 2008 paper "Interval Tree Clocks: A Logical Clock for Dynamic Systems"[pdf](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) as well as extensions on the idea.

## Usage

```rust
use treeclocks::ItcPair;

let n0 = ItcPair::new();

let n1 = n0.fork();
let n2 = n0.fork();
let n3 = n1.fork();
let n4 = n2.fork();

n1.event();
n3.event();

n0.join(&n1);
```
