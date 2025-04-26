#Requires -RunAsAdministrator

<#
	.DESCRIPTION
	Installs FuckScreenConnect either from the current directory or a specified path.
	If there is an error, regardless of any options specified, the script will immediately halt and any issues must then be resolved manually.
	.PARAMETER ApplicationPath
		Optional; the current directory is used by default. Specifies the directory containing fsc_service.exe and fsc_core.exe.
	.PARAMETER Type [Install | Uninstall]
		Optional; defaults to Install. Installs or uninstalls the service.
	.PARAMETER ServiceName
		Optional; "FSC Service" by default. Specifies the name of the FSC service that will appear in e.g., the Services application or Task Manager.
	.PARAMETER AutomaticStartup
		Optional; false by default. If set, the service will be set to automatically start at boot. If unset, the service will not and can be started manually.
	.PARAMETER ShowDebugMessages
		Optional; unset by default. If set, debug messages are shown. Primarily meant for troubleshooting and developer use, and the messages shown are subject to change.
#>

param
(
	[string]$ApplicationPath = $null,
	[string][ValidateSet('Install','Uninstall')]$Type = 'Install',
	[string]$ServiceName = $null,
	[Switch]$AutomaticStartup,
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

if (-not $ApplicationPath) { $ApplicationPath = $PWD.Path }
$ApplicationPath = "$ApplicationPath", '\fsc_service.exe' -join ''
Write-Log -Level DEBUG -Message "$PWD"
$is_absolute = [System.IO.Path]::IsPathRooted($ApplicationPath);
if (!$is_absolute) { $ApplicationPath = "$PWD", "$ApplicationPath" -join '\' }
if (-not $ServiceName) { $ServiceName = "FSC Service" }
if (-not $AutomaticStartup) { $AutomaticStartup = $false }
switch ($Type)
{
	'Install'
	{
		Write-Log -Level INFO -Message "Installing FSC."
		Set-Installation $ServiceName $AutomaticStartup $ApplicationPath | Out-Null
		break
	}
	'Uninstall'
	{
		Write-Log -Level INFO -Message "Uninstalling FSC."
		Remove-Installation $ServiceName
		break
	}
	default { Write-Log -Level ERROR -Message "$Type is not a valid installation type." }
}

Write-Log -Level INFO -Message 'Finished.'
Wait-Logging