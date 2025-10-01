use std::{hash::Hash, sync::Arc};

use dashmap::{DashMap, Entry};

use crate::utils::clock::{AppClock, AppTime, Clock};

#[derive(Clone)]
pub(crate) struct CacheEntry<V> {
    pub value: Arc<V>,
    pub version: u64,
    pub expires_at: Option<AppTime>,
}

impl<V> CacheEntry<V> {
    #[inline]
    pub fn new(value: V, version: u64, expires_at: Option<AppTime>) -> Self {
        Self {
            value: Arc::new(value),
            version,
            expires_at,
        }
    }
}

pub(crate) struct Cache<K: Hash, V> {
    map: DashMap<K, CacheEntry<V>>,
    clock: Arc<AppClock>,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    pub fn new() -> Arc<Self> {
        let this = Arc::new(Self {
            map: DashMap::new(),
            clock: Arc::new(AppClock::new()),
        });

        this
    }

    pub fn put(&self, key: K, value: V, expires_at: Option<AppTime>) {
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

    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        let now = self.clock.now_millis();

        if let Some(entry) = self.map.get(key) {
            if entry
                .expires_at
                .as_ref()
                .is_some_and(|exp| exp.is_before_or_eq(&now))
            {
                //drop(entry);
                //let _ = self.map.remove(key);
                return None;
            }
            return Some(entry.value.clone());
        }
        None
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_put_and_len() {
        let cache = Cache::<&str, &str>::new();
        assert_eq!(cache.len(), 0);

        cache.put("key1", "value1", None);
        assert_eq!(cache.len(), 1);

        cache.put("key2", "value2", Some(AppTime::new(12345)));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_overwrite_value() {
        let cache = Cache::<&str, &str>::new();
        cache.put("key1", "value1", None);
        assert_eq!(cache.len(), 1);

        // Sobrescribe la misma clave
        cache.put("key1", "value2", Some(AppTime::new(100)));
        assert_eq!(cache.len(), 1);

        let entry = cache.map.get("key1").unwrap();
        assert_eq!(entry.value.as_ref(), &"value2");
        assert_eq!(entry.version, 2);
        assert_eq!(entry.expires_at, Some(AppTime::new(100)));
    }

    #[test]
    fn get_returns_none_when_key_missing() {
        let cache = Cache::<&str, &str>::new();
        assert!(cache.get(&"missing").is_none());
    }

    #[test]
    fn get_returns_value_when_not_expired() {
        let cache = Cache::<&str, &str>::new();

        // Sin expiración -> debe devolver el valor
        cache.put("k1", "v1", None);

        let got = cache.get(&"k1").expect("value should exist");

        assert_eq!(&*got, &"v1");
    }

    #[test]
    fn get_returns_none_when_expired_in_past_and_does_not_remove() {
        let cache = Cache::<&str, &str>::new();

        cache.put("k2", "v2", Some(AppTime::new(0)));

        assert!(
            cache.get(&"k2").is_none(),
            "expired value should not be returned"
        );

        assert!(
            cache.map.contains_key(&"k2"),
            "entry stays in map because get() doesn't remove"
        );
    }

    #[test]
    fn get_returns_none_when_expiration_equals_now_boundary() {
        let cache = Cache::<&str, &str>::new();

        let now = cache.clock.now_millis();
        cache.put("k3", "v3", Some(now));

        assert!(
            cache.get(&"k3").is_none(),
            "value at exact now must be treated as expired (<=)"
        );
    }

    #[test]
    fn get_does_not_clone_value_contents_only_arc() {
        let cache = Cache::<&str, String>::new();
        cache.put("k4", "payload".to_string(), None);

        let a1 = cache.get(&"k4").unwrap();
        let a2 = cache.get(&"k4").unwrap();
        assert!(
            Arc::ptr_eq(&a1, &a2),
            "get() should clone only the Arc, not the inner data"
        );
        assert_eq!(&*a1, "payload");
    }
}
