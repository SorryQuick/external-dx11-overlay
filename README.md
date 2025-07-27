# external-dx11-overlay
Allows an external overlay to render inside a game as a DX11 render hook. Originally made for BlishHUD and Guild Wars 2.
It can easily be changed to work for any game with any overlay.

# How to use
For simple steps without having to compile anything: see https://github.com/SorryQuick/external-dx11-overlay/blob/master/Simple-User-Guide.md

There are four steps to make this work.

- Compile this repo and get the dll.
- Compile a forked version of BlishHUD from https://github.com/SorryQuick/Blish-HUD
- If you are on Linux/Mac, you need to somehow get BlishHUD launched into your wine prefix. A quick way to do this (which probably won't work with steam) is to run something like this:
  ```WINEFSYNC=1 WINEPREFIX=<prefix> <wine binary> "Blish HUD.exe"```. With the same prefix and wine binary you used to launch the game (eg proton's wine binary).
- You need to load this DLL into the game's process. It will react well with any LoadLibraryW loader. You can also just google or search github for any dll injector out there and run it in the same prefix just like Blish. Eventually, this could support existing loaders like arcdps. I've been using https://github.com/SorryQuick/Gw2-Simple-Addon-Loader

# Current status
A lot of the core issues have been solved and it should now work pretty well. 
If you encounter any problem, create an issue on github.
Current known issues:

- Sound is currently disabled. Seemingly cause by Wine and out of my control, though could be worked around eventually. 
- Shared Memory should be allocated dynamically. Currently it allocates way too much.

# Compiling
```cargo +nightly build --release```
