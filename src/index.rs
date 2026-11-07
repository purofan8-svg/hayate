//! Индекс: компактное хранение путей, запись на диск, поиск.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use memchr::memmem;
use rayon::prelude::*;

use crate::settings::{Match, Sort};

pub const MAGIC: &[u8; 8] = b"HAYATE01";
pub const ATTR_DIR: u8 = 1;
pub const ATTR_HIDDEN: u8 = 2;
const KIND_SHIFT: u8 = 2;

// ── типы файлов ────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Other = 0,
    Image = 1,
    Video = 2,
    Audio = 3,
    Doc = 4,
    Text = 5,
    Archive = 6,
    Code = 7,
    Exec = 8,
    Font = 9,
    Model = 10,
}

impl Kind {
    pub fn bit(self) -> u16 {
        1 << (self as u16)
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Other => crate::lang::t("other"),
            Kind::Image => crate::lang::t("images"),
            Kind::Video => crate::lang::t("video"),
            Kind::Audio => crate::lang::t("audio"),
            Kind::Doc => crate::lang::t("documents"),
            Kind::Text => crate::lang::t("text"),
            Kind::Archive => crate::lang::t("archives"),
            Kind::Code => crate::lang::t("code"),
            Kind::Exec => crate::lang::t("programs"),
            Kind::Font => crate::lang::t("fonts"),
            Kind::Model => crate::lang::t("3D models"),
        }
    }

    pub fn from_name(s: &str) -> Option<Kind> {
        Some(match s.to_lowercase().as_str() {
            "other" | "прочее" => Kind::Other,
            "image" | "img" | "картинки" => Kind::Image,
            "video" | "видео" => Kind::Video,
            "audio" | "аудио" => Kind::Audio,
            "doc" | "document" | "документы" => Kind::Doc,
            "text" | "текст" => Kind::Text,
            "archive" | "архивы" => Kind::Archive,
            "code" | "код" => Kind::Code,
            "exec" | "exe" | "программы" => Kind::Exec,
            "font" | "шрифты" => Kind::Font,
            "model" | "3d" | "модели" => Kind::Model,
            _ => return None,
        })
    }

    pub const ALL: [Kind; 11] = [
        Kind::Other,
        Kind::Image,
        Kind::Video,
        Kind::Audio,
        Kind::Doc,
        Kind::Text,
        Kind::Archive,
        Kind::Code,
        Kind::Exec,
        Kind::Font,
        Kind::Model,
    ];
}

const IMAGE: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff", "ico", "svg", "psd", "avif", "heic",
    "dds", "tga",
];
const VIDEO: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "vob",
];
const AUDIO: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "mid", "midi", "xm", "mod",
];
const DOC: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "rtf", "epub", "djvu", "fb2",
    "cbz", "cbr",
];
const TEXT: &[&str] = &[
    "txt",
    "md",
    "markdown",
    "log",
    "ini",
    "cfg",
    "conf",
    "json",
    "yaml",
    "yml",
    "toml",
    "xml",
    "csv",
    "env",
    "properties",
    "nfo",
    "srt",
    "sub",
];
const ARCHIVE: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst", "jar", "mrpack", "tmod", "iso", "img",
    "pak",
];
const CODE: &[&str] = &[
    "rs", "java", "py", "js", "mjs", "cjs", "ts", "tsx", "jsx", "c", "h", "cpp", "hpp", "cc", "cs",
    "go", "rb", "php", "lua", "sh", "bash", "bat", "ps1", "html", "htm", "css", "scss", "sql",
    "kt", "swift", "dart", "gradle", "cmake", "mk", "glsl", "frag", "vert", "wgsl",
];
const EXEC: &[&str] = &[
    "exe", "msi", "dll", "sys", "so", "dylib", "apk", "deb", "rpm", "appimage", "bin", "elf",
    "class", "pyc",
];
const FONT: &[&str] = &["ttf", "otf", "woff", "woff2", "ttc"];
const MODEL: &[&str] = &[
    "obj", "fbx", "gltf", "glb", "blend", "vox", "stl", "ply", "dae", "usd",
];

