# mandala_quicksilver

master
[![Master branch build status](https://github.com/N-of-1/mandala-quicksilver/workflows/Rust/badge.svg?branch=master)](https://github.com/N-of-1/mandala-quicksilver/actions) &emsp; dev
[![Test branch build status](https://github.com/N-of-1/mandala-quicksilver/workflows/Rust/badge.svg?branch=dev)](https://github.com/N-of-1/mandala-quicksilver/actions)

Rust library to parse and display an SVG in quicksilver v0.3

This was created for a science display but may be useful in other projects

Example of use:

```
cargo run --example mandala
cargo run --example logo
```

If targeting quicksilver v0.3 on Jetson Nano or Raspberry Pi, edit `Cargo.toml` as labeled there. This is a minimum-effort hack regarding upstream dependencies that should disappear as libraries progress.
