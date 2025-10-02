use std::{hash::Hash, sync::Arc};

use app_core::clock::{AppClock, AppTime, Clock};
use dashmap::{DashMap, Entry};
use parking_lot::Mutex;
use tokio::time;

use crate::core::services::cache::{lru::LruState, timing_wheel::TimingWheel};

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

pub struct Cache<K: Eq + Hash + Clone + Send + Sync + 'static, V: Send + Sync + 'static> {
    map: DashMap<K, CacheEntry<V>>,
    clock: Arc<AppClock>,
    lru: Mutex<LruState<K>>,
    wheel: TimingWheel<K>,
}

impl<K: Eq + Hash + Clone + Send + Sync + 'static, V: Send + Sync + 'static> Cache<K, V> {
    pub fn new_with_capacity(capacity: usize, wheel_size: usize, tick_ms: u64) -> Arc<Self> {
        assert!(capacity > 0, "capacity must be > 0");

        let clock = Arc::new(AppClock::new());
        let now = clock.now_millis().as_millis_u64();

        let this = Arc::new(Self {
            map: DashMap::new(),
            clock,
            lru: Mutex::new(LruState::new(capacity)),
            wheel: TimingWheel::new(wheel_size, tick_ms, now),
        });

        this.start_reaper();

        this
    }

    pub fn new() -> Arc<Self> {
        Self::new_with_capacity(1024, 1024, 1000)
    }

    pub fn put(&self, key: K, value: V, ttl: Option<u64>) -> bool {
        let expires_at = match ttl {
            Some(ttl_ms) => Some(AppTime::new(
                self.clock.now_millis().as_millis_u64() + ttl_ms,
            )),
            None => None,
        };

        if let Some(exp) = &expires_at {
            self.wheel.schedule(key.clone(), exp.as_millis_u64());
        } else {
            // Sin expiración -> por si estaba previamente agendado
            self.wheel.deschedule(&key);
        }

        match self.map.entry(key.clone()) {
            Entry::Occupied(mut occ) => {
                let next = occ.get().version.saturating_add(1);
                *occ.get_mut() = CacheEntry::new(value, next, expires_at);
            }
            Entry::Vacant(vac) => {
                vac.insert(CacheEntry::new(value, 1, expires_at));
            }
        }

        let to_evict = {
            let mut lru = self.lru.lock();
            lru.touch(key.clone());
            if lru.over_capacity() {
                lru.pop_back()
            } else {
                None
            }
        };

        if let Some(evict_key) = to_evict
            && evict_key != key
        {
            self.wheel.deschedule(&evict_key);
            let _ = self.map.remove(&evict_key);
        }

        true
    }

    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        let now = self.clock.now_millis();

