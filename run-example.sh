#!/bin/bash
cargo build || exit 1
RUST_LOG=info WOLFRAM_ID=INSERT_ID_HERE DISCORD_TOKEN=INSERT_TOKEN_HERE ./target/debug/rossbot