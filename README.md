# pcm-bit-detect
Exercise program in Rust to detect file type of raw audio 16/24 bit PCM files

# Installation

Install latest Rust: https://rustup.rs/

# Compile & run

Example run (test-s24be.pcm is included):

```sh
cargo run test-s24be.pcm
```
Output:
```
…snip…
     Running `target/debug/pcm-bit-detect test-s24be.pcm`
test-s24be.pcm: signed 24bit big-endian
```

# Tests

Unit tests can be run by:

```sh
cargo test
```
