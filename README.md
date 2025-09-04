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
- Texture Sharing must be enabled. Usually this means with proton and a recent version of DXVK.

# Nexus Integration

This project optionally supports integration with the [Nexus](https://github.com/zerthox/nexus) addon framework. When built with the `nexus` feature, it provides:

## Benefits of Nexus Integration

- **Simplified Installation**: Just drop the DLL in your Nexus addons directory
- **User-Friendly Interface**: ImGui-based configuration window with file browser
- **Automatic Management**: Nexus handles addon loading, unloading, and dependency management
- **Quick Access**: Keyboard shortcut (ALT+SHIFT+1) to open the configuration window
- **Persistent Settings**: Executable path and launch preferences are automatically saved

## Building with Nexus Support

To build with Nexus integration enabled:

```bash
cargo build --features nexus --release
```

## Using with Nexus

1. Build the project with the `nexus` feature enabled
2. Place the resulting DLL in your Nexus addons directory
3. Launch Guild Wars 2 with Nexus
4. Enable the "Blish HUD overlay loader" addon in Nexus
5. Use ALT+SHIFT+1 to open the configuration window
6. Browse and select your BlishHUD executable
7. Launch BlishHUD from the interface

## Nexus vs Traditional Method

| Feature | Nexus Integration | Traditional Method |
|---------|------------------|-------------------|
| Installation | Drop DLL in addons folder | Manual DLL injection |
| Configuration | GUI with file browser | Manual configuration files |
| Startup | Automatic via Nexus | Manual loader execution |
| Keybinds | Built-in shortcuts | Custom keybind configuration |
| Error Handling | Nexus-managed | Manual troubleshooting |

# Current status
A lot of the core issues have been solved and it should now work pretty well.
If you encounter any problem, create an issue on github.

# Compiling
```cargo +nightly build --release```
