## Humpty Dumpty

The goal of this library is to be able to define types that cannot be implicitly
dropped except in controlled situations.

A sketch of the design can be found [here](https://gist.github.com/Manishearth/045ee457d6f81183ec6b). The design does not handle branches,
though it can be extended to do so. It's also a bit different from what I finally implemented

The idea is, that for a type that is marked `#[drop_protection]`, only functions annotated with `#[allowed_on_protected]` can use these,
and each local *must* be dropped with a function marked `#[allowed_drop]` before its scope finishes.

Current status: Is able to track such types and report on their usage. Maintains a list of what has been dropped properly to 
detect implicit drops.

Some missing (but planned) functionality:

 - Cannot yet handle conditional drops, i.e. those in branches. 
 - Cannot yet handle any bindings other than a let binding
 - Allowed functions cannot yet take &/&mut inputs
 - Cannot yet mark method calls as allowed


To test, run `cargo run --example test`, or even better `rustc examples/test.rs -L target` (after building). The latter is better
because it will rebuild every time, and we're only interested in build output.