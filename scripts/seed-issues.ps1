# PowerShell wrapper for seed-issues.sh.
#
#   .\scripts\seed-issues.ps1 --dry-run
#   .\scripts\seed-issues.ps1
#
# It exists because of a failure worth avoiding rather than documenting: PowerShell
# will not run a `.sh`, and running `./scripts/seed-issues.sh` there prints nothing at
# all and returns success. Silence reads as "it worked", and it had not.
#
# The logic lives in the shell script; this only finds a bash to run it with, and
# hands it a path that bash will recognise — which is less obvious than it sounds,
# because the two kinds of bash a Windows machine has do not agree on what a path is.

$ErrorActionPreference = 'Stop'

$script = Join-Path $PSScriptRoot 'seed-issues.sh'
if (-not (Test-Path $script)) {
    Write-Error "seed-issues.sh not found next to this wrapper ($PSScriptRoot)."
}

# Git for Windows first. Its bash shares the Windows filesystem, so `D:/src/...` means
# what it says, and it is the one this repository's tooling assumes.
$bash = $null
foreach ($candidate in @(
    "$env:ProgramFiles\Git\bin\bash.exe",
    "${env:ProgramFiles(x86)}\Git\bin\bash.exe",
    "$env:LOCALAPPDATA\Programs\Git\bin\bash.exe"
)) {
    if (Test-Path $candidate) { $bash = $candidate; break }
}

# Otherwise whatever `bash` is on PATH — which on a machine with WSL installed and no
# Git Bash is `System32\bash.exe`, WSL's. That one has its own filesystem view, where
# this repository is under `/mnt/d/...` and `D:/...` is simply not a path. Handing it
# a Windows path produces "No such file or directory" naming a file that plainly
# exists, which is a confusing half-hour for whoever hits it.
$translateForWsl = $false
if (-not $bash) {
    $onPath = (Get-Command bash -ErrorAction SilentlyContinue).Source
    if ($onPath) {
        $bash = $onPath
        $translateForWsl = $onPath -match '\\(System32|WindowsApps)\\'
    }
}

if (-not $bash) {
    Write-Error "No bash found. Install Git for Windows (https://git-scm.com/download/win), or run scripts/seed-issues.sh from any shell that has one."
}

if ($translateForWsl) {
    # D:\src\x  ->  /mnt/d/src/x
    $full = (Resolve-Path $script).Path
    $drive = $full.Substring(0, 1).ToLower()
    $script = "/mnt/$drive" + ($full.Substring(2) -replace '\\', '/')
} else {
    # Forward slashes even here: bash reads `\` as an escape, so a Windows path
    # arrives with its separators eaten — `D:srcprojects...`.
    $script = $script -replace '\\', '/'
}

# Back to `Continue` before handing over. Under `Stop`, Windows PowerShell 5.1 turns
# every line the script writes to stderr into a terminating error — so a skipped file,
# reported exactly as intended, would abort the run and be reported as a crash.
$ErrorActionPreference = 'Continue'

& $bash $script @args
exit $LASTEXITCODE
