use std::ffi::{c_char, c_int};
use std::marker::{PhantomData, PhantomPinned};

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct Abc_Frame_t {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

extern "C" {
    pub fn Abc_Start();
    pub fn Abc_Stop();
    pub fn Abc_FrameGetGlobalFrame() -> *mut Abc_Frame_t;
    pub fn Cmd_CommandExecute(framework: *mut Abc_Frame_t, command: *const c_char) -> c_int;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        unsafe {
            Abc_Start();
            Abc_Stop();
        }
    }
}
