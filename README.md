# Lightrift — native League companion

A lightweight Windows companion for League of Legends, built in Rust with a native interface. Version 1.0.1 includes champion builds, runes, draft planning, live match information and three independently controlled overlays.

[Download Lightrift for Windows](https://github.com/ElCabrii/Lightrift/releases/latest)

## Run

Use the **Windows setup EXE** for a standard installation with Start menu shortcut and uninstall support. Alternatively, extract the portable ZIP and open `Lightrift.exe`. No account, Riot developer key, Node.js, browser runtime, or backend is required. See [installation and upgrade instructions](docs/INSTALLATION.md). The official champion/item/rune/spell catalog and sprite images are embedded in the executable. The release currently bundles Data Dragon **16.19.1**.

1. Search a champion (Ctrl+K) or filter by champion class. Stars mark favorites.
   In **Recommended**, choose a lane and **Find builds**. Click a core-item row to compare published three-item paths, with each option's win rate, pick rate and sample size. **Save selected build** adds a separate editable local variant and opens the editor; it does not modify League. Runes, spells, skills, starting items and boots remain the shared service recommendation. Boots are copied into Situational for you to place in your purchase plan.
2. In **Build planner**, click **Add items** or an empty slot in a block (Starter, Core, Situational). Search the item library and click `+`. Click an item in a block to remove it. Items are ordered as added. The header stays visible as you scroll; wide windows place skills and notes beside the item plan.
3. In **Runes & spells**, select primary runes, two secondary runes from different rows, three shards, and D/F spells.
4. Save the build. **New build**, **Duplicate**, and the **Saved builds** selector manage multiple variants per champion and role. Renaming updates the same variant. Switching builds/champions saves valid pending edits automatically; invalid edits keep the current build selected.
5. Open League and use **Connect to League**. If discovery fails, set the installation folder or exact `lockfile` path in **Settings**.
6. **Import into League** stays at the top of the editor. **Apply runes** creates or reuses Lightrift’s temporary League rune page and selects it, using the same temporary-page mechanism as the client’s recommender. Saved rune pages and Lightrift builds are preserved. **Save rune page** separately creates/updates the named `Lightrift:` page and selects it. **Save item set** updates the champion’s Lightrift item set. **Apply spells** is available during champion select for the same champion. Each action reads back the result. Recommended also offers **Apply runes** directly without saving the recommendation.
7. **Live** shows both teams’ picks and bans during champion select, then their players, scores, items and rune information during the game. Updates run while Live is open; **Follow in background** keeps them running across screens. Following your champion selects a saved build for its role without applying it or leaving Live. **Practice draft** offers five picks and five bans per team, excluding duplicates. **Item gold** sums held item/component prices and stack counts; completed recipes are counted once, with no pocket gold included. Full rune choices are exposed only for your player; others show keystones and rune trees.

## Automatic overlay panels

Choose **Overlays** in the navigation rail to open the overlay studio. All three are enabled by default, and automatic overlays connect to League without requiring Follow in background. Runes and summoners share a small draft-only panel (`ChampSelect`). Items and skill order have separate game-only panels (`InProgress`). Other phases, failed connections and phase information older than ten seconds hide every panel. Phase checks run every two seconds while enabled; transitions are detected on the next successful check, not instantaneously.

- **Ctrl+Shift+O:** pause/resume all automatic overlays. Resuming still respects League's phase and each panel's toggle.
- **Ctrl+Shift+L:** lock/unlock all panels. They start locked when shortcuts are available, so mouse clicks pass through to League. Unlock to drag each header, collapse each panel, or turn it Off.
- **Overlay studio:** independently enable Items, Skill order, and Runes & summoners; adjust opacity or reset positions. Off disables only that panel until re-enabled here. Inline previews show the selected build and never open floating panels outside their phase.

Use borderless/windowed League. These are ordinary transparent, always-on-top native windows; exclusive fullscreen is unsupported. They display the selected build, not live purchases or skill levels. The existing draft auto-open option also works while automatic overlays are enabled; disable it in Live if you prefer manual build selection. If you start Lightrift mid-game, select your build manually. Positions and each panel's collapsed state last for this app session, including hide/show and phase changes. Panel toggles, master enable and opacity are saved. If global shortcuts cannot register, click-through is disabled and the Overlays workspace explains why. Keep only one Lightrift instance open.

In Build planner, Q/W/E/R buttons append skill choices and Undo removes the last. Blank builds have no recommended sequence; provider sequences can end before level 18. No levels are invented.

## Implemented and limitations

- Searchable catalog, class filters, favorites, official portraits, champion stats and ability descriptions.
- Three item blocks, prices, item filtering, matchup notes, legal rune selection, summoner spell selection, local saves, JSON copy/export.
- Local League discovery, phase/summoner reads, draft champion reads, rune/item-set/spell import code with readback.
- Manual draft planning and three borderless, click-through overlay panels.
- Champion ability details fetch from official Data Dragon on demand, then stay cached by patch. Network work runs off the UI thread.

Recommendations target Ranked Summoner's Rift, all ranks. Shared items/runes/spells/skills come from OP.GG's documented public service, whose response does not identify a region. Core-item alternatives come from the public champion page, explicitly filtered to Global and All ranks and checked against the champion, lane and patch. The number of alternatives depends on published data; the tested champions currently expose five. If the page is unavailable or its format changes, the service recommendation remains usable with a notice. These alternatives change core items only, not an entire independently matched rune/spell setup. Item/rune win rates use separate samples and are not a combined build win rate. Each response displays its patch and retrieval age. It is cached for six hours; network failures can show older cached results with a notice. Conversion to an editable build validates every identifier and requires the recommendation patch to match the bundled catalog's major/minor patch.

Tier lists, matchup scores, remote profiles/match histories, automated picks, recording, injected overlays and jungle timers are not implemented. New blank builds use editable default runes, **not recommendations**. Live League integration uses unsupported endpoints; see `docs/VALIDATION.md` for what was actually tested.

Delete a saved variant from its **Playbook** card, then confirm **Delete build**. Deletion removes only that local variant and its active selection reference. If it is open in the editor, Lightrift switches to another saved variant or a new blank build, preventing autosave from restoring the deleted entry. League rune pages and item sets are unaffected.

The portable edition stores settings and caches in `data/` beside the executable. The installed edition uses `%LOCALAPPDATA%\Lightrift`. Settings displays the active path. Keep that folder writable. No credentials are persisted. Older saves migrate automatically; the exact skill-sequence format from older OP.GG notes is extracted without changing those notes. The first 0.4 save backs up existing settings as `data/settings-before-0.4.json`. Previous backups remain untouched. Close old Lightrift versions before running the upgrade to avoid competing saves. A malformed settings file is preserved and saving is disabled until repaired. Exit autosave is best effort; use Save before closing.

## Upgrading from Rift

Close Rift before launching Lightrift. For portable use, copy your existing `data` folder beside `Lightrift.exe`. For the installed edition, copy its contents to `%LOCALAPPDATA%\Lightrift` before first launch. This keeps your builds, favorites and preferences. Lightrift recognizes existing Rift temporary/named rune pages; item-set IDs stay compatible. Your data folder is never included in public releases.

## Development

Install Rust and its Windows platform prerequisites, then run (the toolchain file pins Rust 1.94.0):

```powershell
cargo build --release --locked
cargo test --release --locked
cargo clippy --release --locked
```

The project uses `eframe`/`egui` with OpenGL and no embedded webview. Textures come from 16 embedded sprite atlases and individual rune icons. Lists virtualize visible rows. Automatic overlays poll the local client every two seconds. Live polls every two seconds during champion select and games, and five seconds otherwise. In-game reads use the local Live Client Data API only when Live is open or background following is enabled. Separate local-client and recommendation workers keep provider latency away from draft updates. No LLM, MCP SDK or JavaScript runtime is used in the app; the public service is called directly over HTTPS.

The delivered executable was built with the Windows GNU toolchain and w64devkit 2.10. A normal MSVC Rust installation can use the Cargo commands above from a Visual Studio developer shell with the Windows SDK resource compiler available. GNU builds also require windres on PATH. No compiler is required to run the executable.

`Cargo.lock` pins dependencies. The data refresh process and future service design are described in `docs/ARCHITECTURE.md`.

See `docs/VALIDATION.md` for measured resource use and the completed and outstanding checks. Public releases contain no personal builds or settings.

## Attribution

Data and game images: [Riot Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon). Local interface reference: [League Client API](https://developer.riotgames.com/docs/lol#league-client-api). Recommendations: [OP.GG's documented public service](https://github.com/opgginc/opgg-mcp). Design references: [OP.GG Desktop](https://op.gg/desktop/en) and [DPM.LOL](https://dpm.lol/). Provider availability and schemas can change. The public endpoint working today is not a service-level or redistribution guarantee. No private OP.GG/DPM APIs are used.

Lightrift is not endorsed by Riot Games and does not reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
