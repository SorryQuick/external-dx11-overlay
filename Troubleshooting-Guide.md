# Troubleshooting Guide
This guide is meant to help debug issues as easily as possible. Common issue can be found towards the end. The two goals are as follows:
- Help you find exactly what your issue is and how to solve it
- Failing that, help you pinpoint the exact cause so it can be reported in a way that's easy to fix.

## What is causing my issue?
The first important step it understanding where the issue lies. There are three parts to making Blish work on linux.
1. The external-dx11-overlay dll
2. The loader (Gw2-Simple-Addon-Loader)
3. BlishHUD itself

Figuring out which one of those three is causing the issue makes troubleshooting much easier.

### Steps to follow to pinpoint the failing part (if the issue itself lies in Blish not working/showing/loading)
- Is the game loading? If not, then the problem is either the loader, or your steam/lutris setup.
- Is the dll loaded? This can be verified by looking for a log in LOADER_public/logs/loader-xxxxxxx which should start with 'dll-' and with the date and time you launched the game at. The only way for this log to exist is if the dll was loaded in the first place, as it is the one creating the file.
- Is BlishHUD running? This can be verified via top/ps/htop/btop or any task manager you might have. If it is running, there will typically be an icon in your tray, if you have one.

Hopefully by following these steps you now know what part is failing. 

### The loader is the problem. Now what?
- First, check the logs at LOADER_public/logs/loader-xxxxxxx. If something major happened, it should be there.
- Try looking for the direct terminal logs. Eg. the Show Logs button on lutris.
- Try running it in the terminal and looking for errors. Eg. Go into the LOADER_public directory and run ```WINEPREFIX=YOURPREFIX ./Gw2-Simple-Addon-Loader.exe``` with YOURPREFIX being your wine prefix.
- Try creating a new wine prefix instead. On steam, just remove the one in compatdata (Not the gw2 one, but the custom loader one. You can usually know which is it with the modify date)
  
If it did not help, or if you are now left with some sort of an error, report an issue here on github, or ask on the blishhud discord. Make sure to include as much information as you managed to gather.

### BlishHUD is the problem. Now what?
- Try running BlishHUD itself directly in a terminal and inspect the output. ```WINEPREFIX=YOURPREFIX ./PATH_TO_BLISH_HUD.exe``` with YOURPREFIX being the same wine prefix you use to launch the loader with.
- If you see a ```mscoree.dll not found``` type error, it is likely related to dotnet48 being missing or corrupted. See the installation guide on how to install it. If it's corrupted, you may need to create a new prefix instead.
- You can also look for BlishHUD logs found in LOADER_public/logs/BlishHUD-xxxxxxx
- The original Blish logs (unrelated to this, but could be helpful sometimes) can be found inside your wine prefix, in ```drive_c/users/MYUSER/Documents/Guild Wars 2/addons/blishhud/logs```

If it did not help, or if you are now left with some sort of an error, report an issue here on github, or ask on the blishhud discord. Make sure to include as much information as you managed to gather.

### The DLL is the problem. Now what?
- Any and all panics or crashes as well as regular logs are all found in LOADER_public/logs/dll-xxxxxxx
- They can also be seen in-game in the debug overlay by pressing (by default) CTRL-ALT-D. They are not as detailed as the log file itself.
- If you are experiencing performance issues, you can pinpoint if it's rendering or processing related by disabling either or both of them temporarily, with CTRL-ALT-B and CTRL-ALT-N respectively. Expect visual glitches.
- If the game itself crashes, then the DLL is the problem.
- Generally, this is the part least prone to silent failure. If it fails, it will panic, crash an/or freeze the game, but will generally not fail while the game keeps working flawlessly.

If it did not help, or if you are now left with some sort of an error, report an issue here on github, or ask on the blishhud discord. Make sure to include as much information as you managed to gather.

## Investigating Crashes

The first step is to figure out what part crashes. 
- If the game is launched successfully, then it is not the loader. Look for logs in LOADER_public/logs or terminal output. This can be run independently.
- If the game itself crashes, then it is definitely the DLL. Look for logs in LOADER_public/logs.
- The easiest way to tell if Blish itself crashed is to check with top/ps/htop/btop or any task manager and see if it's still running. You can also look for an icon in your system tray. You can look for logs in LOADER_public/logs, but it's entirely possible for it to crash silently. You can try to restart it with CTRL-ALT-O.

From there, open an issue on github or ask for help on the blishhud discord. Make sure to include as much information as you managed to gather.

## Common Issues / Things to verify

- Make sure you extracted the zip file in the right directory. You should see the following path: path_to_gw2/addons/LOADER_public/Gw2-Simple-Addon-Loader.exe
- Make sure you are running the game with the latest proton version (on linux). You may also want to have the same proton version for both the game AND the loader on steam. If you are not using steam, you can still use proton via UMU, now used by default on lutris.
- Make sure steam is set to use a **specific** version of proton, as the steam global one may not apply and/or cause issues.
- If for whatever reason Blish asks you for an update, do **NOT** update it. For now, the only way to update it is to download a new release from this repo. Modules themselves may still be updated freely.
- If you are using the Event Table module, it needs a specific version that works with WINE, you cannot use the normal one.
- Issues installing dotnet48. First, make sure this is required to begin with, it usually is not. If it tells you it's already installed or a more recent version exists but it still does not work, try making a new prefix and installing dotnet48 into that fresh one. If you get error messages related to 32/64 bits, this is normal, just ignore.


## Known issues
These are some known issues that are being worked on. This **may be outdated**. For more information, look at the issues page.

- Performance issues, especially when actively using Pathing.
- A variety of crashes on the Blish side.
- No audio
- Issues and crashes adding api-key and enabling modules. This can be brute-forced until it works..
- Potential incompatibilities with reshade.
