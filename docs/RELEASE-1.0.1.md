# Lightrift 1.0.1

- Replaces the default egui icon with Lightrift's mint logo in the window and taskbar.
- Embeds a multi-resolution Windows icon and product/version metadata in the executable for Explorer, shortcuts and Installed apps.
- Adds a standard per-user Windows installer with a Start menu shortcut, optional desktop shortcut and uninstaller. No administrator rights are required.
- Installed builds keep preferences in `%LOCALAPPDATA%\Lightrift`, separate from program files; upgrades and uninstall preserve this data. Portable builds keep their existing adjacent data folder.
- Settings now displays the active data-folder path.

## Download

For a standard installation, use **Lightrift-1.0.1-windows-x64-setup.exe**. The ZIP and standalone executable are still available for portable use. All downloads are Windows x64 and unsigned. SHA-256 checksums are attached.

To migrate a portable playbook, close the app and copy the old data folder's contents into `%LOCALAPPDATA%\Lightrift` before first launch. See [installation instructions](https://github.com/ElCabrii/Lightrift/blob/main/docs/INSTALLATION.md).
