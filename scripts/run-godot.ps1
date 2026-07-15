param(
    [switch]$Editor,
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$godot = 'C:\Godot\Godot_v4.7.1-stable_win64.exe'
if (-not (Test-Path -LiteralPath $godot)) {
    throw "Godot was not found at $godot"
}

$buildArgs = @('build', '-p', 'threadnations-app', '--bin', 'godot_bridge')
if ($Release) {
	$buildArgs += '--release'
}
& cargo @buildArgs
$profile = if ($Release) { 'release' } else { 'debug' }
$bridgePath = Join-Path $root "target\\$profile\\godot_bridge.exe"
Get-Process -Name godot_bridge -ErrorAction SilentlyContinue | Stop-Process -Force
$temp = Join-Path $root '.tmp'
$snapshot = Join-Path $temp 'godot-world.json'
$stdout = Join-Path $temp 'godot-bridge.stdout.log'
$stderr = Join-Path $temp 'godot-bridge.stderr.log'
New-Item -ItemType Directory -Force -Path $temp | Out-Null
Remove-Item -LiteralPath $snapshot, $stdout, $stderr -Force -ErrorAction SilentlyContinue
$bridge = Start-Process -FilePath $bridgePath -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$deadline = (Get-Date).AddSeconds(5)
while (-not (Test-Path -LiteralPath $snapshot) -and -not $bridge.HasExited -and (Get-Date) -lt $deadline) {
	Start-Sleep -Milliseconds 100
}
if (-not (Test-Path -LiteralPath $snapshot)) {
	if (-not $bridge.HasExited) {
		Stop-Process -Id $bridge.Id -Force
	}
	$details = Get-Content -Raw -LiteralPath $stderr -ErrorAction SilentlyContinue
	throw "The Rust snapshot bridge did not start. $details"
}
Write-Host "Rust simulation ready: $snapshot"
$godotArgs = @('--path', $root)
if ($Editor) {
	$godotArgs += '--editor'
}
& $godot @godotArgs
