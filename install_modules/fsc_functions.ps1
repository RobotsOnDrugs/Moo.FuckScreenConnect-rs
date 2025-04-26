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
	Write-Log -Level DEBUG -Message ("$($service.PathName)" -replace " $service_name$")
	if (!(Test-Path -Path ($($service.PathName) -Replace " $service_name$"))) { Write-Log -Level DEBUG -Message "Installation status of ${service_name}: Broken"; return [ExistingInstallationStatus]::Broken }
	Write-Log -Level DEBUG -Message "Installation status of ${service_name}: Normal"
	return [ExistingInstallationStatus]::Normal
}

function Set-Installation
{
	param([string]$service_name, [bool]$is_automatic, [string]$path)
	$not_installed = ((Get-InstallationStatus "$service_name") -eq [ExistingInstallationStatus]::None)
	if (!$not_installed) { Remove-Installation "$service_name" }
	Write-Log -Level INFO -Message "Installing $service_name."
	if ($is_automatic) { $start_type = 'Automatic' }
	else { $start_type = 'Manual' }
	New-Service -Name "$service_name" -BinaryPathName "$path $service_name" -DisplayName "$service_name" -StartupType $start_type | Out-Null
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
	param([string]$service_name)
	Stop-Service -Name "$service_name" -Force | Out-Null
	Invoke-Expression -Command ("sc.exe " + "delete " + '"$service_name"') # there is no Remove-Service in Powershell 5.1 but thankfully this is a very simple command in sc
	$status = (Get-InstallationStatus $service_name)
	Write-Log -Level DEBUG "After deletion: $status"
	if ($status -ne [ExistingInstallationStatus]::None) { Stop-ScriptWithError -ErrorMessage "Could not delete existing service $service_name." }
	Write-Log -Level INFO -Message "Deleted existing service $service_name."
}