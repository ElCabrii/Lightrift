# Version 1.0 validation

- Rust release build, formatting and Clippy with warnings denied passed.
- 28 automated tests passed. Two opt-in tests requiring a running League client or public provider access are skipped by default.
- Tests cover valid bundled builds, rune legality, provider parsing and bounded fallbacks, draft bans, item inventory value, active-player rune assignment, overlay phase rules, saved-build migration/deletion and temporary-page ownership/readback.
- The preceding 0.8 build was exercised against a real League lobby: applying runes created and selected a temporary page, repeated applies reused it, and saved page contents were preserved. Both Build planner and Recommended actions were checked. Version 1.0 adds the Lightrift name and compatibility with earlier Rift page names.
- Earlier desktop checks covered practice draft, bans, recommendations, overlay settings and a local custom-game scoreboard. Full rune choices for other players are unavailable through the supported live data source.
- Permanent rune-page saving and item-set saving have readback verification in code but were not exercised during the 1.0 publication pass. Multi-monitor/DPI behavior and exclusive-fullscreen overlays are not certified.

The portable release includes no personal settings, developer credentials or local match data. No Riot developer API key is required. The executable is unsigned.
