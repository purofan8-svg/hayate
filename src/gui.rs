//! GUI в стиле Windows 11: тёмная тема, скругления, Segoe UI, чипы фильтров.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use eframe::egui;
use egui::{
    Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, Margin, Rect,
    RichText, Rounding, Sense, Stroke, pos2, vec2,
};

use crate::index::{Filter, Index, Kind};

// Windows 11 dark palette
const BG: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1f);
const SURFACE: Color32 = Color32::from_rgb(0x25, 0x25, 0x2b);
const SURFACE_HI: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x36);
const HOVER: Color32 = Color32::from_rgb(0x33, 0x33, 0x3b);
const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf5);
const DIM: Color32 = Color32::from_rgb(0x9a, 0x9a, 0xa6);
const ACCENT: Color32 = Color32::from_rgb(0x4c, 0xc2, 0xff);
const ACCENT_SOFT: Color32 = Color32::from_rgb(0x1e, 0x3a, 0x4d);
const ROW_H: f32 = 38.0;

pub fn run() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([700.0, 420.0])
            .with_title("hayate"),
        vsync: true,
        ..Default::default()
    };
    eframe::run_native("hayate", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

type LoadRx = Receiver<Result<Arc<Index>, String>>;

enum Load {
    Loading(LoadRx),
    Ready(Arc<Index>),
    Failed(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    All,
    Dirs,
    Files,
    Kind(Kind),
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::All => "Всё",
            Mode::Dirs => "Папки",
            Mode::Files => "Файлы",
            Mode::Kind(k) => k.name(),
        }
    }

    fn chip_label(self) -> String {
        match self {
            Mode::All => "Всё".into(),
            Mode::Dirs => "Папки".into(),
            Mode::Files => "Файлы".into(),
            Mode::Kind(k) => {
                let n = k.name();
                let mut c = n.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            }
        }
    }

    fn filter(self) -> Filter {
        match self {
            Mode::All => Filter::default(),
            Mode::Dirs => Filter { dirs_only: true, ..Default::default() },
            Mode::Files => Filter { files_only: true, ..Default::default() },
            Mode::Kind(k) => Filter { files_only: true, kinds: Some(k.bit()), ..Default::default() },
        }
    }

    fn all() -> Vec<Mode> {
        let mut v = vec![Mode::All, Mode::Dirs, Mode::Files];
        v.extend(Kind::ALL.iter().map(|&k| Mode::Kind(k)));
        v
    }
}

struct App {
    load: Load,
    query: String,
    mode: Mode,
    results: Vec<usize>,
    sel: usize,
    took_ms: f64,
    dirty: bool,
    scroll_to_sel: bool,
    scanned: Option<std::process::Child>,
    titlebar_done: bool,
}

const MAX_ROWS: usize = 200;

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        install_fonts(&cc.egui_ctx);
        let mut v = egui::Visuals::dark();
        v.panel_fill = BG;
        v.window_fill = BG;
        v.extreme_bg_color = SURFACE;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = ACCENT_SOFT;
        cc.egui_ctx.set_visuals(v);
        cc.egui_ctx.style_mut(|s| {
            s.spacing.item_spacing = vec2(8.0, 8.0);
            s.spacing.button_padding = vec2(10.0, 5.0);
        });

        let load = match crate::index_path().exists() {
            true => Load::Loading(spawn_load()),
            false => Load::Failed("Индекс ещё не построен".into()),
        };
        App {
            load,
            query: String::new(),
            mode: Mode::All,
            results: Vec::new(),
            sel: 0,
            took_ms: 0.0,
            dirty: true,
            scroll_to_sel: false,
            scanned: None,
            titlebar_done: false,
        }
    }

    fn refresh(&mut self) {
        let Load::Ready(ix) = &self.load else { return };
        let r = ix.search(&self.query, MAX_ROWS, &self.mode.filter());
        self.results = r.hits;
        self.took_ms = r.took.as_secs_f64() * 1000.0;
        if self.sel >= self.results.len() {
            self.sel = 0;
        }
        self.dirty = false;
    }

    fn ix(&self) -> Option<&Arc<Index>> {
        match &self.load {
            Load::Ready(ix) => Some(ix),
            _ => None,
        }
    }

    fn open_selected(&self) {
        let Some(ix) = self.ix() else { return };
        if let Some(&i) = self.results.get(self.sel) {
            open_path(ix.path_at(i));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.titlebar_done {
            set_dark_titlebar(frame);
            self.titlebar_done = true;
        }

        // подхватываем результат фоновой загрузки
        if let Load::Loading(rx) = &self.load {
            match rx.try_recv() {
                Ok(Ok(ix)) => {
                    self.load = Load::Ready(ix);
                    self.dirty = true;
                }
                Ok(Err(e)) => self.load = Load::Failed(e),
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(40)),
                Err(_) => self.load = Load::Failed("поток загрузки упал".into()),
            }
        }

        // скан из GUI
        if let Some(child) = &mut self.scanned {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.scanned = None;
                    self.load = Load::Loading(spawn_load());
                }
                Ok(None) => ctx.request_repaint_after(Duration::from_millis(400)),
                Err(_) => self.scanned = None,
            }
        }

        if self.dirty {
            self.refresh();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                self.header(ui, ctx);
                ui.add_space(12.0);
                let failed = match &self.load { Load::Failed(m) => Some(m.clone()), _ => None };
                let ready = match &self.load { Load::Ready(ix) => Some(ix.clone()), _ => None };
                if let Some(msg) = failed {
                    self.empty_state(ui, &msg);
                } else if let Some(ix) = ready {
                    self.chips(ui);
                    ui.add_space(10.0);
                    self.list(ui, &ix);
                } else {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Загружаю индекс…").size(15.0).color(DIM));
                    });
                }
                self.status(ui);
            });

        keys(self, ctx);
    }
}

