use fnv::FnvHashMap as HashMap;

use crate::OpaqueKey;

pub trait TableKey: OpaqueKey {}

impl TableKey for u8 {}
impl TableKey for u16 {}
impl TableKey for u32 {}
impl TableKey for u64 {}

// Based on https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html

pub struct SymbolTable<I = u32> {
    str_key_map: HashMap<&'static str, I>,
    lookup_table: Vec<&'static str>,
    storage: String,
    archived: Vec<String>,
}

impl<I: Clone + TableKey> SymbolTable<I> {
    pub fn with_capacity(capacity: usize) -> Self {
        let str_key_map = HashMap::default();
        let lookup_table = Vec::new();
        let storage = String::with_capacity(capacity.next_power_of_two());
        let archived = Vec::new();

        Self {
            str_key_map,
            lookup_table,
            storage,
            archived,
        }
    }

    pub fn entry(&mut self, string: &str) -> I {
        if let Some(interned) = self.str_key_map.get(string) {
            return interned.clone();
        }

        assert!(self.lookup_table.len() <= I::max_index());

        // SAFETY: `interned` is not shared outself of `self` as `'static`
        let interned = unsafe { self.alloc(string) };
        let key = I::from_index(self.lookup_table.len());
        self.str_key_map.insert(interned, key.clone());
        self.lookup_table.push(interned);

        key
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn lookup<'a>(&'a self, key: I) -> &'a str {
        let key = key.as_index();
        assert!(key < self.lookup_table.len());

        self.lookup_table[key]
    }

    /// # Safety
    /// The caller must ensure that the returned string reference does not outlive `self`
    unsafe fn alloc(&mut self, string: &str) -> &'static str {
        use std::cmp;
        use std::mem;

        let capacity = self.storage.capacity();
        if capacity < self.storage.len() + string.len() {
            let new_capacity = cmp::max(capacity, string.len() + 1).next_power_of_two();
            let new_storage = String::with_capacity(new_capacity);
            let old_storage = mem::replace(&mut self.storage, new_storage);
            self.archived.push(old_storage);
        }

        let start = self.storage.len();
        self.storage.push_str(string);
        let interned = &self.storage[start..];

        &*(interned as *const str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let mut table = SymbolTable::<u32>::with_capacity(1);
        let id1 = table.entry("test");
        let id2 = table.entry("test2");
        assert_eq!(table.lookup(id1), "test");
        assert_eq!(table.lookup(id2), "test2");

        let id1_copy = table.entry("test");
        assert_eq!(id1, id1_copy);
        assert_ne!(id1, id2);

        let id3 = table.entry("test3");
        assert_eq!(table.lookup(id3), "test3");
        assert_eq!(table.lookup(id1), "test");
        assert_eq!(table.lookup(id2), "test2");
    }
}
