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
    pub map: DashMap<K, CacheEntry<V>>,
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

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
}