pub fn classify(name: &str) -> Kind {
    let Some(dot) = name.rfind('.') else {
        return Kind::Other;
    };
    let ext = name[dot + 1..].to_ascii_lowercase();
    let e = ext.as_str();
    if IMAGE.contains(&e) {
        Kind::Image
    } else if VIDEO.contains(&e) {
        Kind::Video
    } else if AUDIO.contains(&e) {
        Kind::Audio
    } else if DOC.contains(&e) {
        Kind::Doc
    } else if TEXT.contains(&e) {
        Kind::Text
    } else if ARCHIVE.contains(&e) {
        Kind::Archive
    } else if CODE.contains(&e) {
        Kind::Code
    } else if EXEC.contains(&e) {
        Kind::Exec
    } else if FONT.contains(&e) {
        Kind::Font
    } else if MODEL.contains(&e) {
        Kind::Model
    } else {
        Kind::Other
    }
}

// ── фильтр ─────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct Filter {
    pub dirs_only: bool,
    pub files_only: bool,
    /// Битовая маска Kind; None = без ограничения по типу
    pub kinds: Option<u16>,
    /// Расширение с точкой, в нижнем регистре
    pub ext: Option<String>,
    /// Скрывать скрытые файлы
    pub hide_hidden: bool,
    pub match_mode: Match,
    pub sort: Sort,
}

impl Filter {
    pub fn any(&self) -> bool {
        !self.dirs_only
            && !self.files_only
            && self.kinds.is_none()
            && self.ext.is_none()
            && !self.hide_hidden
    }

    #[inline]
    fn accept(&self, attrs: u8, folded: &[u8]) -> bool {
        if self.hide_hidden && attrs & ATTR_HIDDEN != 0 {
            return false;
        }
        let is_dir = attrs & ATTR_DIR != 0;
        if self.dirs_only && !is_dir {
            return false;
        }
        // Тип и расширение относятся только к файлам
        if (self.files_only || self.kinds.is_some() || self.ext.is_some()) && is_dir {
            return false;
        }
        if let Some(mask) = self.kinds {
            if mask & (1u16 << (attrs >> KIND_SHIFT)) == 0 {
                return false;
            }
        }
        if let Some(ext) = &self.ext {
            let e = ext.as_bytes();
            if folded.len() < e.len() || &folded[folded.len() - e.len()..] != e {
                return false;
            }
        }
        true
    }
}

#[derive(Default, Clone)]
pub struct Index {
    buf: String,
    fbuf: String,
    ends: Vec<u32>,
    fends: Vec<u32>,
    sizes: Vec<u64>,
    mtimes: Vec<i64>,
    attrs: Vec<u8>,
    pub roots: Vec<String>,
}

#[derive(Clone, Copy)]
struct RawHit {
    score: i32,
    len: u32,
    idx: u32,
}

pub struct SearchResult {
    pub hits: Vec<usize>,
    pub took: std::time::Duration,
}

impl Index {
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    /// Слияние шарда в основной индекс (сдвиг смещений).
    pub fn merge(&mut self, other: Index) {
        let off = self.buf.len() as u32;
        let foff = self.fbuf.len() as u32;
        self.buf.push_str(&other.buf);
        self.fbuf.push_str(&other.fbuf);
        self.ends.extend(other.ends.iter().map(|&e| e + off));
        self.fends.extend(other.fends.iter().map(|&e| e + foff));
        self.sizes.extend_from_slice(&other.sizes);
        self.mtimes.extend_from_slice(&other.mtimes);
        self.attrs.extend_from_slice(&other.attrs);
    }

    pub fn path_at(&self, i: usize) -> &str {
        let end = self.ends[i] as usize;
        let start = if i == 0 { 0 } else { self.ends[i - 1] as usize };
        &self.buf[start..end]
    }

    pub fn size_at(&self, i: usize) -> u64 {
        self.sizes[i]
    }

    fn fpath(&self, i: usize) -> &str {
        let end = self.fends[i] as usize;
        let start = if i == 0 {
            0
        } else {
            self.fends[i - 1] as usize
        };
        &self.fbuf[start..end]
    }

    fn sort_hits(&self, hits: &mut [usize], sort: Sort) {
        match sort {
            Sort::Relevance | Sort::Date => {
                hits.sort_unstable_by(|&a, &b| self.mtimes[b].cmp(&self.mtimes[a]))
            }
            Sort::Name => hits.sort_unstable_by(|&a, &b| self.fpath(a).cmp(self.fpath(b))),
            Sort::Size => hits.sort_unstable_by(|&a, &b| self.sizes[b].cmp(&self.sizes[a])),
        }
    }

    pub fn mtime_at(&self, i: usize) -> i64 {
        self.mtimes[i]
    }

    pub fn is_dir(&self, i: usize) -> bool {
        self.attrs[i] & ATTR_DIR != 0
    }

