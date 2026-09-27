use super::*;

pub(super) fn phase_title(phase: &str) -> &str {
    match phase {
        "None" | "Lobby" => "In the lobby",
        "Matchmaking" => "Finding a match",
        "ReadyCheck" => "Match found",
        "ChampSelect" => "Champion select",
        "InProgress" => "In game",
        "GameStart" => "Game starting",
        "EndOfGame" | "PreEndOfGame" | "WaitingForStats" => "Game finished",
        "Reconnect" => "Reconnecting",
        _ => phase,
    }
}

pub(super) fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(BG).strong())
        .fill(MINT)
        .stroke(Stroke::NONE)
        .corner_radius(7)
}

pub(super) fn section(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(18.0).strong());
        if !detail.is_empty() {
            ui.label(RichText::new(detail).size(11.0).color(MUTED));
        }
    });
}

pub(super) fn metric(ui: &mut egui::Ui, value: &str, caption: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(value).size(25.0).strong().color(color));
        label(ui, caption);
    });
}

pub(super) fn nav(ui: &mut egui::Ui, index: usize, title: &str, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(66.0, 66.0), egui::Sense::click());
    let color = if active {
        MINT
    } else if response.hovered() {
        TEXT
    } else {
        MUTED
    };
    let p = ui.painter();
    if response.has_focus() {
        p.rect_stroke(
            rect.shrink(1.0),
            10,
            Stroke::new(1.5, MINT),
            egui::StrokeKind::Inside,
        );
    }
    if active || response.hovered() {
        p.rect_filled(
            rect.shrink(2.0),
            10,
            if active {
                Color32::from_rgb(22, 48, 48)
            } else {
                PANEL
            },
        );
    }
    if active {
        p.rect_filled(
            egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(0.0, 20.0),
                egui::vec2(3.0, 22.0),
            ),
            2,
            MINT,
        );
    }
    let c = rect.center_top() + egui::vec2(0.0, 23.0);
    let stroke = Stroke::new(1.5, color);
    match index {
        0 => {
            for x in [-7.0, 5.0] {
                for y in [-7.0, 5.0] {
                    p.rect_stroke(
                        egui::Rect::from_min_size(c + egui::vec2(x, y), egui::vec2(8.0, 8.0)),
                        2,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        1 => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(19.0, 21.0)),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            for y in [-4.0, 2.0, 7.0] {
                p.line_segment([c + egui::vec2(-4.0, y), c + egui::vec2(5.0, y)], stroke);
            }
        }
        2 => {
            for x in [-7.0, 7.0] {
                p.circle_stroke(c + egui::vec2(x, -5.0), 3.5, stroke);
                p.line_segment(
                    [c + egui::vec2(x - 4.0, 5.0), c + egui::vec2(x + 4.0, 5.0)],
                    stroke,
                );
            }
            p.line_segment(
                [c + egui::vec2(0.0, -10.0), c + egui::vec2(0.0, 10.0)],
                Stroke::new(1.0, LINE),
            );
        }
        3 => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(22.0, 17.0)),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            p.rect_filled(
                egui::Rect::from_min_size(c + egui::vec2(1.0, 0.0), egui::vec2(13.0, 10.0)),
                2,
                BG,
            );
            p.rect_stroke(
                egui::Rect::from_min_size(c + egui::vec2(1.0, 0.0), egui::vec2(13.0, 10.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        _ => {
            for (y, x) in [(-6.0, -4.0), (1.0, 5.0), (8.0, -1.0)] {
                p.line_segment([c + egui::vec2(-10.0, y), c + egui::vec2(10.0, y)], stroke);
                p.circle_filled(c + egui::vec2(x, y), 3.0, BG);
                p.circle_stroke(c + egui::vec2(x, y), 3.0, stroke);
            }
        }
    }
    p.text(
        rect.center_bottom() - egui::vec2(0.0, 13.0),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(10.0),
        color,
    );
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, title));
    response.on_hover_text(title)
}

pub(super) fn skills(ui: &mut egui::Ui, skills: &[&str], compact: bool) {
    let side = if compact { 25.0 } else { 30.0 };
    let count = ((ui.available_width() + 4.0) / (side + 4.0))
        .floor()
        .max(1.0) as usize;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 5.0);
        for (row, chunk) in skills.chunks(count).enumerate() {
            ui.horizontal(|ui| {
                for (col, key) in chunk.iter().enumerate() {
                    let (r, response) =
                        ui.allocate_exact_size(egui::vec2(side, side + 15.0), egui::Sense::hover());
                    let color = match *key {
                        "Q" => MINT,
                        "W" => ACCENT,
                        "E" => Color32::from_rgb(122, 180, 234),
                        _ => GOLD,
                    };
                    let tile = egui::Rect::from_min_size(
                        r.min + egui::vec2(0.0, 15.0),
                        egui::Vec2::splat(side),
                    );
                    ui.painter()
                        .rect_filled(tile, 5, color.gamma_multiply(0.13));
                    ui.painter().text(
                        r.center_top() + egui::vec2(0.0, 5.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", row * count + col + 1),
                        egui::FontId::proportional(9.0),
                        MUTED,
                    );
                    ui.painter().text(
                        tile.center(),
                        egui::Align2::CENTER_CENTER,
                        key,
                        egui::FontId::proportional(14.0),
                        color,
                    );
                    response.on_hover_text(format!("Level {}: {}", row * count + col + 1, key));
                }
            });
        }
    });
}
