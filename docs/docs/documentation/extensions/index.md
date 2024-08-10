---
weight: 0
---

# Introduction

This is a place for the documentation on the usage of each extension implemented in this repository. The currently implemented extensions are at the moment

* [`dummy`](./dummy.md): an extension intended for testing, showcasing and debugging purposes.
* [`lightgbm`](./lightgbm.md): exposes a minimal API of the LightGBM C library for evaluating models in runtime.

## Want to build your own extension?

Then, check out the [`jyafn-ext`](https://crates.io/crates/jyafn-ext) crate. It comes with utilities to easily create JYAFN extensions in pure Rust, without the need for writing unsafe code.