extern "C" {
    pub fn smoke_test();
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test() {
        unsafe { super::smoke_test() };
    }
}
