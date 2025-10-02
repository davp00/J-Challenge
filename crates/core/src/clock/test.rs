#[cfg(test)]
mod tests {
    use crate::clock::clock::{AppClock, Clock};
    use crate::clock::time::AppTime;

    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    /// Helper de compile-time para asegurar Send + Sync.
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn app_time_is_before_true_when_less() {
        let t1 = AppTime::new(100);
        let t2 = AppTime::new(200);
        assert!(t1.is_before(&t2));
        assert!(!t2.is_before(&t1));
    }

    #[test]
    fn app_time_is_before_or_eq_handles_equality() {
        let t1 = AppTime::new(1234);
        let t2 = AppTime::new(1234);
        assert!(t1.is_before_or_eq(&t2));
        assert!(t2.is_before_or_eq(&t1));

        let t3 = AppTime::new(1235);
        assert!(t1.is_before_or_eq(&t3));
        assert!(!t3.is_before_or_eq(&t1));
    }

    #[test]
    fn app_time_from_u128_builds_correctly() {
        let raw: u128 = 9_876_543_210;
        let t: AppTime = raw.into();

        // Comparación indirecta usando las funciones provistas
        let same = AppTime::from(raw);
        assert!(t.is_before_or_eq(&same) && same.is_before_or_eq(&t));
    }

    #[test]
    fn app_clock_now_millis_is_non_decreasing_and_progresses() {
        let clock = AppClock::new();

        let a = clock.now_millis();
        // Espera mínima para garantizar tick de milisegundos y evitar flakiness.
        thread::sleep(Duration::from_millis(2));
        let b = clock.now_millis();

        assert!(a.is_before(&b), "Se esperaba avance de tiempo (a < b)");
    }

    #[test]
    fn app_clock_is_send_and_sync() {
        // Validación en tiempo de compilación
        assert_send_sync::<AppClock>();
        // Y también para el trait-obj detrás de Arc<dyn Clock>
        fn _assert_arc_dyn_clock_send_sync<T: Send + Sync>() {}
        _assert_arc_dyn_clock_send_sync::<std::sync::Arc<dyn Clock>>();
    }

    /// MockClock para probar el trait `Clock`.
    struct MockClock(AtomicU64);

    impl MockClock {
        fn new(v: u64) -> Self {
            Self(AtomicU64::new(v))
        }
        fn set(&self, v: u64) {
            self.0.store(v, Ordering::SeqCst);
        }
    }

    impl Clock for MockClock {
        fn now_millis(&self) -> AppTime {
            AppTime::new(self.0.load(Ordering::SeqCst))
        }
    }

    #[test]
    fn mock_clock_returns_controlled_time() {
        let mock = MockClock::new(1_000);
        let t1 = mock.now_millis();
        assert!(AppTime::new(1_000).is_before_or_eq(&t1));

        mock.set(2_000);
        let t2 = mock.now_millis();
        assert!(t1.is_before(&t2));
        assert!(AppTime::new(2_000).is_before_or_eq(&t2));
    }
}
