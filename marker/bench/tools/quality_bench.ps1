# Orchestration only. Geometry, pairing, scores and load-outlier decisions run in Rust.
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string[]]$BenchmarkArguments,
    [int]$QuietWaitSeconds = 1800,
    [ValidateRange(1,120)][int]$QuietSamples = 3,
    [ValidateRange(1,100)][int]$MaxStartCpu = 100
)
$ErrorActionPreference = 'Stop'
function Get-QualityWork([int]$ExcludeId = 0) {
    @(Get-CimInstance Win32_Process | Where-Object {
        $_.ProcessId -ne $ExcludeId -and (
            $_.ExecutablePath -like '*cargo-target*marker-quality*' -or
            ($_.Name -eq 'rustc.exe' -and $_.CommandLine -like '*cargo-target*marker-quality*') -or
            ($_.Name -eq 'cargo.exe' -and $_.CommandLine -like '*marker-quality*')
        )
    } | Select-Object @{Name='Id';Expression={$_.ProcessId}},Name)
}
$deadline = (Get-Date).AddSeconds($QuietWaitSeconds)
$idle = 0
$background = [System.Collections.Generic.List[object]]::new()
while ($idle -lt $QuietSamples) {
    $ours = @(Get-QualityWork)
    $load = (Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average
    if ($ours.Count -eq 0 -and $null -ne $load -and $load -le $MaxStartCpu) {
        $idle++
        $background.Add(@{unix_ms=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds();cpu_percent=$load})
    } else { $idle = 0; $background.Clear() }
    if ((Get-Date) -ge $deadline) { throw 'Our own build/benchmark process did not become idle. No benchmark started.' }
    if ($idle -lt $QuietSamples) { Start-Sleep -Seconds 5 }
}
if (Test-Path -LiteralPath $Output) { throw 'Use a fresh output directory.' }
New-Item -ItemType Directory -Path $Output | Out-Null
$Output = (Resolve-Path -LiteralPath $Output).Path
if ($BenchmarkArguments -contains '@OUTPUT@') {
    $commandArgs = @($BenchmarkArguments | ForEach-Object { if ($_ -eq '@OUTPUT@') { $Output } else { $_ } })
} else { $commandArgs = @($BenchmarkArguments) + @('--out', $Output) }
$env:RAYON_NUM_THREADS = '3'
$quoted = $commandArgs | ForEach-Object { '"' + $_.Replace('"','\"') + '"' }
$conditions = @{
    protocol = 'shared-paired-v1'
    started_utc = (Get-Date).ToUniversalTime().ToString('o')
    started_unix_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    background_cpu_samples = @($background.ToArray())
    background_cpu_percent_at_start = $load
    no_own_work_at_start = $true
    own_idle_samples = $idle
    shared_machine = $true
    rayon_num_threads = $env:RAYON_NUM_THREADS
    executable_sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
    executable = [IO.Path]::GetFileName($Executable)
    args = $BenchmarkArguments
    note = 'User-authorized shared-machine measurement. Unrelated programs remain running. Only our marker-quality builds/benchmarks gate startup; CPU load is recorded, not suppressed.'
}
$conditions | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Output 'conditions.json')
$process = Start-Process -FilePath $Executable -ArgumentList $quoted -WorkingDirectory (Resolve-Path "$PSScriptRoot/../..").Path -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $Output 'stdout.log') -RedirectStandardError (Join-Path $Output 'stderr.log')
$qualityNativeHandle = $process.Handle
$conditions.process_id = $process.Id
$samples = [System.Collections.Generic.List[object]]::new()
$interrupted = $false
do {
    $ours = @(Get-QualityWork -ExcludeId $process.Id)
    $sample = @{
        utc = (Get-Date).ToUniversalTime().ToString('o')
        unix_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        cpu_percent = (Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average
        other_own_work = $ours
    }
    $samples.Add($sample)
    $sample | ConvertTo-Json -Depth 5 -Compress | Add-Content -LiteralPath (Join-Path $Output 'load.jsonl')
    if ($ours.Count -gt 0) {
        $interrupted = $true
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -ErrorAction SilentlyContinue }
    }
    $process.WaitForExit(5000) | Out-Null
    $process.Refresh()
} while (-not $process.HasExited)
$process.WaitForExit()
$conditions.cpu_samples = @($samples.ToArray())
$conditions.finished_utc = (Get-Date).ToUniversalTime().ToString('o')
$conditions.exit_code = $process.ExitCode
$conditions | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Output 'conditions.json')
@{own_work_interrupted=$interrupted;compiler_interrupted=$interrupted;exit_code=$process.ExitCode;finished_utc=$conditions.finished_utc} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Output 'completion.json')
if ($interrupted) { throw "Our own build/benchmark work overlapped; partial evidence retained in $Output" }
if ($process.ExitCode -ne 0) { throw "Benchmark failed with exit code $($process.ExitCode); evidence retained in $Output" }
