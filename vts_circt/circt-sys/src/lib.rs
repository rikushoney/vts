#![allow(dead_code)]

#[link(name = "circt-sys-wrapper")]
extern "C" {
    fn it_works();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        unsafe { super::it_works() }
    }
}
