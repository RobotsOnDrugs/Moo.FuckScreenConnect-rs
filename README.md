# Moo.FuckScreenConnect-rs


This is a port of the original program written in C# to Rust, made simpler and easier to understand and maintain. Proper documentation coming soon.

## Prerequisites
### MSVC
This program requires the Microsoft Visual C++ redistributable libraries. Download and install them from the Microsoft website if you encounter issues regarding missing DLLs: ( [x86](https://aka.ms/vs/17/release/vc_redist.x86.exe) | [x64](https://aka.ms/vs/17/release/vc_redist.x64.exe) )

### PsExec
This program **must** run as the SYSTEM account *and* access the interactive desktop in order to modify ScreenConnect Client's windows.
A special tool called [PsExec](https://learn.microsoft.com/en-us/sysinternals/downloads/psexec) can facilitate this. Put this command in the task scheduler and/or run it with a silent access tool.

```powershell
psexec -s -i 1 -w 'C:\your-fun-folder' 'C:\your-fun-folder\moo_fuck_screen_connect.exe'
```
`-s` to run it as SYSTEM, `-i 1` to run it on the first window station, and `-w` to specify the working directory.

Alternatively, there is [ExecTI](https://winaero.com/download-execti-run-as-trustedinstaller/), which runs with a GUI. You can run the command directly or launch PowerShell and run it from there.

Both require administrator privileges.

### Task scheduler (optional, but recommended)
In the task scheduler, be sure to run it as your user but have it run whether user is logged on or not, check the "Do not store password" box, and run with highest privileges.


