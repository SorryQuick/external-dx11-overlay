# How to use BlishHUD on Linux / Mac / Steam Deck
This is a simple guide on how to use BlishHUD on Linux / Mac / Steam Deck. This method works the exact same **regardless** of distro/DE/WM.
For more technical information, read the README of this repo. This is not a "hacky" way that relies on window transparency to work. 
Instead, the BlishHUD window is hidden entirely and Blish is rendered inside the game, similarly to other addons such as arcdps. This has the side-effect of allowing you to play in full-screen if you wish.

## Steps
### Step 1
Go to the [release section](https://github.com/SorryQuick/external-dx11-overlay/releases) of this repository and download the latest release zip file (not the source code).
<img width="1371" height="578" alt="image" src="https://github.com/user-attachments/assets/69b84fd4-15ef-4112-b9ff-24a3a8c666b9" />

### Step 2
Unzip to your Guild Wars 2 installation folder. The "addons" folder found in the zip file should sit at the same level as Gw2-64.exe.
Eg. /home/user/.steam/steam/steamapps/common/Guild Wars 2/addons

### Step 3 (Using Steam)
Go to your steam library and click add non-steam game.
<img width="361" height="121" alt="image" src="https://github.com/user-attachments/assets/487a71bb-ef29-4f04-8f4b-1847f9f72ddd" />

Under Browse, select "Gw2-Simple-Addon-Loader.exe, which can be found inside the addons folder you copied earlier. The click Add Selected Program.

Example Path: /home/user/.steam/steam/steamapps/common/Guild Wars 2/addons/LOADER_public/Gw2-Simple-Addon-Loader.exe

You may need to change the compatibility tool (proton version) for that new entry. Statistics like time played will still be attributed to the real steam game. The steam overlay will also work if enabled.

### Step 3 (No steam)
This will depend on what you use (lutris, bottles, crossover, kegworks...). However, generally, you want to add a new game and point it to Gw2-Simple-Addon-loader.exe which you unzipped earlier. 
You will then be very likely to need to perform step 5. A general troubleshoot step here is to start with a brand new wine prefix and then install dotnet48 to it (step 5). If you are on Mac, using DXMT is highly recommended.

### Step 4
Try launching this new steam shortcut. You should have to enter your credentials on the gw2 launcher again as steam created a new wine prefix for the new entry. A flicker is expected when launching the game. At that point, everything should work and BlishHUD should have loaded along with the game.

### Step 5 (optional/troubleshoot)
Sometimes, BlishHUD may not load correctly due to dotnet48 not being installed in the prefix. This is even more likely if you created the prefix yourself, or are not using steam. To fix it:

If using steam, simply run protontricks. Eg ```protontricks``` in a terminal. Then select your new: "Non-Steam shortcut: Gw2-Simple-Addon-Loader.exe.
From here, "Install an application" -> "Cancel" -> "Install a Windows DLL or component". From the list, choose "dotnet48" and follow the instructions.

If you are **not** using steam, do the same thing, but instead of using protontricks, run ```WINEPREFIX=your_prefix winetricks```.

## Troubleshoot
If it does not work, read the ```log.txt``` file located in the same folder as Gw2-Simple-Addon-Loader.exe. For further help, ask on the blish hud discord.

## Screenshots
<img width="1918" height="1079" alt="image" src="https://github.com/user-attachments/assets/d5f72a1f-5e0f-406b-ad7c-e6692d1acb5f" />


