#[cfg(test)]
mod tests {
    use app_core::clock::Clock;

    use crate::core::services::Cache;

    #[test]
    fn test_put_and_len() {
        let cache = Cache::<&str, &str>::new();
        assert_eq!(cache.len(), 0);

        cache.put("key1", "value1", None);
        assert_eq!(cache.len(), 1);

        // ahora directo: now + 12345ms
        let exp = cache.clock.now_millis().as_millis_u64() + 12_345;
        cache.put("key2", "value2", Some(exp));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_overwrite_value() {
        let cache = Cache::<&str, &str>::new();
        cache.put("key1", "value1", None);
        assert_eq!(cache.len(), 1);

        let exp = cache.clock.now_millis().as_millis_u64() + 100;
        cache.put("key1", "value2", Some(exp));
        assert_eq!(cache.len(), 1);

        let entry = cache.map.get("key1").unwrap();
        assert_eq!(entry.value.as_ref(), &"value2");
        assert_eq!(entry.version, 2);
        assert!(entry.expires_at.is_some());
    }

    #[test]
    fn get_returns_none_when_expired_in_past_and_does_not_remove() {
        let cache = Cache::<&str, &str>::new();

        // ya expirado
        let exp = cache.clock.now_millis().as_millis_u64().saturating_sub(1);
        cache.put("k2", "v2", Some(exp));

        assert!(cache.get(&"k2").is_none());
        assert!(!cache.contains_key(&"k2"));
    }

    #[test]
    fn get_returns_none_when_expiration_equals_now_boundary() {
        let cache = Cache::<&str, &str>::new();

        let exp = cache.clock.now_millis().as_millis_u64();
        cache.put("k3", "v3", Some(exp));

        assert!(cache.get(&"k3").is_none());
    }

    #[test]
    fn wheel_expires_after_advancing_to_now() {
        let cache = Cache::new_with_capacity(128, 16, 10);

        let exp = cache.clock.now_millis().as_millis_u64() + 30;
        cache.put("kx", "vx", Some(exp));
        assert!(cache.get(&"kx").is_some());

        std::thread::sleep(std::time::Duration::from_millis(35));
        cache.advance_wheel_to_now();

        assert!(cache.get(&"kx").is_none());
        assert!(!cache.contains_key(&"kx"));
    }

    #[test]
    fn wheel_does_not_expire_if_ttl_extended_before_tick() {
        let cache = Cache::new_with_capacity(128, 16, 10);

        let exp1 = cache.clock.now_millis().as_millis_u64() + 20;
        cache.put("kext", "v", Some(exp1));

        // antes de tick, lo extendemos
        let exp2 = cache.clock.now_millis().as_millis_u64() + 200;
        cache.put("kext", "v", Some(exp2));

        std::thread::sleep(std::time::Duration::from_millis(50));
        cache.advance_wheel_to_now();
        assert!(cache.get(&"kext").is_some());

        std::thread::sleep(std::time::Duration::from_millis(170));
        cache.advance_wheel_to_now();
        assert!(cache.get(&"kext").is_none());
    }
}
