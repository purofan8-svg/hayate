//! Установщик hayate: кладёт приложение в %LOCALAPPDATA%\Programs\hayate,
//! делает ярлыки и запись в «Установка и удаление программ».
//!
//! Запуск: hayate-setup.exe  ·  Удаление: тот же файл с --uninstall
//! (при установке копируется в папку установки как uninstall.exe)

use std::path::{Path, PathBuf};
use std::process::Command;

const APP: &str = "hayate.exe";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ICO: &[u8] = include_bytes!("../../assets/hayate.ico");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--uninstall") {
        uninstall();
        return;
    }
    install(args.iter().any(|a| a == "--silent"));
}

fn local_appdata() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()))
}

fn install_dir() -> PathBuf {
    local_appdata().join("Programs").join("hayate")
}

fn self_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn install(silent: bool) {
    let here = self_exe()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let src = here.join(APP);
    if !src.exists() {
        println!("Рядом с установщиком нет {APP}.");
        println!("Распакуй архив целиком и запусти hayate-setup.exe из папки.");
        if !silent {
            pause();
        }
        return;
    }

    let dir = install_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        println!("Не удалось создать {}: {e}", dir.display());
        if !silent {
            pause();
        }
        return;
    }

    // закрыть запущенную копию
    let _ = Command::new("taskkill").args(["/IM", APP, "/F"]).output();

    if let Err(e) = std::fs::copy(&src, dir.join(APP)) {
        println!("Не удалось скопировать {APP}: {e}");
        if !silent {
            pause();
        }
        return;
    }
    let _ = std::fs::copy(self_exe(), dir.join("uninstall.exe"));
    let _ = std::fs::write(dir.join("hayate.ico"), ICO);

    shortcuts(&dir);
    register(&dir);

    println!("hayate {VERSION} установлен.");
    println!("Папка: {}", dir.display());
    println!("Ярлыки: меню Пуск и рабочий стол.");
    let _ = Command::new(dir.join(APP))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    if !silent {
        pause();
    }
}

fn shortcuts(dir: &Path) {
    let exe = dir.join(APP);
    let icon = dir.join("hayate.ico");
    let mut targets: Vec<PathBuf> = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        targets.push(
            PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs\\hayate.lnk"),
        );
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        targets.push(PathBuf::from(profile).join("Desktop\\hayate.lnk"));
    }

    let mut script = String::new();
    for t in targets {
        if let Some(parent) = t.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        script.push_str(&format!(
            "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');\
             $s.TargetPath='{}';$s.WorkingDirectory='{}';$s.IconLocation='{}';$s.WindowStyle=7;$s.Save();",
            t.display(),
            exe.display(),
            dir.display(),
            icon.display()
        ));
    }
    if !script.is_empty() {
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output();
    }
}

fn register(dir: &Path) {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\hayate";
    let uninstall = dir.join("uninstall.exe");
    let sets: [(&str, String); 6] = [
        ("DisplayName", "hayate — поиск файлов".to_string()),
        ("DisplayVersion", VERSION.to_string()),
        ("Publisher", "hayate".to_string()),
        ("InstallLocation", dir.display().to_string()),
        ("DisplayIcon", dir.join("hayate.ico").display().to_string()),
        (
            "UninstallString",
            format!("\"{}\" --uninstall", uninstall.display()),
        ),
    ];
    for (name, value) in sets {
        let _ = Command::new("reg")
            .args(["add", key, "/v", name, "/t", "REG_SZ", "/d", &value, "/f"])
            .output();
    }
}

fn uninstall() {
    let _ = Command::new("taskkill").args(["/IM", APP, "/F"]).output();

    if let Ok(appdata) = std::env::var("APPDATA") {
        let _ = std::fs::remove_file(
            PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs\\hayate.lnk"),
        );
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let _ = std::fs::remove_file(PathBuf::from(profile).join("Desktop\\hayate.lnk"));
    }
    let _ = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\hayate",
            "/f",
        ])
        .output();

    let dir = install_dir();
    println!("hayate удалён. Индекс и настройки тоже удаляются.");

    // папку уносим после выхода: сам процесс лежит внутри неё
    let script = format!(
        "ping -n 2 127.0.0.1 >nul & rmdir /s /q \"{}\"",
        dir.display()
    );
    let _ = Command::new("cmd").args(["/C", &script]).spawn();
    pause();
}

fn pause() {
    println!();
    println!("Нажми Enter, чтобы закрыть это окно.");
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
}
