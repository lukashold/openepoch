$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
$previousTargetDir = $env:CARGO_TARGET_DIR
try {
    $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot 'target/public-sync'
    cargo build --offline --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE) { throw 'OpenEpoch build failed' }
    Copy-Item "$env:CARGO_TARGET_DIR/wasm32-unknown-unknown/release/openepoch.wasm" 'docs/openepoch.wasm' -Force
    cargo build --offline --release --target wasm32-unknown-unknown --features ee2-reference
    if ($LASTEXITCODE) { throw 'EE2 profile build failed' }
    Copy-Item "$env:CARGO_TARGET_DIR/wasm32-unknown-unknown/release/openepoch.wasm" 'docs/openepoch-ee2.wasm' -Force
    cargo build --offline --release --target wasm32-unknown-unknown --features ee2-ordered-claims217
    if ($LASTEXITCODE) { throw 'Own UP roster build failed' }
    foreach ($leaf in 'openepoch-ee2-up15.wasm','openepoch-ee2-roster6.wasm','openepoch-citizen9.wasm') {
        Copy-Item "$env:CARGO_TARGET_DIR/wasm32-unknown-unknown/release/openepoch.wasm" "docs/$leaf" -Force
    }
    cargo build --offline --release --target wasm32-unknown-unknown --features ee2-ordered-claims217,ee2-valley205-completed-refund,ee2-valley215-citizen-contact,ee2-rect291-ai-scout
    if ($LASTEXITCODE) { throw 'Own rectangular runtime build failed' }
    Copy-Item "$env:CARGO_TARGET_DIR/wasm32-unknown-unknown/release/openepoch.wasm" 'docs/openepoch-ee2-death-valley2.wasm' -Force
    python tools/build_source_cloth.py
    if ($LASTEXITCODE) { throw 'Own procedural cloth build failed' }
} finally {
    $env:CARGO_TARGET_DIR = $previousTargetDir
    Pop-Location
}
