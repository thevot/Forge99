//! Vector wrapper with a monotonic revision counter bumped on every mutable access.

use std::borrow::Borrow;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Revision 0 is reserved by callers as a sentinel for "uninitialized / absent".
static NEXT_TRACKED_VEC_REV: AtomicU64 = AtomicU64::new(1);

#[inline]
fn next_rev() -> u64 {
    NEXT_TRACKED_VEC_REV.fetch_add(1, Ordering::Relaxed)
}

/// A Copy-on-Write (`Arc<Vec<T>>`) wrapper whose monotonic revision advances on every mutation or replacement.
#[derive(Debug)]
pub struct TrackedVec<T> {
    inner: Arc<Vec<T>>,
    rev: u64,
}

impl<T> TrackedVec<T> {
    /// Creates an empty `TrackedVec` with a fresh revision.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Vec::new()),
            rev: next_rev(),
        }
    }

    /// Creates an empty `TrackedVec` with preallocated capacity and a fresh revision.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Vec::with_capacity(capacity)),
            rev: next_rev(),
        }
    }

    /// Returns an O(1) immutable Copy-on-Write handle without bumping `rev`.
    #[inline]
    pub fn shared_snapshot(&self) -> Arc<Vec<T>> {
        Arc::clone(&self.inner)
    }

    /// Returns the number of strong owners sharing the underlying buffer.
    #[inline]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Returns the current monotonic revision without modifying it.
    #[inline]
    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Explicitly marks this collection as modified.
    #[inline]
    pub fn bump(&mut self) {
        self.rev = next_rev();
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the allocated capacity in elements.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Borrows the underlying elements as an immutable slice without bumping `rev`.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    /// Returns an immutable iterator over the elements without bumping `rev`.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    /// Consumes the wrapper and returns the inner `Vec<T>`.
    #[inline]
    pub fn into_vec(self) -> Vec<T>
    where
        T: Clone,
    {
        Arc::unwrap_or_clone(self.inner)
    }

    /// Clones elements into a plain `Vec<T>` without bumping `rev`.
    #[inline]
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        (*self.inner).clone()
    }

    /// Appends an element to the back of the collection and bumps `rev`.
    #[inline]
    pub fn push(&mut self, value: T)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).push(value);
    }

    /// Removes the last element from the collection and returns it, bumping `rev`.
    #[inline]
    pub fn pop(&mut self) -> Option<T>
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).pop()
    }

    /// Clears the vector, removing all values, and bumps `rev`.
    #[inline]
    pub fn clear(&mut self)
    where
        T: Clone,
    {
        self.bump();
        if Arc::strong_count(&self.inner) > 1 {
            self.inner = Arc::new(Vec::new());
        } else {
            Arc::make_mut(&mut self.inner).clear();
        }
    }

    /// Removes and returns the element at `index`, preserving order, and bumps `rev`.
    #[inline]
    pub fn remove(&mut self, index: usize) -> T
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).remove(index)
    }

    /// Removes and returns the element at `index` by swapping with the last element, bumping `rev`.
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).swap_remove(index)
    }

    /// Inserts `element` at `index`, shifting subsequent elements right, and bumps `rev`.
    #[inline]
    pub fn insert(&mut self, index: usize, element: T)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).insert(index, element);
    }

    /// Retains only the elements specified by the predicate and bumps `rev`.
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).retain(f);
    }

    /// Retains only the elements specified by the mutable predicate and bumps `rev`.
    #[inline]
    pub fn retain_mut<F>(&mut self, f: F)
    where
        T: Clone,
        F: FnMut(&mut T) -> bool,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).retain_mut(f);
    }

    /// Clones and appends all elements in a slice to the collection and bumps `rev`.
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).extend_from_slice(other);
    }

    /// Shortens the vector, keeping the first `len` elements, and bumps `rev`.
    #[inline]
    pub fn truncate(&mut self, len: usize)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).truncate(len);
    }

    /// Moves all elements of `other` into `self` and bumps `rev`.
    #[inline]
    pub fn append(&mut self, other: &mut Vec<T>)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).append(other);
    }

    /// Removes the specified range from the vector in bulk, returning the removed items and bumping `rev`.
    #[inline]
    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, T>
    where
        T: Clone,
        R: RangeBounds<usize>,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).drain(range)
    }

    /// Resizes the vector in-place to `new_len` using `value` and bumps `rev`.
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).resize(new_len, value);
    }

    /// Resizes the vector in-place to `new_len` using closure `f` and bumps `rev`.
    #[inline]
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        T: Clone,
        F: FnMut() -> T,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).resize_with(new_len, f);
    }

    /// Creates a splicing iterator replacing `range` with `replace_with` and bumps `rev`.
    #[inline]
    pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> std::vec::Splice<'_, I::IntoIter>
    where
        T: Clone,
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).splice(range, replace_with)
    }

    /// Removes consecutive repeated elements and bumps `rev`.
    #[inline]
    pub fn dedup(&mut self)
    where
        T: Clone + PartialEq,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).dedup();
    }

    /// Removes consecutive elements matching `same_bucket` and bumps `rev`.
    #[inline]
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        T: Clone,
        F: FnMut(&mut T, &mut T) -> bool,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).dedup_by(same_bucket);
    }

    /// Removes consecutive elements with equal mapped keys and bumps `rev`.
    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, key: F)
    where
        T: Clone,
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).dedup_by_key(key);
    }

    /// Splits the collection at `at`, returning elements `[at..]` and bumping `rev`.
    #[inline]
    pub fn split_off(&mut self, at: usize) -> Vec<T>
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).split_off(at)
    }

    /// Shrinks capacity as much as possible and bumps `rev`.
    #[inline]
    pub fn shrink_to_fit(&mut self)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).shrink_to_fit();
    }

    /// Reserves capacity for at least `additional` more elements and bumps `rev`.
    #[inline]
    pub fn reserve(&mut self, additional: usize)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).reserve(additional);
    }

    /// Reserves the minimum capacity for `additional` more elements and bumps `rev`.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize)
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).reserve_exact(additional);
    }

    /// Returns a mutable slice of all elements and bumps `rev`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T]
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner).as_mut_slice()
    }

    /// Replaces the backing `Vec<T>`, returns the previous contents, and bumps `rev`.
    #[inline]
    pub fn replace(&mut self, vec: Vec<T>) -> Vec<T>
    where
        T: Clone,
    {
        self.bump();
        let prev = std::mem::replace(&mut self.inner, Arc::new(vec));
        Arc::unwrap_or_clone(prev)
    }

    /// Exposes the backing `&mut Vec<T>` directly after bumping `rev`.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut Vec<T>
    where
        T: Clone,
    {
        self.bump();
        Arc::make_mut(&mut self.inner)
    }
}

