# Building

Install stable Rust, then run from the repository root:

```sh
cargo build --locked --release
cargo test --locked
cargo fmt --check
```

The standalone executable is `target/release/family-policy` (`.exe` on Windows).
No GUI toolkit or JavaScript tools are required. CI builds native macOS ARM64
(including M4), Linux x86_64, and Windows MSVC x86_64 binaries.

Push a `v<version>` tag after updating Cargo.toml and Cargo.lock to publish a
release after the platform test and build matrix succeeds.
