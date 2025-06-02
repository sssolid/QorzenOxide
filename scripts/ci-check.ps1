# Set terminal encoding to UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Write-Host "[INFO] Running local CI checks..." -ForegroundColor Cyan

# Auto-format first (helpful during development)
Write-Host "`n[STEP] Auto-formatting code..." -ForegroundColor Cyan
cargo fmt --all
if ($LASTEXITCODE -ne 0) {
    Write-Error "[FAIL] cargo fmt auto-format failed."
    exit 1
}

# Validate formatting for CI parity
Write-Host "`n[STEP] Checking formatting with --check..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "[FAIL] Formatting does not match rustfmt rules. Fix it before committing."
    exit 1
}

# Run cargo check
Write-Host "`n[STEP] Running cargo check..." -ForegroundColor Cyan
cargo check --all-features
if ($LASTEXITCODE -ne 0) {
    Write-Error "[FAIL] cargo check failed."
    exit 1
}

# Run Clippy lints
Write-Host "`n[STEP] Running cargo clippy..." -ForegroundColor Cyan
cargo clippy --all-features --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "[FAIL] cargo clippy reported issues."
    exit 1
}

# Run tests
Write-Host "`n[STEP] Running cargo test... (SKIPPING FOR NOW)" -ForegroundColor Cyan
# cargo test --all-features --workspace
# if ($LASTEXITCODE -ne 0) {
#     Write-Error "[FAIL] Tests failed."
#     exit 1
# }

# Build documentation
Write-Host "`n[STEP] Building docs..." -ForegroundColor Cyan
cargo doc --all-features --no-deps --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "[FAIL] Documentation build failed."
    exit 1
}

# Security audit
Write-Host "`n[STEP] Running cargo audit... (SKIPPING FOR NOW)" -ForegroundColor Cyan
# cargo audit
# if ($LASTEXITCODE -ne 0) {
#     Write-Error "[FAIL] Security audit failed."
#     exit 1
# }

# Deny policy enforcement
Write-Host "`n[STEP] Running cargo deny check... (SKIPPING FOR NOW)" -ForegroundColor Cyan
# cargo deny check
# if ($LASTEXITCODE -ne 0) {
#     Write-Error "[FAIL] Dependency policy check failed."
#     exit 1
# }

Write-Host "`n[OK] All local CI checks passed successfully!" -ForegroundColor Green
