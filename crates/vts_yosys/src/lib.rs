use std::ffi::{c_char, CString};
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("an instance of Yosys already exists")]
    InstanceExists,
}

pub type Result<T> = std::result::Result<T, Error>;

static YOSYS_LOCKED: AtomicBool = AtomicBool::new(false);

pub struct Yosys;

fn yosys_setup() {
    unsafe { vts_yosys_sys::vts_yosys_setup() }
}

fn yosys_shutdown() {
    unsafe { vts_yosys_sys::vts_yosys_shutdown() }
}

fn yosys_run_pass(command: *const c_char) {
    unsafe { vts_yosys_sys::vts_yosys_run_pass(command, std::ptr::null_mut()) }
}

fn yosys_run_frontend(filename: *const c_char, command: *const c_char) -> i32 {
    unsafe { vts_yosys_sys::vts_yosys_run_frontend(filename, command, std::ptr::null_mut()) }
}

fn yosys_run_backend(filename: *const c_char, command: *const c_char) {
    unsafe { vts_yosys_sys::vts_yosys_run_backend(filename, command, std::ptr::null_mut()) }
}

impl Yosys {
    pub fn new() -> Result<Self> {
        let locked = YOSYS_LOCKED.swap(true, Ordering::SeqCst);
        if !locked {
            yosys_setup();
            Ok(Self)
        } else {
            Err(Error::InstanceExists)
        }
    }

    pub(crate) fn run_pass(&self, command: &str) {
        let command = CString::new(command).expect("command should not contain nul bytes");
        yosys_run_pass(command.as_ptr());
    }

    pub(crate) fn run_frontend(&self, filename: &str, command: &str) -> i32 {
        let filename = CString::new(filename).expect("filename should not contain nul bytes");
        let command = CString::new(command).expect("command should not contain nul bytes");
        yosys_run_frontend(filename.as_ptr(), command.as_ptr())
    }

    pub(crate) fn run_backend(&self, filename: &str, command: &str) {
        let filename = CString::new(filename).expect("filename should not contain nul bytes");
        let command = CString::new(command).expect("command should not contain nul bytes");
        yosys_run_backend(filename.as_ptr(), command.as_ptr());
    }
}

impl Drop for Yosys {
    fn drop(&mut self) {
        yosys_shutdown();
        let was_locked = YOSYS_LOCKED.swap(false, Ordering::SeqCst);
        debug_assert!(was_locked);
    }
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use super::*;

    #[test]
    fn test_yosys_new() {
        let _yosys = Yosys::new().unwrap();
    }

    #[test]
    fn test_yosys_is_not_threadsafe() {
        {
            let _yosys = Yosys::new().unwrap();
            assert!(matches!(Yosys::new(), Err(Error::InstanceExists)));
        }
        let _yosys = Yosys::new().unwrap();
    }
}
