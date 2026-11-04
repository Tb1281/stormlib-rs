#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
// The bulk of this crate is bindgen-generated code included below.
// Its style is not actionable (e.g. bindgen 0.69 emits `transmute` for
// bitfields, which current clippy/rustc flag as unnecessary), so those
// lints are silenced crate-wide here; hand-written code lives in
// `stormlib`, where clippy -D warnings still applies.
#![allow(clippy::all)]
#![allow(unnecessary_transmutes)]

#[cfg(target_os = "windows")]
include!("./bindings_windows.rs");
#[cfg(target_os = "linux")]
include!("./bindings_linux.rs");
#[cfg(target_os = "macos")]
include!("./bindings_macos.rs");

// Last-error retrieval, per platform. StormLib 9.40 renamed the portable
// error functions to SErrGetLastError/SErrSetLastError (the old
// non-Windows SetLastError/GetLastError symbols no longer exist in the
// library); on Windows they wrap the Win32 APIs, so `stormlib` keeps
// calling this single safe `GetLastError` name.
#[cfg(target_os = "windows")]
mod win32 {
  extern "C" {
    pub fn GetLastError() -> u32;
  }
}
#[cfg(target_os = "windows")]
pub fn GetLastError() -> u32 {
  unsafe { win32::GetLastError() }
}
#[cfg(not(target_os = "windows"))]
pub fn GetLastError() -> u32 {
  unsafe { SErrGetLastError() }
}

#[test]
fn test_w3x() {
  use std::ffi::*;
  use std::ptr;

  #[cfg(not(target_os = "windows"))]
  let path = CString::new("../../samples/test_tft.w3x").unwrap();
  #[cfg(target_os = "windows")]
  let path: Vec<_> = {
    "../../samples/test_tft.w3x"
      .as_bytes()
      .iter()
      .cloned()
      .map(u16::from)
      .chain(std::iter::once(0))
      .collect()
  };

  let file = CString::new("war3map.j").unwrap();
  unsafe {
    let mut handle: HANDLE = ptr::null_mut();
    let ok = SFileOpenArchive(
      path.as_ptr(),
      0,
      MPQ_OPEN_NO_LISTFILE | MPQ_OPEN_NO_ATTRIBUTES,
      &mut handle as *mut HANDLE,
    );

    assert!(ok);

    let mut file_handle: HANDLE = ptr::null_mut();
    let ok = SFileOpenFileEx(handle, file.as_ptr(), 0, &mut file_handle as *mut HANDLE);
    assert!(ok);

    let mut size_high: DWORD = 0;
    let size = SFileGetFileSize(file_handle, &mut size_high as *mut DWORD);
    assert!(ok);

    println!("file size = {}", size);

    let mut buf = vec![0u8; size as usize];

    let mut read: DWORD = 0;
    let ok = SFileReadFile(
      file_handle,
      buf.as_mut_ptr() as *mut std::ffi::c_void,
      size,
      &mut read as *mut DWORD,
      ptr::null_mut(),
    );
    assert!(ok);

    println!("read size = {}", read);

    assert_eq!(buf, std::fs::read("../../samples/war3map.j").unwrap());

    assert!(SFileCloseFile(file_handle));

    assert!(SFileCloseArchive(handle));
  }
}
