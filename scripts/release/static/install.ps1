#!/usr/bin/env pwsh

$ErrorActionPreference = 'Stop'

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# Create bin directory for sandbox-agent
$BinDir = $env:BIN_DIR
$SandboxAgentInstall = if ($BinDir) {
	$BinDir
} else {
	"${Home}\.sandbox-agent\bin"
}

if (!(Test-Path $SandboxAgentInstall)) {
	New-Item $SandboxAgentInstall -ItemType Directory | Out-Null
}

$SandboxAgentExe = "$SandboxAgentInstall\sandbox-agent.exe"
$Version = '__VERSION__'
$FileName = 'sandbox-agent-x86_64-pc-windows-gnu.exe'

Write-Host
Write-Host "> Installing sandbox-agent ${Version}"

# Download binary
$DownloadUrl = "https://releases.rivet.dev/sandbox-agent/${Version}/binaries/${FileName}"
Write-Host
Write-Host "> Downloading ${DownloadUrl}"
Invoke-WebRequest $DownloadUrl -OutFile $SandboxAgentExe -UseBasicParsing

# Install to PATH
Write-Host
Write-Host "> Installing sandbox-agent"
$User = [System.EnvironmentVariableTarget]::User
$Path = [System.Environment]::GetEnvironmentVariable('Path', $User)
if (!(";${Path};".ToLower() -like "*;${SandboxAgentInstall};*".ToLower())) {
	[System.Environment]::SetEnvironmentVariable('Path', "${Path};${SandboxAgentInstall}", $User)
	$Env:Path += ";${SandboxAgentInstall}"
    Write-Host "Please restart your PowerShell session or run the following command to refresh the environment variables:"
    Write-Host "[System.Environment]::SetEnvironmentVariable('Path', '${Path};${SandboxAgentInstall}', [System.EnvironmentVariableTarget]::Process)"
}

Write-Host
Write-Host "> Checking installation"
sandbox-agent.exe --version

Write-Host
Write-Host "sandbox-agent was installed successfully to ${SandboxAgentExe}."
Write-Host "Run 'sandbox-agent --help' to get started."
Write-Host
