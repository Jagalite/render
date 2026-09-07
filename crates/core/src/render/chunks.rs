//! Disposable immutable evaluated storage. Runtime slots are not durable IDs.
use crate::{Error, Result};
use std::{collections::BTreeMap, ops::Index, sync::Arc};

pub const CHUNK_LEN: usize = 64;

#[derive(Debug)]
pub struct Chunks<T> {
    root: Arc<Vec<Arc<[T]>>>,
    len: usize,
}
impl<T> Clone for Chunks<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            len: self.len,
        }
    }
}
impl<T> Default for Chunks<T> {
    fn default() -> Self {
        Self {
            root: Arc::new(Vec::new()),
            len: 0,
        }
    }
}
impl<T> From<Vec<T>> for Chunks<T> {
    fn from(values: Vec<T>) -> Self {
        let len = values.len();
        let mut values = values.into_iter();
        let mut root = Vec::with_capacity(len.div_ceil(CHUNK_LEN));
        while values.len() != 0 {
            root.push(Arc::from(
                values.by_ref().take(CHUNK_LEN).collect::<Vec<_>>(),
            ));
        }
        Self {
            root: Arc::new(root),
            len,
        }
    }
}
impl<T> Chunks<T> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn chunk_count(&self) -> usize {
        self.root.len()
    }
    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        Some(&self.root[index / CHUNK_LEN][index % CHUNK_LEN])
    }
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            values: self,
            front: 0,
            back: self.len,
        }
    }
    /// Payload and root table layout, excluding Self, nested owned allocations,
    /// Arc headers and allocator overhead. Shared allocations are counted once
    /// for this version, not deduplicated across versions.
    pub fn retained_layout_bytes(&self) -> usize {
        size_of::<Vec<Arc<[T]>>>()
            + self.root.capacity() * size_of::<Arc<[T]>>()
            + self.len * size_of::<T>()
    }
    pub fn shares_root(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.root, &other.root)
    }
    pub fn shared_chunks(&self, other: &Self) -> usize {
        self.root
            .iter()
            .zip(other.root.iter())
            .filter(|(a, b)| Arc::ptr_eq(a, b))
            .count()
    }
}
impl<T> Index<usize> for Chunks<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.get(index).expect("runtime chunk index out of bounds")
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyCost {
    pub root_references: usize,
    pub chunks: usize,
    /// Includes overwritten old values. Nested allocations follow T::clone.
    pub elements: usize,
}
impl<T: Clone> Chunks<T> {
    /// Atomically construct a replacement version. The caller owns revision and
    /// semantic validation; this internal storage seam only admits existing slots.
    /// Callers must bound updates and their nested payloads before this operation.
    pub fn replaced(
        &self,
        updates: BTreeMap<usize, T>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(Self, CopyCost)> {
        let mut check = || {
            if cancelled() {
                Err(Error::new(
                    "cancelled",
                    "runtime chunk replacement cancelled",
                ))
            } else {
                Ok(())
            }
        };
        check()?;
        for &index in updates.keys() {
            check()?;
            if index >= self.len {
                return Err(Error::new(
                    "index",
                    "runtime replacement index is out of bounds",
                ));
            }
        }
        if updates.is_empty() {
            return Ok((self.clone(), CopyCost::default()));
        }
        let mut root = self.root.as_ref().clone();
        let mut cost = CopyCost {
            root_references: root.len(),
            ..CopyCost::default()
        };
        let mut updates = updates.into_iter().peekable();
        while let Some(&(index, _)) = updates.peek() {
            check()?;
            let chunk = index / CHUNK_LEN;
            let mut values = root[chunk].to_vec();
            cost.chunks += 1;
            cost.elements += values.len();
            while updates.peek().is_some_and(|(i, _)| i / CHUNK_LEN == chunk) {
                check()?;
                let (i, value) = updates.next().unwrap();
                values[i % CHUNK_LEN] = value;
            }
            root[chunk] = Arc::from(values);
        }
        check()?;
        Ok((
            Self {
                root: Arc::new(root),
                len: self.len,
            },
            cost,
        ))
    }
}

pub struct Iter<'a, T> {
    values: &'a Chunks<T>,
    front: usize,
    back: usize,
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        let i = self.front;
        self.front += 1;
        Some(&self.values[i])
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}
impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(&self.values[self.back])
    }
}
impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> std::iter::FusedIterator for Iter<'_, T> {}
impl<'a, T> IntoIterator for &'a Chunks<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
