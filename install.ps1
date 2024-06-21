##Requires -RunAsAdministrator

# There's a lot of null-passing and null-checking. This sucks, but I'm not sure how to better handle bad states in PowerShell.
# It's a lot clunkier and awkward to pass around exceptions in PS than in C# or with Result<T> in Rust, but this script would probably take longer to write in Rust. >_>
# Feel free to open an issue if you have a better way. It's possibly just a skill issue since I'm a lot less familiar with PS scripting.

<#
	.DESCRIPTION
	Installs FuckScreenConnect either from the current directory or updates it from a GitHub release.
	If there is an error, regardless of any options specified, and with the exception of missing files or the scheduled task, the script will immediately halt and any issues must then be resolved manually.
	.PARAMETER InstallationPath
		Required if there is no existing installation or if the existing installation is broken.
		The full path to the main FSC executable where it is to be installed. You may name this executable whatever you wish as long as the extension is .exe.
		If there is an existing installation, it will be fully uninstalled.
	.PARAMETER Type [Install | Uninstall]
		Optional; defaults to Install.
		Install copies required files to the installation directory and creates a scheduled task. If -InstallationPath is not set, only the binaries will be copied, leaving your configuration as-is, which is basically just an update.
		Uninstall prompts for confirmation and then deletes any installed files and the scheduled task as well as any installation configuration.
	.PARAMETER InstallationSource [ GitHub | Local | Auto ]
		Optional; defaults to Auto. Support for installing/updating via GitHub is currently unimplemented but planned in future releases, so it only performs a local installation.
		Auto will compare the version of the installed binary to the latest release on GitHub and to the binary in the same directory in this script, and the newest one will be used. In the case of a tie, doing nothing is preferred over a local install, which is preferred over grabbing from GitHub.
		If Local is used, installation will proceed from the current directory regardless of the installed or GitHub versions, and abort if there is nothing to install from the current directory. GitHub will always pull the latest release on GitHub and install it.
	.PARAMETER Force
		Optional; unset by default.
		If set, (un)installation will clobber or forcibly delete any existing installations with no confirmation.
	.PARAMETER NonInteractive
		Optional; unset by default.
		If set, all confirmations are suppressed and all defaults for confirmations are used. Additionally, no debug or information messages other than "Starting." and "Finished." will be shown.
		Existing installations that do not need to be updated will not be overwritten unless -Force is set.
	.PARAMETER ShowDebugMessages
		Optional; unset by default. If set, debug messages are shown. Primarily meant for troubleshooting and developer use, and the messages shown are subject to change.
#>

param
(
	[string]$InstallationPath = $null,
	[string][ValidateSet('Install','Uninstall')]$Type = 'Install',
	[string][ValidateSet('Auto','Local', 'GitHub')]$InstallationSource = 'Auto',
	[Switch]$Force,
	[Switch]$NonInteractive,
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

if ($($InstallationPath.Length) -eq 0) { $install_path = $null }
else { $install_path = $InstallationPath }
switch ($Type)
{
	'Install'
	{
		Write-Log -Level INFO -Message "Installing FSC."
		Uninstall-Fsc $Force $install_path
		Install-Fsc $InstallationSource $install_path !$Force
		break
	}
	'Uninstall'
	{
		Write-Log -Level INFO -Message "Uninstalling FSC."
		Uninstall-Fsc $true $install_path
		break
	}
	default { Write-Log -Level ERROR -Message "$Type is not a valid installation type." }
}

Write-Log -Level INFO -Message 'Finished.'
Wait-Logging