use fnv::FnvHashMap as HashMap;

/// A key for looking up strings in the interner database
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct StringId(u32);

/// A [string interning](https://en.wikipedia.org/wiki/String_interning) database
///
/// Implementation based on <https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html>
#[derive(Default)]
pub struct Interner {
    strings: HashMap<&'static str, StringId>,
    table: Vec<&'static str>,
    current: String,
    filled: Vec<String>,
}

impl Interner {
    /// Create a new interner with at least `capacity`
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        Self {
            strings: HashMap::default(),
            table: vec![],
            current: String::with_capacity(capacity),
            filled: vec![],
        }
    }

    /// Add `string` to the interner database
    pub fn intern(&mut self, string: &str) -> StringId {
        if let Some(interned) = self.strings.get(string) {
            return *interned;
        }

        let interned = unsafe { self.alloc(string) };
        let id = StringId(self.table.len() as u32);
        self.strings.insert(interned, id);
        self.table.push(interned);

        id
    }

    /// Lookup `id` in the interner database
    ///
    /// Panics: if the `id` is not in the database
    pub fn lookup(&self, id: StringId) -> &str {
        debug_assert!((id.0 as usize) < self.table.len(), "id is not in database");
        self.table[id.0 as usize]
    }

    unsafe fn alloc(&mut self, string: &str) -> &'static str {
        let capacity = self.current.capacity();
        if capacity < self.current.len() + string.len() {
            let new_capacity = capacity.max(string.len() + 1).next_power_of_two();
            let new_buffer = String::with_capacity(new_capacity);
            let old_buffer = std::mem::replace(&mut self.current, new_buffer);
            self.filled.push(old_buffer);
        }

        let start = self.current.len();
        self.current.push_str(string);
        let interned = &self.current[start..];

        &*(interned as *const str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let mut interner = Interner::default();
        let id1 = interner.intern("test");
        let id2 = interner.intern("test2");
        let id3 = interner.intern("test");
        assert_eq!(interner.lookup(id1), "test");
        assert_eq!(interner.lookup(id2), "test2");
        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
    }
}
