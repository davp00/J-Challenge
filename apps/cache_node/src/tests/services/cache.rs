#[cfg(test)]
mod tests {
    use crate::core::services::Cache;

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
            !cache.contains_key(&"k2"),
            "expired entry should be removed from both map and LRU"
        );
    }

    #[test]
    fn get_returns_none_when_expiration_equals_now_boundary() {
        let cache = Cache::<&str, &str>::new();

        cache.put("k3", "v3", Some(0));

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

        assert!(!cache.contains_key(&"k1"), "k1 debió ser evictada");
        assert!(cache.contains_key(&"k2"));
        assert!(cache.contains_key(&"k3"));
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
            cache.contains_key(&"k1"),
            "k1 no debe salir porque fue refrescada con get()"
        );
        assert!(!cache.contains_key(&"k2"), "k2 debió ser eliminada");
        assert!(cache.contains_key(&"k3"));
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

        assert!(!cache.contains_key(&"k1"));
        // fuerza una posible evicción para asegurar que "k1" no interfiere
        cache.put("k3", "v3", None);

        assert!(cache.contains_key(&"k2"));
        assert!(cache.contains_key(&"k3"));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn wheel_expires_after_advancing_to_now() {
        // rueda chica: wheel_size=16 (potencia de 2), tick=10ms
        let cache = Cache::new_with_capacity(128, 16, 10);

        cache.put("kx", "vx", Some(30));

        // Aún no debe expirar si no avanzamos el wheel y no ha pasado suficiente tiempo real
        assert!(cache.get(&"kx").is_some());

        // Espera > 30ms para garantizar que el "now" real supere el expires_at
        thread::sleep(Duration::from_millis(35));

        // Avanza la rueda hasta "ahora" (usa AppClock real)
        cache.advance_wheel_to_now();

        // Debe estar expirado y removido
        assert!(cache.get(&"kx").is_none());
        assert!(!cache.contains_key(&"kx"));
    }

    #[test]
    fn wheel_does_not_expire_if_ttl_extended_before_tick() {
        let cache = Cache::new_with_capacity(128, 16, 10);

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
