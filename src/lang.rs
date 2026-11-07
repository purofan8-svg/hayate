//! Языки интерфейса: русский, английский, китайский.
//! Строки лежат одной таблицей: [en, ru, zh].

use std::sync::RwLock;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    /// По системной локали
    #[default]
    Auto,
    Ru,
    En,
    Zh,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::Auto, Lang::Ru, Lang::En, Lang::Zh];

    pub fn label(self) -> &'static str {
        match self {
            Lang::Auto => t("Auto"),
            Lang::Ru => "Русский",
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }
}

static CUR: RwLock<Lang> = RwLock::new(Lang::Ru);

pub fn set(l: Lang) {
    let resolved = resolve(l);
    *CUR.write().unwrap_or_else(|e| e.into_inner()) = resolved;
}

pub fn current() -> Lang {
    *CUR.read().unwrap_or_else(|e| e.into_inner())
}

pub fn resolve(l: Lang) -> Lang {
    match l {
        Lang::Auto => detect(),
        other => other,
    }
}

/// Язык системы: ru / zh / остальное → английский.
fn detect() -> Lang {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
        let id = GetUserDefaultUILanguage() as u32;
        match id & 0x3ff {
            0x19 => Lang::Ru,
            0x04 => Lang::Zh,
            _ => Lang::En,
        }
    }
    #[cfg(not(windows))]
    {
        Lang::En
    }
}

fn idx() -> usize {
    match current() {
        Lang::Ru => 1,
        Lang::Zh => 2,
        _ => 0,
    }
}

