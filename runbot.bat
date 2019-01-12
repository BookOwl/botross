setlocal
    cargo build || exit /b
    set RUST_LOG=info 
    set DISCORD_TOKEN=INSERT_TOKEN_HERE 
    ./target/debug/rossbot
endlocal