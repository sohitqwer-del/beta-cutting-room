# After the matrix finishes, build/test the new audit and recheck every study.
# Replay with the frozen original executable, not the newly built engine.
param(
    [Parameter(Mandatory=$true)][string]$FrozenExecutable,
    [Parameter(Mandatory=$true)][string]$AuditExecutable,
    [Parameter(Mandatory=$true)][string]$EvidenceRoot
)
$ErrorActionPreference = 'Stop'
$plan = Get-Content -LiteralPath "$PSScriptRoot/../experiments/v03/study-plan.json" -Raw | ConvertFrom-Json
$names = $plan.active
foreach ($name in $names) {
    $completion = Join-Path $EvidenceRoot "paired-$name/completion.json"
    if (-not (Test-Path -LiteralPath $completion)) { throw 'Finish the original matrix before re-auditing.' }
    $state = Get-Content -LiteralPath $completion -Raw | ConvertFrom-Json
    if ($state.exit_code -ne 0 -or $state.compiler_interrupted) { throw "Original study $name did not finish cleanly." }
}
foreach ($name in $names) {
    & "$PSScriptRoot/quality_study.ps1" -Executable $FrozenExecutable -AuditExecutable $AuditExecutable -Output (Join-Path $EvidenceRoot "paired-$name") -Jobs 2 -Workers 3 -RetryOffset 10
}
