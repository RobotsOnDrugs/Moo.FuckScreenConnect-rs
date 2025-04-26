# Moo.FuckScreenConnect-rs

Moo.FuckScreenConnect is a tool to make the privacy screen for ScreenConnect Client (and Remote Utilities) semi-transparent, allowing you to see what a scammer is doing while they think their actions are secret.\
Support for remote desktop software (e.g., AnyDesk) that does not use a standard window for its privacy screen is outside the scope of this project, but may be supported in a separate project in the future.

This is a port of the original formerly private program written in C# to Rust, made simpler and easier to understand and maintain. Now with support for Remote Utilities!

## Planned features
- Elimination of the need for psexec.
- A better installation experience. The current script is known to be fragile and overly complicated.
- ~~Support for other remote desktop software that uses a window for its privacy screen.~~ (maybe - most remote desktop software uses much more complicated methods to provide a privacy screen, and it requires more research to begin manipluating such privacy screens.)

Feature requests are welcome, but please file an issue here on GitHub rather than making a direct request elsewhere.

## Prerequisites
### MSVC
This program requires the Microsoft Visual C++ redistributable libraries. Download and install them from the Microsoft website if you encounter issues regarding missing DLLs: ( [x86](https://aka.ms/vs/17/release/vc_redist.x86.exe) | [x64](https://aka.ms/vs/17/release/vc_redist.x64.exe) ). It is likely that the x86 version is not needed, but installing it anyway.\
This program runs only on 64-bit versions of Windows and is not tested on versions of Windows older than Windows 10. If you have a use case for 32-bit binaries or official support for an older version of Windows, file a feature request.

### PsExec
This program **must** run as the SYSTEM account *and* have access to the interactive desktop in order to modify windows there.
A special tool called [PsExec](https://learn.microsoft.com/en-us/sysinternals/downloads/psexec) can facilitate this. Place this command in the task scheduler and/or run it with a silent access tool. This is a sample where PsExec has been installed via Chocolatey, but you may place it wherever you like as long as the full path to it is specified. You must also specify the full path to the FSC executable.

```powershell
C:\ProgramData\chocolatey\bin\psexec.exe -s -i 1 -w 'C:\your-fun-folder' 'C:\your-fun-folder\moo_fuck_screen_connect.exe'
```
`-s` to run it as SYSTEM, `-i 1` to run it on the first window station (which is the interactive desktop), and `-w` to specify the working directory. Administrator privileges are required.

### Scheduled task (optional, but recommended)
In the task scheduler, be sure to run it as your user but have it run whether user is logged on or not, set the trigger to run it at log on of your user, check the "Do not store password" box, and run with highest privileges.
