enum ExistingInstallationStatus
{
	None
	Normal
	Broken
}

function Get-InstallationStatus
{
	param([string]$service_name)
	Write-Log -Level DEBUG -Message "Getting installation status for $service_name."
	$service = Get-WmiObject win32_service | Where-Object {$_.Name -eq $service_name}
	if ($null -eq $service) { Write-Log -Level DEBUG -Message "Installation status of ${service_name}: None"; return [ExistingInstallationStatus]::None }
	Write-Log -Level DEBUG -Message ("$($service.PathName)" -replace " `"$service_name`"" -replace '"')
	if (!(Test-Path -Path ("$($service.PathName)" -Replace " `"$service_name`"" -replace '"'))) { Write-Log -Level DEBUG -Message "Installation status of ${service_name}: Broken"; return [ExistingInstallationStatus]::Broken }
	Write-Log -Level DEBUG -Message "Installation status of ${service_name}: Normal"
	return [ExistingInstallationStatus]::Normal
}

function Set-Installation
{
	param([string]$service_name, [bool]$is_automatic, [string]$path)
	$clean = ((Get-InstallationStatus "$service_name") -eq [ExistingInstallationStatus]::None)
	if (!$clean) { Remove-Installation "$service_name" "$path" }
	Write-Log -Level INFO -Message "Installing $service_name."
	if ($is_automatic) { $start_type = 'AutomaticDelayedStart' }
	else { $start_type = 'Manual' }
	New-Item -ItemType Directory -Path "$path" -Force | Out-Null
	Copy-Item '.\fsc_service.exe' "$path"
	Copy-Item '.\fsc_core.exe' "$path"
	$service_path = "$path", '\fsc_service.exe' -join ''
	New-Service -Name "$service_name" -BinaryPathName "`"$service_path`" `"$service_name`"" -DisplayName "$service_name" -StartupType $start_type | Out-Null
	$status = (Get-InstallationStatus $service_name)
	if ($status -ne [ExistingInstallationStatus]::Normal) { Stop-ScriptWithError -ErrorMessage "Could not create service $service_name." }
	Start-Service -Name "$service_name" | Out-Null
	$start_status = (Get-Service -Name "$service_name").Status
	Write-Log -Level DEBUG "Start status: $start_status"
	if (!$start_status.Equals([System.ServiceProcess.ServiceControllerStatus]::Running)) { Stop-ScriptWithError -ErrorMessage "Could not start service $service_name." }

	return Get-InstallationStatus $service_name
}

function Remove-Installation
{
	param([string]$service_name, [string]$path)
	Stop-Service -Name "$service_name" -Force -ErrorAction Ignore | Out-Null
	Invoke-Expression -Command ("sc.exe " + "delete " + '"$service_name"') | Out-Null # there is no Remove-Service in Powershell 5.1 but thankfully this is a very simple command in sc
	$status = (Get-InstallationStatus $service_name)
	Write-Log -Level DEBUG "After deletion: $status"
	if ($status -ne [ExistingInstallationStatus]::None) { Stop-ScriptWithError -ErrorMessage "Could not delete existing service $service_name." }
	Write-Log -Level INFO -Message "Deleted service $service_name."
	Remove-Item -Path ("$path", '\fsc_service.exe' -join '') -Force -ErrorAction Ignore | Out-Null
	Remove-Item -Path ("$path", '\fsc_core.exe' -join '') -Force -ErrorAction Ignore | Out-Null
}