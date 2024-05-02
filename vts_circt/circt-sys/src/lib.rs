#![allow(dead_code)]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

include!("bindings.rs");

#[cfg(test)]
mod tests {
    use std::ffi::{c_char, CString};

    #[test]
    fn it_works() {
        unsafe { super::it_works() };
        let input = CString::new("test.mlir").unwrap();
        unsafe { super::simplify(input.as_ptr() as *const c_char) };
    }
}
