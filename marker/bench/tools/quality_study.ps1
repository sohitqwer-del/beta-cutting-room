# Run a recorded benchmark, audit CPU load and sampling coverage, repeat flagged tasks, then
# materialize effective CSVs without deleting or score-selecting raw evidence.
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [string]$AuditExecutable = '',
    [Parameter(Mandatory=$true)][string]$Output,
    [string[]]$BenchmarkArguments = @(),
    [int]$Jobs = 2,
    [int]$Workers = 3,
    [ValidateRange(1,100)][int]$MaxStartCpu = 100,
    [int]$LoadRetries = 3,
    [int]$RetryOffset = 0
)
$ErrorActionPreference = 'Stop'
if (-not $AuditExecutable) { $AuditExecutable = $Executable }
$completionFile = Join-Path $Output 'completion.json'
$complete = $false
if (Test-Path -LiteralPath $completionFile) {
    $state = Get-Content -LiteralPath $completionFile -Raw | ConvertFrom-Json
    $complete = $state.exit_code -eq 0 -and -not $state.compiler_interrupted
}
if (-not $complete) {
    if ($BenchmarkArguments.Count -eq 0) { throw 'A new study needs benchmark arguments.' }
    if (Test-Path -LiteralPath $Output) { throw 'Incomplete output exists; keep it and choose a new output directory.' }
    & "$PSScriptRoot/quality_bench.ps1" -Executable $Executable -Output $Output -BenchmarkArguments $BenchmarkArguments -MaxStartCpu $MaxStartCpu
}
& $AuditExecutable bench audit --run $Output
if ($LASTEXITCODE -ne 0) { throw 'CPU-load audit failed.' }
$reference = Join-Path $Output 'load-audit.json'
$current = $Output
for ($attempt = 1; $attempt -le $LoadRetries; $attempt++) {
    $audit = Get-Content -LiteralPath (Join-Path $current 'load-audit.json') -Raw | ConvertFrom-Json
    if (@($audit.outlier_tasks).Count -eq 0) { break }
    $inputFile = Join-Path $current 'load-rerun.json'
    $retry = Join-Path $Output ('reruns/{0:D2}' -f ($attempt + $RetryOffset))
    if (-not (Test-Path -LiteralPath (Join-Path $retry 'completion.json'))) {
        Write-Output "Repeating CPU-load outliers, attempt ${attempt}: $($audit.outlier_tasks.Count) tasks"
        & "$PSScriptRoot/quality_bench.ps1" -Executable $Executable -Output $retry -BenchmarkArguments @('bench','replay','--input',$inputFile,'--jobs',"$Jobs",'--threads-per-task',"$Workers") -MaxStartCpu $MaxStartCpu
    }
    & $AuditExecutable bench audit --run $retry --reference $reference
    if ($LASTEXITCODE -ne 0) { throw 'Retry CPU audit failed.' }
    $current = $retry
}
& $AuditExecutable bench finalize --run $Output
if ($LASTEXITCODE -ne 0) { throw 'Effective result finalization failed.' }
