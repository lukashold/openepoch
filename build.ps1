# Build the simulation to WebAssembly and drop it next to the page.
$ErrorActionPreference = 'Stop'
cargo build --release --target wasm32-unknown-unknown
Copy-Item target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch.wasm -Force
Write-Host 'Built docs/openepoch.wasm - serve the docs/ folder, e.g.: python -m http.server --directory docs 8123'
