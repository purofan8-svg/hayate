//! Размытые обои рабочего стола — подложка в стиле Mica.

use std::path::PathBuf;

/// Путь к текущим обоям из реестра Windows.
pub fn current_path() -> Option<PathBuf> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
        let sub: Vec<u16> = "Control Panel\\Desktop\0".encode_utf16().collect();
        let value: Vec<u16> = "WallPaper\0".encode_utf16().collect();
        let mut buf = vec![0u16; 2048];
        let mut size = (buf.len() * 2) as u32;
        let rc = RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            &mut size,
        );
        if rc != 0 {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        let s = String::from_utf16_lossy(&buf[..len]);
        if s.is_empty() { None } else { Some(PathBuf::from(s)) }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Уменьшенные и размытые обои: (rgba, ширина, высота).
pub fn blurred(max_side: u32, radius: f32) -> Option<(Vec<u8>, u32, u32)> {
    let path = current_path()?;
    let img = image::open(path).ok()?;
    let small = img.resize(max_side, max_side, image::imageops::FilterType::Triangle).to_rgba8();
    let out = image::imageops::blur(&small, radius);
    let (w, h) = (out.width(), out.height());
    Some((out.into_raw(), w, h))
}