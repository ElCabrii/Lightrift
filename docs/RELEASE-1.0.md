# Lightrift 1.0

First public release of Lightrift, a lightweight native Windows companion for League of Legends.

- Champion recommendations with alternative core-item paths, runes, summoners and skill order.
- Local playbook with multiple builds per champion, editing, duplication and deletion.
- Temporary rune application, separate persistent rune-page and item-set saves.
- Live draft picks/bans, practice draft with bans, and in-game scores, items, rune information and item gold.
- Separate item, skill-order and rune/summoner overlays with phase-aware visibility and independent toggles.
- Native Rust interface, embedded game assets and no Riot developer key or backend requirement.

## Windows download

Download `Lightrift-1.0.0-windows-x64.zip`, extract it to a writable folder and run `Lightrift.exe`. The standalone executable is also attached. Windows x64 only; the executable is unsigned. SHA-256 checksums are included.

For overlays, use windowed or borderless League. The bundled catalog is Data Dragon 16.19.1. Recommendations require internet access; the catalog and saved builds work offline.

## Existing Rift users

Close Rift first, then copy its `data` folder beside `Lightrift.exe`. Existing builds and preferences are compatible. Lightrift recognizes old Rift rune-page names and retains item-set IDs.

28 automated tests pass. Release compilation and Clippy pass. Local League integration relies on unsupported client endpoints that can change with patches. Lightrift is not endorsed by Riot Games.
