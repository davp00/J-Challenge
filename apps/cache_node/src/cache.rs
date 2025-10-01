use std::{hash::Hash, sync::Arc};

use dashmap::{DashMap, Entry};

#[derive(Clone)]
pub(crate) struct CacheEntry<V> {
    pub value: Arc<V>,
    pub version: u64,
    pub expires_at: Option<u64>,
}

impl<V> CacheEntry<V> {
    #[inline]
    pub fn new(value: V, version: u64, expires_at: Option<u64>) -> Self {
        Self {
            value: Arc::new(value),
            version,
            expires_at,
        }
    }
}

pub(crate) struct Cache<K: Hash, V> {
    map: DashMap<K, CacheEntry<V>>,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    pub fn new() -> Arc<Self> {
        let this = Arc::new(Self {
            map: DashMap::new(),
        });

        this
    }

    pub fn put(&self, key: K, value: V, expires_at: Option<u64>) {
        match self.map.entry(key) {
            Entry::Occupied(mut occ) => {
                let next_ver = occ.get().version.saturating_add(1);
                *occ.get_mut() = CacheEntry::new(value, next_ver, expires_at);
            }
            Entry::Vacant(vac) => {
                vac.insert(CacheEntry::new(value, 1, expires_at));
            }
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_len() {
        let cache = Cache::<&str, &str>::new();
        assert_eq!(cache.len(), 0);

        cache.put("key1", "value1", None);
        assert_eq!(cache.len(), 1);

        cache.put("key2", "value2", Some(12345));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_overwrite_value() {
        let cache = Cache::<&str, &str>::new();
        cache.put("key1", "value1", None);
        assert_eq!(cache.len(), 1);

        // Sobrescribe la misma clave
        cache.put("key1", "value2", Some(100));
        assert_eq!(cache.len(), 1);

        let entry = cache.map.get("key1").unwrap();
        assert_eq!(entry.value.as_ref(), &"value2");
        assert_eq!(entry.version, 2);
        assert_eq!(entry.expires_at, Some(100));
    }
}
