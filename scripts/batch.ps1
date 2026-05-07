# ==============================================================================
# Angry Birds Fusion Toolkit - Interactive Wrapper Script
# File: scripts/batch.ps1
#
# This script is designed to be placed in the "scripts/" directory.
# It automatically searches for the binary in common locations.
# ==============================================================================

# Allow script execution for this process
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force | Out-Null

$BinName = "angrybirds-fusion-toolkit.exe"

# Define potential paths for the executable
# 1. Current directory
# 2. Cargo build directory (Development)
$PathsToCheck = @(
    ".\$BinName",                  
    "..\target\release\$BinName"   
)

$ToolPath = $null

# Search for the binary
foreach ($path in $PathsToCheck) {
    if (Test-Path $path) {
        $ToolPath = $path
        if ($path -like "*target*") {
            Write-Host "Running in development mode (using target/release binary)" -ForegroundColor Gray
        }
        break
    }
}

if ($null -eq $ToolPath) {
    Write-Host "Error: Could not find '$BinName'." -ForegroundColor Red
    Write-Host "Please ensure the project is built or the binary is placed in the current directory."
    Write-Host "Press any key to exit..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    exit
}

Clear-Host
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "   Angry Birds Fusion Toolkit" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Please select an operation mode:"
Write-Host "1) Decrypt (Game File -> Readable)"
Write-Host "2) Encrypt (Readable -> Game File)"
Write-Host "==========================================" -ForegroundColor Cyan

$modeChoice = Read-Host "Enter number (1-2)"
$useAuto = $false

switch ($modeChoice) {
    "1" { $cmd = "decrypt" }
    "2" { $cmd = "encrypt" }
    Default {
        Write-Host "Invalid choice. Exiting." -ForegroundColor Red
        exit
    }
}

# Input File Selection
Write-Host ""
$inputFile = Read-Host "Enter Input File Path (drag & drop allowed)"
# Remove quotes added by PowerShell when dragging files
$inputFile = $inputFile -replace '"',''

if (-not (Test-Path $inputFile)) {
    Write-Host "Error: File '$inputFile' does not exist." -ForegroundColor Red
    exit
}

# Output File Selection
Write-Host ""
$outputFile = Read-Host "Enter Output File Path (leave empty for auto-naming)"
$outputFile = $outputFile -replace '"',''

# Logic for Game/Category Selection
$targetGame = $null
$targetCategory = $null

if ($cmd -eq "decrypt") {
    Write-Host ""
    Write-Host "Decryption Mode:"
    Write-Host "1) Auto-detect Game & Category (Recommended)"
    Write-Host "2) Manual Selection"
    $detectChoice = Read-Host "Enter number (1-2)"
    if ($detectChoice -eq "1") {
        $useAuto = $true
    }
}

if (-not $useAuto) {
    Write-Host ""
    Write-Host "Select Game:"
    Write-Host "1) Classic"
    Write-Host "2) Rio"
    Write-Host "3) Seasons"
    Write-Host "4) Space"
    Write-Host "5) Friends"
    Write-Host "6) Star Wars"
    Write-Host "7) Star Wars II"
    Write-Host "8) Stella"
    $gameChoice = Read-Host "Enter number (1-8)"

    switch ($gameChoice) {
        "1" { $targetGame = "classic" }
        "2" { $targetGame = "rio" }
        "3" { $targetGame = "seasons" }
        "4" { $targetGame = "space" }
        "5" { $targetGame = "friends" }
        "6" { $targetGame = "starwars" }
        "7" { $targetGame = "starwarsii" }
        "8" { $targetGame = "stella" }
        Default { Write-Host "Invalid game. Exiting." -ForegroundColor Red; exit }
    }

    Write-Host ""
    Write-Host "Select File Category:"
    Write-Host "1) Native (Game Data/Levels)"
    Write-Host "2) Save (Progress/Highscores)"
    Write-Host "3) Downloaded (DLC/Assets)"
    $catChoice = Read-Host "Enter number (1-3)"

    switch ($catChoice) {
        "1" { $targetCategory = "native" }
        "2" { $targetCategory = "save" }
        "3" { $targetCategory = "downloaded" }
        Default { Write-Host "Invalid category. Exiting." -ForegroundColor Red; exit }
    }
}

# Construct arguments list
$argsList = @($cmd, "-i", $inputFile)

if (-not [string]::IsNullOrWhiteSpace($outputFile)) {
    $argsList += "-o"
    $argsList += $outputFile
}

if ($useAuto) {
    $argsList += "--auto"
} else {
    $argsList += "-g"
    $argsList += $targetGame
    $argsList += "-c"
    $argsList += $targetCategory
}

Write-Host ""
Write-Host "Executing..." -ForegroundColor Yellow
Write-Host "------------------------------------------"

# Execute the tool and catch errors
try {
    & $ToolPath $argsList
    if ($LASTEXITCODE -eq 0) {
        Write-Host "------------------------------------------"
        Write-Host "Operation completed successfully!" -ForegroundColor Green
    } else {
        throw "Exit code $LASTEXITCODE"
    }
} catch {
    Write-Host "------------------------------------------"
    Write-Host "Operation failed. Please check the file or logs." -ForegroundColor Red
}

Write-Host "Press any key to exit..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")