$ErrorActionPreference = 'Stop'
Set-Location D:\Ada
$output = cargo doc -p ada-billing --no-deps 2>&1 | Out-String
$output | Out-File -Encoding UTF8 -FilePath D:\Ada\target_doc_baseline.txt
$lines = $output -split "`n"
$missing = ($lines | Where-Object { $_ -match 'missing_docs' }).Count
$warnings = ($lines | Where-Object { $_ -match '^warning:' }).Count
Write-Host "==BASELINE=="
Write-Host "missing_docs=$missing"
Write-Host "warnings=$warnings"
Write-Host "lines=$($lines.Count)"
