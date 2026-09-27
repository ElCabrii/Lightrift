# Architecture

Lightrift is a native Rust application built with eframe/egui and OpenGL. It has no browser runtime or hosted backend. Catalogs and textures are embedded; preferences, builds and recommendation caches stay beside the executable in data/.

- ui.rs owns navigation, editor state and playbook persistence. ui/design.rs provides shared native controls. ui/live.rs renders draft and match views, and ui/overlay.rs owns three independent viewports.
- data.rs validates builds and creates League import payloads. Stable build IDs distinguish same-name variants. Deletion persists a cloned settings snapshot before changing editor state and clears active references.
- client.rs serializes local League requests on a worker thread. Credentials are discovered from the local lockfile, retained in memory and sent only to the loopback client. HTTPS verifies Riot's root certificate, with redirects and proxies disabled.
- live.rs retains only scoreboard fields. Match reads use the local Live Client Data API. Item gold sums held items' catalog total prices and stack counts; missing prices produce an unavailable value. Full runes are assigned only to the exact active player.
- recommendations.rs fetches OP.GG's public champion-analysis service and caches normalized results for six hours. recommendations/core_options.rs parses published alternative item paths with bounded responses and traversal, verifying champion, role, patch and cohort. It never executes downloaded JavaScript.
- hotkeys.rs uses Windows RegisterHotKey for overlay toggles. There are no keyboard hooks, process injection, memory reads or automated picks.

Apply runes creates or reuses an editable temporary page owned by Lightrift, selects it, and verifies the returned ID, temporary/current flags and rune choices. Existing Rift: pages are recognized during upgrades. Save rune page is a separate persistent action. Item-set IDs retain the legacy rift-<champion> format to avoid duplicates.

Runes/summoners overlays appear only during ChampSelect; item and skill panels appear only InProgress. Failed or stale snapshots hide every floating panel. Polling and network requests run off the UI thread; egui is repainted when work completes. Borderless/windowed League is supported; exclusive fullscreen is not.

## Updating bundled game data

Run node scripts/fetch-assets.mjs and node scripts/fetch-runes.mjs, then rebuild and test. These scripts use official Riot Data Dragon manifests and images. Refresh all assets together; do not mix patches. The item planner excludes alternate-mode IDs at or above 100000, a rule to review when updating the catalog.

The local League interface and provider response formats can change. Runtime catalog updates, remote account history, cloud ingestion and automated game actions are not implemented.
