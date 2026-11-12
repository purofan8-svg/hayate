//! Штатное контекстное меню Windows (как в Проводнике): «Открыть», «Открыть с помощью»,
//! «Копировать», «Свойства» и всё, что поставили установщики программ.

#[cfg(windows)]
mod imp {
    use std::ptr::null;

    use windows::core::HSTRING;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{
        IContextMenu, IShellItem, SHCreateItemFromParsingName, BHID_SFUIObject, CMINVOKECOMMANDINFO,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, GetCursorPos, PostMessageW, SetForegroundWindow,
        TrackPopupMenuEx, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
    };

    /// Как CMINVOKECOMMANDINFO, но с lpTitle: Windows смотрит только на cbSize,
    /// поэтому передаём полный вариант (ANSI, 64 байта на x64).
    #[repr(C)]
    struct Invoke {
        cb_size: u32,
        f_mask: u32,
        hwnd: HWND,
        verb: *const u8,
        parameters: *const u8,
        directory: *const u8,
        n_show: i32,
        hot_key: u32,
        h_icon: isize,
        title: *const u8,
    }

    const CMF_NORMAL_EXTENDED: u32 = 0x100; // CMF_NORMAL | CMF_EXTENDEDVERBS

    pub fn show(hwnd: isize, path: &str) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let owner = HWND(hwnd as *mut core::ffi::c_void);

            let wide = HSTRING::from(path);
            let item: IShellItem = match SHCreateItemFromParsingName(&wide, None) {
                Ok(i) => i,
                Err(_) => {
                    CoUninitialize();
                    return;
                }
            };
            let menu_iface: IContextMenu = match item.BindToHandler(None, &BHID_SFUIObject) {
                Ok(m) => m,
                Err(_) => {
                    CoUninitialize();
                    return;
                }
            };

            let menu = match CreatePopupMenu() {
                Ok(m) => m,
                Err(_) => {
                    CoUninitialize();
                    return;
                }
            };

            if menu_iface
                .QueryContextMenu(menu, 0, 1, 0x7FFF, CMF_NORMAL_EXTENDED)
                .is_ok()
            {
                let mut pt = Default::default();
                let _ = GetCursorPos(&mut pt);
                let _ = SetForegroundWindow(owner);

                let picked = TrackPopupMenuEx(
                    menu,
                    (TPM_RETURNCMD | TPM_RIGHTBUTTON).0,
                    pt.x,
                    pt.y,
                    owner,
                    None,
                );

                if picked.0 > 0 {
                    let offset = (picked.0 as u32 - 1) as usize;
                    let ci = Invoke {
                        cb_size: std::mem::size_of::<Invoke>() as u32,
                        f_mask: 0,
                        hwnd: owner,
                        verb: offset as *const u8, // MAKEINTRESOURCE
                        parameters: null(),
                        directory: null(),
                        n_show: 1, // SW_SHOWNORMAL
                        hot_key: 0,
                        h_icon: 0,
                        title: null(),
                    };
                    let _ =
                        menu_iface.InvokeCommand(&ci as *const Invoke as *const CMINVOKECOMMANDINFO);
                }
                let _ = PostMessageW(owner, WM_NULL, WPARAM(0), LPARAM(0));
            }

            let _ = DestroyMenu(menu);
            CoUninitialize();
        }
    }
}

#[cfg(windows)]
pub use imp::show;

#[cfg(not(windows))]
pub fn show(_hwnd: isize, _path: &str) {}