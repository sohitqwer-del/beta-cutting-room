# Run only after development selection, default promotion, tests and release build.
# Each stage keeps raw evidence and repeats tasks selected only by CPU load.
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$EvidenceRoot,
    [int]$Jobs = 2,
    [int]$Workers = 3,
    [ValidateRange(1,100)][int]$MaxStartCpu = 20
)
$ErrorActionPreference = 'Stop'
$marker = (Resolve-Path "$PSScriptRoot/../..").Path
$selectionFile = Join-Path $EvidenceRoot 'selection.json'
$config = Join-Path $EvidenceRoot 'selected-search.json'
if (-not (Test-Path -LiteralPath $selectionFile) -or -not (Test-Path -LiteralPath $config)) {
    throw 'Complete the paired development matrix and freeze its selection first.'
}
$selection = Get-Content -LiteralPath $selectionFile -Raw | ConvertFrom-Json
if (-not $selection.sampling_gap_audit_checked) { throw 'Re-audit development sampling gaps and freeze the corrected selection first.' }
$runs = [int]$selection.search.runs
if ($runs -lt 1) { throw 'Selection must record a positive run count.' }
$common = @('--jobs',"$Jobs",'--threads-per-task',"$Workers")
$search = @('--search-config',$config,'--runs',"$runs")
function Invoke-Study([string]$Name, [string[]]$Arguments) {
    Write-Output "Confirmation stage: $Name"
    & "$PSScriptRoot/quality_study.ps1" -Executable $Executable -Output (Join-Path $EvidenceRoot $Name) -BenchmarkArguments $Arguments -Jobs $Jobs -Workers $Workers -MaxStartCpu $MaxStartCpu
}
Invoke-Study 'lectra-60s' (@('bench','lectra','--input',(Join-Path $EvidenceRoot 'conversion/sample-300.jsonl'),'--time','60','--limit','300','--seed','1') + $common + $search)
Invoke-Study 'lectra-match' (@('bench','lectra','--input',(Join-Path $EvidenceRoot 'conversion/sample-match-60.jsonl'),'--time','match','--limit','60','--seed','1') + $common + $search)
Invoke-Study 'academic-60s' (@('bench','academic','--time','60','--seeds','1,2,3') + $common + $search)
$garments = @('simon','teagan','titan') | ForEach-Object { Join-Path $marker "testdata/open/freesewing/$_-ratio.job.json" }
Invoke-Study 'freesewing-120s' (@('bench','jobs','--inputs',($garments -join ','),'--time','120','--seed','1') + $common)
$teagan = Join-Path $marker 'testdata/open/freesewing/teagan-ratio.job.json'
Invoke-Study 'teagan-layouts-60s' (@('bench','jobs','--inputs',$teagan,'--layouts','as-is,folded,tubular','--time','60','--seed','1') + $common)
$shade = @('length','width') | ForEach-Object { Join-Path $marker "crates/beta-marker-match/tests/fixtures/simon-shade-$_.job.json" }
Invoke-Study 'shade-free-paired-120s' (@('bench','jobs','--inputs',($shade -join ','),'--time','120','--seed','42','--shade-attempts','1','--paired-without-rules') + $common)
Invoke-Study 'shade-search-paired-120s' (@('bench','jobs','--inputs',($shade -join ','),'--time','120','--seed','42','--shade-attempts','4','--baseline-shade-attempts','1') + $common)
