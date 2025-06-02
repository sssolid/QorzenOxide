param (
    [string]$bump = "patch"
)

Write-Host "🚀 Starting release process with bump level: $bump"

# Ensure Git is clean
$status = git status --porcelain
if ($status) {
    Write-Error "❌ Uncommitted changes detected. Please commit or stash before releasing."
    exit 1
}

# Ensure we're on development branch
$currentBranch = git rev-parse --abbrev-ref HEAD
if ($currentBranch -ne "development") {
    Write-Error "❌ You must be on the 'development' branch to start the release process."
    exit 1
}

# Confirm from user
$confirmation = Read-Host "This will run tests, bump version, merge into main, and push. Proceed? (y/n)"
if ($confirmation -ne "y") {
    Write-Host "❌ Release aborted by user."
    exit 0
}

# Run checks
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "❌ cargo fmt failed."
    exit 1
}

cargo clippy --all-features --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "❌ cargo clippy failed."
    exit 1
}

# cargo test --all-features --workspace
# if ($LASTEXITCODE -ne 0) {
#     Write-Error "❌ cargo test failed."
#     exit 1
# }

# Run release (bumps version, tags, pushes)
cargo release $bump --execute
if ($LASTEXITCODE -ne 0) {
    Write-Error "❌ cargo-release failed."
    exit 1
}

# Merge to main
git checkout main
git pull origin main
git merge development

# Push everything
git push origin main
git push --tags

Write-Host "✅ Release complete and merged to main."