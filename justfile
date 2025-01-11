set shell := ["pwsh", "-Command"]

default: run

# Build the project using cargo
build:
    cargo build --release --locked

run type="debug":
  @cargo run {{ if type == "debug" { "" } else if type == "release" { "--release" } else { error("Type " + type + " doesn't exist") } }}

# Clean the project using cargo
clean:
    cargo clean