impl<T> Default for TrackedVec<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for TrackedVec<T> {
    #[inline]
    fn clone(&self) -> Self {
        // O(1) CoW handle with a fresh revision so restoring a snapshot cannot alias an older cached revision.
        Self {
            inner: Arc::clone(&self.inner),
            rev: next_rev(),
        }
    }
}

impl<T> From<Vec<T>> for TrackedVec<T> {
    #[inline]
    fn from(inner: Vec<T>) -> Self {
        Self {
            inner: Arc::new(inner),
            rev: next_rev(),
        }
    }
}

impl<T: Clone> From<TrackedVec<T>> for Vec<T> {
    #[inline]
    fn from(tracked: TrackedVec<T>) -> Self {
        Arc::unwrap_or_clone(tracked.inner)
    }
}

impl<T> FromIterator<T> for TrackedVec<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            inner: Arc::new(Vec::from_iter(iter)),
            rev: next_rev(),
        }
    }
}

impl<T> Deref for TrackedVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Clone> DerefMut for TrackedVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bump();
        Arc::make_mut(&mut self.inner).as_mut_slice()
    }
}

impl<T> AsRef<[T]> for TrackedVec<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        &self.inner
    }
}

impl<T> Borrow<[T]> for TrackedVec<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        &self.inner
    }
}

impl<T, I> Index<I> for TrackedVec<T>
where
    Vec<T>: Index<I>,
{
    type Output = <Vec<T> as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T: Clone, I> IndexMut<I> for TrackedVec<T>
where
    Vec<T>: IndexMut<I>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.bump();
        &mut Arc::make_mut(&mut self.inner)[index]
    }
}

impl<'a, T> IntoIterator for &'a TrackedVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut TrackedVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.bump();
        Arc::make_mut(&mut self.inner).iter_mut()
    }
}

impl<T: Clone> IntoIterator for TrackedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.inner).into_iter()
    }
}

impl<T: Clone> Extend<T> for TrackedVec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.bump();
        Arc::make_mut(&mut self.inner).extend(iter);
    }
}

impl<'a, T: Copy + 'a> Extend<&'a T> for TrackedVec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.bump();
        Arc::make_mut(&mut self.inner).extend(iter);
    }
}

impl<T: PartialEq> PartialEq for TrackedVec<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        *self.inner == *other.inner
    }
}

impl<T: Eq> Eq for TrackedVec<T> {}

impl<T: PartialEq> PartialEq<Vec<T>> for TrackedVec<T> {
    #[inline]
    fn eq(&self, other: &Vec<T>) -> bool {
        &*self.inner == other
    }
}

impl<T: PartialEq> PartialEq<TrackedVec<T>> for Vec<T> {
    #[inline]
    fn eq(&self, other: &TrackedVec<T>) -> bool {
        self == &*other.inner
    }
}

impl<T: PartialEq> PartialEq<[T]> for TrackedVec<T> {
    #[inline]
    fn eq(&self, other: &[T]) -> bool {
        self.inner.as_slice() == other
    }
}

impl<T: PartialEq> PartialEq<&[T]> for TrackedVec<T> {
    #[inline]
    fn eq(&self, other: &&[T]) -> bool {
        self.inner.as_slice() == *other
    }
}

