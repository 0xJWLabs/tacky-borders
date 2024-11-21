set shell := ["pwsh", "-Command"]

default: run

# Build the project using cargo
build:
    cargo build --release --locked

# Run the project using cargo
run:
    cargo run

# Clean the project using cargo
clean:
    cargo clean
