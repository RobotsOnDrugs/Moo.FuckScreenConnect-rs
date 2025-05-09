#Requires -RunAsAdministrator

<#
	.DESCRIPTION
	Installs FuckScreenConnect either from the current directory or a specified path.
	If there is an error, regardless of any options specified, the script will immediately halt and any issues must then be resolved manually.
	.PARAMETER ApplicationPath [Path to custom directory]
		Optional; defaults to the Program Files directory. If set, it will install to the custom directory specified.
	.PARAMETER Type [Install | Uninstall]
		Optional; defaults to Install. Installs or uninstalls the service.
	.PARAMETER ServiceName [Name]
		Optional; defaults to "FSC Service". Specifies the name of the FSC service that will appear in e.g., the Services application or Task Manager.
	.PARAMETER ManualStartup
		Optional. If set, the service must be started or stopped manually. Otherwise, the service will start automatically at boot.
	.PARAMETER ShowDebugMessages
		Optional. If set, debug messages are shown. Primarily meant for troubleshooting and developer use, and the messages shown are subject to change.
#>

param
(
	[string]$ApplicationPath = "$env:PROGRAMFILES\FSC Service",
	[string][ValidateSet('Install','Uninstall')]$Type = 'Install',
	[string]$ServiceName = 'FSC Service',
	[Switch]$ManualStartup,
	[Switch]$ShowDebugMessages
)

$compatible = ($PSVersionTable.PSVersion.Major -eq 5) -and ($PSVersionTable.PSVersion.Minor -eq 1);
if (-not $compatible) { Write-Error "This script must be run with PowerShell 5.1 and is not tested on other versions." }

$ErrorActionPreference = 'Stop'

. .\install_modules\logging.ps1
Write-Log -Level DEBUG -Message 'Logging script run.'
Write-Log -Level DEBUG -Message "Running PowerShell $($PSVersionTable.PSVersion)"
. .\install_modules\fsc_functions.ps1
Write-Log -Level DEBUG -Message 'FSC functions script run.'
Write-Log -Level INFO -Message 'Starting.'

$is_absolute = [System.IO.Path]::IsPathRooted($ApplicationPath);
if (!$is_absolute) { $ApplicationPath = "$PWD", "$ApplicationPath" -join '\' }
if ($ManualStartup) { $ManualStartup = $true }
switch ($Type)
{
	'Install'
	{
		Write-Log -Level INFO -Message 'Installing FSC.'
		Set-Installation "$ServiceName" $ManualStartup "$ApplicationPath"
		break
	}
	'Uninstall'
	{
		Write-Log -Level INFO -Message 'Uninstalling FSC.'
		Remove-Installation "$ServiceName" "$ApplicationPath"
		break
	}
	default { Write-Log -Level ERROR -Message "$Type is not a valid installation type." }
}

Write-Log -Level INFO -Message 'Finished.'
Wait-Logging