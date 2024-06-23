function Stop-ScriptWithError
{
	param([String]$ErrorMessage)
	Write-Log -Level ERROR -Message $ErrorMessage; Wait-Logging; exit
}

$LoggingAvailable = Get-Module Logging
if ($null -eq $LoggingAvailable)
{
	Install-PackageProvider -Name NuGet -MinimumVersion 2.8.5.201 -Force -Confirm:$false | Out-Null
	Install-Module Logging -Confirm:$false -Force
	$LoggingAvailable = Import-Module -Scope Local Logging
}

if ($ShowDebugMessages)
{
	Set-LoggingDefaultLevel -Level 'DEBUG'
	Set-LoggingDefaultFormat -Format '[%{timestamp:+%Y-%m-%d %T UTC%Z}] [%{filename}:%{lineno}] %{level}: %{message} %{execinfo}'
}
else
{
	Set-LoggingDefaultLevel -Level 'INFO'
	Set-LoggingDefaultFormat -Format '[%{timestamp:+%Y-%m-%d %T UTC%Z}] %{level}: %{message}'
}

Add-LoggingTarget -Name Console