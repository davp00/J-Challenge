#[cfg(test)]
mod tests {
    use app_core::{UseCase, UseCaseValidatable};
    use std::sync::Arc;

    use crate::core::domain::models::{AppError, usecases::PutKeyUseCaseInput};

    use crate::core::usecases::PutKeyUseCase;

    // importa tus mocks + MockClock (ajusta el path a donde los tengas)
    use crate::tests::test_mocks::{MockClock, MockHasher, MockNetwork};

    // ---------- Validaciones ----------

    #[tokio::test]
    async fn validate_fails_when_key_is_empty() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        let clock = Arc::new(MockClock::new(0));

        let uc = PutKeyUseCase::new(hasher, net, clock);

        let input = PutKeyUseCaseInput {
            key: "".into(),
            value: "v".into(),
            ttl: None,
        };
        let err = uc.validate(&input).await.unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert_eq!(msg, "Key is empty"),
            _ => panic!("Esperaba BadRequest(\"Key is empty\")"),
        }
    }

    #[tokio::test]
    async fn validate_fails_when_value_is_empty() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        let clock = Arc::new(MockClock::new(0));

        let uc = PutKeyUseCase::new(hasher, net, clock);

        let input = PutKeyUseCaseInput {
            key: "k".into(),
            value: "".into(),
            ttl: None,
        };
        let err = uc.validate(&input).await.unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert_eq!(msg, "Value is empty"),
            _ => panic!("Esperaba BadRequest(\"Value is empty\")"),
        }
    }

    // ---------- Ejecución ----------

    #[tokio::test]
    async fn execute_fails_when_hasher_returns_no_node_for_hash() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(None); // no hay nodo para el hash

        let net = Arc::new(MockNetwork::new());
        let clock = Arc::new(MockClock::new(1_000));

        let uc = PutKeyUseCase::new(hasher, net, clock);

        let input = PutKeyUseCaseInput {
            key: "mykey".into(),
            value: "v".into(),
            ttl: None,
        };
        let err = uc.execute(input).await.unwrap_err();

        match err {
            AppError::NodeNotFound(msg) => {
                assert!(msg.contains("mykey"));
                assert!(msg.contains("hash")); // create_hash() => "hash"
            }
            _ => panic!("Esperaba NodeNotFound"),
        }
    }

    #[tokio::test]
    async fn execute_ok_without_ttl_passes_none_and_returns_true() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-1"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_put_key_result(Ok(true)); // éxito

        let clock = Arc::new(MockClock::new(10_000));

        let uc = PutKeyUseCase::new(hasher.clone(), net.clone(), clock);

        let input = PutKeyUseCaseInput {
            key: "k1".into(),
            value: "v1".into(),
            ttl: None,
        };
        let out = uc.execute(input).await.expect("no debería fallar");

        assert!(out.success);

        // verificar parámetros enviados
        let called = net
            .last_request_put
            .lock()
            .clone()
            .expect("request_put_key no llamado");
        let (node_id, key, value, expires_at) = called;
        assert_eq!(node_id, "node-1");
        assert_eq!(key, "k1");
        assert_eq!(value, "v1");
        assert_eq!(expires_at, None);
    }

    #[tokio::test]
    async fn execute_ok_with_ttl_computes_expires_at_now_plus_ttl() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-2"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_put_key_result(Ok(true));

        // now = 1_000_000 ms
        let clock = Arc::new(MockClock::new(1_000_000));

        let uc = PutKeyUseCase::new(hasher.clone(), net.clone(), clock);

        let input = PutKeyUseCaseInput {
            key: "k2".into(),
            value: "v2".into(),
            ttl: Some(500),
        };
        let out = uc.execute(input).await.expect("no debería fallar");

        assert!(out.success);

        let called = net
            .last_request_put
            .lock()
            .clone()
            .expect("request_put_key no llamado");
        let (node_id, key, value, expires_at) = called;
        assert_eq!(node_id, "node-2");
        assert_eq!(key, "k2");
        assert_eq!(value, "v2");
        assert_eq!(expires_at, Some(1_000_500)); // now + ttl
    }

    #[tokio::test]
    async fn execute_returns_false_when_network_returns_false() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-x"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_put_key_result(Ok(false)); // red responde false

        let clock = Arc::new(MockClock::new(0));
        let uc = PutKeyUseCase::new(hasher.clone(), net.clone(), clock);

        let input = PutKeyUseCaseInput {
            key: "kx".into(),
            value: "vx".into(),
            ttl: None,
        };
        let out = uc.execute(input).await.expect("no debería fallar");

        assert!(!out.success);

        let called = net
            .last_request_put
            .lock()
            .clone()
            .expect("request_put_key no llamado");
        let (node_id, key, value, expires_at) = called;
        assert_eq!(node_id, "node-x");
        assert_eq!(key, "kx");
        assert_eq!(value, "vx");
        assert_eq!(expires_at, None);
    }

    #[tokio::test]
    async fn execute_propagates_network_error() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-e"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_put_key_result(Err(AppError::ConnectionError("boom".into())));

        let clock = Arc::new(MockClock::new(123));
        let uc = PutKeyUseCase::new(hasher.clone(), net.clone(), clock);

        let input = PutKeyUseCaseInput {
            key: "ke".into(),
            value: "ve".into(),
            ttl: Some(1),
        };
        let err = uc.execute(input).await.unwrap_err();

        match err {
            AppError::ConnectionError(msg) => assert_eq!(msg, "boom"),
            _ => panic!("Esperaba ConnectionError(\"boom\")"),
        }

        let called = net
            .last_request_put
            .lock()
            .clone()
            .expect("request_put_key no llamado");
        let (node_id, key, value, expires_at) = called;
        assert_eq!(node_id, "node-e");
        assert_eq!(key, "ke");
        assert_eq!(value, "ve");
        assert_eq!(expires_at, Some(124)); // 123 + 1
    }
}
