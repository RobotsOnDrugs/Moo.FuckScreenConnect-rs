# Moo.FuckScreenConnect-rs

Moo.FuckScreenConnect is a tool to make the privacy screen for ScreenConnect Client (and Remote Utilities) semi-transparent, allowing you to see what a scammer is doing while they think their actions are secret.\
Support for remote desktop software (e.g., AnyDesk) that does not use a standard window for its privacy screen is outside the scope of this project.

This is a port of the original formerly private program written in C# to Rust, made simpler and easier to understand and maintain.

Feature requests are welcome, but please file an issue here on GitHub rather than making a direct request elsewhere.

## Installation
The easiest way to install or uninstall FSC is to download the [latest release](https://github.com/RobotsOnDrugs/Moo.FuckScreenConnect-rs/releases) and use the included PowerShell script `install.ps1`. This script must be run as an administrator from the command line. By default, FSC will be installed to the Program files directory. For additional information and reference, see [the installation wiki page](https://github.com/RobotsOnDrugs/Moo.FuckScreenConnect-rs/wiki/Installation).
## Usage
FSC runs as a Windows service. Once installed, it runs in the background and requires no additional interaction. It can be stopped and started in the same manner as any other Windows service - with `sc.exe`, PowerShell cmdlets `Start-Service` and `Stop-Service`, or the Services application.