    pub fn kind_at(&self, i: usize) -> Kind {
        match self.attrs[i] >> KIND_SHIFT {
            1 => Kind::Image,
            2 => Kind::Video,
            3 => Kind::Audio,
            4 => Kind::Doc,
            5 => Kind::Text,
            6 => Kind::Archive,
            7 => Kind::Code,
            8 => Kind::Exec,
            9 => Kind::Font,
            10 => Kind::Model,
            _ => Kind::Other,
        }
    }

    pub fn push(&mut self, path: &str, size: u64, mtime: i64, attrs: u8) -> bool {
        let start = self.buf.len();
        if start + path.len() + 1 > u32::MAX as usize {
            return false;
        }
        self.buf.push_str(path);
        self.ends.push(self.buf.len() as u32);

        if path.is_ascii() {
            self.fbuf.push_str(&path.to_ascii_lowercase());
        } else {
            for c in path.chars() {
                self.fbuf.extend(c.to_lowercase());
            }
        }
        self.fends.push(self.fbuf.len() as u32);
        self.sizes.push(size);
        self.mtimes.push(mtime);
        self.attrs.push(attrs);
        true
    }

    /// Поиск подстроки с фильтром и ранжированием.
    pub fn search(&self, query: &str, limit: usize, filter: &Filter) -> SearchResult {
        let t0 = std::time::Instant::now();
        let mut hits = Vec::new();
        let q = query.to_lowercase();
        if self.is_empty() {
            return SearchResult {
                hits,
                took: t0.elapsed(),
            };
        }
        if q.is_empty() {
            // Пустой запрос + фильтр = «покажи все файлы такого типа», свежие сверху
            if !filter.any() {
                self.collect_filtered(filter, limit.saturating_mul(16), &mut hits);
                self.sort_hits(&mut hits, filter.sort);
                hits.truncate(limit);
            }
            return SearchResult {
                hits,
                took: t0.elapsed(),
            };
        }
        let needle = q.as_bytes();
        let n = self.len();
        const CHUNK: usize = 1 << 16;
        let ranges: Vec<(usize, usize)> = (0..n)
            .step_by(CHUNK)
            .map(|a| (a, (a + CHUNK).min(n)))
            .collect();

        let parts: Vec<Vec<RawHit>> = ranges
            .par_iter()
            .map(|&(a, b)| self.search_chunk(a, b, needle, filter, 8192))
            .collect();

        let mut raw: Vec<RawHit> = parts.into_iter().flatten().collect();
        match filter.sort {
            Sort::Relevance => {
                raw.sort_unstable_by(|x, y| y.score.cmp(&x.score).then(x.len.cmp(&y.len)))
            }
            Sort::Name => raw.sort_unstable_by(|x, y| {
                self.fpath(x.idx as usize).cmp(self.fpath(y.idx as usize))
            }),
            Sort::Size => raw.sort_unstable_by(|x, y| {
                self.sizes[y.idx as usize].cmp(&self.sizes[x.idx as usize])
            }),
            Sort::Date => raw.sort_unstable_by(|x, y| {
                self.mtimes[y.idx as usize].cmp(&self.mtimes[x.idx as usize])
            }),
        }
        hits.extend(raw.into_iter().take(limit).map(|h| h.idx as usize));
        SearchResult {
            hits,
            took: t0.elapsed(),
        }
    }

    /// Все записи, подходящие под фильтр (без текстового запроса).
    fn collect_filtered(&self, filter: &Filter, limit: usize, out: &mut Vec<usize>) {
        const CHUNK: usize = 1 << 16;
        let n = self.len();
        let ranges: Vec<(usize, usize)> = (0..n)
            .step_by(CHUNK)
            .map(|a| (a, (a + CHUNK).min(n)))
            .collect();
        let fbytes = self.fbuf.as_bytes();
        let parts: Vec<Vec<u32>> = ranges
            .par_iter()
            .map(|&(a, b)| {
                let mut v = Vec::new();
                for i in a..b {
                    let fs = if i == 0 {
                        0
                    } else {
                        self.fends[i - 1] as usize
                    };
                    let fe = self.fends[i] as usize;
                    if filter.accept(self.attrs[i], &fbytes[fs..fe]) {
                        v.push(i as u32);
                        if v.len() >= limit {
                            break;
                        }
                    }
                }
                v
            })
            .collect();
        out.extend(parts.into_iter().flatten().take(limit).map(|i| i as usize));
    }