        if let Some(entry) = self.map.get(key) {
            if entry
                .expires_at
                .as_ref()
                .is_some_and(|exp| exp.is_before_or_eq(&now))
            {
                drop(entry);
                let _ = self.invalidate(key);
                return None;
            }

            let to_evict = {
                let mut lru = self.lru.lock();
                if lru.contains(key) {
                    lru.touch(key.clone());
                } else {
                    lru.push_front(key.clone());
                }
                if lru.over_capacity() {
                    lru.pop_back()
                } else {
                    None
                }
            };

            if let Some(evict_key) = to_evict
                && &evict_key != key
            {
                self.wheel.deschedule(&evict_key);
                let _ = self.map.remove(&evict_key);
            }

            return Some(entry.value.clone());
        }
        None
    }

    pub fn invalidate(&self, key: &K) -> bool {
        self.wheel.deschedule(key);
        let removed_map = self.map.remove(key).is_some();
        let mut lru = self.lru.lock();
        let removed_lru = lru.remove(key);
        removed_map || removed_lru
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    // Limpieza de expirados

    pub fn start_reaper(self: &Arc<Self>) {
        let this = Arc::clone(self);
        let tick_ms = this.wheel.tick_ms;
        tokio::spawn(async move {
            let mut interval = time::interval(time::Duration::from_millis(tick_ms));
            loop {
                interval.tick().await;
                this.advance_wheel_to_now();
            }
        });
    }

    pub fn advance_wheel_to_now(&self) {
        let now = self.clock.now_millis().as_millis_u64();
        self.wheel.advance_to(now, self, |cache, key, now_ms| {
            if let Some(e) = cache.map.get(key) {
                if e.expires_at
                    .as_ref()
                    .is_some_and(|exp| exp.is_before_or_eq(&AppTime::new(now_ms)))
                {
                    drop(e);
                    let _ = cache.invalidate(key);
                } else if let Some(exp) = &e.expires_at {
                    cache.wheel.schedule(key.clone(), exp.as_millis_u64());
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, thread, time::Duration};

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
        assert!(entry.expires_at.is_some());
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

        cache.put("k2", "v2", Some(0));

        assert!(
            cache.get(&"k2").is_none(),
            "expired value should not be returned"
        );

        assert!(
            !cache.map.contains_key(&"k2"),
            "expired entry should be removed from both map and LRU"
        );
    }

    #[test]
    fn get_returns_none_when_expiration_equals_now_boundary() {
        let cache = Cache::<&str, &str>::new();

        let now = cache.clock.now_millis();
        cache.put("k3", "v3", Some(now.as_millis_u64()));

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

    //Testing LRU
    #[test]
    fn lru_evicts_oldest_when_over_capacity() {
        // Capacidad 2, sin expiraciones
        let cache = Cache::new_with_capacity(2, 16, 10);

        cache.put("k1", "v1", None); // uso más antiguo (LRU)
        cache.put("k2", "v2", None); // MRU

        // Insertar tercera clave => debe salir k1 (LRU)
        cache.put("k3", "v3", None);

        assert!(!cache.map.contains_key(&"k1"), "k1 debió ser evictada");
        assert!(cache.map.contains_key(&"k2"));
        assert!(cache.map.contains_key(&"k3"));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn lru_get_refreshes_recency_and_changes_eviction() {
        let cache = Cache::new_with_capacity(2, 16, 10);

        cache.put("k1", "v1", None);
        cache.put("k2", "v2", None);

        let _ = cache.get(&"k1");

        cache.put("k3", "v3", None);

        assert!(
            cache.map.contains_key(&"k1"),
            "k1 no debe salir porque fue refrescada con get()"
        );
        assert!(!cache.map.contains_key(&"k2"), "k2 debió ser eliminada");
        assert!(cache.map.contains_key(&"k3"));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn lru_invalidate_removes_from_both_structures() {
        let cache = Cache::new_with_capacity(2, 16, 10);
        cache.put("k1", "v1", None);
        cache.put("k2", "v2", None);

        assert!(
            cache.invalidate(&"k1"),
            "invalidate debe devolver true si existía"
        );

        assert!(!cache.map.contains_key(&"k1"));
        // fuerza una posible evicción para asegurar que "k1" no interfiere
        cache.put("k3", "v3", None);

        assert!(cache.map.contains_key(&"k2"));
        assert!(cache.map.contains_key(&"k3"));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn wheel_expires_after_advancing_to_now() {
        // rueda chica: wheel_size=16 (potencia de 2), tick=10ms
        let cache = Cache::new_with_capacity(128, 16, 10);

        let now = cache.clock.now_millis();

        cache.put("kx", "vx", Some(30));

        // Aún no debe expirar si no avanzamos el wheel y no ha pasado suficiente tiempo real
        assert!(cache.get(&"kx").is_some());

        // Espera > 30ms para garantizar que el "now" real supere el expires_at
        thread::sleep(Duration::from_millis(35));

        // Avanza la rueda hasta "ahora" (usa AppClock real)
        cache.advance_wheel_to_now();

        // Debe estar expirado y removido
        assert!(cache.get(&"kx").is_none());
        assert!(!cache.map.contains_key(&"kx"));
    }

    #[test]
    fn wheel_does_not_expire_if_ttl_extended_before_tick() {
        let cache = Cache::new_with_capacity(128, 16, 10);

        let now = cache.clock.now_millis();
        let near = AppTime::new(now.as_millis_u64() + 20);
        cache.put("kext", "v", Some(20));

        // Antes de que pase el near, extiendo el TTL a +200ms

        cache.put("kext", "v", Some(200));

        // Espera 50ms: suficiente para que el "near" hubiera expirado, pero no el "later"
        thread::sleep(Duration::from_millis(50));

        cache.advance_wheel_to_now();

        // Debe seguir existiendo (no expira aún)
        assert!(
            cache.get(&"kext").is_some(),
            "no debe expirar porque el TTL fue extendido"
        );

        // Espera el resto para sobrepasar "later"
        thread::sleep(Duration::from_millis(170));
        cache.advance_wheel_to_now();

        assert!(
            cache.get(&"kext").is_none(),
            "debe expirar después del TTL extendido"
        );
    }
}
