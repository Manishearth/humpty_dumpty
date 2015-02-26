## Humpty Dumpty

The goal of this library is to be able to define types that cannot be implicitly
dropped except in controlled situations.

A sketch of the design can be found [here](https://gist.github.com/Manishearth/045ee457d6f81183ec6b). The design does not handle branches,
though it can be extended to do so.

The idea is, that for a type that is marked `#[drop_protection]`, only functions annotated with `#[allowed_on_protected]` can use these,
and each local *must* be dropped with a function marked `#[protected_drop]` before its scope finishes.

Current status: Just reports declarations and stuff, work in progress