    fn search_chunk(
        &self,
        a: usize,
        b: usize,
        needle: &[u8],
        filter: &Filter,
        cap: usize,
    ) -> Vec<RawHit> {
        let bstart = if a == 0 {
            0
        } else {
            self.fends[a - 1] as usize
        };
        let bend = self.fends[b - 1] as usize;
        let bytes = self.fbuf.as_bytes();
        let hay = &bytes[bstart..bend];
        let mut out = Vec::new();
        let mut last = usize::MAX;
        for m in memmem::find_iter(hay, needle) {
            let s_abs = (bstart + m) as u32;
            let e_abs = s_abs + needle.len() as u32;
            let idx = self.fends.partition_point(|&x| x <= s_abs);
            if idx < a || idx >= b || e_abs > self.fends[idx] || idx == last {
                continue;
            }
            last = idx;
            let entry_start = if idx == 0 {
                0
            } else {
                self.fends[idx - 1] as usize
            };
            let entry = &bytes[entry_start..self.fends[idx] as usize];
            let at = self.attrs[idx];
            if !filter.accept(at, entry) {
                continue;
            }
            let name_off = match memrchr_sep(entry) {
                Some(p) => p + 1,
                None => 0,
            };
            let name = &entry[name_off..];
            let mode_ok = match filter.match_mode {
                Match::Contains => true,
                Match::Prefix => name.starts_with(needle),
                Match::Exact => name == needle,
            };
            if !mode_ok {
                continue;
            }

            let mut score: i32 = if memmem::find(name, needle).is_some() {
                1200
            } else {
                100
            };
            if name == needle {
                score += 3000;
            } else if name.starts_with(needle) {
                score += 600;
            }
            score -= (name.len() as i32) / 2;
            score -= (entry.len() as i32) / 12;
            score -= (count_seps(entry) as i32) * 8;
            if at & ATTR_HIDDEN != 0 {
                score -= 50;
            }
            if at & ATTR_DIR != 0 {
                score -= 12;
            }
            out.push(RawHit {
                score,
                len: entry.len() as u32,
                idx: idx as u32,
            });
            if out.len() >= cap {
                break;
            }
        }
        out
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        {
            let mut w = BufWriter::with_capacity(1 << 20, File::create(&tmp)?);
            w.write_all(MAGIC)?;
            let roots_blob = self.roots.join("\n");
            w.write_all(&(self.len() as u64).to_le_bytes())?;
            w.write_all(&(self.buf.len() as u64).to_le_bytes())?;
            w.write_all(&(self.fbuf.len() as u64).to_le_bytes())?;
            w.write_all(&(roots_blob.len() as u64).to_le_bytes())?;
            w.write_all(roots_blob.as_bytes())?;
            w.write_all(self.buf.as_bytes())?;
            w.write_all(self.fbuf.as_bytes())?;
            write_slice(&mut w, &self.ends)?;
            write_slice(&mut w, &self.fends)?;
            write_slice(&mut w, &self.sizes)?;
            write_slice(&mut w, &self.mtimes)?;
            w.write_all(&self.attrs)?;
            w.flush()?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn load(path: &Path) -> std::io::Result<Index> {
        let mut data = Vec::new();
        File::open(path)?.read_to_end(&mut data)?;
        if data.len() < 40 || &data[..8] != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "не hayate-индекс",
            ));
        }
        let mut p = 8usize;
        let n = take_u64(&data, &mut p) as usize;
        let buf_len = take_u64(&data, &mut p) as usize;
        let fbuf_len = take_u64(&data, &mut p) as usize;
        let roots_len = take_u64(&data, &mut p) as usize;

        let roots_blob = &data[p..p + roots_len];
        p += roots_len;
        let roots = if roots_blob.is_empty() {
            Vec::new()
        } else {
            String::from_utf8_lossy(roots_blob)
                .split('\n')
                .map(|s| s.to_string())
                .collect()
        };

        let buf = String::from_utf8_lossy(&data[p..p + buf_len]).to_string();
        p += buf_len;
        let fbuf = String::from_utf8_lossy(&data[p..p + fbuf_len]).to_string();
        p += fbuf_len;

        let ends = read_u32(&data, &mut p, n);
        let fends = read_u32(&data, &mut p, n);
        let sizes = read_u64s(&data, &mut p, n);
        let mtimes = read_i64s(&data, &mut p, n);
        let attrs = data[p..p + n].to_vec();

