use std::ops::Index;

use fnv::FnvHashMap as HashMap;

use crate::OpaqueKey;

pub trait TableKey: OpaqueKey {}

impl TableKey for u8 {}
impl TableKey for u16 {}
impl TableKey for u32 {}
impl TableKey for u64 {}

// Based on https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html

#[derive(Clone, Debug, PartialEq)]
pub struct StringTable<I = u32> {
    str_key_map: HashMap<&'static str, I>,
    lookup_table: Vec<&'static str>,
    storage: String,
    archived: Vec<String>,
}

const DEFAULT_TABLE_CAPACITY: usize = 16;

impl<I: TableKey> Default for StringTable<I> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_TABLE_CAPACITY)
    }
}

impl<I: TableKey> StringTable<I> {
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
        if string.is_empty() {
            return I::from_index(0);
        }

        if let Some(&interned) = self.str_key_map.get(string) {
            return interned;
        }

        assert!(self.lookup_table.len() < I::max_index());

        // SAFETY: `interned` is not shared outside of `self` as 'static
        let interned = unsafe { self.alloc(string) };
        let key = I::from_index(self.lookup_table.len()).increment();
        self.str_key_map.insert(interned, key);
        self.lookup_table.push(interned);

        key
    }

    fn lookup(&self, key: I) -> &str {
        let key = key.as_index();
        if key == 0 {
            return "";
        }

        let key = key - 1;
        assert!(key <= self.lookup_table.len());

        self.lookup_table[key]
    }

    pub fn rlookup(&self, string: &str) -> Option<I> {
        if string.is_empty() {
            return Some(I::from_index(0));
        }

        self.str_key_map.get(string).map(|&key| key)
    }

    /// # Safety
    /// The caller must ensure that the returned string reference does not outlive `self`
    unsafe fn alloc(&mut self, string: &str) -> &'static str {
        use std::cmp;
        use std::mem;

        debug_assert!(!string.is_empty());

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

impl<I: TableKey> Index<I> for StringTable<I> {
    type Output = str;

    fn index(&self, index: I) -> &Self::Output {
        self.lookup(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let mut table = StringTable::<u32>::with_capacity(1);
        let id1 = table.entry("test");
        let id2 = table.entry("test2");
        assert_eq!(&table[id1], "test");
        assert_eq!(&table[id2], "test2");

        let id1_copy = table.entry("test");
        assert_eq!(id1, id1_copy);
        assert_ne!(id1, id2);

        let id3 = table.entry("test3");
        assert_eq!(&table[id3], "test3");
        assert_eq!(&table[id1], "test");
        assert_eq!(&table[id2], "test2");

        assert_eq!(table.entry(""), 0);
        assert!(&table[0].is_empty());
    }
}
