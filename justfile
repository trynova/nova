build profile="dev-fast":
  cargo build --profile {{ profile }}

test262 profile="dev-fast":
  cargo run --bin test262 --profile {{ profile }}

test262-update profile="dev-fast":
  cargo run --bin test262 --profile {{ profile }} -- -u

build-and-test262 profile="dev-fast":
  just build {{ profile }}
  just test262 {{ profile }}

build-and-test262-update profile="dev-fast":
  just build {{ profile }}
  just test262-update {{ profile }}

test262-eval-test path profile="dev-fast":
  cargo run --bin test262 --profile {{ profile }} -- eval-test {{ path }}
