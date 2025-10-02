#[cfg(test)]
mod tests {
    use app_core::{UseCase, UseCaseValidatable};

    use crate::{
        core::{
            domain::models::{AppError, usecases::remove_node_use_case::RemoveNodeUseCaseInput},
            usecases::RemoveNodeUseCase,
        },
        tests::test_mocks::{MockHasher, MockNetwork},
    };
    use std::sync::Arc;

    // Usa los mocks MockHasher y MockNetwork ya definidos en tu módulo de tests anteriores

    #[tokio::test]
    async fn validate_fails_when_node_id_is_empty() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        let uc = RemoveNodeUseCase::new(hasher, net);

        let input = RemoveNodeUseCaseInput { node_id: "".into() };
        let err = uc.validate(&input).await.unwrap_err();
        assert!(matches!(err, AppError::FirstConnectionEmpty));
    }

    #[tokio::test]
    async fn removes_node_when_replica_count_is_one() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        net.set_replica_count(1); // <= 1 → removerá también del hasher
        net.set_remove_result(Ok(true)); // network OK

        let uc = RemoveNodeUseCase::new(hasher.clone(), net.clone());

        let input = RemoveNodeUseCaseInput {
            node_id: "n1".into(),
        };
        let out = uc.execute(input).await.expect("no debería fallar");

        assert!(out.success);
        assert_eq!(hasher.last_remove_node.lock().as_deref(), Some("n1"));
        assert_eq!(net.last_remove_node.lock().as_deref(), Some("n1"));
    }

    #[tokio::test]
    async fn does_not_remove_from_hasher_when_replica_count_greater_than_one() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        net.set_replica_count(2); // > 1 → NO removerá del hasher
        net.set_remove_result(Ok(true)); // network OK

        let uc = RemoveNodeUseCase::new(hasher.clone(), net.clone());

        let input = RemoveNodeUseCaseInput {
            node_id: "n2".into(),
        };
        let out = uc.execute(input).await.expect("no debería fallar");

        // success = hasher_remove(false) && network_remove(true) = false
        assert!(!out.success);
        assert_eq!(hasher.last_remove_node.lock().as_deref(), None);
        assert_eq!(net.last_remove_node.lock().as_deref(), Some("n2"));
    }

    #[tokio::test]
    async fn fails_when_network_remove_returns_false() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        net.set_replica_count(0);
        net.set_remove_result(Ok(false)); // <— network dice “no encontrado”

        let uc = RemoveNodeUseCase::new(hasher, net);

        let input = RemoveNodeUseCaseInput {
            node_id: "n3".into(),
        };
        let err = uc.execute(input).await.unwrap_err();

        match err {
            AppError::NodeNotFound(msg) => assert!(msg.contains("n3")),
            _ => panic!("Esperaba NodeNotFound"),
        }
    }

    #[tokio::test]
    async fn fails_when_network_remove_returns_error() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        net.set_replica_count(0);
        net.set_remove_result(Err(AppError::ConnectionError("fail".into())));

        let uc = RemoveNodeUseCase::new(hasher, net);

        let input = RemoveNodeUseCaseInput {
            node_id: "n4".into(),
        };
        let err = uc.execute(input).await.unwrap_err();

        match err {
            AppError::ConnectionError(msg) => assert_eq!(msg, "fail"),
            _ => panic!("Esperaba ConnectionError"),
        }
    }
}
