# Moo.FuckScreenConnect-rs

Moo.FuckScreenConnect is a tool to make the privacy screen for ScreenConnect Client (and Remote Utilities) semi-transparent, allowing you to see what a scammer is doing while they think their actions are secret.\
Support for remote desktop software (e.g., AnyDesk) that does not use a standard window for its privacy screen is outside the scope of this project.

This is a port of the original formerly private program written in C# to Rust, made simpler and easier to understand and maintain. Now with support for Remote Utilities!

Feature requests are welcome, but please file an issue here on GitHub rather than making a direct request elsewhere.

## Installation
The easiest way to install or uninstall FSC is to download the [latest release](https://github.com/RobotsOnDrugs/Moo.FuckScreenConnect-rs/releases) and use the included PowerShell script `install.ps1`. This script must be run as an administrator from the command line. You may need to set PowerShell's execution policy to allow the script to run.
```powershell
Set-ExecutionPolicy -ExecutionPolicy Bypass
```
or, for only the script:
```powershell
Unblock-File '.\install.ps1'
```
If updating from version 0.3.x or earlier, it is highly advised to uninstall using the installation script from that release before installing version 0.4.

### install.ps1 Optional Parameters
 - `ApplicationPath [custom\path\here]` - Specifies an alternate installation directory. Note: when uninstalling, this parameter must be set to the directory specified during installation.
 - `Type [Install|Uninstall]` - Specifies whether to install or uninstall FSC. By default, it will install.
 - `ServiceName [Name]` - Specifies a custom name for the service which will appear in places where services are displayed such as Task Manager or the Services application. When uninstalling, this parameter must be set to the name specified during installation.
 - `ManualStartup` - Specifies whether to start the service automatically at boot or only when started manually. If set, it will not start automatically at boot and must be started manually.
 - `ShowDebugMessages` - Shows extra information that is intended for aiding development. The messages that are displayed are subject to change.

## Usage
FSC runs as a Windows service. Once installed, it will run in the background and requires no additional interaction if set to automatic startup. It can be stopped and started in the same manner as any other Windows service.