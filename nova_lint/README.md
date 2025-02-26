# Linting

This is an additional set of linter rules specific for the Nova engine and any
applications which may be built on top of it. This ensures we conform to our
conventions, best practices and most importantly the requirements of the engine,
more specifically the garbage collector.

For this we use [dylint](https://github.com/trailofbits/dylint) which is a tool
that allows us to write custom lints for our codebase. It is designed to be used
along side [Clippy](https://doc.rust-lang.org/stable/clippy/index.html).

## Usage

1. Install `cargo-dylint` and `dylint-link`:
  ```bash
  cargo install cargo-dylint dylint-link
  ```
2. Run the linter in the root of the project:
  ```bash
  cargo dylint --all
  ```
