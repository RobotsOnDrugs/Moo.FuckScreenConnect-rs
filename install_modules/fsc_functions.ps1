$ScheduledTaskName = 'FSC Service Task'
$InstallationKeyPath = 'HKLM:\SOFTWARE\Moo\FuckScreenConnect'
$FileNames = @('moo_fuck_screen_connect.exe', 'moo_fuck_screen_connect_debug.exe')

. .\install_modules\external_tools.ps1

enum ExistingInstallationStatus
{
	None
	Normal
	Update
	Broken
}

function Get-Installation
{
	$keypath = $null
	try { $keypath = $(Get-Item -Path $InstallationKeyPath) }
	catch [System.Management.Automation.ItemNotFoundException] { Write-Log -Level DEBUG -Message "Installation configuration is not set."; return $null }
	catch
	{
		Stop-ScriptWithError -ErrorMessage "There was a non-recoverable error getting installation information: $($_.Exception.Message)"
	}
	if ($keypath)
	{
		[Microsoft.Win32.RegistryKey]$keypath = $keypath
		Write-Log -Level DEBUG -Message "Got keypath."
		return $keypath
	}
	else
	{
		Stop-ScriptWithError -ErrorMessage "State for Get-Installation is likely invalid."
	}
}
$InstallationKey = Get-Installation
function Set-Installation
{
	param([string]$InstallationSource, [string]$install_path, [bool]$files_only)
	Write-Log -Level DEBUG -Message "Setting installation."
	New-Installation $InstallationSource $install_path $files_only
	Write-Log -Level DEBUG -Message "Skipping installation."
}
function New-Installation
{
	param([String]$InstallationSource, [String]$install_path, [bool]$files_only)

	if (-not $files_only)
	{
		Write-Log -Level INFO -Message "Creating a new installation."
		Remove-Item -Path $InstallationKeyPath -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
	}

	Remove-Item $install_path -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
	New-Item -Type Directory $install_path -Force -Confirm:$false | Out-Null
	Get-PsExec | Out-Null
	Move-Item 'PsExec.exe' $install_path
	$psexec_path = Join-Path $install_path 'PsExec.exe'
	foreach ($file_name in $FileNames)
	{
		try { Copy-Item $file_name $install_path }
		catch [System.Management.Automation.ItemNotFoundException]
		{
			Remove-Item -Recurse $install_path -Force -ErrorAction SilentlyContinue | Out-Null
			Stop-ScriptWithError -ErrorMessage "$file_name was not found in the current directory. Cannot continue."
		}
		catch
		{
			Remove-Item -Recurse $install_path -Force -ErrorAction SilentlyContinue | Out-Null
			Stop-ScriptWithError -ErrorMessage "Could not copy files: $(Error[0].Message). Cannot continue."
		}
	}
	Write-Log -Level INFO -Message "Files copied to $install_path."

	$key = New-Item -Path $InstallationKeyPath -Force
	$key.SetValue('InstallationPath', $install_path)
	$log_dir = New-Item -Type Directory $(Join-Path $install_path 'logs')
	$key.SetValue('LogDirectory', $log_dir)
	$key.SetValue('PsExecPath', $psexec_path)
	Write-Log -Level INFO -Message "Configuration created."
	if ($files_only)
	{
		Write-Log -Level INFO -Message "Retaining old scheduled task and configuration."
		return
	}

	$full_path = Join-Path $install_path 'moo_fuck_screen_connect.exe'
	$task_action = New-ScheduledTaskAction -Execute $psexec_path -Argument "-s -i 1 -w `"$install_path`" `"$full_path`"" -WorkingDirectory "$install_path"
	$task_settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -Compatibility Win8 -DontStopIfGoingOnBatteries -DontStopOnIdleEnd -Hidden -MultipleInstances IgnoreNew -RestartCount 3 -StartWhenAvailable -WakeToRun
	$task_trigger = New-ScheduledTaskTrigger -AtLogon
	$task_principal = New-ScheduledTaskPrincipal -LogonType S4U -RunLevel Highest -UserId (whoami)
	Register-ScheduledTask -TaskName $ScheduledTaskName -Action $task_action -Principal $task_principal -Settings $task_settings -Trigger $task_trigger | Out-Null
	Write-Log -Level INFO -Message "Scheduled task created."
}

