use super::*;

impl Lightrift {
    pub(super) fn live_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Live").size(30.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut self.draft_preview, "Practice draft");
            });
        });
        ui.label(RichText::new("Both teams. From the first ban to the final score.").color(MUTED));
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            pill(
                ui,
                if self.snapshot.is_some() {
                    "CONNECTED"
                } else {
                    "WAITING FOR LEAGUE"
                },
                if self.snapshot.is_some() { MINT } else { GOLD },
            );
            ui.checkbox(&mut self.watch, "Follow in background");
            if ui
                .add_enabled(!self.pending, egui::Button::new("Refresh"))
                .clicked()
            {
                self.refresh();
            }
            ui.label(RichText::new(&self.connection).size(12.0).color(MUTED))
                .on_hover_text(
                    self.snapshot
                        .as_ref()
                        .map_or("Not connected", |s| s.display_name.as_str()),
                );
        });
        ui.checkbox(
            &mut self.auto_open,
            "Follow my champion’s build when my pick changes",
        );
        ui.add_space(12.0);
        if self.draft_preview {
            ui.horizontal(|ui| {
                pill(ui, "PRACTICE · LOCAL ONLY", GOLD);
                if ui.button("Clear draft").clicked() {
                    self.manual_team = Default::default();
                    self.manual_bans = Default::default();
                }
            });
            self.live_draft(ui, &Value::Null, true);
            return;
        }
        let fresh = self
            .snapshot_at
            .is_some_and(|t| t.elapsed() <= Duration::from_secs(10));
        let snapshot = self.snapshot.clone();
        if fresh && let Some(s) = snapshot {
            if s.phase == "ChampSelect" {
                pill(ui, "CHAMPION SELECT", MINT);
                self.live_draft(ui, &s.draft, false);
                return;
            }
            if s.phase == "InProgress" {
                if let Some(game) = s.game {
                    self.live_scoreboard(ui, &game);
                } else {
                    self.live_waiting(
                        ui,
                        "Connecting to your game",
                        if s.game_error.is_empty() {
                            "Waiting for the in-game player list…"
                        } else {
                            &s.game_error
                        },
                    );
                }
                return;
            }
        }
        self.live_waiting(ui, "Ready when you are", "Picks and bans appear in champion select. The live scoreboard appears once your game starts.");
    }

    fn live_waiting(&self, ui: &mut egui::Ui, title: &str, body: &str) {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width()); ui.add_space(24.0);
            ui.heading(title); ui.label(RichText::new(body).color(MUTED));
            ui.add_space(12.0);
            ui.label("Live updates while this screen is open. Enable Follow in background to keep updates running on other screens.");
            ui.add_space(24.0);
        });
    }

    fn practice_choice(&mut self, ui: &mut egui::Ui, index: usize, ban: bool) {
        let selected = if ban {
            self.manual_bans[index].clone()
        } else {
            self.manual_team[index].clone()
        };
        let available: Vec<_> = self
            .catalog
            .champions
            .iter()
            .filter(|c| {
                c.id == selected
                    || (!self.manual_team.contains(&c.id) && !self.manual_bans.contains(&c.id))
            })
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        let target = if ban {
            &mut self.manual_bans[index]
        } else {
            &mut self.manual_team[index]
        };
        egui::ComboBox::from_id_salt(("practice-choice", ban, index))
            .selected_text(if selected.is_empty() {
                "Choose"
            } else {
                "Change"
            })
            .width(if ban { 58.0 } else { 78.0 })
            .height(260.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(target, String::new(), "Empty");
                for (id, name) in available {
                    ui.selectable_value(target, id, name);
                }
            });
    }

    fn live_draft(&mut self, ui: &mut egui::Ui, draft: &Value, practice: bool) {
        ui.add_space(8.0);
        ui.columns(2, |cols| {
            for (team, title) in ["Your team", "Opponent team"].iter().enumerate() {
                card().inner_margin(12).show(&mut cols[team], |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(*title)
                            .size(20.0)
                            .strong()
                            .color(if team == 0 {
                                MINT
                            } else {
                                Color32::from_rgb(236, 151, 165)
                            }),
                    );
                    label(ui, "BANS");
                    ui.horizontal_wrapped(|ui| {
                        for pos in 0..5 {
                            let index = team * 5 + pos;
                            let id = if practice {
                                self.manual_bans[index].clone()
                            } else {
                                self.catalog
                                    .by_key(
                                        draft["bans"][if team == 0 {
                                            "myTeamBans"
                                        } else {
                                            "theirTeamBans"
                                        }][pos]
                                            .as_i64()
                                            .unwrap_or(0),
                                    )
                                    .map(|c| c.id.clone())
                                    .unwrap_or_default()
                            };
                            ui.vertical(|ui| {
                                if let Some(c) = self.catalog.champion(&id) {
                                    self.icon(ui, &c.image, 34.0)
                                        .on_hover_text(format!("Banned: {}", c.name));
                                } else {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(34.0, 34.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 6, BG);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "—",
                                        egui::FontId::proportional(13.0),
                                        MUTED,
                                    );
                                }
                                if practice {
                                    self.practice_choice(ui, index, true);
                                }
                            });
                        }
                    });
                    ui.add_space(8.0);
                    label(ui, "PICKS");
                    for pos in 0..5 {
                        let index = team * 5 + pos;
                        let entry = &draft[if team == 0 { "myTeam" } else { "theirTeam" }][pos];
                        let locked = entry["championId"].as_i64().unwrap_or(0) > 0;
                        let id = if practice {
                            self.manual_team[index].clone()
                        } else {
                            self.catalog
                                .by_key(
                                    entry["championId"]
                                        .as_i64()
                                        .filter(|i| *i > 0)
                                        .or_else(|| entry["championPickIntent"].as_i64())
                                        .unwrap_or(0),
                                )
                                .map(|c| c.id.clone())
                                .unwrap_or_default()
                        };
                        ui.horizontal(|ui| {
                            if let Some(c) = self.catalog.champion(&id) {
                                self.icon(ui, &c.image, 42.0);
                                if ui.selectable_label(false, &c.name).clicked() {
                                    self.choose(c.id.clone());
                                    self.page = Page::Champions;
                                }
                            } else {
                                ui.add_sized(
                                    [42.0, 42.0],
                                    egui::Label::new(
                                        RichText::new(format!("{:02}", pos + 1)).color(MUTED),
                                    ),
                                );
                                ui.label(RichText::new("Awaiting pick").color(MUTED));
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if practice {
                                        self.practice_choice(ui, index, false);
                                    } else {
                                        ui.label(
                                            RichText::new(if locked {
                                                "Picked"
                                            } else if !id.is_empty() {
                                                "Hover"
                                            } else {
                                                ""
                                            })
                                            .size(11.0)
                                            .color(MUTED),
                                        );
                                        ui.label(
                                            RichText::new(
                                                entry["assignedPosition"].as_str().unwrap_or(""),
                                            )
                                            .size(11.0)
                                            .color(MUTED),
                                        );
                                    }
                                },
                            );
                        });
                        if pos < 4 {
                            ui.separator();
                        }
                    }
                });
            }
        });
        ui.add_space(10.0);
        ui.label(RichText::new(if practice { "Plan picks and bans here. Champions already picked or banned are excluded from other slots." } else { "Only picks, hovers and bans visible in your League client are shown." }).size(12.0).color(MUTED));
    }

    fn live_scoreboard(&self, ui: &mut egui::Ui, game: &crate::live::Game) {
        ui.horizontal_wrapped(|ui| {
            pill(ui, "IN GAME", MINT);
            let seconds = game.seconds as u64;
            ui.strong(format!("{}:{:02}", seconds / 60, seconds % 60));
            ui.label(
                RichText::new(format!(
                    "{} players · updates every 2 seconds",
                    game.players.len()
                ))
                .color(MUTED),
            );
        });
        ui.add_space(10.0);
        ui.columns(2, |cols| {
            for (team, key) in ["ORDER", "CHAOS"].iter().enumerate() {
                let players: Vec<_> = game.players.iter().filter(|p| &p.team == key).collect();
                let kills: u64 = players.iter().map(|p| p.kills).sum();
                cols[team].horizontal(|ui| {
                    ui.label(
                        RichText::new(if team == 0 { "Blue team" } else { "Red team" })
                            .size(20.0)
                            .strong()
                            .color(if team == 0 {
                                MINT
                            } else {
                                Color32::from_rgb(236, 151, 165)
                            }),
                    );
                    ui.label(RichText::new(format!("{kills} kills")).color(MUTED));
                });
                for player in players {
                    self.live_player(&mut cols[team], player);
                    cols[team].add_space(6.0);
                }
            }
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Item gold adds the value of held items and components, including stacks. Pocket gold is excluded. Full runes are available for your player; others show their keystone and rune trees.").size(11.0).color(MUTED));
    }

    fn live_player(&self, ui: &mut egui::Ui, p: &crate::live::Player) {
        card().inner_margin(10).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(champ) = self
                    .catalog
                    .champion(&p.champion)
                    .or_else(|| self.catalog.champions.iter().find(|c| c.name == p.champion))
                {
                    self.icon(ui, &champ.image, 34.0).on_hover_text(&champ.name);
                }
                ui.vertical(|ui| {
                    ui.add(egui::Label::new(RichText::new(&p.name).strong()).truncate())
                        .on_hover_text(&p.name);
                    ui.label(
                        RichText::new(format!(
                            "{} · Lv {}{}",
                            p.champion,
                            p.level,
                            if p.dead { " · Dead" } else { "" }
                        ))
                        .size(11.0)
                        .color(MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.strong(format!("{} / {} / {}", p.kills, p.deaths, p.assists));
                        ui.label(
                            RichText::new(format!("{} CS", p.cs))
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
            });
            ui.horizontal_wrapped(|ui| {
                for (id, count) in &p.items {
                    if let Some(item) = self.catalog.items.get(id) {
                        self.icon(ui, &item.image, 25.0)
                            .on_hover_text(format!("{} ×{}", item.name, count));
                    } else {
                        ui.label(format!("#{id}"))
                            .on_hover_text("Item not in bundled catalog");
                    }
                }
                if p.items.is_empty() {
                    ui.label(RichText::new("No items").size(11.0).color(MUTED));
                }
                ui.separator();
                for id in &p.runes {
                    if let Some(texture) = self.textures.get(&format!("rune{id}")) {
                        let name = self
                            .catalog
                            .runes
                            .iter()
                            .flat_map(|t| &t.slots)
                            .flat_map(|s| &s.runes)
                            .find(|r| r.id == *id)
                            .map_or("Rune", |r| r.name.as_str());
                        ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(23.0, 23.0)))
                            .on_hover_text(name);
                    } else {
                        let name = SHARDS
                            .iter()
                            .flat_map(|row| row.iter())
                            .find(|(n, _)| n == id)
                            .map_or("Rune", |(_, name)| *name);
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(14.0, 23.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 3.5, ACCENT);
                        response.on_hover_text(name);
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} / {}{}",
                        p.primary,
                        p.secondary,
                        if p.full_runes { " · Full runes" } else { "" }
                    ))
                    .size(11.0)
                    .color(ACCENT),
                );
                let gold = p.item_gold(&self.catalog);
                ui.label(RichText::new(gold.map_or_else(|| "Item gold —".into(), |g| format!("Item gold {g} g"))).size(11.0).color(GOLD))
                    .on_hover_text(if gold.is_some() { "Sum of held item prices × stack counts. Completed items include their recipe cost. No pocket gold." } else { "An inventory item is missing its price in the bundled catalog; the total is unavailable." });
            });
        });
    }
}
