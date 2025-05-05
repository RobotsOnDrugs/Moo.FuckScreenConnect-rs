
<#
  .DESCRIPTION
  Builds the FSC executable and prepares it for release. Final binaries will be copied to .\build and compressed to 7zip (LZMA) and zip archives.
  .PARAMETER FileDescription <Description>
  Optional; no metadata is set if unset. Specifies the value of FileDescription in the binary's manifest, which is what will displayed in Task Manager. Specifying this with something innocuous this is recommended in case a very suspicious scammer looks at Task Manager.
  .PARAMETER AdditionalOutputDirs
  Optional. An array of additional directories to copy binaries to. Must be absolute paths. Files copied to additional directories will not be packed into archives.
#>

param
(
	[Parameter()][string]$FileDescription = 'FSC Service',
	[Parameter()][System.Collections.Generic.HashSet[String]]$AdditionalOutputDirs
)
$ErrorActionPreference = 'Stop'

$name = 'Moo.FuckScreenConnect-rs'
$version_common = (cargo.exe read-manifest --manifest-path .\fsc_common\Cargo.toml | ConvertFrom-Json).version
$version_service = (cargo.exe read-manifest --manifest-path .\fsc_service\Cargo.toml | ConvertFrom-Json).version
$version_core = (cargo.exe read-manifest --manifest-path .\fsc_core\Cargo.toml | ConvertFrom-Json).version
$bad_version = $false
if ($version_common -ne $version_service) { $bad_version = $true }
if ($version_core -ne $version_service) { $bad_version = $true }
if ($bad_version) { Write-Error -Message "Package versions do not match. This is a bug and should be reported." }

$env:BINARY_FILE_DESCRIPTION="$FileDescription"

if ($Clean) { cargo.exe clean }
if ($LASTEXITCODE -eq 0)
{
	$new_path = '.\build'
	$(Remove-Item -Recurse -Force -Path $new_path -ErrorAction SilentlyContinue) | Out-Null
	$(New-Item -Force -Type Directory $new_path) | Out-Null
	cargo.exe build
	cargo.exe build --release
	Copy-Item -Path '.\target\debug\fsc_core.exe' -Destination '.\build\fsc_core.exe'
	Copy-Item -Path '.\target\debug\fsc_service.exe' -Destination '.\build\fsc_service.exe'
	Copy-Item -Path '.\target\release\fsc_core.exe' -Destination '.\build\fsc_core.exe'
	Copy-Item -Path '.\target\release\fsc_service.exe' -Destination '.\build\fsc_service.exe'
	Copy-Item -Path '.\install.ps1' -Destination '.\build\install.ps1'
	Copy-Item -Recurse -Path '.\install_modules' -Destination '.\build\'
}
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Push-Location -Path .\build
foreach ($additional_output_dir in $AdditionalOutputDirs)
{
	if (![System.IO.Path]::IsPathRooted($additional_output_dir)) { Write-Error "'$($additional_output_dir)' is not an absolute path!" }
	if (!(Test-Path $additional_output_dir)) { Out-Null $(New-Item -Type Directory $additional_output_dir) }
	$(Remove-Item -Recurse -Path $additional_output_dir -ErrorAction SilentlyContinue) | Out-Null
	Copy-Item -Recurse '.' $additional_output_dir
}
$(Remove-Item -Force -Path test.7z -ErrorAction SilentlyContinue) | Out-Null
7za a -t7z -m0=LZMA2 -mx9 -mmt8 -aoa "${name}.v${version}.7z" * | Out-Null
7za a -tzip -m0=Deflate64 -mpass=15 -mfb=256 -mx9 -mmt8 -aoa "${name}.v${version}.zip" * | Out-Null
Pop-Location