        Ok(Index {
            buf,
            fbuf,
            ends,
            fends,
            sizes,
            mtimes,
            attrs,
            roots,
        })
    }
}

#[inline]
fn memrchr_sep(s: &[u8]) -> Option<usize> {
    match memchr::memrchr(b'/', s) {
        Some(p) => Some(p),
        None => memchr::memrchr(b'\\', s),
    }
}

#[inline]
fn count_seps(s: &[u8]) -> usize {
    memchr::memchr_iter(b'/', s).count() + memchr::memchr_iter(b'\\', s).count()
}

fn write_slice<T>(w: &mut impl Write, v: &[T]) -> std::io::Result<()> {
    // Только little-endian хосты (x86_64 / aarch64) — других у нас нет.
    let bytes =
        unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) };
    w.write_all(bytes)
}

fn take_u64(d: &[u8], p: &mut usize) -> u64 {
    let v = u64::from_le_bytes(d[*p..*p + 8].try_into().unwrap());
    *p += 8;
    v
}

fn read_u32(d: &[u8], p: &mut usize, n: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(u32::from_le_bytes(d[*p..*p + 4].try_into().unwrap()));
        *p += 4;
    }
    out
}

fn read_u64s(d: &[u8], p: &mut usize, n: usize) -> Vec<u64> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(take_u64(d, p));
    }
    out
}

fn read_i64s(d: &[u8], p: &mut usize, n: usize) -> Vec<i64> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(i64::from_le_bytes(d[*p..*p + 8].try_into().unwrap()));
        *p += 8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Index {
        let mut ix = Index::default();
        ix.push(
            "C:/mods/Create.jar",
            100,
            0,
            (Kind::Archive as u8) << KIND_SHIFT,
        );
        ix.push(
            "C:/mods/config/create.json",
            10,
            0,
            (Kind::Code as u8) << KIND_SHIFT,
        );
        ix.push(
            "C:/pics/logo.png",
            2048,
            0,
            (Kind::Image as u8) << KIND_SHIFT,
        );
        ix.push("C:/pics", 0, 0, ATTR_DIR);
        ix
    }

    #[test]
    fn path_and_roundtrip() {
        let ix = demo();
        assert_eq!(ix.path_at(0), "C:/mods/Create.jar");
        assert_eq!(ix.path_at(3), "C:/pics");
        let p = std::env::temp_dir().join(format!("hayate-test-{}.bin", std::process::id()));
        ix.save(&p).unwrap();
        let back = Index::load(&p).unwrap();
        assert_eq!(back.len(), ix.len());
        assert_eq!(back.path_at(2), "C:/pics/logo.png");
        assert_eq!(back.size_at(2), 2048);
        assert_eq!(back.kind_at(1), Kind::Code);
        assert!(back.is_dir(3));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn search_ranks_name_matches() {
        let ix = demo();
        let r = ix.search("create", 10, &Filter::default());
        assert_eq!(
            r.hits.first().copied(),
            Some(0),
            "имя важнее пути: {:?}",
            r.hits
        );
        assert_eq!(r.hits.len(), 2);
    }

    #[test]
    fn kind_filter_excludes_dirs_and_other_types() {
        let ix = demo();
        let f = Filter {
            kinds: Some(Kind::Image.bit()),
            ..Default::default()
        };
        let r = ix.search("", 10, &f);
        assert_eq!(r.hits, vec![2]);
        let d = Filter {
            dirs_only: true,
            ..Default::default()
        };
        assert_eq!(ix.search("", 10, &d).hits, vec![3]);
    }

    #[test]
    fn ext_filter() {
        let ix = demo();
        let f = Filter {
            ext: Some(".json".into()),
            ..Default::default()
        };
        assert_eq!(ix.search("", 10, &f).hits, vec![1]);
    }

    #[test]
    fn classify_kinds() {
        assert_eq!(classify("a.PNG"), Kind::Image);
        assert_eq!(classify("Mod.JAR"), Kind::Archive);
        assert_eq!(classify("noext"), Kind::Other);
        assert_eq!(classify("main.rs"), Kind::Code);
    }

    #[test]
    fn merge_shifts_offsets() {
        let mut a = demo();
        let mut b = Index::default();
        b.push("D:/x.png", 1, 0, (Kind::Image as u8) << KIND_SHIFT);
        a.merge(b);
        assert_eq!(a.len(), 5);
        assert_eq!(a.path_at(4), "D:/x.png");
        assert_eq!(a.kind_at(4), Kind::Image);
    }
}
