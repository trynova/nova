_default: 
    @just --list -u

# Run the Test262 test suite. Run `just test262 --help` for more information.
test262 *ARGS:
    cargo run --bin test262 -- {{ARGS}}

# Start a REPL session
repl:
    cargo run --bin nova_cli -- repl

# Fix Clippy, rustfmt, and typos issues
fmt:
    cargo clippy --no-deps --all-targets --fix --allow-staged --allow-dirty
    cargo fmt --all
    typos -w
    git status
