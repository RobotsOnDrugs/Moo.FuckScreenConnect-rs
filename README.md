# Moo.FuckScreenConnect-rs


This is a port of the original program written in C# to Rust, made simpler and easier to understand and maintain. Proper documentation coming soon.

This program **must** run as the SYSTEM account *and* access the interactive desktop in order to change ScreenConnect's windows.

[PsExec](https://learn.microsoft.com/en-us/sysinternals/downloads/psexec) can facilitate this. You can put this in the task scheduler and/or run it with a silent access tool.
`-s` to run it as SYSTEM, `-i 1` to run it on the first window station, and `-w` to specify the working directory.

In the task scheduler, be sure to run it as your user but have it run whether user is logged on or not, check the "Do not store password" box, and run with highest privileges.

`psexec -s -i 1 -w 'C:\your-fun-folder' 'C:\your-fun-folder\moo_fuck_screen_connect.exe'`
