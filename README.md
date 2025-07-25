# external-dx11-overlay
Allows an external overlay to render inside a game as a DX11 render hook. Originally made for BlishHUD and Guild Wars 2.
It can easily be changed to work for any game with any overlay.

# How to use
There are four steps to make this work.

- Compile this repo and get the dll.
- Compile a forked version of BlishHUD from https://github.com/SorryQuick/Blish-HUD
- If you are on Linux/Mac, you need to somehow get BlishHUD launched into your wine prefix. A quick way to do this (which probably won't work with steam) is to run something like this:
  ```WINEFSYNC=1 WINEPREFIX=<prefix> <wine binary> "Blish HUD.exe"```. With the same prefix and wine binary you used to launch the game (eg proton's wine binary).
- You need to load this DLL into the game's process. It will react well with any LoadLibraryW loader. You can also just google or search github for any dll injector out there and run it in the same prefix just like Blish. Eventually, this could support existing loaders like arcdps. I've been using https://github.com/SorryQuick/Gw2-Simple-Addon-Loader

# Current status
Obviously this is very early in development. As I write this, there are still a few issues. Some of those being:

- Sound is currently disabled. Even a vanilla BlishHUD crashes on my system without disabling its sound. Eventually, this should be fixed. The commented code is in the first commit.
- There is currently a significant amount of lag. It shouldn't affect the game's performance at all, but the overlay itself is not very smooth. This is very fixeable however, and will be a pretty big priority soon.
- Shared Memory should be allocated dynamically. Currently it allocates way too much.
- Sometimes Blish crashes when I open it. It's very random and only happened once or twice.
- The Blish console gets flooded with "resource released while mapped". This sometimes happens because of wine (while windows silently ignores it). It could be worth seeing if this is relevant or not.
- Probably a lot more bugs to discover.

# Compiling
```cargo +nightly build --release```
