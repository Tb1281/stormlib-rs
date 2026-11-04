use std::ffi::*;
use std::path::Path;
use std::ptr;
use stormlib_sys::*;

#[macro_use]
mod util;

mod constants;
pub use constants::*;

pub mod error;
use error::*;

/// MPQ archive
#[derive(Debug)]
pub struct Archive {
  handle: HANDLE,
}

impl Archive {
  /// Opens a MPQ archive
  pub fn open<P: AsRef<Path>>(path: P, flags: OpenArchiveFlags) -> Result<Self> {
    #[cfg(not(target_os = "windows"))]
    let cpath = {
      let pathstr = path.as_ref().to_str().ok_or(StormError::NonUtf8)?;
      CString::new(pathstr)?
    };
    #[cfg(target_os = "windows")]
    let cpath = {
      use widestring::U16CString;
      U16CString::from_os_str(path.as_ref())
        .map_err(|_| StormError::InteriorNul)?
        .into_vec()
    };
    let mut handle: HANDLE = ptr::null_mut();
    // unsafe_try_call! already expands to an unsafe block.
    unsafe_try_call!(SFileOpenArchive(
      cpath.as_ptr(),
      0,
      flags.bits(),
      &mut handle as *mut HANDLE,
    ));
    Ok(Archive { handle })
  }

  /// Quick check if the file exists within MPQ archive, without opening it
  pub fn has_file(&mut self, path: &str) -> Result<bool> {
    let cpath = CString::new(path)?;
    unsafe {
      let r = SFileHasFile(self.handle, cpath.as_ptr());
      let err = GetLastError();
      if !r && err != ERROR_FILE_NOT_FOUND {
        return Err(From::from(ErrorCode(err)));
      }
      Ok(r)
    }
  }

  /// Opens a file from MPQ archive
  pub fn open_file<'a>(&'a mut self, path: &str) -> Result<File<'a>> {
    let mut file_handle: HANDLE = ptr::null_mut();
    let cpath = CString::new(path)?;
    unsafe_try_call!(SFileOpenFileEx(
      self.handle,
      cpath.as_ptr(),
      0,
      &mut file_handle as *mut HANDLE
    ));
    Ok(File {
      archive: self,
      file_handle,
      size: None,
      need_reset: false,
    })
  }
}

impl std::ops::Drop for Archive {
  fn drop(&mut self) {
    unsafe {
      SFileCloseArchive(self.handle);
    }
  }
}

/// Opened file
#[derive(Debug)]
pub struct File<'a> {
  // Never read directly: it ties the file's lifetime to its archive so
  // the archive cannot be closed while a file is still open.
  #[allow(dead_code)]
  archive: &'a Archive,
  file_handle: HANDLE,
  size: Option<u64>,
  need_reset: bool,
}

impl<'a> File<'a> {
  /// Retrieves a size of the file within archive
  pub fn get_size(&mut self) -> Result<u64> {
    if let Some(size) = self.size {
      Ok(size)
    } else {
      let mut high: DWORD = 0;
      let low = unsafe { SFileGetFileSize(self.file_handle, &mut high as *mut DWORD) };
      if low == SFILE_INVALID_SIZE {
        return Err(From::from(ErrorCode(GetLastError())));
      }
      let high = (high as u64) << 32;
      let size = high | (low as u64);
      self.size = Some(size);
      Ok(size)
    }
  }

  /// Reads all data from the file
  pub fn read_all(&mut self) -> Result<Vec<u8>> {
    if self.need_reset {
      unsafe {
        if SFileSetFilePointer(self.file_handle, 0, ptr::null_mut(), 0) == SFILE_INVALID_SIZE {
          return Err(From::from(ErrorCode(GetLastError())));
        }
      }
    }

    let size = self.get_size()?;
    let mut buf = vec![0u8; size as usize];
    let mut read: DWORD = 0;
    self.need_reset = true;
    unsafe_try_call!(SFileReadFile(
      self.file_handle,
      buf.as_mut_ptr() as *mut std::ffi::c_void,
      size as u32,
      &mut read as *mut DWORD,
      ptr::null_mut(),
    ));
    if (read as u64) < size {
      buf.truncate(read as usize);
    }
    Ok(buf)
  }
}

impl<'a> std::ops::Drop for File<'a> {
  fn drop(&mut self) {
    unsafe {
      SFileCloseFile(self.file_handle);
    }
  }
}

/// One entry returned by [`Archive::find_files`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundFile {
  /// File name inside the archive (lossy UTF-8 if the stored name is not
  /// valid UTF-8).
  pub name: String,
  /// Uncompressed size in bytes.
  pub size: u32,
  /// Compressed size in bytes.
  pub compressed_size: u32,
  /// `MPQ_FILE_*` flags.
  pub flags: u32,
  /// File locale.
  pub locale: u32,
}

/// Iterator over the files in an archive matching a mask (`*` and `?`
/// wildcards, e.g. `"*"` or `"*.j"`). Created by [`Archive::find_files`].
#[derive(Debug)]
pub struct FindFiles<'a> {
  #[allow(dead_code)]
  archive: &'a Archive,
  handle: HANDLE,
  data: SFILE_FIND_DATA,
  first: bool,
  done: bool,
}

impl<'a> FindFiles<'a> {
  fn entry(&self) -> FoundFile {
    let raw = &self.data.cFileName;
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    FoundFile {
      name: String::from_utf8_lossy(&raw[..end].iter().map(|&c| c as u8).collect::<Vec<u8>>())
        .into_owned(),
      size: self.data.dwFileSize,
      compressed_size: self.data.dwCompSize,
      flags: self.data.dwFileFlags,
      locale: self.data.lcLocale,
    }
  }
}

