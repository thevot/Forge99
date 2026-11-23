use std::collections::{HashSet, VecDeque};

const MAX_ENTRIES: usize = 2048;
const MAX_KEY_BYTES: usize = 1024;

/// Namespace-scoped negative lookups. FIFO admission bounds both entry count and
/// retained reference length; oversized references still use the normal resolver.
#[derive(Default)]
pub(super) struct MissingTextureCache {
    names: HashSet<String>,
    order: VecDeque<String>,
}

impl MissingTextureCache {
    pub(super) fn clear(&mut self) {
        self.names.clear();
        self.order.clear();
    }

    pub(super) fn resolve<T>(&mut self, key: &str, lookup: impl FnOnce() -> Option<T>) -> Option<T> {
        if key.len() > MAX_KEY_BYTES { return lookup(); }
        if self.names.contains(key) { return None; }
        let result = lookup();
        if result.is_none() {
            if self.order.len() == MAX_ENTRIES {
                if let Some(oldest) = self.order.pop_front() { self.names.remove(&oldest); }
            }
            self.names.insert(key.to_string());
            self.order.push_back(key.to_string());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_misses_call_the_resolver_only_once_until_invalidated() {
        let mut cache = MissingTextureCache::default(); let mut calls = 0;
        for _ in 0..100 {
            assert_eq!(cache.resolve("audit.missing", || { calls += 1; None::<usize> }), None);
        }
        assert_eq!(calls, 1);
        cache.clear();
        assert_eq!(cache.resolve("audit.missing", || { calls += 1; Some(42) }), Some(42));
        assert_eq!(calls, 2);
    }

    #[test]
    fn successes_are_not_negative_cache_entries() {
        let mut cache = MissingTextureCache::default(); let mut calls = 0;
        for _ in 0..2 { assert_eq!(cache.resolve("audit.valid", || { calls += 1; Some(7) }), Some(7)); }
        assert_eq!(calls, 2);
        assert!(cache.names.is_empty());
    }

    #[test]
    fn bounded_cache_retries_evicted_references_and_keeps_recent_misses() {
        let mut cache = MissingTextureCache::default();
        for index in 0..=MAX_ENTRIES { cache.resolve(&format!("audit.missing{index}"), || None::<()>); }
        assert_eq!(cache.names.len(), MAX_ENTRIES); assert_eq!(cache.order.len(), MAX_ENTRIES);
        let mut retried = false;
        cache.resolve(&format!("audit.missing{MAX_ENTRIES}"), || -> Option<()> { panic!("recent miss was not cached") });
        cache.resolve("audit.missing0", || { retried = true; None::<()> });
        assert!(retried); assert_eq!(cache.names.len(), MAX_ENTRIES);
    }

    #[test]
    fn oversized_keys_are_resolved_without_being_retained() {
        let mut cache = MissingTextureCache::default(); let mut calls = 0;
        let key = "x".repeat(MAX_KEY_BYTES + 1);
        for _ in 0..2 { cache.resolve(&key, || { calls += 1; None::<()> }); }
        assert_eq!(calls, 2); assert!(cache.names.is_empty()); assert!(cache.order.is_empty());
    }
}
