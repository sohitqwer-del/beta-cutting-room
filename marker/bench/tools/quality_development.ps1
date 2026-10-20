param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$EvidenceRoot,
    [int]$Jobs = 2,
    [int]$Workers = 3
)
$ErrorActionPreference = 'Stop'
$marker = (Resolve-Path "$PSScriptRoot/../..").Path
$inputFile = Join-Path $EvidenceRoot 'conversion/development-40.jsonl'
if (-not (Test-Path -LiteralPath $inputFile)) { throw 'Freeze and convert development IDs first.' }
$baseline = Join-Path $marker 'bench/experiments/v03/baseline.json'
$plan = Get-Content -LiteralPath (Join-Path $marker 'bench/experiments/v03/study-plan.json') -Raw | ConvertFrom-Json
foreach ($name in $plan.active) {
    $output = Join-Path $EvidenceRoot "paired-$name"
    $config = Join-Path $marker "bench/experiments/v03/$name.json"
    $runs = if ($name -in @('sequential3','race3')) { 3 } else { 1 }
    $benchArgs = @('bench','lectra','--input',$inputFile,'--time','60','--limit','40','--jobs',"$Jobs",'--threads-per-task',"$Workers",'--seed','1','--search-config',$config,'--runs',"$runs",'--paired-baseline',$baseline)
    Write-Output "Paired development experiment ${name}: same tasks/seeds, alternating baseline order"
    & "$PSScriptRoot/quality_study.ps1" -Executable $Executable -Output $output -BenchmarkArguments $benchArgs -Jobs $Jobs -Workers $Workers
    Write-Output "Finished paired development experiment $name"
}