impl App {
    fn header(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("hayate").size(20.0).strong().color(TEXT));
            ui.label(RichText::new("поиск файлов").size(12.0).color(DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(ix) = self.ix() {
                    let txt = format!("{} записей", fmt_num(ix.len()));
                    ui.label(RichText::new(txt).size(12.0).color(DIM));
                }
            });
        });
        ui.add_space(10.0);

        // строка поиска
        egui::Frame::none()
            .fill(SURFACE)
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::symmetric(14.0, 9.0))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x38, 0x42)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 22.0), Sense::hover());
                    let c = rect.center() + vec2(-2.0, -2.0);
                    let p = ui.painter();
                    p.circle_stroke(c, 6.0, Stroke::new(1.7_f32, DIM));
                    p.line_segment([c + vec2(4.4, 4.4), c + vec2(8.4, 8.4)], Stroke::new(1.7_f32, DIM));

                    let before = self.query.clone();
                    let te = egui::TextEdit::singleline(&mut self.query)
                        .frame(false)
                        .hint_text(RichText::new("Поиск файлов и папок…").color(DIM))
                        .font(FontId::proportional(16.0))
                        .text_color(TEXT)
                        .desired_width(f32::INFINITY)
                        .margin(Margin::ZERO);
                    let resp = ui.add(te);
                    if self.query != before {
                        self.dirty = true;
                        self.sel = 0;
                        self.scroll_to_sel = true;
                    }
                    if !resp.has_focus() {
                        resp.request_focus();
                    }
                });
            });
    }

    fn chips(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.horizontal_wrapped(|ui| {
            for m in Mode::all() {
                if chip(ui, &m.chip_label(), m == self.mode).clicked() {
                    self.mode = m;
                    self.sel = 0;
                    self.dirty = true;
                    self.scroll_to_sel = true;
                }
            }
        });
    }

    fn list(&mut self, ui: &mut egui::Ui, ix: &Arc<Index>) {
        if self.results.is_empty() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                let msg = if self.query.is_empty() && self.mode == Mode::All {
                    "Начни печатать — или выбери тип сверху"
                } else {
                    "Ничего не нашлось"
                };
                ui.label(RichText::new(msg).size(14.0).color(DIM));
            });
            return;
        }

        let mut clicked: Option<usize> = None;
        let mut opened: Option<usize> = None;
        let mut moved = false;

        let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        area.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (row, &idx) in self.results.iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                let selected = row == self.sel;
                draw_row(ui, &resp, rect, ix, idx, selected);
                if resp.clicked() {
                    clicked = Some(row);
                }
                if resp.double_clicked() {
                    opened = Some(row);
                }
                if selected && self.scroll_to_sel {
                    ui.scroll_to_rect(rect, Some(Align::Center));
                    moved = true;
                }
            }
        });
        if moved {
            self.scroll_to_sel = false;
        }
        if let Some(r) = clicked {
            self.sel = r;
        }
        if let Some(r) = opened {
            self.sel = r;
            self.open_selected();
        }
    }

    fn empty_state(&mut self, ui: &mut egui::Ui, msg: &str) {
        ui.add_space(50.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Индекса нет").size(18.0).color(TEXT));
            ui.add_space(6.0);
            ui.label(RichText::new(msg).size(13.0).color(DIM));
            ui.add_space(16.0);
            let scanning = self.scanned.is_some();
            let btn = egui::Button::new(
                RichText::new(if scanning { "Сканирую…" } else { "Построить индекс" })
                    .size(14.0)
                    .color(if scanning { DIM } else { BG }),
            )
            .fill(if scanning { SURFACE_HI } else { ACCENT })
            .rounding(Rounding::same(8.0))
            .min_size(vec2(200.0, 36.0));
            if ui.add_enabled(!scanning, btn).clicked() {
                if let Ok(exe) = std::env::current_exe() {
                    self.scanned = std::process::Command::new(exe).arg("index").spawn().ok();
                }
            }
        });
    }

    fn status(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if let Load::Ready(_) = self.load {
                    let txt = format!("{} {} · {} мс", self.results.len(), plural(self.results.len()), fmt_ms(self.took_ms));
                    ui.label(RichText::new(txt).size(11.5).color(DIM));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Enter — открыть · ↑↓ — выбор · Esc — очистить").size(11.5).color(DIM));
                    });
                } else if self.scanned.is_some() {
                    ui.label(RichText::new("Сканирую диски, это может занять пару минут…").size(11.5).color(DIM));
                }
            });
        });
    }
}

