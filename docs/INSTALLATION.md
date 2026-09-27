# Windows installation

## Standard installation

Download and run `Lightrift-1.0.1-windows-x64-setup.exe` from the GitHub release.

- Installs for your Windows account into `%LOCALAPPDATA%\Programs\Lightrift` by default. Administrator rights are not required.
- Adds Lightrift to the Start menu and Windows Installed apps, with an optional desktop shortcut.
- Saves builds, preferences and caches in `%LOCALAPPDATA%\Lightrift`, separate from the program files.
- Run a newer installer to upgrade. The stable application ID keeps one uninstall entry and reuses the installation directory.
- Uninstall through Windows Settings. Uninstall removes the program and its shortcuts, while preserving your builds and preferences.

The installer and application are currently unsigned.

## Move an existing portable playbook

Close every running copy of Lightrift or Rift. Before first launching the installed app, copy the **contents** of the old `data` folder into `%LOCALAPPDATA%\Lightrift`. For example, `data\settings.json` becomes `%LOCALAPPDATA%\Lightrift\settings.json`. Back up existing destination settings before replacing them if you have already used the installed version. Settings shows the exact active data-folder path.

## Portable edition

The ZIP and standalone executable remain portable: they store data beside the executable in `data/`. The installer places `installed.flag` beside its executable to select AppData storage. Do not copy that marker into a portable folder. Avoid running installed and portable copies simultaneously because they share overlay shortcuts and the League connection.

## Build the installer

1. Build and test the Rust release on Windows. GNU builds require `windres` on PATH (or set `RC`); MSVC builds require the Windows SDK resource compiler `rc.exe` on PATH.
2. Use Inno Setup 7 (or a compatible current 6.x compiler):

```powershell
ISCC.exe packaging/windows.iss
```

The default input is `target/release/lightrift.exe`; output is `dist/`. Override paths when using a custom Cargo target directory:

```powershell
ISCC.exe /DBinaryPath="C:\build\lightrift.exe" /DReleaseDir="C:\releases" packaging/windows.iss
```

The window icon comes from `assets/lightrift.png`; Explorer, shortcuts and installer icons use the multi-size `assets/lightrift.ico`. `build.rs` embeds the icon and Windows version information into the executable. To regenerate both icon files, run `python scripts/generate-icon.py` with Pillow installed.
