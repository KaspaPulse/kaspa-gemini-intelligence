$LogFile = "build_report.log"
Write-Host "🚀 Running Unified System Checks and saving details to $LogFile..." -ForegroundColor Cyan

"=== KASPA AI - SECURITY & BUILD REPORT ===" > $LogFile
Get-Date >> $LogFile
"" >> $LogFile

Write-Host "🧹 1. Formatting code (cargo fmt)..." -ForegroundColor Yellow
"--- 1. CARGO FMT ---" >> $LogFile
cargo fmt 2>&1 >> $LogFile

Write-Host "🛡️ 2. Running Comprehensive Security Guard (cargo deny)..." -ForegroundColor Yellow
"--- 2. CARGO DENY (Advisories, Bans, Licenses, Sources) ---" >> $LogFile
cargo deny --color never check 2>&1 >> $LogFile

Write-Host "✅ All operations completed!" -ForegroundColor Green
Write-Host "📄 Open '$LogFile' in VS Code to review the final clean report." -ForegroundColor White