/// Перевод по английскому ключу.
pub fn t(key: &'static str) -> &'static str {
    match row(key) {
        Some(r) => r[idx()],
        None => key,
    }
}

fn row(key: &str) -> Option<[&'static str; 3]> {
    Some(match key {
        "file search" => ["file search", "поиск файлов", "文件搜索"],
        "Search files and folders…" => [
            "Search files and folders…",
            "Поиск файлов и папок…",
            "搜索文件或文件夹…",
        ],
        "Loading index…" => ["Loading index…", "Загружаю индекс…", "正在加载索引…"],
        "No index yet" => ["No index yet", "Индекса нет", "索引尚未建立"],
        "Index is not built yet" => [
            "Index is not built yet",
            "Индекс ещё не построен",
            "索引尚未建立",
        ],
        "Scanning…" => ["Scanning…", "Сканирую…", "正在扫描…"],
        "Build index" => ["Build index", "Построить индекс", "建立索引"],
        "Scanning disks, this may take a few minutes…" => [
            "Scanning disks, this may take a few minutes…",
            "Сканирую диски, это может занять пару минут…",
            "正在扫描磁盘，可能需要几分钟…",
        ],
        "Start typing — or pick a type above" => [
            "Start typing — or pick a type above",
            "Начни печатать — или выбери тип сверху",
            "开始输入，或在上方选择类型",
        ],
        "Nothing found" => ["Nothing found", "Ничего не нашлось", "什么也没找到"],
        "Enter — open · ↑↓ — select · Esc — clear · Ctrl+, — settings" => [
            "Enter — open · ↑↓ — select · Esc — clear · Ctrl+, — settings",
            "Enter — открыть · ↑↓ — выбор · Esc — очистить · Ctrl+, — настройки",
            "Enter — 打开 · ↑↓ — 选择 · Esc — 清空 · Ctrl+, — 设置",
        ],
        "Settings" => ["Settings", "Настройки", "设置"],
        "folder" => ["folder", "папка", "文件夹"],
        "Reset" => ["Reset", "Сбросить", "重置"],
        "saved automatically" => [
            "saved automatically",
            "сохраняется автоматически",
            "自动保存",
        ],
        "APPEARANCE" => ["APPEARANCE", "ОФОРМЛЕНИЕ", "外观"],
        "LIST" => ["LIST", "СПИСОК", "列表"],
        "SEARCH" => ["SEARCH", "ПОИСК", "搜索"],
        "Theme" => ["Theme", "Тема", "主题"],
        "Accent" => ["Accent", "Акцент", "强调色"],
        "Background opacity" => ["Background opacity", "Непрозрачность фона", "背景不透明度"],
        "Backdrop" => ["Backdrop", "Подложка", "背景效果"],
        "Density" => ["Density", "Плотность", "密度"],
        "Show hidden" => ["Show hidden", "Показывать скрытые", "显示隐藏文件"],
        "Show size" => ["Show size", "Показывать размер", "显示大小"],
        "Result limit" => ["Result limit", "Лимит результатов", "结果数量上限"],
        "Sort" => ["Sort", "Сортировка", "排序"],
        "Match" => ["Match", "Совпадение", "匹配方式"],
        "Interface scale" => ["Interface scale", "Масштаб интерфейса", "界面缩放"],
        "Language" => ["Language", "Язык", "语言"],
        "Auto" => ["Auto", "Авто", "自动"],
        "of" => ["of", "из", "/"],
        "Tab — type · ↑↓ — select · Enter — open · Esc — exit" => [
            "Tab — type · ↑↓ — select · Enter — open · Esc — exit",
            "Tab — тип · ↑↓ — выбор · Enter — открыть · Esc — выход",
            "Tab — 类型 · ↑↓ — 选择 · Enter — 打开 · Esc — 退出",
        ],
        // типы файлов
        "other" => ["other", "прочее", "其他"],
        "images" => ["images", "картинки", "图片"],
        "video" => ["video", "видео", "视频"],
        "audio" => ["audio", "аудио", "音频"],
        "documents" => ["documents", "документы", "文档"],
        "text" => ["text", "текст", "文本"],
        "archives" => ["archives", "архивы", "压缩包"],
        "code" => ["code", "код", "代码"],
        "programs" => ["programs", "программы", "程序"],
        "fonts" => ["fonts", "шрифты", "字体"],
        "3D models" => ["3D models", "3D-модели", "3D 模型"],
        // короткие подписи чипов
        "All" => ["All", "Всё", "全部"],
        "Folders" => ["Folders", "Папки", "文件夹"],
        "Files" => ["Files", "Файлы", "文件"],
        "Photos" => ["Photos", "Фото", "图片"],
        "Docs" => ["Docs", "Доки", "文档"],
        "3D" => ["3D", "3D", "3D"],
        // тема
        "Dark" => ["Dark", "Тёмная", "深色"],
        "Light" => ["Light", "Светлая", "浅色"],
        // акценты
        "Blue" => ["Blue", "Синий", "蓝色"],
        "Teal" => ["Teal", "Бирюзовый", "青色"],
        "Green" => ["Green", "Зелёный", "绿色"],
        "Purple" => ["Purple", "Фиолетовый", "紫色"],
        "Orange" => ["Orange", "Оранжевый", "橙色"],
        "Pink" => ["Pink", "Розовый", "粉色"],
        "Red" => ["Red", "Красный", "红色"],
        // плотность
        "Compact" => ["Compact", "Плотно", "紧凑"],
        "Normal" => ["Normal", "Обычно", "标准"],
        "Relaxed" => ["Relaxed", "Просторно", "宽松"],
        // сортировка
        "Relevance" => ["Relevance", "По совпадению", "按相关度"],
        "Name" => ["Name", "По имени", "按名称"],
        "Size" => ["Size", "По размеру", "按大小"],
        "Date" => ["Date", "По дате", "按日期"],
        // совпадение
        "Contains" => ["Contains", "Содержит", "包含"],
        "From name start" => ["From name start", "С начала имени", "名称开头"],
        "Whole name" => ["Whole name", "Имя целиком", "完全匹配"],
        // подложка
        "Solid" => ["Solid", "Сплошной", "纯色"],
        "Acrylic" => ["Acrylic", "Акрил", "亚克力"],
        _ => return None,
    })
}

/// «N результатов» с правильными формами.
pub fn results(n: usize, capped: bool) -> String {
    let plus = if capped { "+" } else { "" };
    match current() {
        Lang::Ru => format!(
            "{n}{plus} {}",
            plural_ru(n, "результат", "результата", "результатов")
        ),
        Lang::Zh => format!("{n}{plus} 个结果"),
        _ => format!("{n}{plus} {}", if n == 1 { "result" } else { "results" }),
    }
}

/// «N записей» для счётчика индекса.
pub fn records(n: usize) -> String {
    let num = fmt_num(n);
    match current() {
        Lang::Ru => format!("{num} {}", plural_ru(n, "запись", "записи", "записей")),
        Lang::Zh => format!("{num} 条记录"),
        _ => format!("{num} {}", if n == 1 { "record" } else { "records" }),
    }
}

/// Число + «мс» / «ms» / «毫秒».
pub fn ms(v: f64) -> String {
    let num = match current() {
        Lang::Ru => format!("{v:.1}").replace('.', ","),
        _ => format!("{v:.1}"),
    };
    match current() {
        Lang::Ru => format!("{num} мс"),
        Lang::Zh => format!("{num} 毫秒"),
        _ => format!("{num} ms"),
    }
}

fn plural_ru<'a>(n: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let m100 = n % 100;
    let m10 = n % 10;
    if (11..=14).contains(&m100) {
        many
    } else if m10 == 1 {
        one
    } else if (2..=4).contains(&m10) {
        few
    } else {
        many
    }
}

pub fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    out
}
