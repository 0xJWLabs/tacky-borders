param(
    [string]$mode = ""
)

cargo clean

if ($mode -eq "release") {
    Write-Output "Building in release mode..."
    cargo build --release
} else {
    Write-Output "Running in debug mode..."
    cargo run
}