//! Захват экрана под окном и размытие — «настоящий» акрил.

/// Забрать область экрана (x,y,w,h), уменьшить до out_w×out_h и размыть.
/// Возвращает (rgba, ширина, высота).
#[cfg(windows)]
pub fn grab_blurred(x: i32, y: i32, w: i32, h: i32, out_w: i32, out_h: i32, radius: f32) -> Option<(Vec<u8>, u32, u32)> {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDC, HALFTONE, ReleaseDC, SRCCOPY, SelectObject, SetBrushOrgEx, SetStretchBltMode, StretchBlt,
    };

    if w <= 0 || h <= 0 || out_w <= 0 || out_h <= 0 {
        return None;
    }
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        if screen.is_null() {
            return None;
        }
        let mem = CreateCompatibleDC(screen);
        if mem.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen);
            return None;
        }

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = out_w;
        bi.bmiHeader.biHeight = -out_h; // сверху вниз
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(mem, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        if dib.is_null() || bits.is_null() {
            DeleteDC(mem);
            ReleaseDC(std::ptr::null_mut(), screen);
            return None;
        }
        let old = SelectObject(mem, dib);

        SetStretchBltMode(mem, HALFTONE);
        SetBrushOrgEx(mem, 0, 0, std::ptr::null_mut());
        let ok = StretchBlt(mem, 0, 0, out_w, out_h, screen, x, y, w, h, SRCCOPY);

        let mut px = vec![0u8; (out_w as usize) * (out_h as usize) * 4];
        if ok != 0 {
            std::ptr::copy_nonoverlapping(bits as *const u8, px.as_mut_ptr(), px.len());
        }

        SelectObject(mem, old);
        DeleteObject(dib);
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);

        if ok == 0 {
            return None;
        }

        // BGRA -> RGBA
        for c in px.chunks_exact_mut(4) {
            c.swap(0, 2);
            c[3] = 255;
        }
        let img = image::RgbaImage::from_raw(out_w as u32, out_h as u32, px)?;
        let blurred = image::imageops::blur(&img, radius);
        let (bw, bh) = blurred.dimensions();
        Some((blurred.into_raw(), bw, bh))
    }
}

#[cfg(not(windows))]
pub fn grab_blurred(_x: i32, _y: i32, _w: i32, _h: i32, _ow: i32, _oh: i32, _r: f32) -> Option<(Vec<u8>, u32, u32)> {
    None
}

/// Прятать окно от захвата экрана, чтобы блюр не ловил сам себя.
#[cfg(windows)]
pub fn exclude_from_capture(hwnd: *mut core::ffi::c_void, exclude: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};
    unsafe {
        let flag = if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
        SetWindowDisplayAffinity(hwnd, flag);
    }
}

#[cfg(not(windows))]
pub fn exclude_from_capture(_hwnd: *mut core::ffi::c_void, _exclude: bool) {}