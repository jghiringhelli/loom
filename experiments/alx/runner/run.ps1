# ALX Runner — Adversarial Loom eXperiment
# ==========================================
# Runs Phase 2 (verification) of the ALX protocol.
# Phase 1 (blind derivation) is performed manually by loading a fresh
# AI context with ONLY loom.loom + language-spec.md.
#
# Usage: .\experiments\alx\runner\run.ps1
# Run from repo root.

$env:PATH = "$HOME\.cargo\bin;$env:PATH"
$repoRoot  = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$derivedDir = Join-Path $repoRoot "experiments\alx\derived"
$evidenceDir = Join-Path $repoRoot "experiments\alx\evidence"
$testsDir   = Join-Path $repoRoot "tests"
$cargoToml  = Join-Path $repoRoot "Cargo.toml"

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  ALX — Adversarial Loom eXperiment — Phase 2: Verification" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Check derived/src exists
if (-not (Test-Path (Join-Path $derivedDir "src"))) {
    Write-Host "ERROR: experiments/alx/derived/src/ not found." -ForegroundColor Red
    Write-Host "Run Phase 1 first: load fresh context with ONLY:" -ForegroundColor Yellow
    Write-Host "  - experiments/alx/spec/loom.loom" -ForegroundColor Yellow
    Write-Host "  - docs/language-spec.md" -ForegroundColor Yellow
    Write-Host "Then have the AI derive src/ into experiments/alx/derived/src/" -ForegroundColor Yellow
    exit 1
}

Write-Host "Phase 2: Copying test suite and Cargo.toml..." -ForegroundColor Yellow

# Copy tests
if (Test-Path $testsDir) {
    Copy-Item "$testsDir\*" (Join-Path $derivedDir "tests") -Recurse -Force
    Write-Host "  ✓ tests/ copied" -ForegroundColor Green
}

# Copy Cargo.toml (adjust package name to loom-alx-derived)
$cargoContent = Get-Content $cargoToml -Raw
$cargoContent = $cargoContent -replace 'name = "loom"', 'name = "loom-alx-derived"'
Set-Content (Join-Path $derivedDir "Cargo.toml") $cargoContent
Write-Host "  ✓ Cargo.toml copied (package renamed to loom-alx-derived)" -ForegroundColor Green

# Run cargo test
Write-Host ""
Write-Host "Running cargo test in derived/..." -ForegroundColor Yellow
Push-Location $derivedDir
$testOutput = cargo test --no-fail-fast 2>&1
Pop-Location

# Save output
$testOutput | Out-File (Join-Path $evidenceDir "test-output.txt") -Encoding UTF8
Write-Host "  ✓ Test output saved to evidence/test-output.txt" -ForegroundColor Green

# Compute S_realized
$passing = ($testOutput | Select-String "test result: ok\. (\d+) passed" | ForEach-Object {
    if ($_.Line -match "(\d+) passed") { [int]$matches[1] }
} | Measure-Object -Sum).Sum

$failing = ($testOutput | Select-String "FAILED" | Measure-Object).Count

$total = $passing + $failing

if ($total -gt 0) {
    $sRealized = [math]::Round($passing / $total, 4)
} else {
    $sRealized = 0
    Write-Host "WARNING: No tests found in output." -ForegroundColor Yellow
}

$scoreText = "S_realized = $passing / $total = $sRealized"
$scoreText | Out-File (Join-Path $evidenceDir "s-realized.txt") -Encoding UTF8

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  RESULT: $scoreText" -ForegroundColor $(if ($sRealized -ge 0.90) { "Green" } elseif ($sRealized -ge 0.80) { "Yellow" } else { "Red" })
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

if ($sRealized -ge 0.95) {
    Write-Host "STATUS: PUBLICATION READY — Submit white paper + release crate" -ForegroundColor Green
} elseif ($sRealized -ge 0.90) {
    Write-Host "STATUS: NEAR COMPLETE — Close remaining gaps and re-run" -ForegroundColor Yellow
} elseif ($sRealized -ge 0.80) {
    Write-Host "STATUS: SIGNIFICANT GAPS — Targeted spec improvements needed" -ForegroundColor Yellow
} else {
    Write-Host "STATUS: SPEC INCOMPLETE — Major sections need rewriting" -ForegroundColor Red
}

Write-Host ""
Write-Host "Next: Phase 3 — Gap Analysis" -ForegroundColor Cyan
Write-Host "  For each failing test: identify which section of loom.loom was insufficient." -ForegroundColor Gray
Write-Host "  Record in experiments/alx/evidence/correction-log.md" -ForegroundColor Gray
Write-Host "  Improve loom.loom, re-run Phase 1 for affected section, re-run this script." -ForegroundColor Gray
