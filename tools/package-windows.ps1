# Makes the Windows packages (ADR 0033, ADR 0043):
#   dist\<name>-windows-portable.zip   the program in a folder; nothing is installed
#   dist\<name>-windows-setup.exe      the installer (with -Installer; needs Inno Setup 6)
#
# <name> is ShokujoDX-<version> for a commit with the tag v<version>,
# else ShokujoDX-<version>-<commit> (a test build).
#
# The folder has the program, its .pdb file (for the function names in
# crash reports), the licenses, the tester guide and the feedback form.
# The installer installs the same folder.
#
# Use:  powershell -ExecutionPolicy Bypass -File tools\package-windows.ps1 [-SkipTests] [-Installer]

param([switch]$SkipTests, [switch]$Installer)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

function Run($exe, [string[]]$arguments) {
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "$exe $($arguments -join ' ') failed ($LASTEXITCODE)" }
}

if (git status --porcelain --untracked-files=no) {
    Write-Warning "Files have changes that are not in a commit: the build name gets '+changes'."
}
if (-not $SkipTests) {
    Run cargo @('test', '--workspace')
}
Run cargo @('build', '--release', '-p', 'emb-app', '-p', 'emb-thumbnail')

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$build = (git rev-parse --short=7 HEAD).Trim()
$tags = @(git tag --points-at HEAD)
$name = if ($tags -contains "v$version") { "ShokujoDX-$version" } else { "ShokujoDX-$version-$build" }
$dist = Join-Path $root 'dist'
$out = Join-Path $dist $name
if (Test-Path $out) { Remove-Item -LiteralPath $out -Recurse -Force }
New-Item -ItemType Directory -Force (Join-Path $out 'licenses') | Out-Null

$exe = Join-Path $out 'ShokujoDX.exe'
Copy-Item target\release\emb-app.exe $exe
# The .pdb keeps its own name: the program finds it by that name.
Copy-Item target\release\emb_app.pdb $out
# The thumbnail handler of Explorer (ADR 0046; the installer registers it).
Copy-Item target\release\emb_thumbnail.dll $out
Copy-Item docs\testing\TESTERS.md (Join-Path $out 'READ ME FIRST.txt')
Copy-Item docs\testing\FEEDBACK.md (Join-Path $out 'FEEDBACK FORM.txt')
Copy-Item LICENSE (Join-Path $out 'licenses\LICENSE (GPL-3.0).txt')
Copy-Item data\licenses\THIRD-PARTY.txt (Join-Path $out 'licenses\THIRD-PARTY.txt')
# TrueType fonts and the embroidery fonts made from them have licenses of
# their own (names that differ only in capital letters on Windows).
Get-ChildItem data\fonts -Filter '*OFL.txt' | ForEach-Object {
    Copy-Item $_.FullName (Join-Path $out "licenses\TrueType font $($_.Name)")
}
Get-ChildItem data\fonts\embroidery -Filter '*OFL.txt' | ForEach-Object {
    Copy-Item $_.FullName (Join-Path $out "licenses\embroidery font $($_.Name)")
}

# The program must start (it writes its version and stops).
$v = & $exe --version | Out-String
if (-not $v.Contains("ShokujoDX $version")) { throw "The program did not start: $v" }

$zip = Join-Path $dist "$name-windows-portable.zip"
if (Test-Path $zip) { Remove-Item -LiteralPath $zip -Force }
Compress-Archive -Path (Join-Path $out '*') -DestinationPath $zip
$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Output "Wrote $zip ($size MB)"

if ($Installer) {
    $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
    if (-not $iscc) {
        $iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
            "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
    }
    if (-not $iscc) { throw 'Inno Setup 6 (ISCC.exe) is not installed.' }
    Run $iscc @('/Q', "/DVersion=$version", "/DSource=$out", "/DOutputDir=$dist",
        "/DOutputName=$name-windows-setup", 'packaging\windows\installer.iss')
    $setup = Join-Path $dist "$name-windows-setup.exe"
    $size = [math]::Round((Get-Item $setup).Length / 1MB, 1)
    Write-Output "Wrote $setup ($size MB)"
}