impl<'a> Iterator for FindFiles<'a> {
  type Item = Result<FoundFile>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.done {
      return None;
    }
    if self.first {
      self.first = false;
      return Some(Ok(self.entry()));
    }
    let ok = unsafe { SFileFindNextFile(self.handle, &mut self.data) };
    if ok {
      Some(Ok(self.entry()))
    } else {
      self.done = true;
      let err = GetLastError();
      if err == ERROR_NO_MORE_FILES {
        None
      } else {
        Some(Err(From::from(ErrorCode(err))))
      }
    }
  }
}

impl<'a> std::ops::Drop for FindFiles<'a> {
  fn drop(&mut self) {
    if !self.handle.is_null() {
      unsafe {
        SFileFindClose(self.handle);
      }
    }
  }
}

impl Archive {
  /// Lists the files in the archive whose names match `mask`
  /// (`*` and `?` wildcards). Note this needs the archive's listfile:
  /// opening with `MPQ_OPEN_NO_LISTFILE` leaves file names unknown, so
  /// open without that flag to get names back. Prefer opening read-only
  /// (`MPQ_OPEN_READ_ONLY`) when only listing: on Windows a read-write
  /// open of a file another handle has open fails with
  /// ERROR_SHARING_VIOLATION.
  pub fn find_files<'a>(&'a mut self, mask: &str) -> Result<FindFiles<'a>> {
    let cmask = CString::new(mask)?;
    let mut data: SFILE_FIND_DATA = unsafe { std::mem::zeroed() };
    let handle =
      unsafe { SFileFindFirstFile(self.handle, cmask.as_ptr(), &mut data, ptr::null_mut()) };
    if handle.is_null() {
      let err = GetLastError();
      if err == ERROR_NO_MORE_FILES || err == ERROR_FILE_NOT_FOUND {
        // No match: return an already-finished iterator. Use a dummy
        // handle value; Drop calls SFileFindClose on it, which is a
        // no-op for a null handle, so keep it null and guard in Drop.
        return Ok(FindFiles {
          archive: self,
          handle: ptr::null_mut(),
          data,
          first: false,
          done: true,
        });
      }
      return Err(From::from(ErrorCode(err)));
    }
    Ok(FindFiles {
      archive: self,
      handle,
      data,
      first: true,
      done: false,
    })
  }
}

#[test]
fn test_find() {
  // Listfile must be enabled for names to come back, and read-only
  // so the archive can be opened concurrently (on Windows a
  // read-write open of a file another handle has open fails with
  // ERROR_SHARING_VIOLATION).
  let mut archive = Archive::open(
    "../../samples/test_tft.w3x",
    OpenArchiveFlags::MPQ_OPEN_READ_ONLY,
  )
  .unwrap();

  let names: Vec<String> = archive
    .find_files("*")
    .unwrap()
    .map(|r| r.unwrap().name)
    .collect();
  assert!(names.iter().any(|n| n == "war3map.j"));
  assert!(names.iter().any(|n| n == "(listfile)"));

  let js: Vec<FoundFile> = archive
    .find_files("*.j")
    .unwrap()
    .map(|r| r.unwrap())
    .collect();
  assert_eq!(js.len(), 1);
  assert_eq!(js[0].name, "war3map.j");
  assert_eq!(js[0].size, 14115);

  // No match yields an empty iterator, not an error.
  assert_eq!(archive.find_files("*.nope").unwrap().count(), 0);
}

#[test]
fn test_read() {
  // MPQ_OPEN_READ_ONLY: cargo test runs tests in parallel, and on
  // Windows concurrent read-write opens of the same file fail with
  // ERROR_SHARING_VIOLATION (32).
  let mut archive = Archive::open(
    "../../samples/test_tft.w3x",
    OpenArchiveFlags::MPQ_OPEN_NO_LISTFILE
      | OpenArchiveFlags::MPQ_OPEN_NO_ATTRIBUTES
      | OpenArchiveFlags::MPQ_OPEN_READ_ONLY,
  )
  .unwrap();

  assert!(!archive.has_file("invalid").unwrap());
  assert!(archive.has_file("war3map.j").unwrap());
  let mut f = archive.open_file("war3map.j").unwrap();
  assert_eq!(f.get_size().unwrap(), 14115);
  assert_eq!(
    f.read_all().unwrap(),
    std::fs::read("../../samples/war3map.j").unwrap()
  );
}

#[cfg(target_os = "windows")]
#[test]
fn test_read_unicode() {
  use std::os::windows::ffi::OsStringExt;
  use widestring::U16CString;
  let mut archive = Archive::open(
    OsString::from_wide(
      &U16CString::from_str("../../samples/中文.w3x")
        .unwrap()
        .into_vec(),
    ),
    OpenArchiveFlags::MPQ_OPEN_NO_LISTFILE
      | OpenArchiveFlags::MPQ_OPEN_NO_ATTRIBUTES
      | OpenArchiveFlags::MPQ_OPEN_READ_ONLY,
  )
  .unwrap();
  let mut f = archive.open_file("war3map.j").unwrap();
  assert_eq!(
    f.read_all().unwrap(),
    std::fs::read("../../samples/war3map.j").unwrap()
  );
}

#[cfg(target_os = "macos")]
#[test]
fn test_read_utf8() {
  let mut archive = Archive::open(
    "../../samples/中文.w3x",
    OpenArchiveFlags::MPQ_OPEN_NO_LISTFILE
      | OpenArchiveFlags::MPQ_OPEN_NO_ATTRIBUTES
      | OpenArchiveFlags::MPQ_OPEN_READ_ONLY,
  )
  .unwrap();
  let mut f = archive.open_file("war3map.j").unwrap();
  assert_eq!(
    f.read_all().unwrap(),
    std::fs::read("../../samples/war3map.j").unwrap()
  );
}
