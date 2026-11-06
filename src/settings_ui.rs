//! Окно настроек в стиле Windows 11.

use eframe::egui::{
    self, Align, Color32, FontId, Layout, Margin, RichText, Rounding, Sense, Stroke, pos2, vec2,
};

use crate::settings::{Accent, Density, Match, Settings, Sort, Theme, pal};

pub fn window(ctx: &egui::Context, st: &mut Settings, open: &mut bool) {
    let mut is_open = *open;
    egui::Window::new(RichText::new("Настройки").size(15.0).strong().color(pal().text))
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .default_width(400.0)
        .frame(
            egui::Frame::none()
                .fill(solid(pal().surface))
                .rounding(Rounding::same(14.0))
                .stroke(Stroke::new(1.0_f32, pal().border))
                .inner_margin(Margin::same(18.0)),
        )
        .show(ctx, |ui| body(ui, st));
    *open = is_open;
}

/// Цвет без альфы — панель настроек не должна просвечивать
fn solid(c: Color32) -> Color32 {
    Color32::from_rgb(c.r(), c.g(), c.b())
}

fn body(ui: &mut egui::Ui, st: &mut Settings) {
    ui.spacing_mut().item_spacing.y = 10.0;

    section(ui, "ОФОРМЛЕНИЕ");
    row(ui, "Тема", |ui| {
        segmented(ui, &mut st.theme, &[(Theme::Dark, "Тёмная"), (Theme::Light, "Светлая")]);
    });
    row(ui, "Акцент", |ui| {
        ui.horizontal(|ui| {
            for a in Accent::ALL {
                if swatch(ui, a, st.accent == a) {
                    st.accent = a;
                }
            }
        });
    });
    row(ui, "Прозрачность", |ui| {
        let mut pct = (st.opacity * 100.0).round();
        if ui.add(egui::Slider::new(&mut pct, 55.0..=100.0).suffix(" %").show_value(true)).changed() {
            st.opacity = pct / 100.0;
        }
    });
    row(ui, "Mica (Win11)", |ui| {
        toggle(ui, &mut st.mica);
    });

    section(ui, "СПИСОК");
    row(ui, "Плотность", |ui| {
        segmented(
            ui,
            &mut st.density,
            &[(Density::Compact, "Плотно"), (Density::Normal, "Обычно"), (Density::Relaxed, "Свободно")],
        );
    });
    row(ui, "Показывать скрытые", |ui| {
        toggle(ui, &mut st.show_hidden);
    });
    row(ui, "Показывать размер", |ui| {
        toggle(ui, &mut st.show_sizes);
    });
    row(ui, "Лимит результатов", |ui| {
        let mut lim = st.limit as f32;
        if ui.add(egui::Slider::new(&mut lim, 50.0..=2000.0).step_by(50.0).integer()).changed() {
            st.limit = lim as usize;
        }
    });
    row(ui, "Сортировка", |ui| {
        combo(ui, "sort", &mut st.sort, &Sort::ALL.map(|s| (s, s.name())));
    });

    section(ui, "ПОИСК");
    row(ui, "Совпадение", |ui| {
        combo(ui, "match", &mut st.match_mode, &Match::ALL.map(|m| (m, m.name())));
    });
    row(ui, "Масштаб интерфейса", |ui| {
        let mut sc = st.font_scale;
        if ui.add(egui::Slider::new(&mut sc, 0.85..=1.3).step_by(0.05)).changed() {
            st.font_scale = sc;
        }
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let reset = egui::Button::new(RichText::new("Сбросить").size(12.5).color(pal().text))
            .fill(pal().surface_hi)
            .rounding(Rounding::same(8.0))
            .min_size(vec2(110.0, 30.0));
        if ui.add(reset).clicked() {
            let window = st.window;
            *st = Settings::default();
            st.window = window;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(settings_path_hint()).size(10.5).color(pal().dim));
        });
    });
}

fn settings_path_hint() -> String {
    "сохраняется автоматически".to_string()
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).size(10.5).color(pal().dim));
}

fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(pal().text));
        ui.with_layout(Layout::right_to_left(Align::Center), add);
    });
}

fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, opts: &[(T, &str)]) -> bool {
    let mut changed = false;
    egui::Frame::none()
        .fill(pal().surface_hi)
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(2.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (v, label) in opts {
                    let sel = *value == *v;
                    let w = ui
                        .painter()
                        .layout_no_wrap(label.to_string(), FontId::proportional(12.0), pal().text)
                        .rect
                        .width()
                        + 18.0;
                    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                    if sel {
                        ui.painter().rect_filled(rect, Rounding::same(6.0), pal().surface);
                    }
                    let color = if sel { pal().text } else { pal().dim };
                    let g = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(12.0), color);
                    ui.painter().galley(rect.center() - g.rect.size() / 2.0, g, color);
                    if resp.clicked() && !sel {
                        *value = *v;
                        changed = true;
                    }
                }
            });
        });
    changed
}

fn toggle(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(38.0, 20.0), Sense::click());
    let t = ui.ctx().animate_bool(resp.id, *on);
    let bg = if *on { pal().accent } else { pal().surface_hi };
    ui.painter().rect_filled(rect, Rounding::same(10.0), bg);
    let cx = rect.left() + 10.0 + t * (rect.width() - 20.0);
    let knob = if pal().dark { Color32::from_rgb(0x14, 0x14, 0x18) } else { Color32::WHITE };
    ui.painter().circle_filled(pos2(cx, rect.center().y), 7.0, knob);
    if resp.clicked() {
        *on = !*on;
        return true;
    }
    false
}

fn swatch(ui: &mut egui::Ui, a: Accent, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    let (r, g, b) = a.rgb();
    let c = Color32::from_rgb(r, g, b);
    ui.painter().circle_filled(rect.center(), 8.0, c);
    if selected {
        ui.painter().circle_stroke(rect.center(), 10.0, Stroke::new(1.6_f32, pal().text));
    } else if resp.hovered() {
        ui.painter().circle_stroke(rect.center(), 10.0, Stroke::new(1.0_f32, pal().dim));
    }
    resp.on_hover_text(a.name()).clicked()
}

fn combo<T: PartialEq + Copy>(ui: &mut egui::Ui, id: &str, value: &mut T, opts: &[(T, &str)]) {
    let current = opts.iter().find(|(v, _)| v == value).map(|(_, l)| *l).unwrap_or("—");
    egui::ComboBox::from_id_salt(id)
        .selected_text(RichText::new(current).size(12.0).color(pal().text))
        .width(170.0)
        .show_ui(ui, |ui| {
            for (v, label) in opts {
                if ui.selectable_label(*value == *v, RichText::new(*label).size(12.0)).clicked() {
                    *value = *v;
                }
            }
        });
}