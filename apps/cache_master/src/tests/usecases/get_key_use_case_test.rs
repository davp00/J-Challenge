#[cfg(test)]
mod tests {
    use app_core::{UseCase, UseCaseValidatable};
    use std::sync::Arc;

    use crate::core::domain::models::{AppError, usecases::GetKeyUseCaseInput};
    use crate::core::usecases::GetKeyUseCase;
    use crate::tests::test_mocks::{MockHasher, MockNetwork}; // ajusta el path a tus mocks

    #[tokio::test]
    async fn validate_fails_when_key_is_empty() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        let uc = GetKeyUseCase::new(hasher, net);

        let input = GetKeyUseCaseInput { key: "".into() };
        let err = uc.validate(&input).await.unwrap_err();

        match err {
            AppError::BadRequest(msg) => assert_eq!(msg, "Key is empty"),
            _ => panic!("Esperaba BadRequest(\"Key is empty\")"),
        }
    }

    #[tokio::test]
    async fn execute_fails_when_hasher_returns_no_node_for_hash() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(None); // no hay nodo asignado al hash

        let net = Arc::new(MockNetwork::new());
        let uc = GetKeyUseCase::new(hasher, net);

        let input = GetKeyUseCaseInput {
            key: "mykey".into(),
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
    async fn execute_ok_with_value() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-1"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_get_key_result(Ok(Some("value-123".to_string())));

        let uc = GetKeyUseCase::new(hasher.clone(), net.clone());

        let input = GetKeyUseCaseInput { key: "k1".into() };
        let out = uc.execute(input).await.expect("no debería fallar");

        assert!(out.success);
        assert_eq!(out.result, "value-123");

        // Evitar ICE: extrae y compara strings normales
        let called = net.last_request_get.lock().clone();
        let (node_id, key) = called.expect("request_get_key no fue llamado");
        assert_eq!(node_id, "node-1");
        assert_eq!(key, "k1");
    }

    #[tokio::test]
    async fn execute_ok_with_none_maps_to_empty_string() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-2"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_get_key_result(Ok(None)); // unwrap_or_default() → ""

        let uc = GetKeyUseCase::new(hasher.clone(), net.clone());

        let input = GetKeyUseCaseInput { key: "k2".into() };
        let out = uc.execute(input).await.expect("no debería fallar");

        assert!(out.success);
        assert_eq!(out.result, "");

        let called = net.last_request_get.lock().clone();
        let (node_id, key) = called.expect("request_get_key no fue llamado");
        assert_eq!(node_id, "node-2");
        assert_eq!(key, "k2");
    }

    #[tokio::test]
    async fn execute_propagates_network_error() {
        let hasher = Arc::new(MockHasher::new());
        hasher.set_node_for_hash(Some("node-3"));

        let net = Arc::new(MockNetwork::new());
        net.set_request_get_key_result(Err(AppError::ConnectionError("boom".into())));

        let uc = GetKeyUseCase::new(hasher.clone(), net.clone());

        let input = GetKeyUseCaseInput { key: "k3".into() };
        let err = uc.execute(input).await.unwrap_err();

        match err {
            AppError::ConnectionError(msg) => assert_eq!(msg, "boom"),
            _ => panic!("Esperaba ConnectionError(\"boom\")"),
        }

        let called = net.last_request_get.lock().clone();
        let (node_id, key) = called.expect("request_get_key no fue llamado");
        assert_eq!(node_id, "node-3");
        assert_eq!(key, "k3");
    }
}
