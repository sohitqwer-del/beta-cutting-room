# Offline narration: one WAV per line with a Windows OneCore voice (WinRT Windows.Media.SpeechSynthesis).
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File tts.ps1 -Lines <lines.json> -OutDir <folder> [-Voice en-IN] [-Rate 0.9]
# lines.json: [{ "id": "hook-1", "say": "Text to speak" }, ...]. Writes <id>.wav and voice.json (the voice that was used).
# No network: OneCore voices are installed with Windows. Prefers an en-IN voice, then en-GB, then any English voice.
param(
  [Parameter(Mandatory = $true)][string]$Lines,
  [Parameter(Mandatory = $true)][string]$OutDir,
  [string]$Voice = 'en-IN',
  [string]$VoiceName = '',
  [double]$Rate = 0.9
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Media.SpeechSynthesis.SpeechSynthesizer, Windows.Media.SpeechSynthesis, ContentType = WindowsRuntime] | Out-Null
[Windows.Storage.Streams.DataReader, Windows.Storage.Streams, ContentType = WindowsRuntime] | Out-Null

$asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
  })[0]
function Await($op, [Type]$type) {
  $task = $asTask.MakeGenericMethod($type).Invoke($null, @($op))
  $task.Wait(-1) | Out-Null
  $task.Result
}

$all = [Windows.Media.SpeechSynthesis.SpeechSynthesizer]::AllVoices
$pick = $null
if ($VoiceName) { $pick = $all | Where-Object { $_.DisplayName -eq $VoiceName } | Select-Object -First 1 }
if (-not $pick) { $pick = $all | Where-Object { $_.Language -eq $Voice } | Select-Object -First 1 }
if (-not $pick) { $pick = $all | Where-Object { $_.Language -eq 'en-GB' } | Select-Object -First 1 }
if (-not $pick) { $pick = $all | Where-Object { $_.Language -like 'en-*' } | Select-Object -First 1 }
if (-not $pick) { throw 'No English OneCore voice is installed.' }

$synth = New-Object Windows.Media.SpeechSynthesis.SpeechSynthesizer
$synth.Voice = $pick
$synth.Options.SpeakingRate = $Rate
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$items = Get-Content -Raw -Encoding UTF8 $Lines | ConvertFrom-Json
foreach ($item in $items) {
  $stream = Await ($synth.SynthesizeTextToStreamAsync([string]$item.say)) ([Windows.Media.SpeechSynthesis.SpeechSynthesisStream])
  $size = [uint32]$stream.Size
  $reader = New-Object Windows.Storage.Streams.DataReader($stream.GetInputStreamAt(0))
  Await ($reader.LoadAsync($size)) ([uint32]) | Out-Null
  $bytes = New-Object byte[] $size
  $reader.ReadBytes($bytes)
  [System.IO.File]::WriteAllBytes((Join-Path $OutDir ($item.id + '.wav')), $bytes)
  $reader.Dispose(); $stream.Dispose()
}
@{ name = $pick.DisplayName; language = $pick.Language; gender = [string]$pick.Gender; rate = $Rate } |
  ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $OutDir 'voice.json')
Write-Output ("TTS: {0} lines with {1} ({2}) at rate {3}" -f @($items).Count, $pick.DisplayName, $pick.Language, $Rate)
