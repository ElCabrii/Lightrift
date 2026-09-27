use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Panel {
    Items,
    Skills,
    Runes,
}
const PANELS: [Panel; 3] = [Panel::Items, Panel::Skills, Panel::Runes];
const PHASE_MAX_AGE: Duration = Duration::from_secs(10);
pub(super) struct PanelState {
    position: Option<egui::Pos2>,
    drag_offset: Option<egui::Vec2>,
    collapsed: bool,
}
impl PanelState {
    pub fn new(collapsed: bool) -> Self {
        Self {
            position: None,
            drag_offset: None,
            collapsed,
        }
    }
}
impl Panel {
    fn title(self) -> &'static str {
        match self {
            Self::Items => "Items",
            Self::Skills => "Skill order",
            Self::Runes => "Runes & summoners",
        }
    }
    fn viewport(self) -> egui::ViewportId {
        egui::ViewportId::from_hash_of(("rift-panel", self as usize))
    }
    fn enabled(self, settings: &OverlaySettings) -> bool {
        match self {
            Self::Items => settings.items,
            Self::Skills => settings.skills,
            Self::Runes => settings.runes,
        }
    }
    fn enabled_mut(self, settings: &mut OverlaySettings) -> &mut bool {
        match self {
            Self::Items => &mut settings.items,
            Self::Skills => &mut settings.skills,
            Self::Runes => &mut settings.runes,
        }
    }
    fn visible(
        self,
        settings: &OverlaySettings,
        phase: Option<&str>,
        age: Option<Duration>,
    ) -> bool {
        settings.enabled
            && self.enabled(settings)
            && age.is_some_and(|age| age < PHASE_MAX_AGE)
            && match self {
                Self::Runes => phase == Some("ChampSelect"),
                Self::Items | Self::Skills => phase == Some("InProgress"),
            }
    }
    fn default_position(self) -> egui::Pos2 {
        match self {
            Self::Items | Self::Runes => egui::pos2(40.0, 80.0),
            Self::Skills => egui::pos2(40.0, 410.0),
        }
    }
}
impl Lightrift {
    pub(super) fn toggle_overlays(&mut self) {
        self.settings.overlay.enabled = !self.settings.overlay.enabled;
        if self.settings.overlay.enabled {
            self.snapshot_at = None;
            self.refresh();
        }
        self.persist();
    }
    pub(super) fn overlay_settings(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Keep the essentials in sight.")
                .size(30.0)
                .strong(),
        );
        ui.label(
            RichText::new("Independent panels. The right information, at the right moment.")
                .color(MUTED),
        );
        ui.add_space(20.0);
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(design::primary(if self.settings.overlay.enabled {
                        "Pause overlays"
                    } else {
                        "Enable overlays"
                    }))
                    .clicked()
                {
                    self.toggle_overlays();
                }
                if ui
                    .add_enabled(
                        self.hotkeys.available,
                        egui::Button::new(if self.overlay_locked {
                            "Unlock to reposition"
                        } else {
                            "Lock all panels"
                        }),
                    )
                    .clicked()
                {
                    self.overlay_locked = !self.overlay_locked;
                }
                if ui.button("Reset positions").clicked() {
                    for panel in PANELS {
                        let pos = panel.default_position();
                        self.overlay_panels[panel as usize].position = Some(pos);
                        ui.ctx().send_viewport_cmd_to(
                            panel.viewport(),
                            egui::ViewportCommand::OuterPosition(pos),
                        );
                    }
                }
                let changed = ui
                    .add(
                        egui::Slider::new(&mut self.settings.overlay.opacity, 45..=100)
                            .text("Opacity")
                            .suffix("%"),
                    )
                    .changed();
                if changed {
                    self.persist();
                }
            });
            ui.add_space(4.0);
            let phase = self.snapshot.as_ref().map(|s| s.phase.as_str());
            let age = self.snapshot_at.map(|at| at.elapsed());
            let shown = PANELS
                .into_iter()
                .filter(|p| p.visible(&self.settings.overlay, phase, age))
                .map(Panel::title)
                .collect::<Vec<_>>();
            let status = if !self.settings.overlay.enabled {
                "Paused · Your panel preferences are saved".into()
            } else if age.is_none_or(|a| a >= PHASE_MAX_AGE) {
                "Waiting for League · Panels hidden until connected".into()
            } else if shown.is_empty() {
                "Ready · Panels will appear automatically during draft or game".into()
            } else {
                format!("Live · {}", shown.join(" + "))
            };
            ui.label(RichText::new(status).size(12.0).color(MINT));
        });
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            design::section(ui, "Your panels", "");
            ui.label(
                RichText::new(format!(
                    "Previewing {} · {}",
                    self.selected, self.build.name
                ))
                .size(12.0)
                .color(MUTED),
            );
        });
        ui.add_space(6.0);
        if ui.available_width() >= 970.0 {
            ui.columns(3, |cols| {
                for (index, panel) in PANELS.iter().enumerate() {
                    self.overlay_preview(&mut cols[index], *panel);
                }
            });
        } else {
            for panel in PANELS {
                self.overlay_preview(ui, panel);
                ui.add_space(10.0);
            }
        }
        ui.add_space(24.0);
        card().show(ui,|ui|{
            ui.set_width(ui.available_width());
            ui.columns(2,|cols|{
                design::section(&mut cols[0],"One shortcut away","");
                cols[0].horizontal(|ui|{pill(ui,"Ctrl + Shift + O",TEXT);ui.label("Pause or resume overlays");});
                cols[0].horizontal(|ui|{pill(ui,"Ctrl + Shift + L",TEXT);ui.label("Lock or unlock all panels");});
                design::section(&mut cols[1],"Out of your way","");
                cols[1].label(RichText::new("Locked panels let clicks pass through. Unlock to drag the header, collapse a panel, or turn it off.").size(12.0).color(MUTED));
                cols[1].label(RichText::new("Use borderless or windowed League. Panels hide after a game and on connection loss.").size(12.0).color(MUTED));
            });
            if !self.hotkeys.available {ui.label(RichText::new("Shortcuts unavailable. Close other Lightrift instances or resolve shortcut conflicts, then restart. Click-through is disabled.").color(GOLD).size(12.0));}
        });
    }
    fn overlay_preview(&mut self, ui: &mut egui::Ui, panel: Panel) {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(panel.title()).size(16.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .checkbox(panel.enabled_mut(&mut self.settings.overlay), "Enabled")
                        .changed()
                    {
                        self.persist();
                    }
                });
            });
            pill(
                ui,
                if panel == Panel::Runes {
                    "DURING DRAFT"
                } else {
                    "IN GAME"
                },
                if panel == Panel::Runes { ACCENT } else { MINT },
            );
            ui.add_space(12.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(10, 16, 22))
                .stroke(Stroke::new(1.0, LINE))
                .corner_radius(10)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.set_width(270.0);
                    ui.set_min_height(210.0);
                    ui.horizontal(|ui| {
                        label(ui, "LIGHTRIFT");
                        ui.label(RichText::new(panel.title()).size(11.0).color(MINT));
                    });
                    ui.add_space(6.0);
                    self.panel_contents(ui, panel);
                });
            ui.add_space(8.0);
            ui.label(
                RichText::new(match panel {
                    Panel::Items => {
                        "Your purchase plan at a glance. Starter, core and situational items."
                    }
                    Panel::Skills => {
                        "Numbered levels. Clear ability colors. Only your saved sequence."
                    }
                    Panel::Runes => {
                        "Your keystone, supporting runes and D / F spells before the game."
                    }
                })
                .size(12.0)
                .color(MUTED),
            );
        });
    }

    pub(super) fn show_overlay(&mut self, ctx: &egui::Context) {
        let phase = self.snapshot.as_ref().map(|s| s.phase.as_str());
        let age = self.snapshot_at.map(|at| at.elapsed());
        let visibility = PANELS.map(|p| p.visible(&self.settings.overlay, phase, age));
        for (panel, visible) in PANELS.into_iter().zip(visibility) {
            if visible {
                self.show_panel(ctx, panel);
            } else {
                self.overlay_panels[panel as usize].drag_offset = None;
            }
        }
    }
    fn show_panel(&mut self, ctx: &egui::Context, panel: Panel) {
        let index = panel as usize;
        let width = 300.0;
        let position = self.overlay_panels[index]
            .position
            .unwrap_or(panel.default_position());
        let locked = self.overlay_locked && self.hotkeys.available;
        ctx.show_viewport_immediate(
            panel.viewport(),
            egui::ViewportBuilder::default()
                .with_title(format!("Lightrift · {}", panel.title()))
                .with_inner_size([width, 150.0])
                .with_position(position)
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(true)
                .with_taskbar(false)
                .with_active(false)
                .with_always_on_top()
                .with_mouse_passthrough(locked),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    *panel.enabled_mut(&mut self.settings.overlay) = false;
                    self.persist();
                }
                if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
                    self.overlay_panels[index].position = Some(rect.min);
                }
                let opacity = self.settings.overlay.opacity.clamp(45, 100);
                let mut height = 0.0;
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        ui.spacing_mut().button_padding = egui::vec2(7.0, 3.0);
                        let response = egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(
                                10,
                                16,
                                22,
                                (u16::from(opacity) * 255 / 100) as u8,
                            ))
                            .stroke(Stroke::new(1.0, if locked { LINE } else { ACCENT }))
                            .corner_radius(12)
                            .inner_margin(12)
                            .show(ui, |ui| {
                                ui.set_width(width - 26.0);
                                let (header, _) = ui.allocate_exact_size(
                                    egui::vec2(width - 26.0, 18.0),
                                    egui::Sense::drag(),
                                );
                                ui.painter().text(
                                    header.left_center(),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{}  /  LIGHTRIFT", panel.title()),
                                    egui::FontId::proportional(12.0),
                                    MINT,
                                );
                                let state = &mut self.overlay_panels[index];
                                if locked {
                                    state.drag_offset = None;
                                } else {
                                    let mut target = None;
                                    for event in ctx.input(|i| i.events.clone()) {
                                        match event {
                                            egui::Event::PointerButton {
                                                pos,
                                                button: egui::PointerButton::Primary,
                                                pressed: true,
                                                ..
                                            } if header.contains(pos) => {
                                                state.drag_offset = Some(pos.to_vec2())
                                            }
                                            egui::Event::PointerMoved(pos) => {
                                                if let Some(offset) = state.drag_offset {
                                                    target = Some(
                                                        state.position.unwrap_or(position)
                                                            + pos.to_vec2()
                                                            - offset,
                                                    );
                                                }
                                            }
                                            egui::Event::PointerButton {
                                                button: egui::PointerButton::Primary,
                                                pressed: false,
                                                ..
                                            }
                                            | egui::Event::WindowFocused(false) => {
                                                state.drag_offset = None
                                            }
                                            _ => {}
                                        }
                                    }
                                    if let Some(pos) = target {
                                        state.position = Some(pos);
                                        ctx.send_viewport_cmd(
                                            egui::ViewportCommand::OuterPosition(pos),
                                        );
                                    }
                                }
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{} · {}",
                                            self.selected, self.build.name
                                        ))
                                        .size(11.0)
                                        .color(MUTED),
                                    )
                                    .truncate(),
                                );
                                if !locked {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(
                                                self.hotkeys.available,
                                                egui::Button::new("Lock all"),
                                            )
                                            .clicked()
                                        {
                                            self.overlay_locked = true;
                                        }
                                        if ui
                                            .button(if self.overlay_panels[index].collapsed {
                                                "Expand"
                                            } else {
                                                "Collapse"
                                            })
                                            .clicked()
                                        {
                                            self.overlay_panels[index].collapsed =
                                                !self.overlay_panels[index].collapsed;
                                        }
                                        if ui.button("Off").clicked() {
                                            *panel.enabled_mut(&mut self.settings.overlay) = false;
                                            self.persist();
                                        }
                                    });
                                }
                                if !self.overlay_panels[index].collapsed {
                                    ui.separator();
                                    self.panel_contents(ui, panel);
                                }
                            });
                        height = response.response.rect.height().ceil();
                    });
                if ctx.input(|i| {
                    i.viewport()
                        .inner_rect
                        .is_some_and(|r| (r.height() - height).abs() > 1.0)
                }) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        width, height,
                    )));
                }
            },
        );
    }
    fn panel_contents(&self, ui: &mut egui::Ui, panel: Panel) {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 7.0);
        match panel {
            Panel::Items => {
                for (title, ids, size) in [
                    ("START", &self.build.starter, 28.0),
                    ("CORE PATH", &self.build.core, 36.0),
                    ("ADAPT / BOOTS", &self.build.situational, 28.0),
                ] {
                    label(ui, title);
                    ui.horizontal_wrapped(|ui| {
                        if ids.is_empty() {
                            ui.label(RichText::new("No items selected").size(11.0).color(MUTED));
                        }
                        for (index, id) in ids.iter().take(6).enumerate() {
                            if let Some(item) = self.catalog.items.get(id) {
                                ui.vertical(|ui| {
                                    self.icon(ui, &item.image, size).on_hover_text(format!(
                                        "{}. {} · {} gold",
                                        index + 1,
                                        item.name,
                                        item.gold["total"]
                                    ));
                                    if title == "CORE PATH" {
                                        ui.label(
                                            RichText::new(format!("{:02}", index + 1))
                                                .size(9.0)
                                                .color(MINT),
                                        );
                                    }
                                });
                            }
                        }
                    });
                }
            }
            Panel::Skills => {
                let skills = self.build.overlay_skills();
                if skills.is_empty() {
                    ui.label(
                        RichText::new(
                            "Save a recommended build or add your skill order in the editor.",
                        )
                        .size(11.0)
                        .color(MUTED),
                    );
                } else {
                    design::skills(ui, &skills, true);
                }
            }
            Panel::Runes => {
                label(ui, "PRIMARY");
                ui.horizontal(|ui| {
                    for (i, id) in self.build.primary_runes.iter().enumerate() {
                        if let Some(tex) = self.textures.get(&format!("rune{id}")) {
                            let size = if i == 0 { 38.0 } else { 26.0 };
                            let response =
                                ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(size)));
                            if let Some(r) = self
                                .catalog
                                .runes
                                .iter()
                                .flat_map(|t| &t.slots)
                                .flat_map(|s| &s.runes)
                                .find(|r| r.id == *id)
                            {
                                response.on_hover_text(&r.name);
                            }
                        }
                    }
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    for id in &self.build.secondary_runes {
                        if let Some(tex) = self.textures.get(&format!("rune{id}")) {
                            ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(26.0)));
                        }
                    }
                    ui.add_space(10.0);
                    for (i, id) in self.build.spells.iter().enumerate() {
                        if let Some(s) = self
                            .catalog
                            .spells
                            .iter()
                            .find(|s| s.key.parse::<u32>().ok() == Some(*id))
                        {
                            ui.vertical(|ui| {
                                self.icon(ui, &s.image, 28.0).on_hover_text(&s.name);
                                ui.label(RichText::new(["D", "F"][i]).size(9.0).color(GOLD));
                            });
                        }
                    }
                });
                ui.label(
                    RichText::new(
                        self.build
                            .shards
                            .iter()
                            .enumerate()
                            .map(|(row, id)| {
                                SHARDS[row]
                                    .iter()
                                    .find(|(n, _)| n == id)
                                    .map_or("", |(_, name)| *name)
                            })
                            .collect::<Vec<_>>()
                            .join(" / "),
                    )
                    .size(10.0)
                    .color(MUTED),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panels_follow_only_their_real_game_phase() {
        let settings = OverlaySettings::default();
        for (phase, expected) in [
            (Some("ChampSelect"), [false, false, true]),
            (Some("InProgress"), [true, true, false]),
            (None, [false; 3]),
            (Some("None"), [false; 3]),
            (Some("Lobby"), [false; 3]),
            (Some("Matchmaking"), [false; 3]),
            (Some("ReadyCheck"), [false; 3]),
            (Some("GameStart"), [false; 3]),
            (Some("Reconnect"), [false; 3]),
            (Some("WaitingForStats"), [false; 3]),
            (Some("PreEndOfGame"), [false; 3]),
            (Some("EndOfGame"), [false; 3]),
            (Some("Unknown"), [false; 3]),
        ] {
            assert_eq!(
                PANELS.map(|p| p.visible(&settings, phase, Some(Duration::ZERO))),
                expected,
                "{phase:?}"
            );
        }
    }
    #[test]
    fn toggles_survive_phases_but_stale_connections_never_show() {
        let mut settings = OverlaySettings {
            items: false,
            ..Default::default()
        };
        assert_eq!(
            PANELS.map(|p| p.visible(&settings, Some("InProgress"), Some(Duration::ZERO))),
            [false, true, false]
        );
        assert_eq!(
            PANELS.map(|p| p.visible(&settings, Some("ChampSelect"), Some(Duration::ZERO))),
            [false, false, true]
        );
        settings.enabled = false;
        assert!(PANELS.into_iter().all(|p| !p.visible(
            &settings,
            Some("InProgress"),
            Some(Duration::ZERO)
        )));
        settings.enabled = true;
        for phase in ["ChampSelect", "InProgress"] {
            for age in [None, Some(PHASE_MAX_AGE), Some(Duration::from_secs(60))] {
                assert!(
                    PANELS
                        .into_iter()
                        .all(|p| !p.visible(&settings, Some(phase), age))
                );
            }
        }
        let restored: OverlaySettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(!restored.items && restored.skills && restored.runes && restored.enabled);
    }
    #[test]
    fn old_preferences_gain_independent_panels() {
        let settings: OverlaySettings =
            serde_json::from_str(r#"{"opacity":65,"collapsed":true}"#).unwrap();
        assert_eq!(settings.opacity, 65);
        assert!(
            settings.collapsed
                && settings.items
                && settings.skills
                && settings.runes
                && settings.enabled
        );
        assert_ne!(Panel::Items.viewport(), Panel::Skills.viewport());
        assert_ne!(Panel::Skills.viewport(), Panel::Runes.viewport());
    }
}