impl<T: Serialize> Serialize for TrackedVec<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for TrackedVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<T>::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_operations_preserve_revision() {
        let tv: TrackedVec<i32> = vec![10, 20, 30].into();
        let rev0 = tv.rev();
        assert_ne!(rev0, 0);
        assert_eq!(tv.len(), 3);
        assert!(!tv.is_empty());
        assert!(tv.capacity() >= 3);
        assert_eq!(tv.as_slice(), &[10, 20, 30]);
        assert_eq!(tv.get(1), Some(&20));
        assert_eq!(tv[2], 30);
        let sum: i32 = tv.iter().sum();
        assert_eq!(sum, 60);
        let mut iterated = Vec::new();
        for &item in &tv {
            iterated.push(item);
        }
        assert_eq!(iterated, vec![10, 20, 30]);
        assert_eq!(tv.to_vec(), vec![10, 20, 30]);
        assert_eq!(tv.rev(), rev0);
    }

    #[test]
    fn mutating_operations_advance_revision() {
        let mut tv: TrackedVec<i32> = TrackedVec::new();
        let mut last_rev = tv.rev();

        let assert_bumped = |tv: &TrackedVec<i32>, last: &mut u64| {
            let cur = tv.rev();
            assert!(cur > *last, "expected rev {cur} > {last}");
            *last = cur;
        };

        tv.push(1);
        assert_bumped(&tv, &mut last_rev);

        tv.insert(1, 2);
        assert_bumped(&tv, &mut last_rev);

        tv.extend([3, 4]);
        assert_bumped(&tv, &mut last_rev);

        tv[0] = 10;
        assert_bumped(&tv, &mut last_rev);

        if let Some(slot) = tv.get_mut(1) {
            *slot = 20;
        }
        assert_bumped(&tv, &mut last_rev);

        for item in tv.iter_mut() {
            *item += 1;
        }
        assert_bumped(&tv, &mut last_rev);

        for item in &mut tv {
            *item += 1;
        }
        assert_bumped(&tv, &mut last_rev);

        tv.retain(|&x| x >= 20);
        assert_bumped(&tv, &mut last_rev);

        let _ = tv.pop();
        assert_bumped(&tv, &mut last_rev);

        tv.push(99);
        tv.push(100);
        last_rev = tv.rev();
        let _ = tv.remove(0);
        assert_bumped(&tv, &mut last_rev);

        tv.clear();
        assert_bumped(&tv, &mut last_rev);

        let prev = tv.replace(vec![7, 8, 9]);
        assert!(prev.is_empty());
        assert_bumped(&tv, &mut last_rev);
    }

    #[test]
    fn constructors_assign_distinct_revs_and_compare_contents() {
        let a: TrackedVec<i32> = TrackedVec::default();
        let b: TrackedVec<i32> = Vec::<i32>::new().into();
        let c: TrackedVec<i32> = [1, 2, 3].into_iter().collect();
        let d = c.clone();
        assert_ne!(a.rev(), b.rev());
        assert_ne!(b.rev(), c.rev());
        assert_ne!(c.rev(), d.rev());
        assert_eq!(a, b);
        assert_eq!(c, d);
        assert_eq!(c, vec![1, 2, 3]);
    }

    #[test]
    fn cow_shared_snapshot_is_zero_copy_until_mutation() {
        let mut tv: TrackedVec<i32> = vec![10, 20, 30].into();
        let rev0 = tv.rev();
        assert_eq!(tv.strong_count(), 1);
        let ptr_before = tv.as_slice().as_ptr();

        // Unique mutation reuses buffer in-place.
        tv.push(40);
        assert_eq!(tv.strong_count(), 1);
        let ptr_after_unique = tv.as_slice().as_ptr();
        assert_eq!(tv.as_slice(), &[10, 20, 30, 40]);

        // Snapshot shares buffer in O(1) without bumping rev.
        let rev1 = tv.rev();
        let snap = tv.shared_snapshot();
        assert_eq!(tv.rev(), rev1);
        assert_ne!(rev1, rev0);
        assert_eq!(tv.strong_count(), 2);
        assert!(std::ptr::eq(snap.as_ptr(), ptr_after_unique));
        let cloned = tv.clone();
        assert_eq!(tv.strong_count(), 3);
        assert!(std::ptr::eq(cloned.as_slice().as_ptr(), ptr_after_unique));
        drop(cloned);
        assert_eq!(tv.strong_count(), 2);

        // Mutating while snapshot is alive detaches to a new backing Vec, keeping snapshot intact.
        tv[0] = 999;
        assert_eq!(tv.strong_count(), 1);
        assert!(!std::ptr::eq(tv.as_slice().as_ptr(), ptr_after_unique));
        assert_eq!(tv.as_slice(), &[999, 20, 30, 40]);
        assert_eq!(snap.as_slice(), &[10, 20, 30, 40]);
        let _ = ptr_before;
    }
}