function Get-FileInstallationStatus
{
	param([String]$install_path, [bool]$install)

	if (-not $(Get-Installation)) { return [ExistingInstallationStatus]::None }
	$expected_path_properties = @('InstallationPath', 'LogDirectory')
	foreach ($property in $expected_path_properties)
	{
		$value = $InstallationKey.GetValue($property)
		if ($null -eq $value)
		{
			Write-Log -Level WARNING -Message "Value for $property is missing."
			return [ExistingInstallationStatus]::Broken
		}
		if ($null -eq $(Get-Item $value -ErrorAction SilentlyContinue)) {return [ExistingInstallationStatus]::None }
	}

	if (-not $install_path) { $install_path = $InstallationKey.GetValue('InstallationPath') }
	try { Get-Item $install_path -ErrorAction Stop }
	catch
	{
		if (-not $install_path) { Write-Log -Level DEBUG -Message "No installation path was specified." }
		else { Write-Log -Level DEBUG -Message "$install_path missing." }
		return [ExistingInstallationStatus]::None
	}
	return [ExistingInstallationStatus]::Normal
}
function Get-ScheduledTaskInstallationStatus
{
	try { $null = Get-ScheduledTask -TaskName $ScheduledTaskName -ErrorAction Stop }
	catch [Microsoft.PowerShell.Cmdletization.Cim.CimJobException]
	{
		if ($_.CategoryInfo.Category -eq [System.Management.Automation.ErrorCategory]::ObjectNotFound) { return [ExistingInstallationStatus]::None }
		else
		{
			Write-Log -Level ERROR -Message "Could not access the FSC scheduled task: $($_.CategoryInfo)"
			return [ExistingInstallationStatus]::Broken
		}
	}
	catch
	{
		Write-Log -Level ERROR -Message "Could not access the FSC scheduled task: $($_)"
		return [ExistingInstallationStatus]::Broken
	}
	return [ExistingInstallationStatus]::Normal
}
function Get-InstallationStatus
{
	param([String]$install_path, [bool]$install)
	$files = Get-FileInstallationStatus $install_path $install
	if ($files -eq [ExistingInstallationStatus]::Broken)
	{
		Write-Log -Level DEBUG -Message "Files broken."
		return [ExistingInstallationStatus]::Broken
	}
	$task = Get-ScheduledTaskInstallationStatus
	if ($task -eq [ExistingInstallationStatus]::Broken) { Write-Log -Level DEBUG -Message "Current scheduled task is broken."; return [ExistingInstallationStatus]::Broken }
	if ($task -eq [ExistingInstallationStatus]::Normal) { Write-Log -Level DEBUG -Message "Current scheduled task is normal." }
	if ($task -eq [ExistingInstallationStatus]::None) { Write-Log -Level DEBUG -Message "No scheduled task is currently registered." }

	if ($files -eq [ExistingInstallationStatus]::Normal) { Write-Log -Level DEBUG -Message "Current file installation is normal." }
	if ($files -eq [ExistingInstallationStatus]::None) { Write-Log -Level DEBUG -Message "Files are not currently installed." }
	if (($files -eq [ExistingInstallationStatus]::Normal) -and ($task -eq [ExistingInstallationStatus]::Normal)) { return [ExistingInstallationStatus]::Normal }
	if (($files -eq [ExistingInstallationStatus]::None) -and ($task -eq [ExistingInstallationStatus]::None)) { return [ExistingInstallationStatus]::None }
	if (($files -eq [ExistingInstallationStatus]::None) -and ($task -eq [ExistingInstallationStatus]::Normal)) { return [ExistingInstallationStatus]::Update }
	return [ExistingInstallationStatus]::Broken
}

function Install-Fsc
{
	param([String]$install_source, [String]$install_path, [Switch]$force)
	$status = Get-InstallationStatus $install_path $true
	if ($status -eq [ExistingInstallationStatus]::Broken)
	{
		Stop-ScriptWithError -Message "The existing installation is broken. Cannot continue. Please try forcibly uninstalling FSC or resolving any issues manually."
	}
	if ($install_path)
	{
		Write-Log -Level INFO -Message "Installation location: ${install_path}."
		switch ($status)
		{
			([ExistingInstallationStatus]::Normal)
			{
				Write-Log -Level INFO -Message "FSC is already installed."
				if ($Force) { Set-Installation $install_source $install_path $false }
				return
			}
			([ExistingInstallationStatus]::None)
			{
				Write-Log -Level INFO -Message "Installing from scratch."
				Set-Installation $install_source $install_path $false
				break
			}
			([ExistingInstallationStatus]::Update)
			{
				Write-Log -Level INFO -Message "Updating files."
				Set-Installation $install_source $install_path $true
				break
			}
			default { "Installation is broken." }
		}
	}
	else
	{
		switch ($status)
		{
			([ExistingInstallationStatus]::None) { Stop-ScriptWithError -ErrorMessage "No existing installation was found and no installation path was given. Cannot continue."; break }
			{ (([ExistingInstallationStatus]::Normal) -or ([ExistingInstallationStatus]::Update)) }
			{
				$install_path = $InstallationKey.GetValue('InstallationPath')
				Install-Fsc $install_source $install_path $force
				break
			}
			([ExistingInstallationStatus]::Broken) { Stop-ScriptWithError -ErrorMessage "Installation is broken."; break }
			default { Stop-ScriptWithError -ErrorMessage "Invalid state when getting installation status. This is a bug."; break }
		}
	}

}
function Uninstall-Fsc
{
	param([bool]$FullUninstall, [String]$install_path)

	$status = Get-InstallationStatus $install_path $false
	if ($status -eq [ExistingInstallationStatus]::None) { return }
	if ($status -eq [ExistingInstallationStatus]::Broken)
	{
		Write-Log -Level INFO -Message "The current installation of FSC is broken."
	}
	if ($FullUninstall -or ($status -eq [ExistingInstallationStatus]::Broken))
	{
		Write-Log -Level INFO -Message "Performing a full uninstallation of FSC."
		if (-not $install_path) { $install_path = $InstallationKey.GetValue('InstallationPath') }
		Unregister-ScheduledTask -TaskName "$ScheduledTaskName" -TaskPath "*\" -Confirm:$False -ErrorAction SilentlyContinue
		Remove-Item -Recurse $install_path -Force -ErrorAction SilentlyContinue
		Remove-Item -Recurse $InstallationKeyPath -Force -ErrorAction SilentlyContinue
		return
	}
	if ($status -eq [ExistingInstallationStatus]::Normal)
	{
		if (-not $Force) { return }
		$status = [ExistingInstallationStatus]::Update
	}
	if ($status -eq [ExistingInstallationStatus]::Update)
	{
		Write-Log -Level INFO -Message "Keeping current configuration and updating files."
		trap { Remove-Item -Recurse $install_path -Force -ErrorAction SilentlyContinue }
		return
	}
	Stop-ScriptWithError -ErrorMessage "Installation state is undefined."
}