fn keys(app: &mut App, ctx: &egui::Context) {
    if app.results.is_empty() {
        return;
    }
    let (down, up, enter, esc) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        )
    });
    if down && app.sel + 1 < app.results.len() {
        app.sel += 1;
        app.scroll_to_sel = true;
    }
    if up && app.sel > 0 {
        app.sel -= 1;
        app.scroll_to_sel = true;
    }
    if enter {
        app.open_selected();
    }
    if esc {
        app.query.clear();
        app.dirty = true;
    }
}

fn chip(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let font = FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), TEXT);
    let size = vec2(galley.rect.width() + 22.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    let (fill, stroke, color) = if selected {
        (ACCENT_SOFT, Stroke::new(1.0_f32, Color32::from_rgb(0x2f, 0x7f, 0xa8)), ACCENT)
    } else if hovered {
        (HOVER, Stroke::NONE, TEXT)
    } else {
        (Color32::TRANSPARENT, Stroke::NONE, DIM)
    };
    let p = ui.painter();
    p.rect(rect, Rounding::same(9.0), fill, stroke);
    let g = p.layout_no_wrap(label.to_string(), font, color);
    p.galley(rect.center() - g.rect.size() / 2.0, g, color);
    resp
}

fn draw_row(ui: &egui::Ui, resp: &egui::Response, rect: Rect, ix: &Index, idx: usize, selected: bool) {
    let p = ui.painter();
    let hovered = resp.hovered();
    if selected {
        p.rect_filled(rect, Rounding::same(8.0), ACCENT_SOFT);
    } else if hovered {
        p.rect_filled(rect, Rounding::same(8.0), HOVER);
    }

    let is_dir = ix.is_dir(idx);
    let kind = ix.kind_at(idx);
    let (tag, color) = if is_dir { ("папка", ACCENT) } else { (kind.name(), kind_color(kind)) };

    // пилюля типа
    let tag_rect = Rect::from_min_size(rect.left_top() + vec2(8.0, 9.0), vec2(86.0, 20.0));
    p.rect_filled(tag_rect, Rounding::same(6.0), color.gamma_multiply(0.16));
    let tg = p.layout_no_wrap(tag.to_string(), FontId::proportional(11.5), color);
    p.galley(tag_rect.center() - tg.rect.size() / 2.0, tg, color);

    let right = rect.right() - 10.0;
    let size_txt = if is_dir { "—".to_string() } else { human(ix.size_at(idx)) };
    let sg = p.layout_no_wrap(size_txt, FontId::proportional(11.5), DIM);
    let size_left = right - sg.rect.width();
    p.galley(pos2(size_left, rect.center().y - sg.rect.height() / 2.0), sg, DIM);

    // путь: каталог тускло, имя ярко, лишнее режется
    let path = ix.path_at(idx);
    let (dir, name) = match path.rfind(['\\', '/']) {
        Some(k) => (&path[..=k], &path[k + 1..]),
        None => ("", path),
    };
    let text_left = tag_rect.right() + 10.0;
    let avail = (size_left - 8.0) - text_left;
    let name_font = FontId::proportional(13.5);
    let ng = p.layout_no_wrap(name.to_string(), name_font.clone(), TEXT);
    let name_w = ng.rect.width();

    if name_w >= avail {
        let cut = truncate_right(name, avail, name_w);
        let g = p.layout_no_wrap(cut, name_font, TEXT);
        p.galley(pos2(text_left, rect.center().y - g.rect.height() / 2.0), g, TEXT);
    } else {
        let dir_space = avail - name_w;
        let dg = p.layout_no_wrap(dir.to_string(), FontId::proportional(12.5), DIM);
        let mut dir_w = dg.rect.width();
        if dir_w <= dir_space {
            p.galley(pos2(text_left, rect.center().y - dg.rect.height() / 2.0), dg, DIM);
        } else {
            let cut = truncate_left(dir, dir_space, dir_w);
            let g = p.layout_no_wrap(cut, FontId::proportional(12.5), DIM);
            dir_w = g.rect.width().min(dir_space);
            p.galley(pos2(text_left, rect.center().y - g.rect.height() / 2.0), g, DIM);
        }
        p.galley(pos2(text_left + dir_w, rect.center().y - ng.rect.height() / 2.0), ng, TEXT);
    }
}

