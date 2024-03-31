use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::slice;

use crate::OpaqueKey;

pub trait DbKey: OpaqueKey {}

impl DbKey for u8 {}
impl DbKey for u16 {}
impl DbKey for u32 {}
impl DbKey for u64 {}

#[derive(Clone, Debug, PartialEq)]
pub struct Database<T, I = u32> {
    lookup_table: Vec<*const T>,
    current: Vec<T>,
    archived: Vec<Vec<T>>,
    _unused: PhantomData<I>,
}

const DEFAULT_DATABASE_CAPACITY: usize = 16;

impl<T, I: DbKey> Default for Database<T, I> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_DATABASE_CAPACITY)
    }
}

impl<T, I: DbKey> Database<T, I> {
    pub fn with_capacity(capacity: usize) -> Self {
        let lookup_table = Vec::new();
        let current = Vec::with_capacity(capacity.next_power_of_two());
        let archived = Vec::new();

        Self {
            lookup_table,
            current,
            archived,
            _unused: PhantomData,
        }
    }

    pub fn entry(&mut self, entity: T) -> I {
        use std::mem;

        assert!(self.lookup_table.len() < I::max_index());

        if self.current.len() == self.current.capacity() {
            let new_capacity = self.current.capacity().next_power_of_two();
            let new_storage = Vec::with_capacity(new_capacity);
            let old_storage = mem::replace(&mut self.current, new_storage);
            self.archived.push(old_storage);
        }

        self.current.push(entity);

        let id = I::from_index(self.lookup_table.len());
        let ptr = {
            let index = self.current.len() - 1;
            &self.current[index] as *const T
        };
        self.lookup_table.push(ptr);

        id
    }

    fn lookup(&self, id: I) -> &T {
        let index = id.as_index();
        assert!(index < self.lookup_table.len());
        let ptr = self.lookup_table[index];

        // SAFETY: `ptr` is valid for the same lifetime as `self`.
        unsafe { &*ptr }
    }

    pub fn lookup_mut(&mut self, id: I) -> &mut T {
        let index = id.as_index();
        assert!(index < self.lookup_table.len());
        let ptr = self.lookup_table[index] as *mut T;

        // SAFETY: we have an exclusive reference to `self` (which implies
        // exclusive access to the value behind `ptr`) and `ptr` is valid for
        // the same lifetime as `self`.
        unsafe { &mut *ptr }
    }

    pub fn len(&self) -> usize {
        self.lookup_table.len()
    }

    pub fn iter(&self) -> DatabaseIter<'_, T, I> {
        DatabaseIter {
            iter: self.lookup_table.iter(),
            index: I::from_index(0),
        }
    }
}

impl<T, I: DbKey> Index<I> for Database<T, I> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.lookup(index)
    }
}

impl<T, I: DbKey> IndexMut<I> for Database<T, I> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.lookup_mut(index)
    }
}

pub struct DatabaseIter<'a, T, I> {
    iter: slice::Iter<'a, *const T>,
    index: I,
}

impl<'a, T, I: DbKey> Iterator for DatabaseIter<'a, T, I> {
    type Item = (I, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&ptr) = self.iter.next() {
            let index = self.index;
            self.index = I::from_index(index.as_index() + 1);

            // SAFETY: pointers in the database are valid for the lifetime of the database
            Some((index, unsafe { &*ptr }))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct DatabaseIterMut<'a, T, I> {
    iter: slice::Iter<'a, *mut T>,
    index: I,
    _database: &'a mut Database<T, I>,
}

impl<'a, T, I: DbKey> Iterator for DatabaseIterMut<'a, T, I> {
    type Item = (I, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&ptr) = self.iter.next() {
            let index = self.index;
            self.index = I::from_index(index.as_index() + 1);

            // SAFETY: pointers in the database are valid for the lifetime of the database
            // we also have an exclusive reference to the database (via `_database`) which
            // means is it safe to mutate values
            Some((index, unsafe { &mut *ptr }))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database() {
        #[derive(Debug, PartialEq)]
        struct Foo {
            bar: usize,
        }

        let mut db = Database::<Foo>::with_capacity(1);
        let test1 = db.entry(Foo { bar: 1 });
        let test2 = db.entry(Foo { bar: 2 });
        assert_eq!(db[test1], Foo { bar: 1 });
        assert_eq!(db[test2], Foo { bar: 2 });

        let test3 = db.entry(Foo { bar: 3 });
        assert_eq!(db[test1], Foo { bar: 1 });
        assert_eq!(db[test2], Foo { bar: 2 });
        assert_eq!(db[test3], Foo { bar: 3 });
    }
}
