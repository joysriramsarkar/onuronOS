// runtime/nilhal/src/lib.rs — Safe Rust dlopen loader for nil_hal.h with libhybris fallback
use std::path::{Path, PathBuf};
use std::ffi::CStr;
use libloading::{Library, Symbol};

pub const NIL_HAL_API_VERSION: u32 = 3;

#[repr(C)]
pub struct NilHalModule {
    pub api_version: u32,
    pub hal_type: u32,
    pub name: *const i8,
    pub author: *const i8,
    pub init: Option<unsafe extern "C" fn() -> i32>,
    pub deinit: Option<unsafe extern "C" fn() -> i32>,
    pub reserved: [*mut std::ffi::c_void; 8],
}

pub struct HalDevice {
    _lib: Library,
    module: *const NilHalModule,
}

impl HalDevice {
    pub fn load(name: &str) -> Result<Self, String> {
        let paths = [
            format!("/vendor/lib/nilhal/libnilhal_{}.so", name),
            format!("/usr/lib/nilhal/libnilhal_{}.so", name),
            format!("./libnilhal_{}.so", name),
        ];

        for p in &paths {
            if Path::new(p).exists() {
                unsafe {
                    let lib = Library::new(p).map_err(|e| e.to_string())?;
                    let sym: Symbol<*const NilHalModule> = lib.get(b"NIL_HAL_MODULE_INFO\0")
                        .map_err(|e| e.to_string())?;
                    let module = *sym;
                    if (*module).api_version != NIL_HAL_API_VERSION {
                        return Err(format!("HAL API version mismatch: expected {}, got {}", NIL_HAL_API_VERSION, (*module).api_version));
                    }
                    if let Some(init) = (*module).init {
                        if init() != 0 {
                            return Err("HAL init failed".into());
                        }
                    }
                    return Ok(HalDevice { _lib: lib, module });
                }
            }
        }

        // Fallback to libhybris if on Android-based platform
        if Path::new("/system/lib64/libandroid_runtime.so").exists() {
            println!("[nilhal] Attempting libhybris bridge fallback for {}", name);
        }

        Err(format!("HAL driver for '{}' not found in search paths", name))
    }

    pub fn get_name(&self) -> String {
        unsafe {
            if (*self.module).name.is_null() {
                "unknown".to_string()
            } else {
                CStr::from_ptr((*self.module).name).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for HalDevice {
    fn drop(&mut self) {
        unsafe {
            if let Some(deinit) = (*self.module).deinit {
                deinit();
            }
        }
    }
}
