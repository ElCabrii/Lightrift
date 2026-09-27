use crate::{
    client::{self, Command, Event, Snapshot},
    data::*,
    recommendations::{self, Recommendation},
    rune_icons::RUNE_ICONS,
    sprites::SPRITES,
};
use eframe::egui::{self, Color32, Margin, RichText, Stroke, Vec2};
use serde_json::Value;
mod design;
mod live;
mod overlay;
use std::{
    collections::BTreeMap,
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};
const BG: Color32 = Color32::from_rgb(14, 18, 24);
const PANEL: Color32 = Color32::from_rgb(22, 28, 36);
const LINE: Color32 = Color32::from_rgb(40, 49, 61);
const TEXT: Color32 = Color32::from_rgb(230, 236, 247);
const MUTED: Color32 = Color32::from_rgb(137, 151, 172);
const ACCENT: Color32 = Color32::from_rgb(169, 157, 242);
const MINT: Color32 = Color32::from_rgb(109, 226, 196);
const GOLD: Color32 = Color32::from_rgb(232, 197, 119);
#[derive(PartialEq, Clone, Copy)]
enum Page {
    Champions,
    Builds,
    Live,
    Settings,
    Overlays,
}
#[derive(PartialEq, Clone, Copy)]
enum DetailTab {
    Recommended,
    Build,
    Runes,
    Abilities,
}
pub struct Lightrift {
    catalog: Catalog,
    settings: Settings,
    settings_ok: bool,
    textures: BTreeMap<String, egui::TextureHandle>,
    page: Page,
    tab: DetailTab,
    search: String,
    class: String,
    favorites_only: bool,
    selected: String,
    build: Build,
    dirty: bool,
    item_search: String,
    item_group: usize,
    item_filter: String,
    detail: Option<Value>,
    detail_status: String,
    tx: Sender<Command>,
    rx: Receiver<Event>,
    snapshot: Option<Snapshot>,
    snapshot_at: Option<Instant>,
    connection: String,
    pending: bool,
    applying: bool,
    last_poll: Instant,
    watch: bool,
    auto_open: bool,
    last_draft_pick: Option<(String, String)>,
    message: String,
    message_error: bool,
    overlay_locked: bool,
    overlay_panels: [overlay::PanelState; 3],
    hotkeys: crate::hotkeys::Hotkeys,
    draft_preview: bool,
    manual_team: [String; 10],
    manual_bans: [String; 10],
    export_text: String,
    show_export: bool,
    show_item_library: bool,
    delete_build_id: Option<String>,
    recommendations_tx: Sender<recommendations::Request>,
    recommendations_rx: Receiver<recommendations::Reply>,
    recommendation: Option<Recommendation>,
    recommendation_core: Option<usize>,
    recommendation_pending: bool,
    recommendation_status: String,
    recommendation_role: String,
}
fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .corner_radius(10)
        .inner_margin(16)
}
fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(MUTED).size(11.0).strong());
}
fn sample_label(ui: &mut egui::Ui, v: &Value) {
    if let Some(play) = v["play"].as_u64().filter(|n| *n > 0) {
        let win = v["win"].as_u64().filter(|n| *n <= play);
        let text = win.map_or(format!("{play} games"), |w| {
            format!(
                "{play} games · {:.1}% win rate",
                w as f64 / play as f64 * 100.0
            )
        });
        ui.label(RichText::new(text).size(11.0).color(MUTED));
        if play < 200 {
            ui.label(
                RichText::new("Small sample — use with caution")
                    .size(11.0)
                    .color(GOLD),
            );
        }
    }
}
fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.12))
        .corner_radius(5)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(color));
        });
}
impl Lightrift {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = Color32::from_rgb(10, 14, 21);
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = Color32::from_rgb(35, 60, 61);
        style.visuals.selection.stroke = Stroke::new(1.0, MINT);
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::same(7);
            widget.bg_stroke = Stroke::new(1.0, LINE);
            widget.weak_bg_fill = Color32::from_rgb(28, 35, 44);
            widget.bg_fill = Color32::from_rgb(28, 35, 44);
            widget.fg_stroke = Stroke::new(1.0, TEXT);
        }
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(42, 53, 65);
        style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(38, 67, 65);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        style.visuals.hyperlink_color = MINT;
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 8.0);
        style.spacing.interact_size = Vec2::new(36.0, 32.0);
        style.visuals.slider_trailing_fill = true;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(20.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
        ctx.set_style(style);
        let catalog = Catalog::load();
        let (settings, message, settings_ok) = match load_settings() {
            Ok(s) => (s, String::new(), true),
            Err(e) => (Settings::default(), e, false),
        };
        let selected = settings
            .favorites
            .iter()
            .find(|id| catalog.champion(id).is_some())
            .cloned()
            .unwrap_or("Jinx".into());
        let build = settings
            .preferred_build(&selected, None, &catalog)
            .cloned()
            .unwrap_or_else(|| catalog.new_build(&selected));
        let recommendation_role = if build.role == "Any role" {
            "Bottom".into()
        } else {
            build.role.clone()
        };
        let mut textures = BTreeMap::new();
        for (name, bytes) in SPRITES.iter().chain(RUNE_ICONS.iter()) {
            if let Ok(img) = image::load_from_memory(bytes) {
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                textures.insert(
                    name.to_string(),
                    ctx.load_texture(
                        *name,
                        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
        let (tx, rx) = client::worker(ctx.clone());
        let (recommendations_tx, recommendations_rx) = recommendations::worker(ctx.clone());
        let overlay_panels =
            std::array::from_fn(|_| overlay::PanelState::new(settings.overlay.collapsed));
        let hotkeys = crate::hotkeys::Hotkeys::new(ctx.clone());
        let overlay_locked = hotkeys.available;
        let _ = tx.send(Command::Detail(selected.clone()));
        Self {
            catalog,
            settings,
            settings_ok,
            textures,
            page: Page::Champions,
            tab: DetailTab::Recommended,
            search: String::new(),
            class: "All classes".into(),
            favorites_only: false,
            selected,
            build,
            dirty: false,
            item_search: String::new(),
            item_group: 1,
            item_filter: "All items".into(),
            detail: None,
            detail_status: "Loading abilities…".into(),
            tx,
            rx,
            snapshot: None,
            snapshot_at: None,
            connection: "Not connected".into(),
            pending: false,
            applying: false,
            last_poll: Instant::now() - Duration::from_secs(5),
            watch: false,
            auto_open: true,
            last_draft_pick: None,
            message,
            message_error: !settings_ok,
            overlay_locked,
            overlay_panels,
            hotkeys,
            draft_preview: false,
            manual_team: Default::default(),
            manual_bans: Default::default(),
            export_text: String::new(),
            show_export: false,
            show_item_library: false,
            delete_build_id: None,
            recommendations_tx,
            recommendations_rx,
            recommendation: None,
            recommendation_core: None,
            recommendation_pending: false,
            recommendation_status: String::new(),
            recommendation_role,
        }
    }
    fn notify(&mut self, result: Result<String, String>) {
        match result {
            Ok(s) => {
                self.message = s;
                self.message_error = false
            }
            Err(s) => {
                self.message = s;
                self.message_error = true
            }
        }
    }
    fn persist(&mut self) {
        if !self.settings_ok {
            self.notify(Err(
                "Existing settings could not be read. Repair data/settings.json before saving."
                    .into(),
            ));
            return;
        }
        if let Err(e) = save_settings(&self.settings) {
            self.notify(Err(e));
        }
    }
    fn save_build(&mut self) -> bool {
        if let Err(e) = self.catalog.validate(&self.build) {
            self.notify(Err(e));
            return false;
        }
        if !self.settings_ok {
            self.persist();
            return false;
        }
        let mut builds = self.settings.builds.clone();
        if let Some(b) = builds.iter_mut().find(|b| b.id == self.build.id) {
            *b = self.build.clone()
        } else {
            builds.push(self.build.clone())
        }
        let old = std::mem::replace(&mut self.settings.builds, builds);
        let previous_active = self
            .settings
            .active_builds
            .insert(self.selected.clone(), self.build.id.clone());
        match save_settings(&self.settings) {
            Ok(()) => {
                self.dirty = false;
                self.notify(Ok("Build saved locally.".into()));
                true
            }
            Err(e) => {
                self.settings.builds = old;
                if let Some(id) = previous_active {
                    self.settings
                        .active_builds
                        .insert(self.selected.clone(), id);
                } else {
                    self.settings.active_builds.remove(&self.selected);
                }
                self.notify(Err(e));
                false
            }
        }
    }
    fn delete_build(&mut self, id: &str) {
        if !self.settings_ok {
            self.persist();
            return;
        }
        let mut next = self.settings.clone();
        let Some(removed) = next.remove_build(id) else {
            self.delete_build_id = None;
            return;
        };
        if let Err(e) = save_settings(&next) {
            self.notify(Err(e));
            return;
        }
        self.settings = next;
        if self.build.id == id {
            self.build = self
                .settings
                .preferred_build(&self.selected, None, &self.catalog)
                .cloned()
                .unwrap_or_else(|| self.catalog.new_build(&self.selected));
            self.dirty = false;
        }
        self.delete_build_id = None;
        self.notify(Ok(format!("Deleted {} from your playbook.", removed.name)));
    }
    fn choose(&mut self, id: String) {
        if id == self.selected {
            return;
        }
        if self.dirty && !self.save_build() {
            return;
        }
        self.selected = id.clone();
        self.recommendation = None;
        self.recommendation_status.clear();
        self.build = self
            .settings
            .preferred_build(&id, None, &self.catalog)
            .cloned()
            .unwrap_or_else(|| self.catalog.new_build(&id));
        self.detail = None;
        self.detail_status = "Loading abilities…".into();
        let _ = self.tx.send(Command::Detail(id));
        self.dirty = false;
        self.recommendation_role = if self.build.role != "Any role" {
            self.build.role.clone()
        } else {
            "Mid".into()
        };
    }
    fn open_build(&mut self, build: Build) {
        if self.dirty && !self.save_build() {
            return;
        }
        if self.catalog.validate(&build).is_err() {
            self.notify(Err(
                "This saved build contains data unavailable in the bundled patch.".into(),
            ));
            return;
        }
        self.choose(build.champion.clone());
        self.build = build;
        self.settings
            .active_builds
            .insert(self.selected.clone(), self.build.id.clone());
        self.persist();
        self.page = Page::Champions;
    }
    fn new_variant(&mut self, duplicate: bool) {
        if self.dirty && !self.save_build() {
            return;
        }
        let mut b = if duplicate {
            self.build.clone()
        } else {
            self.catalog.new_build(&self.selected)
        };
        b.id = new_build_id();
        b.name = if duplicate {
            format!(
                "{} copy",
                self.build.name.chars().take(54).collect::<String>()
            )
        } else {
            "New build".into()
        };
        self.build = b;
        self.dirty = true;
    }
    fn icon(&self, ui: &mut egui::Ui, sprite: &Sprite, size: f32) -> egui::Response {
        if let Some(tex) = self.textures.get(&sprite.sprite) {
            let dims = tex.size_vec2();
            let uv = egui::Rect::from_min_max(
                egui::pos2(sprite.x as f32 / dims.x, sprite.y as f32 / dims.y),
                egui::pos2(
                    (sprite.x + sprite.w) as f32 / dims.x,
                    (sprite.y + sprite.h) as f32 / dims.y,
                ),
            );
            ui.add(
                egui::Image::new(tex)
                    .uv(uv)
                    .maintain_aspect_ratio(false)
                    .fit_to_exact_size(Vec2::splat(size))
                    .corner_radius(6),
            )
        } else {
            ui.allocate_response(Vec2::splat(size), egui::Sense::hover())
        }
    }
    fn rune_button(&self, ui: &mut egui::Ui, rune: &Rune, selected: bool) -> egui::Response {
        let button = if let Some(texture) = self.textures.get(&format!("rune{}", rune.id)) {
            egui::Button::image(egui::Image::new(texture).fit_to_exact_size(Vec2::splat(30.0)))
        } else {
            egui::Button::new(&rune.name)
        };
        let response = ui.add(
            button
                .min_size(egui::vec2(48.0, 48.0))
                .fill(if selected {
                    Color32::from_rgb(30, 52, 51)
                } else {
                    BG
                })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        MINT.gamma_multiply(0.6)
                    } else {
                        LINE
                    },
                )),
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                ui.is_enabled(),
                selected,
                &rune.name,
            )
        });
        response.on_hover_text(format!("{}\n{}", rune.name, plain(&rune.description)))
    }
    fn refresh(&mut self) {
        if !self.pending {
            self.pending = true;
            self.last_poll = Instant::now();
            let _ = self.tx.send(Command::Refresh(
                self.settings.league_path.clone(),
                self.watch || self.page == Page::Live,
            ));
        }
    }
    fn process(&mut self) {
        while let Ok((champion, role, result)) = self.recommendations_rx.try_recv() {
            self.recommendation_pending = false;
            if champion == self.selected && role == self.recommendation_role {
                match result {
                    Ok(r) => {
                        self.recommendation_core = if r.core_options.is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                        self.recommendation_status = r.notice.clone();
                        self.recommendation = Some(r);
                    }
                    Err(e) => {
                        self.recommendation_status = e;
                    }
                }
            }
        }
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Snapshot(result) => {
                    self.pending = false;
                    match result {
                        Ok(s) => {
                            self.snapshot_at = Some(Instant::now());
                            self.connection = format!("League · {}", design::phase_title(&s.phase));
                            if s.phase != "ChampSelect" {
                                self.last_draft_pick = None;
                            }
                            if (self.watch
                                || self.settings.overlay.enabled
                                || self.page == Page::Live)
                                && self.auto_open
                                && !self.draft_preview
                                && let Some((key, role)) = client::draft_selection(&s.draft)
                                && let Some(champ) = self.catalog.by_key(key)
                            {
                                let selection = (champ.id.clone(), role);
                                if self.last_draft_pick.as_ref() != Some(&selection) {
                                    self.last_draft_pick = Some(selection.clone());
                                    if !self.dirty || self.save_build() {
                                        self.choose(selection.0.clone());
                                        if let Some(b) = self
                                            .settings
                                            .preferred_build(
                                                &selection.0,
                                                Some(&selection.1),
                                                &self.catalog,
                                            )
                                            .cloned()
                                        {
                                            self.open_build(b);
                                        }
                                        if self.page != Page::Live {
                                            self.page = Page::Champions;
                                            self.tab = DetailTab::Runes;
                                        }
                                        self.notify(Ok(format!(
                                            "Draft: {} · {}. Review your setup before applying.",
                                            selection.0, selection.1
                                        )));
                                    }
                                }
                            }
                            self.snapshot = Some(s)
                        }
                        Err(e) => {
                            self.snapshot_at = None;
                            self.connection = e;
                            self.snapshot = None
                        }
                    }
                }
                Event::Detail(id, result) => {
                    if id == self.selected {
                        match result {
                            Ok(d) => {
                                self.detail = Some(d);
                                self.detail_status.clear()
                            }
                            Err(e) => self.detail_status = e,
                        }
                    }
                }
                Event::Applied(r) => {
                    self.applying = false;
                    self.notify(r)
                }
            }
        }
    }
    fn navigation(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(82.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(10, 14, 19))
                    .inner_margin(Margin::symmetric(8, 16)),
            )
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(66.0, 52.0), egui::Sense::hover());
                let c = r.center();
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        c + egui::vec2(-12.0, -16.0),
                        c + egui::vec2(13.0, -16.0),
                        c + egui::vec2(3.0, -3.0),
                        c + egui::vec2(10.0, 16.0),
                        c + egui::vec2(-4.0, 16.0),
                        c + egui::vec2(-12.0, -4.0),
                    ],
                    MINT,
                    Stroke::NONE,
                ));
                ui.add_space(22.0);
                for (i, page, title) in [
                    (0, Page::Champions, "Champions"),
                    (1, Page::Builds, "Playbook"),
                    (2, Page::Live, "Live"),
                    (3, Page::Overlays, "Overlays"),
                ] {
                    if design::nav(ui, i, title, self.page == page).clicked() {
                        self.page = page;
                    }
                    ui.add_space(6.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.label(RichText::new("LIGHTRIFT 1.0.1").size(9.0).color(MUTED));
                    ui.add_space(16.0);
                    if design::nav(ui, 4, "Settings", self.page == Page::Settings).clicked() {
                        self.page = Page::Settings;
                    }
                });
            });
        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(24, 13)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("LIGHTRIFT").size(17.0).strong());
                    ui.label(RichText::new("/").color(LINE));
                    ui.label(
                        RichText::new(match self.page {
                            Page::Champions => "Champion workspace",
                            Page::Builds => "Your playbook",
                            Page::Live => "Live match",
                            Page::Overlays => "Overlay studio",
                            Page::Settings => "Settings",
                        })
                        .color(MUTED)
                        .size(12.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                !self.pending,
                                egui::Button::new(if self.pending {
                                    "Connecting…"
                                } else if self.snapshot.is_some() {
                                    "League connected"
                                } else {
                                    "Connect League"
                                })
                                .fill(PANEL),
                            )
                            .clicked()
                        {
                            self.refresh();
                        }
                        let (dot, _) =
                            ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(
                            dot.center(),
                            3.0,
                            if self.snapshot.is_some() { MINT } else { MUTED },
                        );
                        ui.label(
                            RichText::new(format!("Patch {}", PATCH.trim()))
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(24, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(if self.message.is_empty() {
                                "Ready for your next game"
                            } else {
                                &self.message
                            })
                            .size(11.0)
                            .color(if self.message_error {
                                Color32::LIGHT_RED
                            } else {
                                MUTED
                            }),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(if self.settings.overlay.enabled {
                                "Overlays on"
                            } else {
                                "Overlays paused"
                            })
                            .clicked()
                        {
                            self.toggle_overlays();
                        }
                        if self.watch && ui.small_button("Following draft · Pause").clicked() {
                            self.watch = false;
                        }
                    });
                });
            });
    }

    fn champion_browser(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("champions")
            .exact_width(216.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(BG).inner_margin(16))
            .show(ctx, |ui| {
                ui.heading("Champions");
                ui.label(
                    RichText::new(format!("{} champions", self.catalog.champions.len()))
                        .color(MUTED)
                        .size(11.0),
                );
                ui.add_space(10.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search  /  Ctrl K")
                        .margin(egui::vec2(10.0, 8.0))
                        .desired_width(f32::INFINITY),
                );
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::K)) {
                    edit.request_focus();
                }
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("class")
                        .selected_text(&self.class)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for class in [
                                "All classes",
                                "Fighter",
                                "Tank",
                                "Mage",
                                "Assassin",
                                "Marksman",
                                "Support",
                            ] {
                                ui.selectable_value(&mut self.class, class.into(), class);
                            }
                        });
                    ui.toggle_value(&mut self.favorites_only, "★")
                        .on_hover_text("Favorites only");
                });
                ui.add_space(6.0);
                let query = self.search.to_lowercase();
                let rows: Vec<_> = self
                    .catalog
                    .champions
                    .iter()
                    .filter(|c| {
                        c.name.to_lowercase().contains(&query)
                            && (self.class == "All classes" || c.tags.contains(&self.class))
                            && (!self.favorites_only || self.settings.favorites.contains(&c.id))
                    })
                    .cloned()
                    .collect();
                if rows.is_empty() {
                    ui.add_space(24.0);
                    ui.label(RichText::new("No champions found").strong());
                    ui.label(
                        RichText::new("Try another name or clear your filters.")
                            .size(12.0)
                            .color(MUTED),
                    );
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(ui, 56.0, rows.len(), |ui, range| {
                        for i in range {
                            let c = &rows[i];
                            let selected = c.id == self.selected;
                            let response = egui::Frame::new()
                                .fill(if selected {
                                    Color32::from_rgb(25, 47, 47)
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(8)
                                .inner_margin(6)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        self.icon(ui, &c.image, 36.0);
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&c.name)
                                                    .strong()
                                                    .color(if selected { ACCENT } else { TEXT }),
                                            );
                                            ui.label(
                                                RichText::new(c.tags.join(" / "))
                                                    .size(10.0)
                                                    .color(MUTED),
                                            );
                                        });
                                    });
                                })
                                .response;
                            if ui
                                .interact(response.rect, ui.id().with(&c.id), egui::Sense::click())
                                .clicked()
                            {
                                self.choose(c.id.clone());
                            }
                        }
                    });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(24))
            .show(ctx, |ui| {
                self.champion_detail(ui);
            });
    }
    fn champion_detail(&mut self, ui: &mut egui::Ui) {
        let champ = self.catalog.champion(&self.selected).unwrap().clone();
        ui.horizontal(|ui| {
            self.icon(ui, &champ.image, 64.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&champ.name).size(32.0).strong());
                ui.label(RichText::new(&champ.title).color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let fav = self.settings.favorites.contains(&champ.id);
                if ui
                    .button(if fav { "★ Saved" } else { "☆ Favorite" })
                    .clicked()
                {
                    if fav {
                        self.settings.favorites.remove(&champ.id);
                    } else {
                        self.settings.favorites.insert(champ.id.clone());
                    }
                    self.persist();
                }
            });
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, DetailTab::Recommended, "Recommended");
            ui.selectable_value(&mut self.tab, DetailTab::Build, "Build planner");
            ui.selectable_value(&mut self.tab, DetailTab::Runes, "Runes & spells");
            ui.selectable_value(&mut self.tab, DetailTab::Abilities, "Champion guide");
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(("workspace", self.tab as u8, self.selected.clone()))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.tab == DetailTab::Recommended {
                    self.recommendations_ui(ui);
                    return;
                }
                if self.tab == DetailTab::Abilities {
                    self.abilities_ui(ui, &champ);
                    return;
                }
                self.import_controls(ui);
                ui.add_space(8.0);
                let variants: Vec<_> = self
                    .settings
                    .builds
                    .iter()
                    .filter(|b| b.champion == self.selected)
                    .cloned()
                    .collect();
                ui.horizontal_wrapped(|ui| {
                    label(ui, "SAVED BUILDS");
                    let mut chosen = self.build.id.clone();
                    egui::ComboBox::from_id_salt("build-variant")
                        .selected_text(format!("{} · {}", self.build.name, self.build.role))
                        .width(230.0)
                        .show_ui(ui, |ui| {
                            for b in &variants {
                                ui.selectable_value(
                                    &mut chosen,
                                    b.id.clone(),
                                    format!("{} · {}", b.name, b.role),
                                );
                            }
                        });
                    if chosen != self.build.id
                        && let Some(b) = variants.iter().find(|b| b.id == chosen)
                    {
                        self.open_build(b.clone());
                    }
                    if ui.button("New build").clicked() {
                        self.new_variant(false);
                    }
                    if ui.button("Duplicate").clicked() {
                        self.new_variant(true);
                    }
                });
                ui.horizontal(|ui| {
                    label(ui, "BUILD NAME");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.build.name)
                                .margin(egui::vec2(10.0, 8.0))
                                .desired_width(240.0)
                                .char_limit(60),
                        )
                        .changed()
                    {
                        self.dirty = true;
                    }
                    egui::ComboBox::from_id_salt("role")
                        .selected_text(&self.build.role)
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for role in ["Any role", "Top", "Jungle", "Mid", "Bottom", "Support"] {
                                if ui
                                    .selectable_value(&mut self.build.role, role.into(), role)
                                    .changed()
                                {
                                    self.dirty = true;
                                }
                            }
                        });
                    if ui
                        .button(if self.dirty {
                            "Save changes •"
                        } else {
                            "Save build"
                        })
                        .clicked()
                    {
                        self.save_build();
                    }
                });
                ui.add_space(12.0);
                match self.tab {
                    DetailTab::Recommended => {}
                    DetailTab::Build => self.build_ui(ui),
                    DetailTab::Runes => self.runes_ui(ui),
                    DetailTab::Abilities => self.abilities_ui(ui, &champ),
                }
            });
    }
    fn import_controls(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(Color32::from_rgb(22, 37, 38))
            .stroke(Stroke::new(1.0, Color32::from_rgb(40, 65, 61)))
            .corner_radius(9)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Import into League").strong().color(MINT));
                    ui.separator();
                    let enabled = self.snapshot.is_some() && !self.applying;
                    if ui
                        .add_enabled(enabled, egui::Button::new("Apply runes"))
                        .on_hover_text("Apply using League’s temporary rune page. Your saved rune pages and Lightrift builds are preserved.")
                        .clicked()
                    {
                        self.apply(0);
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Save rune page"))
                        .on_hover_text("Create or update a named Lightrift rune page in League and select it.")
                        .clicked()
                    {
                        self.apply(3);
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Save item set"))
                        .clicked()
                    {
                        self.apply(1);
                    }
                    if ui
                        .add_enabled(
                            enabled
                                && self
                                    .snapshot
                                    .as_ref()
                                    .is_some_and(|s| s.phase == "ChampSelect"),
                            egui::Button::new("Apply spells"),
                        )
                        .on_hover_text("Available during champion select for this champion")
                        .clicked()
                    {
                        self.apply(2);
                    }
                    if ui.button("Export JSON").clicked() {
                        self.export_text =
                            serde_json::to_string_pretty(&self.build).unwrap_or_default();
                        self.show_export = true;
                    }
                });
                ui.label(
                    RichText::new(if self.applying {
                        "Applying and checking League’s response…"
                    } else if self.snapshot.is_some() {
                        "Ready · Imports use the build selected below."
                    } else {
                        "Open League and connect to enable imports."
                    })
                    .size(11.0)
                    .color(MUTED),
                );
            });
    }

    fn apply(&mut self, kind: u8) {
        self.apply_build(kind, self.build.clone());
    }
    fn apply_build(&mut self, kind: u8, b: Build) {
        if self.applying {
            return;
        }
        if let Err(e) = self.catalog.validate(&b) {
            self.notify(Err(e));
            return;
        }
        self.applying = true;
        let p = self.settings.league_path.clone();
        let _ = self.tx.send(match kind {
            0 => Command::Runes(p, b),
            1 => Command::Items(p, b),
            3 => Command::SaveRunes(p, b),
            _ => Command::Spells(p, b),
        });
    }
    fn load_recommendations(&mut self, force: bool) {
        if self.recommendation_pending {
            return;
        }
        if let Some(c) = self.catalog.champion(&self.selected) {
            let request = recommendations::Request {
                champion: c.id.clone(),
                name: c.name.clone(),
                role: self.recommendation_role.clone(),
                force,
            };
            if self.recommendations_tx.send(request).is_ok() {
                self.recommendation_pending = true;
                self.recommendation_status = "Loading OP.GG builds…".into();
            }
        }
    }
    fn recommendations_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let old = self.recommendation_role.clone();
            for (role, _) in recommendations::ROLES {
                ui.selectable_value(&mut self.recommendation_role, (*role).into(), *role);
            }
            if old != self.recommendation_role {
                self.recommendation = None;
                self.recommendation_status.clear();
            }
            ui.separator();
            if ui
                .add_enabled(
                    !self.recommendation_pending,
                    design::primary(if self.recommendation_pending {
                        "Loading…"
                    } else if self.recommendation.is_some() {
                        "Refresh builds"
                    } else {
                        "Find builds"
                    }),
                )
                .clicked()
            {
                self.load_recommendations(self.recommendation.is_some());
            }
        });
        if !self.recommendation_status.is_empty() {
            ui.label(
                RichText::new(&self.recommendation_status)
                    .color(GOLD)
                    .size(12.0),
            );
        }
        let Some(r) = self.recommendation.clone() else {
            card().show(ui,|ui| {
                ui.set_width(ui.available_width());
                ui.add_space(38.0);
                ui.vertical_centered(|ui| {
                    if self.recommendation_pending { ui.spinner(); }
                    ui.label(RichText::new(if self.recommendation_pending {"Preparing your next game"}else{"A better starting point"}).size(26.0).strong());
                    ui.label(RichText::new("Choose your lane, then find published item paths, runes and skill orders.").color(MUTED));
                    ui.add_space(12.0);
                    ui.label(RichText::new("OP.GG · Ranked · All ranks").size(11.0).color(ACCENT));
                    ui.add_space(38.0);
                });
            });
            return;
        };
        let summary = &r.data["summary"]["average_stats"];
        card().inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.columns(4, |cols| {
                design::metric(
                    &mut cols[0],
                    &summary["win_rate"]
                        .as_f64()
                        .map_or("—".into(), |v| format!("{:.1}%", v * 100.0)),
                    "CHAMPION WIN RATE",
                    MINT,
                );
                design::metric(
                    &mut cols[1],
                    &summary["pick_rate"]
                        .as_f64()
                        .map_or("—".into(), |v| format!("{:.1}%", v * 100.0)),
                    "PICK RATE",
                    TEXT,
                );
                design::metric(
                    &mut cols[2],
                    &summary["play"]
                        .as_u64()
                        .map_or("—".into(), |v| format!("{:.1}k", v as f64 / 1000.0)),
                    "GAMES ANALYZED",
                    TEXT,
                );
                design::metric(&mut cols[3], r.patch(), "SOURCE PATCH", ACCENT);
            });
        });
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            design::section(ui, "Choose your core", "Three items, in purchase order");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(design::primary("Save selected build")).clicked() {
                    match r.to_build_option(&self.catalog, self.recommendation_core) {
                        Ok(b) => {
                            if !self.dirty || self.save_build() {
                                let previous = self.build.clone();
                                self.build = b;
                                self.dirty = true;
                                if self.save_build() {
                                    self.tab = DetailTab::Build;
                                } else {
                                    self.build = previous;
                                    self.dirty = false;
                                }
                            }
                        }
                        Err(e) => self.notify(Err(e)),
                    }
                }
                if ui.add_enabled(self.snapshot.is_some() && !self.applying, egui::Button::new("Apply runes"))
                    .on_hover_text("Apply this recommendation using League’s temporary rune page, without saving a rune page or Lightrift build.")
                    .clicked() {
                    match r.to_build_option(&self.catalog, self.recommendation_core) {
                        Ok(b) => self.apply_build(0, b),
                        Err(e) => self.notify(Err(e)),
                    }
                }
            });
        });
        if ui.available_width() >= 820.0 {
            let width = ui.available_width();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(width * 0.57 - 8.0, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| self.core_choices(ui, &r),
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(width * 0.43 - 8.0, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| self.recommendation_setup(ui, &r),
                );
            });
        } else {
            self.core_choices(ui, &r);
            ui.add_space(16.0);
            self.recommendation_setup(ui, &r);
        }
        ui.add_space(8.0);
        let age = recommendations::now().saturating_sub(r.fetched_at);
        ui.label(
            RichText::new(format!(
                "OP.GG · Shared setup region unspecified · Retrieved {} min ago{}",
                age / 60,
                if age >= 6 * 3600 {
                    " · Older cached data"
                } else {
                    ""
                }
            ))
            .color(MUTED)
            .size(11.0),
        );
        ui.hyperlink_to(
            "View source on OP.GG",
            format!(
                "https://op.gg/lol/champions/{}/build",
                self.selected.to_lowercase()
            ),
        );
    }
    fn core_choices(&mut self, ui: &mut egui::Ui, r: &Recommendation) {
        if r.core_options.is_empty() {
            self.recommendation_core = None;
            ui.label(
                RichText::new("One service build available. Refresh to retry alternatives.")
                    .color(GOLD)
                    .size(12.0),
            );
            self.recommended_items(ui, r, "core_items", "Core items");
        } else {
            for (i, option) in r.core_options.iter().enumerate() {
                let selected = self.recommendation_core == Some(i);
                let response =
                    egui::Frame::new()
                        .fill(if selected {
                            Color32::from_rgb(24, 43, 43)
                        } else {
                            PANEL
                        })
                        .stroke(Stroke::new(
                            1.0,
                            if selected {
                                MINT.gamma_multiply(0.55)
                            } else {
                                LINE
                            },
                        ))
                        .corner_radius(9)
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (mark, _) = ui.allocate_exact_size(
                                    egui::vec2(24.0, 34.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().circle_stroke(
                                    mark.center(),
                                    7.0,
                                    Stroke::new(1.4, if selected { MINT } else { MUTED }),
                                );
                                if selected {
                                    ui.painter().circle_filled(mark.center(), 3.5, MINT);
                                }
                                for id in &option.ids {
                                    if let Some(item) = self.catalog.items.get(id) {
                                        self.icon(ui, &item.image, 34.0).on_hover_text(format!(
                                            "{}\n{}",
                                            item.name,
                                            plain(&item.description)
                                        ));
                                    }
                                }
                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(if i == 0 {
                                            "Most played".into()
                                        } else {
                                            format!("Alternative {}", i + 1)
                                        })
                                        .strong()
                                        .color(if selected { MINT } else { TEXT }),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} games · {:.1}% pick",
                                            option.games,
                                            option.pick_rate * 100.0
                                        ))
                                        .size(11.0)
                                        .color(MUTED),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(76.0, 42.0),
                                            egui::Layout::top_down(egui::Align::RIGHT),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:.2}%",
                                                        option.win_rate * 100.0
                                                    ))
                                                    .size(20.0)
                                                    .strong()
                                                    .color(MINT),
                                                );
                                                label(ui, "WIN RATE");
                                            },
                                        );
                                    },
                                );
                            });
                        })
                        .response;
                let response = ui.interact(
                    response.rect,
                    ui.id().with(("core-choice", i)),
                    egui::Sense::click(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        true,
                        selected,
                        self.core_option_title(i, option),
                    )
                });
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        response.rect,
                        9,
                        Stroke::new(2.0, MINT),
                        egui::StrokeKind::Inside,
                    );
                }
                if option.games < 200 {
                    ui.label(
                        RichText::new("Small sample · fewer than 200 games")
                            .size(11.0)
                            .color(GOLD),
                    );
                }
                if response
                    .on_hover_text(self.core_option_title(i, option))
                    .clicked()
                {
                    self.recommendation_core = Some(i);
                }
            }
        }
        ui.label(RichText::new("Core paths: Global · All ranks. Win rates describe these item samples, not a complete loadout.").size(11.0).color(MUTED));
    }
    fn recommendation_setup(&self, ui: &mut egui::Ui, r: &Recommendation) {
        ui.spacing_mut().item_spacing.y = 6.0;
        label(ui, "SHARED SETUP · ALL CORE OPTIONS");
        ui.horizontal_wrapped(|ui| {
            for (key, title) in [("starter_items", "Start"), ("boots", "Boots")] {
                label(ui, title);
                if let Ok(ids) = recommendations::numbers(&r.data[key]["ids"]) {
                    for id in ids {
                        if let Some(item) = self.catalog.items.get(&id) {
                            self.icon(ui, &item.image, 30.0).on_hover_text(format!(
                                "{} · {} games",
                                item.name, r.data[key]["play"]
                            ));
                        }
                    }
                }
            }
        });
        ui.add_space(8.0);
        card().inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());

            let runes = &r.data["runes"];
            ui.horizontal_top(|ui| {
                for (key, ids) in [
                    ("primary_page_id", "primary_rune_ids"),
                    ("secondary_page_id", "secondary_rune_ids"),
                ] {
                    if let Some(tree) = runes[key]
                        .as_u64()
                        .and_then(|id| self.catalog.runes.iter().find(|t| t.id as u64 == id))
                    {
                        ui.allocate_ui_with_layout(
                            egui::vec2(
                                if key == "primary_page_id" {
                                    170.0
                                } else {
                                    100.0
                                },
                                0.0,
                            ),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                label(ui, &tree.name);
                                ui.horizontal_wrapped(|ui| {
                                    if let Ok(ids) = recommendations::numbers(&runes[ids]) {
                                        for id in ids {
                                            if let Some(rune) = tree
                                                .slots
                                                .iter()
                                                .flat_map(|s| &s.runes)
                                                .find(|r| r.id == id)
                                                && let Some(texture) =
                                                    self.textures.get(&format!("rune{}", rune.id))
                                            {
                                                ui.add(
                                                    egui::Image::new(texture)
                                                        .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                                                )
                                                .on_hover_text(format!(
                                                    "{}\n{}",
                                                    rune.name,
                                                    plain(&rune.description)
                                                ));
                                            }
                                        }
                                    }
                                });
                            },
                        );
                    }
                }
            });
            if let Ok(shards) = recommendations::numbers(&runes["stat_mod_ids"]) {
                ui.label(
                    RichText::new(
                        shards
                            .iter()
                            .enumerate()
                            .map(|(row, id)| {
                                SHARDS
                                    .get(row)
                                    .and_then(|s| s.iter().find(|(n, _)| n == id))
                                    .map_or(format!("#{id}"), |(_, name)| (*name).into())
                            })
                            .collect::<Vec<_>>()
                            .join(" · "),
                    )
                    .size(11.0)
                    .color(MUTED),
                );
            }
            sample_label(ui, runes);
        });
        ui.add_space(8.0);
        card().inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            label(ui, "SUMMONERS");
            if let Ok(ids) = recommendations::numbers(&r.data["summoner_spells"]["ids"]) {
                ui.horizontal_wrapped(|ui| {
                    for id in ids {
                        if let Some(s) = self
                            .catalog
                            .spells
                            .iter()
                            .find(|s| s.key.parse::<u32>().ok() == Some(id))
                        {
                            self.icon(ui, &s.image, 30.0);
                            ui.label(&s.name);
                        }
                    }
                });
            }
            ui.add_space(2.0);
            label(ui, "SKILL ORDER");
            if let Some(order) = r.data["skills"]["order"].as_array() {
                design::skills(
                    ui,
                    &order.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
                    true,
                );
            }
        });
    }
    fn recommended_items(&self, ui: &mut egui::Ui, r: &Recommendation, key: &str, title: &str) {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            design::section(ui, title, "");
            if let Ok(ids) = recommendations::numbers(&r.data[key]["ids"]) {
                ui.horizontal_wrapped(|ui| {
                    for id in ids {
                        if let Some(item) = self.catalog.items.get(&id) {
                            self.icon(ui, &item.image, 36.0).on_hover_text(format!(
                                "{}\n{}",
                                item.name,
                                plain(&item.description)
                            ));
                        }
                    }
                });
            } else {
                ui.label("No recommendation available");
            }
            sample_label(ui, &r.data[key]);
        });
    }

    fn core_option_title(&self, index: usize, option: &recommendations::CoreOption) -> String {
        format!(
            "{}. {}",
            index + 1,
            option
                .ids
                .iter()
                .map(|id| self
                    .catalog
                    .items
                    .get(id)
                    .map_or_else(|| format!("Item #{id}"), |item| item.name.clone()))
                .collect::<Vec<_>>()
                .join(" / ")
        )
    }
    fn build_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() >= 820.0 {
            let width = ui.available_width();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(width * 0.61 - 8.0, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| self.item_plan(ui),
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(width * 0.39 - 8.0, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        card().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            self.build_notes(ui);
                        });
                    },
                );
            });
        } else {
            self.item_plan(ui);
            self.build_notes(ui);
        }
    }
    fn item_plan(&mut self, ui: &mut egui::Ui) {
        for (index, title, subtitle) in [
            (0, "Starting items", "Your first purchase"),
            (1, "Core build", "Your planned item order"),
            (2, "Situational", "Adapt to the game"),
        ] {
            let ids = match index {
                0 => self.build.starter.clone(),
                1 => self.build.core.clone(),
                _ => self.build.situational.clone(),
            };
            let mut remove = None;
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).strong());
                    ui.label(RichText::new(subtitle).size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(self.item_group == index, "+ Add items")
                            .clicked()
                        {
                            self.item_group = index;
                            self.show_item_library = true;
                        }
                    });
                });
                ui.horizontal(|ui| {
                    let mut total = 0;
                    for (pos, id) in ids.iter().enumerate() {
                        if let Some(item) = self.catalog.items.get(id) {
                            total += item.gold["total"].as_i64().unwrap_or(0);
                            let response = self.icon(ui, &item.image, 42.0).on_hover_text(format!(
                                "{} · {} gold\n{}\nClick to remove",
                                item.name,
                                item.gold["total"],
                                plain(&item.description)
                            ));
                            if response.interact(egui::Sense::click()).clicked() {
                                remove = Some(pos);
                            }
                        }
                    }
                    for _ in ids.len()..6 {
                        let (r, add) =
                            ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::click());
                        add.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Add item to {title}"),
                            )
                        });
                        if add.clicked() {
                            self.item_group = index;
                            self.show_item_library = true;
                        }
                        ui.painter().rect_stroke(
                            r,
                            6,
                            Stroke::new(1.0_f32, LINE),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            "+",
                            egui::FontId::proportional(18.0),
                            LINE,
                        );
                    }
                    ui.label(RichText::new(format!("{total} g")).color(GOLD).size(12.0));
                });
            });
            if let Some(pos) = remove {
                match index {
                    0 => {
                        self.build.starter.remove(pos);
                    }
                    1 => {
                        self.build.core.remove(pos);
                    }
                    _ => {
                        self.build.situational.remove(pos);
                    }
                }
                self.dirty = true;
            }
            ui.add_space(6.0);
        }
    }
    fn build_notes(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        label(ui, "SKILL ORDER · LEVELS 1–18");
        ui.horizontal_wrapped(|ui| {
            for key in ["Q", "W", "E", "R"] {
                if ui
                    .add_enabled(self.build.skill_order.len() < 18, egui::Button::new(key))
                    .clicked()
                {
                    self.build.skill_order.push(key.into());
                    self.dirty = true;
                }
            }
            if ui
                .add_enabled(
                    !self.build.skill_order.is_empty(),
                    egui::Button::new("Undo skill"),
                )
                .clicked()
            {
                self.build.skill_order.pop();
                self.dirty = true;
            }
        });
        design::skills(ui, &self.build.overlay_skills(), false);
        ui.add_space(12.0);
        label(ui, "MATCHUP NOTES");
        if ui
            .add(
                egui::TextEdit::multiline(&mut self.build.notes)
                    .hint_text("Power spikes, matchup reminders, and alternatives…")
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.dirty = true;
        }
    }
    fn item_library(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Item library").size(18.0).strong());
            pill(
                ui,
                [
                    "ADDING TO STARTER",
                    "ADDING TO CORE",
                    "ADDING TO SITUATIONAL",
                ][self.item_group],
                MINT,
            );
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.item_search)
                    .hint_text("Search items…")
                    .margin(egui::vec2(10.0, 8.0))
                    .desired_width(200.0),
            );
            egui::ComboBox::from_id_salt("item-filter")
                .selected_text(&self.item_filter)
                .show_ui(ui, |ui| {
                    for title in [
                        "All items",
                        "Damage",
                        "Magic",
                        "Defense",
                        "Boots",
                        "Support",
                    ] {
                        ui.selectable_value(&mut self.item_filter, title.into(), title);
                    }
                });
        });
        let q = self.item_search.to_lowercase();
        let tag = match self.item_filter.as_str() {
            "Damage" => "Damage",
            "Magic" => "SpellDamage",
            "Defense" => "Armor",
            "Boots" => "Boots",
            "Support" => "GoldPer",
            _ => "",
        };
        let mut items: Vec<_> = self
            .catalog
            .items
            .iter()
            .filter(|(id, it)| {
                available_item(**id, it, &self.selected)
                    && it.name.to_lowercase().contains(&q)
                    && (tag.is_empty() || it.tags.iter().any(|t| t == tag))
            })
            .map(|(id, it)| (*id, it.clone()))
            .collect();
        items.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        egui::ScrollArea::vertical()
            .id_salt("items-scroll")
            .max_height(420.0)
            .show_rows(ui, 46.0, items.len(), |ui, range| {
                for i in range {
                    let (id, it) = &items[i];
                    ui.horizontal(|ui| {
                        self.icon(ui, &it.image, 34.0)
                            .on_hover_text(plain(&it.description));
                        ui.label(&it.name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button("+")
                                .on_hover_text("Add to selected block")
                                .clicked()
                            {
                                let block = match self.item_group {
                                    0 => &mut self.build.starter,
                                    1 => &mut self.build.core,
                                    _ => &mut self.build.situational,
                                };
                                if block.len() < 6 {
                                    block.push(*id);
                                    self.dirty = true;
                                }
                            }
                            ui.label(RichText::new(format!("{} g", it.gold["total"])).color(GOLD));
                        });
                    });
                }
            });
    }
    fn runes_ui(&mut self, ui: &mut egui::Ui) {
        let trees = self.catalog.runes.clone();
        let before = self.build.clone();
        let wide = ui.available_width() >= 820.0;
        ui.columns(if wide { 3 } else { 2 }, |cols| {
            card().show(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                label(ui, "PRIMARY TREE");
                let old = self.build.primary;
                egui::ComboBox::from_id_salt("primary")
                    .selected_text(&self.catalog.tree(self.build.primary).name)
                    .show_ui(ui, |ui| {
                        for t in &trees {
                            ui.selectable_value(&mut self.build.primary, t.id, &t.name);
                        }
                    });
                if old != self.build.primary {
                    self.build.primary_runes = self
                        .catalog
                        .tree(self.build.primary)
                        .slots
                        .iter()
                        .map(|s| s.runes[0].id)
                        .collect();
                    if self.build.secondary == self.build.primary {
                        self.build.secondary = old;
                        self.build.secondary_runes = vec![
                            self.catalog.tree(old).slots[1].runes[0].id,
                            self.catalog.tree(old).slots[2].runes[0].id,
                        ];
                    }
                }
                let tree = self.catalog.tree(self.build.primary).clone();
                for (i, slot) in tree.slots.iter().enumerate() {
                    ui.add_space(4.0);
                    if i == 0 {
                        label(ui, "KEYSTONE");
                    }
                    ui.horizontal_wrapped(|ui| {
                        for r in &slot.runes {
                            if self
                                .rune_button(ui, r, self.build.primary_runes[i] == r.id)
                                .clicked()
                            {
                                self.build.primary_runes[i] = r.id;
                            }
                        }
                    });
                    if let Some(r) = slot
                        .runes
                        .iter()
                        .find(|r| self.build.primary_runes[i] == r.id)
                    {
                        ui.label(RichText::new(&r.name).size(11.0).color(ACCENT));
                    }
                }
            });
            card().show(&mut cols[1], |ui| {
                ui.set_width(ui.available_width());
                label(ui, "SECONDARY TREE");
                let old = self.build.secondary;
                egui::ComboBox::from_id_salt("secondary")
                    .selected_text(&self.catalog.tree(self.build.secondary).name)
                    .show_ui(ui, |ui| {
                        for t in &trees {
                            if t.id != self.build.primary {
                                ui.selectable_value(&mut self.build.secondary, t.id, &t.name);
                            }
                        }
                    });
                if old != self.build.secondary {
                    let t = self.catalog.tree(self.build.secondary);
                    self.build.secondary_runes =
                        vec![t.slots[1].runes[0].id, t.slots[2].runes[0].id];
                }
                ui.label(
                    RichText::new("Choose two runes from different rows.")
                        .size(11.0)
                        .color(MUTED),
                );
                let tree = self.catalog.tree(self.build.secondary).clone();
                for (row, slot) in tree.slots.iter().enumerate().skip(1) {
                    ui.add_space(4.0);
                    label(ui, &format!("ROW {row}"));
                    ui.horizontal_wrapped(|ui| {
                        for r in &slot.runes {
                            let selected = self.build.secondary_runes.contains(&r.id);
                            if self.rune_button(ui, r, selected).clicked() && !selected {
                                if let Some(pos) = self
                                    .build
                                    .secondary_runes
                                    .iter()
                                    .position(|id| slot.runes.iter().any(|r| r.id == *id))
                                {
                                    self.build.secondary_runes[pos] = r.id;
                                } else {
                                    self.build.secondary_runes.remove(0);
                                    self.build.secondary_runes.push(r.id);
                                }
                            }
                        }
                    });
                    let choice = slot
                        .runes
                        .iter()
                        .find(|r| self.build.secondary_runes.contains(&r.id));
                    ui.label(
                        RichText::new(choice.map_or("Not selected", |r| r.name.as_str()))
                            .size(11.0)
                            .color(MUTED),
                    );
                }
            });
            if wide {
                self.rune_extras(&mut cols[2]);
            }
        });
        if !wide {
            self.rune_extras(ui);
        }
        if before != self.build {
            self.dirty = true;
        }
    }
    fn rune_extras(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            label(ui, "STAT SHARDS");
            for (i, options) in SHARDS.iter().enumerate() {
                label(ui, ["Offense", "Flex", "Defense"][i]);
                egui::ComboBox::from_id_salt(("stat-shard", i))
                    .width(190.0)
                    .selected_text(
                        options
                            .iter()
                            .find(|(id, _)| *id == self.build.shards[i])
                            .map_or("Choose shard", |(_, name)| *name),
                    )
                    .show_ui(ui, |ui| {
                        for (id, name) in *options {
                            ui.selectable_value(&mut self.build.shards[i], *id, *name);
                        }
                    });
            }
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            label(ui, "SUMMONER SPELLS");
            for (i, key) in ["D", "F"].iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(*key).strong().color(ACCENT));
                    let spells = self.catalog.spells.clone();
                    if let Some(spell) = spells
                        .iter()
                        .find(|s| s.key.parse::<u32>().ok() == Some(self.build.spells[i]))
                    {
                        self.icon(ui, &spell.image, 28.0);
                    }
                    let name = spells
                        .iter()
                        .find(|s| s.key.parse::<u32>().ok() == Some(self.build.spells[i]))
                        .map(|s| s.name.as_str())
                        .unwrap_or("Select");
                    egui::ComboBox::from_id_salt(("spell", i))
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            for spell in spells {
                                ui.selectable_value(
                                    &mut self.build.spells[i],
                                    spell.key.parse().unwrap_or(0),
                                    &spell.name,
                                )
                                .on_hover_text(plain(&spell.description));
                            }
                        });
                });
            }
        });
    }
    fn abilities_ui(&mut self, ui: &mut egui::Ui, champ: &Champion) {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(plain(&champ.blurb));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for (key, title) in [
                    ("hp", "Base health"),
                    ("attackdamage", "Attack damage"),
                    ("armor", "Armor"),
                    ("movespeed", "Move speed"),
                ] {
                    ui.vertical(|ui| {
                        label(ui, title);
                        ui.label(
                            RichText::new(champ.stats[key].to_string())
                                .size(23.0)
                                .color(ACCENT),
                        );
                    });
                    ui.add_space(20.0);
                }
            });
        });
        ui.add_space(12.0);
        if let Some(detail) = &self.detail {
            let detail = detail.clone();
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                label(ui, "PASSIVE");
                ui.strong(detail["passive"]["name"].as_str().unwrap_or(""));
                ui.label(plain(
                    detail["passive"]["description"].as_str().unwrap_or(""),
                ));
            });
            if let Some(spells) = detail["spells"].as_array() {
                for (i, s) in spells.iter().enumerate() {
                    ui.add_space(8.0);
                    card().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            pill(ui, ["Q", "W", "E", "R"].get(i).unwrap_or(&"?"), ACCENT);
                            ui.strong(s["name"].as_str().unwrap_or(""));
                        });
                        ui.label(plain(s["description"].as_str().unwrap_or("")));
                        ui.label(
                            RichText::new(format!(
                                "Cooldown {} s  ·  Cost {}",
                                s["cooldownBurn"].as_str().unwrap_or("—"),
                                s["costBurn"].as_str().unwrap_or("—")
                            ))
                            .size(12.0)
                            .color(MUTED),
                        );
                    });
                }
            }
        } else {
            ui.label(&self.detail_status);
            if ui.button("Retry abilities").clicked() {
                let _ = self.tx.send(Command::Detail(self.selected.clone()));
            }
        }
        ui.add_space(14.0);
        label(ui, "MORE BUILD RESEARCH");
        ui.horizontal(|ui| {
            ui.hyperlink_to(
                "DPM builds & matchups",
                format!("https://dpm.lol/champions/{}/build", champ.id),
            );
            ui.hyperlink_to(
                "OP.GG champion stats",
                format!(
                    "https://op.gg/lol/champions/{}/build",
                    champ.id.to_lowercase()
                ),
            );
        });
        ui.label(
            RichText::new(
                "External sources open in your browser. Builds are also available in the Recommended tab.",
            )
            .size(11.0)
            .color(MUTED),
        );
    }
    fn builds_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Your playbook").size(30.0).strong());
                ui.label(RichText::new("Good preparation, saved for next time.").color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(design::primary("Find a champion")).clicked() {
                    self.page = Page::Champions;
                }
            });
        });
        ui.add_space(24.0);
        let builds = self.settings.builds.clone();
        if builds.is_empty() {
            card().show(ui,|ui|{ui.set_width(ui.available_width());ui.add_space(30.0);ui.heading("Your next favorite build starts here");ui.label(RichText::new("Save a recommendation or create a personal setup. Every variant stays in your playbook.").color(MUTED));ui.add_space(30.0);});
            return;
        }
        label(ui, &format!("{} SAVED BUILDS", builds.len()));
        for row in builds.chunks(2) {
            ui.columns(2, |cols| {
                for (i, build) in row.iter().enumerate() {
                    card().show(&mut cols[i], |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            if let Some(c) = self.catalog.champion(&build.champion) {
                                self.icon(ui, &c.image, 52.0);
                            }
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&build.champion).size(20.0).strong());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&build.name).size(12.0).color(MUTED),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            pill(ui, &build.role, ACCENT);
                            ui.label(
                                RichText::new(format!("Patch {}", build.patch))
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if build.core.is_empty() {
                                ui.label(
                                    RichText::new("No core items yet").color(MUTED).size(12.0),
                                );
                            }
                            for id in &build.core {
                                if let Some(item) = self.catalog.items.get(id) {
                                    self.icon(ui, &item.image, 32.0).on_hover_text(&item.name);
                                }
                            }
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}",
                                    self.catalog.tree(build.primary).name,
                                    self.catalog.tree(build.secondary).name
                                ))
                                .size(11.0)
                                .color(MUTED),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Edit build").clicked() {
                                        self.open_build(build.clone());
                                        self.tab = DetailTab::Build;
                                    }
                                    if ui
                                        .button("Delete")
                                        .on_hover_text("Remove this build from your local playbook")
                                        .clicked()
                                    {
                                        self.delete_build_id = Some(build.id.clone());
                                    }
                                },
                            );
                        });
                        if self.delete_build_id.as_deref() == Some(&build.id) {
                            ui.separator();
                            ui.label(
                                RichText::new(format!(
                                    "Delete ‘{}’ from your playbook?",
                                    build.name
                                ))
                                .color(GOLD),
                            );
                            ui.label(
                                RichText::new("League rune pages and item sets are unaffected.")
                                    .size(11.0)
                                    .color(MUTED),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Cancel").clicked() {
                                    self.delete_build_id = None;
                                }
                                if ui
                                    .button(
                                        RichText::new("Delete build")
                                            .color(Color32::from_rgb(236, 151, 165)),
                                    )
                                    .clicked()
                                {
                                    self.delete_build(&build.id);
                                }
                            });
                        }
                    });
                }
            });
            ui.add_space(10.0);
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Make yourself at home.").size(30.0).strong());
        ui.label(RichText::new("Connections, local data and a few useful details.").color(MUTED));
        ui.add_space(24.0);
        card().show(ui,|ui|{
            ui.set_width(ui.available_width());
            design::section(ui,"League connection","On this computer");
            ui.label(RichText::new("Open League and Lightrift will find it automatically. If it cannot connect, set the installation folder below.").color(MUTED).size(13.0));
            ui.add_space(8.0);
            label(ui,"INSTALLATION FOLDER OR LOCKFILE");
            ui.add(egui::TextEdit::singleline(&mut self.settings.league_path).margin(egui::vec2(12.0,10.0)).hint_text("Automatic discovery").desired_width(f32::INFINITY));
            ui.horizontal(|ui|{if ui.add(design::primary("Save settings")).clicked(){self.persist();}
if ui.add_enabled(!self.pending,egui::Button::new("Test connection")).clicked(){self.refresh();}ui.label(RichText::new(&self.connection).size(12.0).color(MUTED));});
        });
        ui.add_space(12.0);
        ui.columns(2, |cols| {
            card().show(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                design::section(ui, "Built to stay light", "");
                ui.label(
                    RichText::new("Native Rust. No browser runtime. No background cloud service.")
                        .color(MUTED),
                );
                ui.add_space(12.0);
                design::metric(ui, PATCH.trim(), "BUNDLED GAME PATCH", ACCENT);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!(
                        "{} champions · Images and catalogs available offline",
                        self.catalog.champions.len()
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
            });
            card().show(&mut cols[1], |ui| {
                ui.set_width(ui.available_width());
                design::section(ui, "Your builds stay yours", "");
                ui.label(
                    RichText::new(
                        "Saved on this computer. No account or Riot developer key required.",
                    )
                    .color(MUTED),
                );
                ui.add_space(12.0);
                design::metric(
                    ui,
                    &self.settings.builds.len().to_string(),
                    "SAVED BUILDS",
                    MINT,
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!(
                        "Back up your data folder to keep your playbook:\n{}",
                        data_dir().display()
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
            });
        });
        ui.add_space(16.0);
        ui.collapsing("Data sources & connection details",|ui|{
            ui.label("Recommendations use OP.GG's public service and champion pages, cached for six hours. Ability details use Riot Data Dragon. Samples and source regions remain labeled.");
            ui.label("Automatic overlays check League every two seconds. With overlays paused, following checks every two seconds in draft and five seconds otherwise. Credentials stay in memory; local HTTPS uses 127.0.0.1 with certificate verification.");
            ui.label("League imports use Riot's unsupported local client interface and may need updates after a patch.");
            ui.hyperlink_to("OP.GG data service","https://github.com/opgginc/opgg-mcp");
            ui.hyperlink_to("Riot Data Dragon","https://developer.riotgames.com/docs/lol#data-dragon");
        });
        ui.add_space(16.0);
        ui.label(
            RichText::new("LIGHTRIFT / 1.0.1")
                .size(13.0)
                .strong()
                .color(MINT),
        );
        ui.label(RichText::new("Lightrift is not endorsed by Riot Games and does not reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.").size(11.0).color(MUTED));
    }
}
impl eframe::App for Lightrift {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        while let Ok(action) = self.hotkeys.events.try_recv() {
            match action {
                crate::hotkeys::Action::Toggle => self.toggle_overlays(),
                crate::hotkeys::Action::Lock => {
                    if self.settings.overlay.enabled {
                        self.overlay_locked = !self.overlay_locked;
                    }
                }
            }
        }
        self.process();
        self.navigation(ctx);
        if self.watch || self.settings.overlay.enabled || self.page == Page::Live {
            let interval = if self.settings.overlay.enabled
                || self
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| matches!(s.phase.as_str(), "ChampSelect" | "InProgress"))
            {
                2
            } else {
                5
            };
            if self.last_poll.elapsed() >= Duration::from_secs(interval) {
                self.refresh();
            }
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        if self.page == Page::Champions {
            self.champion_browser(ctx);
        } else {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(BG).inner_margin(28))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                        Page::Builds => self.builds_page(ui),
                        Page::Live => self.live_page(ui),
                        Page::Settings => self.settings_page(ui),
                        Page::Overlays => self.overlay_settings(ui),
                        _ => {}
                    });
                });
        }
        if self.show_item_library {
            let mut open = true;
            egui::Window::new("Add to your build")
                .open(&mut open)
                .default_width(600.0)
                .collapsible(false)
                .show(ctx, |ui| self.item_library(ui));
            self.show_item_library = open;
        }
        if self.show_export {
            let mut open = true;
            egui::Window::new("Build JSON")
                .open(&mut open)
                .default_width(580.0)
                .show(ctx, |ui| {
                    ui.label("Copy this build for your own records.");
                    if ui.button("Copy JSON").clicked() {
                        ctx.copy_text(self.export_text.clone());
                    }
                    egui::ScrollArea::vertical()
                        .max_height(430.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.export_text)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });
            self.show_export = open;
        }
        self.show_overlay(ctx);
    }
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        if self.dirty {
            self.save_build();
        }
    }
}
