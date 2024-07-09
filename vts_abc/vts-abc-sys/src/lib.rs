extern "C" {
    pub fn Abc_Start();
    pub fn Abc_Stop();
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
