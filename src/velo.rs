use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;

#[link(name = "velo_lite")]
extern "C" {
    fn velo_init(path: *const c_char) -> *mut c_void;
    fn velo_set(db: *mut c_void, key: *const c_char, value: *const c_char) -> c_int;
    fn velo_get(db: *mut c_void, key: *const c_char) -> *mut c_char;
    #[allow(dead_code)]
    fn velo_delete(db: *mut c_void, key: *const c_char) -> c_int;
    fn velo_count(db: *mut c_void) -> u64;
    fn velo_free_string(ptr: *mut c_char);
    fn velo_last_error_message() -> *mut c_char;
}

pub struct VeloDb {
    handle: *mut c_void,
    _path: CString,
}

impl VeloDb {
    pub fn open(path: &str) -> Result<Self, String> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
        }
        let c_path =
            CString::new(path).map_err(|_| "database path contains NUL byte".to_string())?;
        let handle = unsafe { velo_init(c_path.as_ptr()) };
        if handle.is_null() {
            return Err(
                last_error().unwrap_or_else(|| "failed to initialize Velo-Lite".to_string())
            );
        }
        Ok(Self {
            handle,
            _path: c_path,
        })
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let key = CString::new(key).map_err(|_| "key contains NUL byte".to_string())?;
        let value = CString::new(value).map_err(|_| "value contains NUL byte".to_string())?;
        let result = unsafe { velo_set(self.handle, key.as_ptr(), value.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(last_error().unwrap_or_else(|| "Velo-Lite set failed".to_string()))
        }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let key = CString::new(key).map_err(|_| "key contains NUL byte".to_string())?;
        let ptr = unsafe { velo_get(self.handle, key.as_ptr()) };
        if ptr.is_null() {
            return Ok(None);
        }
        let value = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
        unsafe { velo_free_string(ptr) };
        Ok(Some(value))
    }

    #[allow(dead_code)]
    pub fn delete(&self, key: &str) -> Result<(), String> {
        let key = CString::new(key).map_err(|_| "key contains NUL byte".to_string())?;
        let result = unsafe { velo_delete(self.handle, key.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(last_error().unwrap_or_else(|| "Velo-Lite delete failed".to_string()))
        }
    }

    #[allow(dead_code)]
    pub fn count(&self) -> u64 {
        unsafe { velo_count(self.handle) }
    }
}

fn last_error() -> Option<String> {
    let ptr = unsafe { velo_last_error_message() };
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe { velo_free_string(ptr) };
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
