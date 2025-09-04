# How to use BlishHUD on Linux / Mac / Steam Deck

This is a simple guide on how to use BlishHUD on Linux / Mac / Steam Deck. This method works the exact same **regardless** of distro/DE/WM.
For more technical information, read the README of this repo. This is not a "hacky" way that relies on window transparency to work.
Instead, the BlishHUD window is hidden entirely and Blish is rendered inside the game, similarly to other addons such as arcdps. This has the side-effect of allowing you to play in full-screen if you wish.

This can also be used on windows, albeit with less advantages. For example, it could allow frame-gen, full-screen gameplay, etc...

## Installation methods available

- [Regular installation.](#regular-installation-steps)
- [Installation with Nexus Addon Loader &amp; Manager](#nexus-integration-alternative-method)

# Regular Installation Steps

## Step 1

Go to the [release section](https://github.com/SorryQuick/external-dx11-overlay/releases) of this repository and download the latest release zip file (not the source code).
<img width="1371" height="578" alt="image" src="https://github.com/user-attachments/assets/8aa44e13-3e2f-4697-ae90-95a113f632a6" />


## Step 2

Unzip to your Guild Wars 2 installation folder. The "addons" folder found in the zip file should sit at the same level as Gw2-64.exe.
Eg. /home/user/.steam/steam/steamapps/common/Guild Wars 2/addons

## Step 3 (Using Steam)

Go to your steam library and click add non-steam game.
<img width="361" height="121" alt="image" src="https://github.com/user-attachments/assets/487a71bb-ef29-4f04-8f4b-1847f9f72ddd" />

Under Browse, select "Gw2-Simple-Addon-Loader.exe, which can be found inside the addons folder you copied earlier. The click Add Selected Program.

Example Path: /home/user/.steam/steam/steamapps/common/Guild Wars 2/addons/LOADER_public/Gw2-Simple-Addon-Loader.exe

You **need** to change the compatibility tool (proton version) for that new entry. The default/global one will not be applied properly for some reason.

Statistics like time played will still be attributed to the real steam game. The steam overlay will also work if enabled.

### Using Steam as the provider / Login with Steam user

If you are using a Steam account instead of an ArenaNet one, you must add USE_STEAM_LOGIN=1 as an envar. For example:

<img width="812" height="566" alt="image" src="https://github.com/user-attachments/assets/6959b115-6a0b-41bd-9198-64a616f3701b" />

## Step 3 (No steam)

This will depend on what you use (lutris, bottles, crossover, kegworks...). However, generally, you want to add a new game and point it to Gw2-Simple-Addon-loader.exe which you unzipped earlier.
You will then be very likely to need to perform step 5. A general troubleshoot step here is to start with a **brand new wine prefix** and then install dotnet48 to it (step 5). If you are on Mac, using DXMT is highly recommended.

## Step 4

Try launching this new steam shortcut. You should have to enter your credentials on the gw2 launcher again as steam created a new wine prefix for the new entry. A flicker is expected when launching the game. At that point, everything should work and BlishHUD should have loaded along with the game.

## Step 5 (optional/troubleshoot)

Sometimes, BlishHUD may not load correctly due to dotnet48 not being installed in the prefix or it conflicting with mono. This is even more likely if you created the prefix yourself, or are not using steam.
To fix it (wine will pop a bunch of warnings, this is normal):

### Remove mono

First, you may need to remove ``mono`` from your prefix.

If using steam, simply run protontricks. Eg ``protontricks`` in a terminal. Then select your new: "Non-Steam shortcut: Gw2-Simple-Addon-Loader.exe.
From here, "Chose the default prefix" -> "Ok" -> "Run uninstaller". Remove mono.

If you are **not** using steam, do the same thing, but instead of using protontricks, run ``WINEPREFIX=your_prefix winetricks``.

### Add dotnet48

If using steam, simply run protontricks. Eg ``protontricks`` in a terminal. Then select your new: "Non-Steam shortcut: Gw2-Simple-Addon-Loader.exe.
From here, "Install an application" -> "Cancel" -> "Install a Windows DLL or component". From the list, choose "dotnet48" and follow the instructions.

If you are **not** using steam, do the same thing, but instead of using protontricks, run ``WINEPREFIX=your_prefix winetricks``.

# Nexus Integration (Alternative Method)

If you prefer a simpler installation process, this project also supports integration with the [Nexus](https://github.com/zerthox/nexus) Addon Loader & Manager. This method provides a user-friendly interface and automatic management of new updates to the addon as well as a growing library of multiple addons.

## Nexus Setup Steps

### Step 1: Install Nexus

First, download and install Nexus Addon Loader & Manager from the [official website](https://raidcore.gg/Nexus). Follow their installation instructions for your platform.

### Step 2: Download the Nexus DLL

Go to the [release section](https://github.com/SorryQuick/external-dx11-overlay/releases) of this repository and download the latest Nexus-enabled DLL file from the releases.

### Step 3: Download the right Blish HUD version

Go to the [release section](https://github.com/SorryQuick/external-dx11-overlay/releases) of this repository and download the external-dx11-overlay.zip file. There is an already compiled version of Blish HUD that is compatible with both the Nexus version and the regular version.

### Step 3: Install the Addon

1. Copy the downloaded DLL to your Gw2 addons directory
2. Launch Guild Wars 2 with Nexus enabled

### Step 4: Configure BlishHUD

1. Once in-game, enable the "Blish HUD overlay loader" addon in Nexus. It should appear in the "installed" tab.
2. A new icon will appear next to the Nexus icon. Clicking it will open the menu to manage Blish.
3. Click "Browse for Executable..." and select your compatible BlishHUD executable file. You can place the blish executable and folder anywhere, but it's strongly recommended to put it in the Gw2 addons folder for easy access.
4. Check "Launch on Startup" if you want BlishHUD to start automatically next time you start the game.
5. Click "Launch" to start BlishHUD

## Advantages of Nexus Integration

- **No Manual DLL Injection**: Nexus handles loading automatically
- **Graphical Interface**: Point-and-click configuration instead of text files
- **Automatic Updates**: Automatic updates when new versions are available
- **Integrated Management**: All your GW2 addons in one place

## Nexus vs Traditional Method

| Aspect                    | Nexus Method                                 | Traditional Method                 |
| ------------------------- | -------------------------------------------- | ---------------------------------- |
| **Installation**    | Drop DLL in Nexus addons folder              | Extract ZIP and configure manually |
| **Configuration**   | GUI with file browser                        | Edit text files manually           |
| **Startup**         | Automatic via Nexus. Just start Gw2 normally | Run separate loader executable     |
| **User Experience** | More user-friendly                           | More technical setup required      |

## Switching from Traditional to Nexus

If you're already using the traditional method and want to switch to Nexus:

1. Download the Nexus-enabled DLL from the [releases section](https://github.com/SorryQuick/external-dx11-overlay/releases)
2. Install Nexus if you haven't already
3. Copy the downloaded DLL to your Gw2 addons directory
4. Disable/remove the traditional loader setup
5. Enable the addon in Nexus and configure as described

Your existing BlishHUD installation and configuration will continue to work - only the loading method changes.

# Troubleshoot

See this [troubleshooting guide](https://github.com/SorryQuick/external-dx11-overlay/blob/master/Troubleshooting-Guide.md).

For further help, ask on the blish hud discord or create an issue on github.

# Screenshots

<img width="1918" height="1079" alt="image" src="https://github.com/user-attachments/assets/d5f72a1f-5e0f-406b-ad7c-e6692d1acb5f" />
