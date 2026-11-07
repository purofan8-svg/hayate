mod capture;
mod gui;
mod index;
mod lang;
mod settings;
mod settings_ui;
mod tui;
mod walk;
mod wallpaper;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, Subcommand};

use crate::index::{Filter, Index, Kind};

#[derive(Parser)]
#[command(name = "hayate", version, about = "Мгновенный поиск файлов: Windows + Linux")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Открыть окно поиска (по умолчанию, если команда не указана)
    Gui,
    /// Построить индекс (по умолчанию — все диски / корень)
    Index {
        /// Каталоги для индексации
        roots: Vec<PathBuf>,
        /// Пропустить каталог с таким именем
        #[arg(long)]
        exclude: Vec<String>,
        /// Куда писать индекс
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Найти по индексу
    Find {
        query: String,
        #[arg(short, long, default_value_t = 40)]
        limit: usize,
        #[arg(long)]
        json: bool,
        /// Только каталоги
        #[arg(long)]
        dirs: bool,
        /// Только файлы
        #[arg(long)]
        files: bool,
        /// Тип: image, video, audio, doc, text, archive, code, exec, font, model, other (можно через запятую)
        #[arg(short = 't', long = "type", value_delimiter = ',')]
        types: Vec<String>,
        /// Точное расширение, напр. png
        #[arg(short = 'e', long)]
        ext: Option<String>,
    },
    /// Интерактивный поиск (TUI)
    Tui,
    /// Статистика индекса
    Stats,
    /// Замер скорости поиска
    Bench {
        query: String,
        #[arg(short, long, default_value_t = 30)]
        runs: usize,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli.cmd.unwrap_or(Cmd::Gui)) {
        eprintln!("hayate: {e}");
        std::process::exit(1);
    }
}

fn run(cmd: Cmd) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Cmd::Gui => {
            // бинарь консольный: при запуске из ярлыка прячем лишнее окно консоли
            #[cfg(windows)]
            unsafe {
                windows_sys::Win32::System::Console::FreeConsole();
            }
            gui::run()?
        }
        Cmd::Index { roots, exclude, out } => {
            let roots = if roots.is_empty() { walk::default_roots() } else { roots };
            println!("Сканирую: {}", roots.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "));
            let (ix, st) = walk::scan(&roots, &exclude);
            let out_path = out.unwrap_or_else(index_path);
            ix.save(&out_path)?;
            let mb = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0) as f64 / 1048576.0;
            println!(
                "Готово: {} записей ({} каталогов, {:.1} ГБ) за {:.2} c · {:.0} файлов/с · {:.1} МБ индекса",
                st.entries, st.dirs, st.bytes as f64 / 1073741824.0, st.secs, st.entries as f64 / st.secs.max(0.001), mb
            );
            println!("Индекс: {}", out_path.display());
        }
        Cmd::Find { query, limit, json, dirs, files, types, ext } => {
            let ix = load()?;
            let filter = build_filter(dirs, files, &types, ext)?;
            let r = ix.search(&query, limit, &filter);
            if json {
                let items: Vec<String> = r.hits.iter().map(|&i| {
                    format!(
                        "{{\"path\":{},\"size\":{},\"dir\":{},\"kind\":\"{}\",\"mtime\":{}}}",
                        json_str(ix.path_at(i)), ix.size_at(i), ix.is_dir(i), ix.kind_at(i).name(), ix.mtime_at(i)
                    )
                }).collect();
                println!("[{}]", items.join(","));
            } else if r.hits.is_empty() {
                println!("Ничего не найдено ({:.2} мс)", r.took.as_secs_f64() * 1000.0);
            } else {
                for &i in &r.hits {
                    let tag = if ix.is_dir(i) { "папка".to_string() } else { ix.kind_at(i).name().to_string() };
                    let size = if ix.is_dir(i) { "—".to_string() } else { human(ix.size_at(i)) };
                    println!("{:<10} {:>9}  {}", tag, size, ix.path_at(i));
                }
                println!("── {} результатов за {:.2} мс", r.hits.len(), r.took.as_secs_f64() * 1000.0);
            }
        }
        Cmd::Tui => {
            let ix = Arc::new(load()?);
            tui::run(ix)?;
        }
        Cmd::Stats => {
            let ix = load()?;
            let p = index_path();
            let disk = std::fs::metadata(&p).map(|m| human(m.len())).unwrap_or_else(|_| "?".into());
            let dirs = (0..ix.len()).filter(|&i| ix.is_dir(i)).count();
            let mut counts = [0u64; 11];
            for i in 0..ix.len() {
                if !ix.is_dir(i) {
                    counts[ix.kind_at(i) as usize] += 1;
                }
            }
            println!("Записей:   {}", ix.len());
            println!("Каталогов: {}", dirs);
            for k in Kind::ALL {
                if counts[k as usize] > 0 {
                    println!("  {:<12} {}", k.name(), counts[k as usize]);
                }
            }
            println!("Корни:     {}", ix.roots.join(", "));
            println!("Индекс:    {} ({})", p.display(), disk);
        }
        Cmd::Bench { query, runs } => {
            let ix = load()?;
            // прогрев
            let f = Filter::default();
            let _ = ix.search(&query, 40, &f);
            let mut best = f64::MAX;
            let mut total = 0.0;
            let mut n = 0;
            for _ in 0..runs.max(1) {
                let t = Instant::now();
                let r = ix.search(&query, 40, &f);
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                best = best.min(ms);
                total += ms;
                n = r.hits.len();
            }
            println!(
                "«{}»: лучший {:.2} мс, средний {:.2} мс ({} прогонов, {} хитов)",
                query, best, total / runs.max(1) as f64, runs, n
            );
        }
    }
    Ok(())
}

fn build_filter(
    dirs: bool,
    files: bool,
    types: &[String],
    ext: Option<String>,
) -> Result<Filter, Box<dyn std::error::Error>> {
    let mut mask: Option<u16> = None;
    for t in types {
        let k = Kind::from_name(t).ok_or_else(|| {
            format!("неизвестный тип «{t}». Есть: image, video, audio, doc, text, archive, code, exec, font, model, other")
        })?;
        mask = Some(mask.unwrap_or(0) | k.bit());
    }
    Ok(Filter {
        dirs_only: dirs,
        files_only: files,
        kinds: mask,
        ext: ext.map(|e| {
            let e = e.trim().to_lowercase();
            if e.starts_with('.') { e } else { format!(".{e}") }
        }),
        ..Default::default()
    })
}

fn load() -> Result<Index, Box<dyn std::error::Error>> {
    let p = index_path();
    if !p.exists() {
        return Err(format!("индекса нет — сначала `hayate index`\nожидался {}", p.display()).into());
    }
    Ok(Index::load(&p)?)
}

pub(crate) fn index_path() -> PathBuf {
    if let Ok(p) = std::env::var("HAYATE_INDEX") {
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
        Path::new(&base).join("hayate").join("index.bin")
    } else {
        let base = std::env::var("XDG_CACHE_HOME")
            .unwrap_or_else(|_| format!("{}/.cache", std::env::var("HOME").unwrap_or_default()));
        Path::new(&base).join("hayate").join("index.bin")
    }
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

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}