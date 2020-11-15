# Serial Line CAN protocol codec

[![crates.io](https://img.shields.io/crates/v/mini-slcan.svg)](https://crates.io/crates/mini-slcan)
[![docs.rs](https://docs.rs/mini-slcan/badge.svg)](https://docs.rs/mini-slcan/)
![CI](https://github.com/jonas-schievink/mini-slcan/workflows/CI/badge.svg)

This crate implements a protocol encoder and decoder for the Serial Line CAN
(SLCAN) protocol, used for transmitting CAN frames over a serial connection.

Please refer to the [changelog](CHANGELOG.md) to see what changed in the last
releases.

## Usage

Add an entry to your `Cargo.toml`:

```toml
[dependencies]
mini-slcan = "0.0.0"
```

Check the [API Documentation](https://docs.rs/mini-slcan/) for how to use the
crate's functionality.

## Rust version support

This crate supports stable Rust. No guarantees are made beyond that.
