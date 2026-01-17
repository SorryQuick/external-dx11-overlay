# How to use BlishHUD on Linux / Mac / Steam Deck
This is a simple guide on how to use BlishHUD on Linux / Mac / Steam Deck. This method works the exact same **regardless** of distro/DE/WM.
For more technical information, read the README of this repo. This is not a "hacky" way that relies on window transparency to work. 
Instead, the BlishHUD window is hidden entirely and Blish is rendered inside the game, similarly to other addons such as arcdps. This has the side-effect of allowing you to play in full-screen if you wish.

This can also be used on windows, albeit with less advantages. For example, it could allow frame-gen, full-screen gameplay, etc...

# Steps
## Step 1
Go to the [release section](https://github.com/SorryQuick/external-dx11-overlay/releases) of this repository and download the latest release zip file (not the source code).

Read the release notes/instructions, it may contain useful information.
<img width="1371" height="578" alt="image" src="https://github.com/user-attachments/assets/69b84fd4-15ef-4112-b9ff-24a3a8c666b9" />

## Step 2
Unzip to your Guild Wars 2 installation folder. The "addons" folder found in the zip file should sit at the same level as Gw2-64.exe.
Eg. /home/user/.steam/steam/steamapps/common/Guild Wars 2/addons. If it already exists, you can safely merge.

## Step 3 (With Steam)
Go to your steam library and click add non-steam game.
<img width="361" height="121" alt="image" src="https://github.com/user-attachments/assets/487a71bb-ef29-4f04-8f4b-1847f9f72ddd" />

Under Browse, select "Gw2-Simple-Addon-Loader.exe, which can be found inside the addons folder you copied earlier. The click Add Selected Program.

Example Path: /home/user/.steam/steam/steamapps/common/Guild Wars 2/addons/LOADER_public/Gw2-Simple-Addon-Loader.exe

You **need** to change the compatibility tool (proton version) for that new entry. The default/global one will not be applied properly for some reason. 

Recent versions of Proton-GE and similar tend to perform much better and have less issues.

Statistics like time played will still be attributed to the real steam game. The steam overlay will also work if enabled.

### Using Steam as the provider / Login with Steam user
If you are using a Steam account instead of an ArenaNet one, you must add USE_STEAM_LOGIN=1 as an envar. For example:

<img width="812" height="566" alt="image" src="https://github.com/user-attachments/assets/6959b115-6a0b-41bd-9198-64a616f3701b" />


## Step 3 (Without steam)
This will depend on what you use (lutris, bottles, crossover, kegworks...). However, generally, you want to add a new game and point it to Gw2-Simple-Addon-loader.exe which you unzipped earlier. 
You will then be very likely to need to perform step 5. A general troubleshoot step here is to start with a **brand new wine prefix** and then install dotnet48 to it (step 5). If you are on Mac, using DXMT is highly recommended.

## Step 4
Try launching this new steam shortcut. You should have to enter your credentials on the gw2 launcher again as steam created a new wine prefix for the new entry. A flicker is expected when launching the game. At that point, everything should work and BlishHUD should have loaded along with the game. Note that if/when the game updates, it's possible blish will not load properly. Simply restart the launcher after such an update.

## Step 5 (optional/troubleshoot)
### **Only do this step if needed.** It's annoying and can be complicated. It is usually not needed if you're using steam and/or modern versions of proton-ge.

Sometimes, BlishHUD may not load correctly due to dotnet48 not being installed in the prefix or it conflicting with mono. This is even more likely if you created the prefix yourself, or are not using steam. 
To fix it (wine will pop a bunch of warnings, this is normal):

### Remove mono

First, you may need to remove ```mono``` from your prefix.

If using steam, simply run protontricks. Eg ```protontricks``` in a terminal. Then select your new: "Non-Steam shortcut: Gw2-Simple-Addon-Loader.exe.
From here, "Chose the default prefix" -> "Ok" -> "Run uninstaller". Remove mono. 

If you are **not** using steam, do the same thing, but instead of using protontricks, run ```WINEPREFIX=your_prefix winetricks```.

### Add dotnet48

If using steam, simply run protontricks. Eg ```protontricks``` in a terminal. Then select your new: "Non-Steam shortcut: Gw2-Simple-Addon-Loader.exe.
From here, "Install an application" -> "Cancel" -> "Install a Windows DLL or component". From the list, choose "dotnet48" and follow the instructions.

If you are **not** using steam, do the same thing, but instead of using protontricks, run ```WINEPREFIX=your_prefix winetricks```.


# Troubleshoot
See this [troubleshooting guide](https://github.com/SorryQuick/external-dx11-overlay/blob/master/Troubleshooting-Guide.md).

For further help, ask on the blish hud discord or create an issue on github.

# Screenshots
<img width="1918" height="1079" alt="image" src="https://github.com/user-attachments/assets/d5f72a1f-5e0f-406b-ad7c-e6692d1acb5f" />


