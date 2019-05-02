setlocal
    cargo build || exit /b
    set RUST_LOG=info 
    set WOLFRAM_ID=INSERT_ID_HERE
    set DISCORD_TOKEN=INSERT_TOKEN_HERE 
    ./target/debug/rossbot
endlocal