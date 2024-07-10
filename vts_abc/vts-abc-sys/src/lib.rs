use std::ffi::{c_char, c_int};
use std::marker::{PhantomData, PhantomPinned};

#[repr(C)]
pub struct AbcFrame {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

extern "C" {
    #[link_name = "Abc_Start"]
    pub fn abc_start();

    #[link_name = "Abc_Stop"]
    pub fn abc_stop();

    #[link_name = "Abc_FrameGetGlobalFrame"]
    pub fn abc_get_global_frame() -> *mut AbcFrame;

    #[link_name = "Cmd_CommandExecute"]
    pub fn abc_execute_command(framework: *mut AbcFrame, command: *const c_char) -> c_int;

    #[link_name = "Abc_FrameSetLutLibrary"]
    pub fn abc_frame_set_lut_library(framework: *mut AbcFrame, library: *const c_char) -> c_int;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        unsafe {
            abc_start();
            abc_stop();
        }
    }
}
