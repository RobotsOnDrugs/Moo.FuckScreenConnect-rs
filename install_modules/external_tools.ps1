$PsExecHash = '3E2272B916DA4BE3C120D17490423230AB62C174'
$FscTempFolder = Join-Path $env:TEMP "fsc_temp"

function Get-PsExec
{
	$PsExecPath = (Get-Command psexec).Source
	$hash = Compare-FileHash $PsExecPath
	# TODO: examine targets of Chocolatey shims to avoid downloading when there's a perfectly good copy laying around.
	# Hint: spawn `C:\ProgramData\chocolatey\bin\PsExec.exe --shimgen-help` via [System.Diagnostics.ProcessStartInfo]::new()
	if ($hash -eq $PsExecHash)
	{
		Write-Log -Level DEBUG -Message 'PsExec was found in the path.'
		Remove-Item -Recurse $FscTempFolder -Force -ErrorAction SilentlyContinue
		return $PsExecPath
	}

	$PsExecPath = Join-Path $PWD 'PsExec.exe'
	$hash = Compare-FileHash $PsExecPath
	if ($hash -eq $PsExecHash)
	{
		Write-Log -Level DEBUG -Message 'PsExec was found in the current directory.'
		Remove-Item -Recurse $FscTempFolder -Force -ErrorAction SilentlyContinue
		return $PsExecPath
	}

	New-Item -Type Directory $FscTempFolder -Force | Out-Null
	Push-Location $FscTempFolder
	$PsExecPath = Join-Path $PWD 'PSTools\PsExec.exe'

	$valid_psexec_in_temp = $hash -eq $PsExecHash
	switch ($valid_psexec_in_temp)
	{
		$true
		{
			Pop-Location
			Copy-Item -Path $PsExecPath -Destination '.' -Force
		}
		$false
		{
			New-Item -Type Directory '.\PSTools' -Force | Out-Null
			$psexec_url = "https://download.sysinternals.com/files/PSTools.zip"
			$curl = $(Get-Command curl).Source
			if ($null -ne $curl) { & $curl -k -s $psexec_url -o PSTools.zip }
			else { Invoke-WebRequest -Uri $psexec_url -OutFile 'PSTools.zip' }

			Expand-Archive -LiteralPath '.\PSTools.zip' -DestinationPath '.\PSTools' -Force | Out-Null
			$psexec_exe_path = Join-Path $FscTempFolder 'PSTools\PsExec.exe'
			Pop-Location
			Copy-Item -Path $psexec_exe_path -Destination '.' -Force
			$PsExecPath = Join-Path $PWD 'PsExec.exe'
		}
	}
	Remove-Item -Recurse $FscTempFolder -Force
	return $PsExecPath
}

function Compare-FileHash
{
	param([String]$FilePath)
	return (Get-FileHash -Algorithm SHA1 $FilePath -ErrorAction SilentlyContinue).Hash -eq $PsExecHash
}