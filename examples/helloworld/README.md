# arceos-helloworld

A simple "Hello, World!" example for [ArceOS](https://github.com/arceos-org/arceos).

## Overview

This crate demonstrates a minimal program that can run either as:
- A standard Rust program (using the standard library)
- A bare-metal application on ArceOS (using `axstd`)

## Usage

### As a Standard Rust Program

```bash
cargo run
```

This will compile and run the program using the standard Rust runtime, printing "Hello, world!".

### On ArceOS (from repository)

To run this example on ArceOS, clone the ArceOS repository and use `make`:

```bash
git clone https://github.com/arceos-org/arceos.git
cd arceos

# First, add axstd dependency to examples/helloworld/Cargo.toml:
# [dependencies]
# axstd = { path = "../../ulib/axstd", optional = true, features = ["defplat", "log-level-warn"] }

make A=examples/helloworld ARCH=riscv64 run
```

### Publishing to crates.io

This crate can be published directly:

```bash
cargo publish --dry-run --allow-dirty  # Test
cargo publish --allow-dirty            # Publish
```

## Features

- `axstd` - Enable ArceOS standard library support. This feature requires the `axstd` crate which is not published to crates.io. When building from the ArceOS repository, add `axstd` as a path dependency.

## Code Structure

The code uses conditional compilation to support both environments:

```rust
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

fn main() {
    println!("Hello, world!");
}
```

## License

This project is licensed under either of:

- GPL-3.0-or-later
- Apache-2.0
- MulanPSL-2.0

at your option.
