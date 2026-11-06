//! Обход файловой системы: параллельные волны по каталогам.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;

use crate::index::{ATTR_DIR, ATTR_HIDDEN, Index, classify};

pub struct ScanStats {
    pub entries: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub secs: f64,
}

const SKIP_DIRS: &[&str] = &["proc", "sys", "dev", "run", "lost+found"];

#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

pub fn scan(roots: &[PathBuf], excludes: &[String]) -> (Index, ScanStats) {
    let t0 = Instant::now();
    // На Windows приводим разделители в корнях к '\', иначе пути смешанные
    #[cfg(windows)]
    let roots: Vec<PathBuf> = roots
        .iter()
        .map(|p| PathBuf::from(p.to_string_lossy().replace('/', "\\")))
        .collect();
    #[cfg(not(windows))]
    let roots: Vec<PathBuf> = roots.to_vec();

    let mut main = Index::default();
    main.roots = roots.iter().map(|p| p.display().to_string()).collect();
    let mut entries = 0u64;
    let mut dirs = 0u64;
    let mut bytes = 0u64;

    let mut frontier: Vec<PathBuf> = roots.to_vec();
    while !frontier.is_empty() {
        let parts: Vec<DirScan> = frontier.par_iter().map(|d| scan_dir(d, excludes)).collect();
        let mut next = Vec::new();
        for part in parts {
            entries += part.ix.len() as u64;
            dirs += part.dirs;
            bytes += part.bytes;
            main.merge(part.ix);
            next.extend(part.subdirs);
        }
        frontier = next;
    }

    let stats = ScanStats { entries, dirs, bytes, secs: t0.elapsed().as_secs_f64() };
    (main, stats)
}

struct DirScan {
    ix: Index,
    subdirs: Vec<PathBuf>,
    dirs: u64,
    bytes: u64,
}

fn scan_dir(dir: &Path, excludes: &[String]) -> DirScan {
    let mut ix = Index::default();
    let mut subdirs = Vec::new();
    let mut dirs = 0u64;
    let mut bytes = 0u64;

    let Ok(rd) = std::fs::read_dir(dir) else {
        return DirScan { ix, subdirs, dirs, bytes };
    };

    for e in rd.flatten() {
        let name_os = e.file_name();
        let name = name_os.to_string_lossy();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if is_dir && (SKIP_DIRS.contains(&name.as_ref()) || excludes.iter().any(|x| x == name.as_ref())) {
            continue;
        }

        let md = e.metadata().ok();
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let reparse = md.as_ref().map(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0).unwrap_or(false);
            if is_dir && reparse {
                continue;
            }
        }
        let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = md
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut attrs = 0u8;
        if is_dir {
            attrs |= ATTR_DIR;
            dirs += 1;
        } else {
            bytes += size;
            attrs |= (classify(&name) as u8) << 2;
        }
        if is_hidden(&name, &md) {
            attrs |= ATTR_HIDDEN;
        }

        let path = e.path();
        let p = path.to_string_lossy();
        if !ix.push(p.as_ref(), size, mtime, attrs) {
            continue;
        }
        if is_dir {
            subdirs.push(path);
        }
    }

    DirScan { ix, subdirs, dirs, bytes }
}

#[cfg(windows)]
fn is_hidden(_name: &str, md: &Option<std::fs::Metadata>) -> bool {
    use std::os::windows::fs::MetadataExt;
    md.as_ref().map(|m| m.file_attributes() & 0x2 != 0).unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden(name: &str, _md: &Option<std::fs::Metadata>) -> bool {
    name.starts_with('.')
}

pub fn default_roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        ('C'..='Z')
            .map(|c| PathBuf::from(format!("{}:\\", c)))
            .filter(|p| p.exists())
            .collect()
    } else {
        vec![PathBuf::from("/")]
    }
}