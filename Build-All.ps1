$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot '..\..\work\openepoch-target'
    cargo build --offline --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE) { throw 'OpenEpoch build failed' }
    Copy-Item "$env:CARGO_TARGET_DIR\wasm32-unknown-unknown\release\openepoch.wasm" 'docs\openepoch.wasm' -Force
    cargo build --offline --release --target wasm32-unknown-unknown --features ee2-reference
    if ($LASTEXITCODE) { throw 'EE2 profile build failed' }
    Copy-Item "$env:CARGO_TARGET_DIR\wasm32-unknown-unknown\release\openepoch.wasm" 'docs\openepoch-ee2.wasm' -Force
    cargo build --offline --release --target wasm32-unknown-unknown --features ee2-citizen9
    if ($LASTEXITCODE) { throw 'EE2 UP roster build failed' }
    Copy-Item "$env:CARGO_TARGET_DIR\wasm32-unknown-unknown\release\openepoch.wasm" 'docs\openepoch-ee2-up15.wasm' -Force
    Copy-Item "$env:CARGO_TARGET_DIR\wasm32-unknown-unknown\release\openepoch.wasm" 'docs\openepoch-ee2-roster6.wasm' -Force
    Copy-Item "$env:CARGO_TARGET_DIR\wasm32-unknown-unknown\release\openepoch.wasm" 'docs\openepoch-citizen9.wasm' -Force
    python tools/build_source_cloth.py
    if ($LASTEXITCODE) { throw 'Own visual cloth build failed' }
} finally { Pop-Location }
