//! Настройки приложения и палитры тем.

use std::path::PathBuf;

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Accent {
    Blue,
    Teal,
    Green,
    Purple,
    Orange,
    Pink,
    Red,
}

impl Accent {
    pub const ALL: [Accent; 7] =
        [Accent::Blue, Accent::Teal, Accent::Green, Accent::Purple, Accent::Orange, Accent::Pink, Accent::Red];

    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Accent::Blue => (0x4c, 0xc2, 0xff),
            Accent::Teal => (0x2a, 0xd8, 0xc8),
            Accent::Green => (0x6c, 0xcf, 0x6c),
            Accent::Purple => (0xb9, 0x9c, 0xff),
            Accent::Orange => (0xff, 0xa5, 0x50),
            Accent::Pink => (0xff, 0x8c, 0xd0),
            Accent::Red => (0xff, 0x7a, 0x7a),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Accent::Blue => "Синий",
            Accent::Teal => "Бирюзовый",
            Accent::Green => "Зелёный",
            Accent::Purple => "Фиолетовый",
            Accent::Orange => "Оранжевый",
            Accent::Pink => "Розовый",
            Accent::Red => "Красный",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Density {
    Compact,
    Normal,
    Relaxed,
}

impl Density {
    pub fn row_height(self) -> f32 {
        match self {
            Density::Compact => 30.0,
            Density::Normal => 38.0,
            Density::Relaxed => 46.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sort {
    #[default]
    Relevance,
    Name,
    Size,
    Date,
}

impl Sort {
    pub fn name(self) -> &'static str {
        match self {
            Sort::Relevance => "По совпадению",
            Sort::Name => "По имени",
            Sort::Size => "По размеру",
            Sort::Date => "По дате",
        }
    }
    pub const ALL: [Sort; 4] = [Sort::Relevance, Sort::Name, Sort::Size, Sort::Date];
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Match {
    #[default]
    Contains,
    Prefix,
    Exact,
}

impl Match {
    pub fn name(self) -> &'static str {
        match self {
            Match::Contains => "Содержит",
            Match::Prefix => "С начала имени",
            Match::Exact => "Имя целиком",
        }
    }
    pub const ALL: [Match; 3] = [Match::Contains, Match::Prefix, Match::Exact];
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub accent: Accent,
    /// Непрозрачность окна, 0.55–1.0
    pub opacity: f32,
    /// Подложка Windows 11 (Mica)
    pub mica: bool,
    pub density: Density,
    /// Масштаб интерфейса, 0.85–1.3
    pub font_scale: f32,
    pub show_hidden: bool,
    pub show_sizes: bool,
    pub limit: usize,
    pub sort: Sort,
    pub match_mode: Match,
    /// Размер окна
    pub window: [f32; 2],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: Accent::Blue,
            opacity: 1.0,
            mica: false,
            density: Density::Normal,
            font_scale: 1.0,
            show_hidden: true,
            show_sizes: true,
            limit: 200,
            sort: Sort::Relevance,
            match_mode: Match::Contains,
            window: [1000.0, 680.0],
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string(path()) else {
            return Settings::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(dir) = path().parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path(), text);
        }
    }
}

pub fn path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("hayate").join("settings.json")
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_hi: Color32,
    pub hover: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub border: Color32,
    pub dark: bool,
}

fn rgba(rgb: (u8, u8, u8), a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(rgb.0, rgb.1, rgb.2, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn palette(s: &Settings) -> Palette {
    let (ar, ag, ab) = s.accent.rgb();
    let a = s.opacity.clamp(0.35, 1.0);
    match s.theme {
        Theme::Dark => Palette {
            bg: rgba((0x17, 0x17, 0x1b), a),
            surface: rgba((0x23, 0x23, 0x29), a),
            surface_hi: rgba((0x2d, 0x2d, 0x35), a),
            hover: rgba((0x33, 0x33, 0x3c), a),
            text: Color32::from_rgb(0xf2, 0xf2, 0xf5),
            dim: Color32::from_rgb(0x9a, 0x9a, 0xa6),
            accent: Color32::from_rgb(ar, ag, ab),
            accent_soft: rgba((ar, ag, ab), 0.20),
            border: rgba((0x3a, 0x3a, 0x45), a),
            dark: true,
        },
        Theme::Light => Palette {
            bg: rgba((0xf3, 0xf3, 0xf6), a),
            surface: rgba((0xff, 0xff, 0xff), a),
            surface_hi: rgba((0xe9, 0xe9, 0xee), a),
            hover: rgba((0xe2, 0xe2, 0xe8), a),
            text: Color32::from_rgb(0x1a, 0x1a, 0x1f),
            dim: Color32::from_rgb(0x4a, 0x4a, 0x56),
            accent: Color32::from_rgb(ar.min(220), ag.min(220), ab.min(220)),
            accent_soft: rgba((ar, ag, ab), 0.22),
            border: rgba((0xc9, 0xc9, 0xd2), a),
            dark: false,
        },
    }
}

// ── глобальное состояние оформления ────────────────────────────

pub const DARK_DEFAULT: Palette = Palette {
    bg: Color32::from_rgb(0x17, 0x17, 0x1b),
    surface: Color32::from_rgb(0x23, 0x23, 0x29),
    surface_hi: Color32::from_rgb(0x2d, 0x2d, 0x35),
    hover: Color32::from_rgb(0x33, 0x33, 0x3c),
    text: Color32::from_rgb(0xf2, 0xf2, 0xf5),
    dim: Color32::from_rgb(0x9a, 0x9a, 0xa6),
    accent: Color32::from_rgb(0x4c, 0xc2, 0xff),
    accent_soft: Color32::from_rgb(0x2a, 0x45, 0x58),
    border: Color32::from_rgb(0x3a, 0x3a, 0x45),
    dark: true,
};

static PAL: std::sync::RwLock<Palette> = std::sync::RwLock::new(DARK_DEFAULT);
static ROW_H: std::sync::RwLock<f32> = std::sync::RwLock::new(38.0);
static SIZES: std::sync::RwLock<bool> = std::sync::RwLock::new(true);

pub fn pal() -> Palette {
    *PAL.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set_pal(p: Palette) {
    *PAL.write().unwrap_or_else(|e| e.into_inner()) = p;
}

pub fn row_h() -> f32 {
    *ROW_H.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set_row_h(v: f32) {
    *ROW_H.write().unwrap_or_else(|e| e.into_inner()) = v;
}

pub fn show_sizes() -> bool {
    *SIZES.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set_show_sizes(v: bool) {
    *SIZES.write().unwrap_or_else(|e| e.into_inner()) = v;
}