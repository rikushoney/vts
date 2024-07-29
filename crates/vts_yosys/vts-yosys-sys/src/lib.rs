use std::ffi::{c_char, c_int};
use std::marker::{PhantomData, PhantomPinned};

#[repr(C)]
pub struct YosysDesign {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

extern "C" {
    pub fn vts_yosys_setup();

    pub fn vts_yosys_shutdown();

    pub fn vts_yosys_get_design() -> *mut YosysDesign;

    pub fn vts_yosys_run_pass(command: *const c_char, design: *mut YosysDesign);

    pub fn vts_yosys_run_frontend(
        filename: *const c_char,
        command: *const c_char,
        design: *mut YosysDesign,
    ) -> c_int;

    pub fn vts_yosys_run_backend(
        filename: *const c_char,
        command: *const c_char,
        design: *mut YosysDesign,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        unsafe {
            vts_yosys_setup();
            vts_yosys_shutdown();
        };
    }
}
