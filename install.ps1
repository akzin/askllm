#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$Repo = "akzin/askllm",
    [string]$BinDir = "$env:LOCALAPPDATA\Programs\askllm",
    [switch]$NoPath,
    [switch]$NoConfig
)

$ErrorActionPreference = "Stop"

try {
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {
    # Older .NET; Invoke-WebRequest uses its default.
}

$architecture = if ($env:PROCESSOR_ARCHITEW6432) {
    $env:PROCESSOR_ARCHITEW6432
} else {
    $env:PROCESSOR_ARCHITECTURE
}
if ($architecture -notin @("AMD64", "ARM64")) {
    throw "askllm releases currently support Windows x86_64 only (detected $architecture)."
}

$BinDir = [IO.Path]::GetFullPath($BinDir)
if ($Version -eq "latest") {
    $releaseBase = "https://github.com/$Repo/releases/latest/download"
} else {
    $releaseBase = "https://github.com/$Repo/releases/download/$Version"
}
$exeUrl = "$releaseBase/askllm.exe"
$exampleRef = if ($Version -eq "latest") { "master" } else { $Version }
$exampleUrl = "https://raw.githubusercontent.com/$Repo/$exampleRef/askllm.toml.example"
$tempExe = Join-Path $env:TEMP ("askllm-" + [Guid]::NewGuid().ToString("N") + ".exe")

try {
    Write-Host "Downloading askllm from $exeUrl"
    Invoke-WebRequest -Uri $exeUrl -OutFile $tempExe -UseBasicParsing
    if ((Get-Item -LiteralPath $tempExe).Length -eq 0) {
        throw "Downloaded file is empty."
    }

    $checksumUrl = "$releaseBase/SHA256SUMS"
    $checksumFile = Join-Path $env:TEMP ("askllm-sha-" + [Guid]::NewGuid().ToString("N") + ".txt")
    $checksumDownloaded = $false
    try {
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumFile -UseBasicParsing
        $checksumDownloaded = $true
    } catch {
        Remove-Item -LiteralPath $checksumFile -ErrorAction SilentlyContinue
        Write-Warning "Could not download SHA256SUMS; skipping verification."
    }
    if ($checksumDownloaded) {
        try {
            $line = Get-Content -LiteralPath $checksumFile |
                Where-Object { $_ -match '\saskllm\.exe$' } |
                Select-Object -First 1
            if ($line) {
                $expected = ($line -split '\s+')[0].ToLowerInvariant()
                $actual = (Get-FileHash -LiteralPath $tempExe -Algorithm SHA256).Hash.ToLowerInvariant()
                if ($actual -ne $expected) {
                    throw "Checksum mismatch for askllm.exe."
                }
                Write-Host "Checksum verified."
            } else {
                Write-Warning "SHA256SUMS does not contain askllm.exe; skipping verification."
            }
        } finally {
            Remove-Item -LiteralPath $checksumFile -ErrorAction SilentlyContinue
        }
    }

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $target = Join-Path $BinDir "askllm.exe"
    try {
        Copy-Item -LiteralPath $tempExe -Destination $target -Force
    } catch {
        throw "Could not replace $target. Is askllm still running? $($_.Exception.Message)"
    }
    Write-Host "Installed askllm to $target"

    if (-not $NoPath) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $normalizedBin = $BinDir.TrimEnd([char]'\')
        $entries = @()
        if ($userPath) {
            $entries = @(
                $userPath -split ';' |
                    Where-Object { $_ -ne '' } |
                    ForEach-Object { $_.TrimEnd([char]'\') }
            )
        }
        if ($entries -notcontains $normalizedBin) {
            $newPath = if ($userPath) { "$userPath;$BinDir" } else { $BinDir }
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
            Write-Host "Added $BinDir to the user PATH."
        }
        $env:Path = "$BinDir;$env:Path"
    }

    if (-not $NoConfig) {
        $configDir = Join-Path $env:APPDATA "askllm"
        $configPath = Join-Path $configDir "config.toml"
        if (-not (Test-Path -LiteralPath $configPath)) {
            New-Item -ItemType Directory -Force -Path $configDir | Out-Null
            try {
                Invoke-WebRequest -Uri $exampleUrl -OutFile $configPath -UseBasicParsing
                Write-Host "Created $configPath"
                Write-Host "Edit that file: set base_url and model before running askllm."
            } catch {
                Remove-Item -LiteralPath $configPath -ErrorAction SilentlyContinue
                Write-Warning "Could not download the example config. Create $configPath from $exampleUrl."
            }
        }
    }

    if ($NoPath) {
        Write-Host "Run: $target <question>"
    } else {
        Write-Host "Start a new terminal, then run: askllm <question>"
    }
} finally {
    Remove-Item -LiteralPath $tempExe -ErrorAction SilentlyContinue
}
