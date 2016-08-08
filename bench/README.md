Due to `criterion` crate this should be built and run with nightly Rust:
```
cd bench
rustup override set nightly
cargo test --test bench --release -- --nocapture --ignored --test-threads 1
```