fn truncate_right(text: &str, max_w: f32, full_w: f32) -> String {
    if full_w <= max_w {
        return text.to_string();
    }
    let ratio = (max_w / full_w).clamp(0.05, 1.0);
    let keep = ((text.chars().count() as f32) * ratio) as usize;
    let mut out: String = text.chars().take(keep.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn truncate_left(text: &str, max_w: f32, full_w: f32) -> String {
    if full_w <= max_w {
        return text.to_string();
    }
    let ratio = (max_w / full_w).clamp(0.05, 1.0);
    let total = text.chars().count();
    let keep = ((total as f32) * ratio) as usize;
    let tail: String = text.chars().skip(total.saturating_sub(keep)).collect();
    format!("…{}", tail.trim_start_matches(['\\', '/']))
}

fn kind_color(k: Kind) -> Color32 {
    match k {
        Kind::Image => Color32::from_rgb(0xbe, 0x96, 0xff),
        Kind::Video => Color32::from_rgb(0xff, 0x78, 0x8c),
        Kind::Audio => Color32::from_rgb(0x78, 0xdc, 0xaa),
        Kind::Doc => Color32::from_rgb(0xeb, 0xc8, 0x78),
        Kind::Text => Color32::from_rgb(0xaa, 0xbe, 0xc8),
        Kind::Archive => Color32::from_rgb(0xe6, 0xa0, 0x5a),
        Kind::Code => Color32::from_rgb(0x82, 0xc8, 0xff),
        Kind::Exec => Color32::from_rgb(0xff, 0xa0, 0x5a),
        Kind::Font => Color32::from_rgb(0xc8, 0xb4, 0xff),
        Kind::Model => Color32::from_rgb(0x8c, 0xe6, 0xd2),
        Kind::Other => DIM,
    }
}

fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/SegUIVar.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ];
    let Some(bytes) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else { return };
    let mut defs = FontDefinitions::default();
    defs.font_data.insert("ui".to_owned(), FontData::from_owned(bytes).into());
    for fam in [FontFamily::Proportional, FontFamily::Monospace] {
        defs.families.entry(fam).or_default().insert(0, "ui".to_owned());
    }
    ctx.set_fonts(defs);
}

fn spawn_load() -> LoadRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let p = crate::index_path();
        let res = Index::load(&p).map(Arc::new).map_err(|e| e.to_string());
        let _ = tx.send(res);
    });
    rx
}

pub fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf())
}

fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    out
}

fn plural(n: usize) -> &'static str {
    let m100 = n % 100;
    let m10 = n % 10;
    if (11..=14).contains(&m100) {
        "результатов"
    } else if m10 == 1 {
        "результат"
    } else if (2..=4).contains(&m10) {
        "результата"
    } else {
        "результатов"
    }
}

fn fmt_ms(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

fn human(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut x = n as f64;
    let mut i = 0;
    while x >= 1024.0 && i < 4 {
        x /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{x:.1} {}", U[i]) }
}

#[cfg(windows)]
fn open_path(p: &str) {
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", p]).spawn();
}

#[cfg(not(windows))]
fn open_path(p: &str) {
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

#[cfg(windows)]
fn set_dark_titlebar(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
    let Ok(h) = frame.window_handle() else { return };
    if let RawWindowHandle::Win32(w) = h.as_raw() {
        let hwnd = w.hwnd.get() as *mut core::ffi::c_void;
        let on: i32 = 1;
        unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, &on as *const _ as *const _, 4);
        }
    }
}

#[cfg(not(windows))]
fn set_dark_titlebar(_frame: &eframe::Frame) {}