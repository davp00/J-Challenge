#[cfg(test)]
mod tests {
    use app_core::{UseCase, UseCaseValidatable};

    use crate::{
        core::{
            domain::models::{
                AppError, NodeType, usecases::assign_node_use_case::AssignNodeUseCaseInput,
            },
            usecases::AssignNodeUseCase,
        },
        tests::test_mocks::{MockHasher, MockNetwork},
    };
    use std::sync::Arc;

    // Usa los MockHasher / MockNetwork que definiste arriba

    #[tokio::test]
    async fn validate_fails_on_empty_node_id() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        let uc = AssignNodeUseCase::new(hasher, net);

        let input = AssignNodeUseCaseInput {
            node_id: "".into(),
            node_type: NodeType::Master,
        };
        let err = uc.validate(&input).await.unwrap_err();
        assert!(matches!(err, AppError::FirstConnectionEmpty));
    }

    #[tokio::test]
    async fn master_insert_happy_path() {
        let hasher = Arc::new(MockHasher::with_exists(true));
        let net = Arc::new(MockNetwork::new());
        let uc = AssignNodeUseCase::new(hasher.clone(), net.clone());

        let input = AssignNodeUseCaseInput {
            node_id: "m1".into(),
            node_type: NodeType::Master,
        };
        let out = uc.execute(input).await.expect("no debería fallar");
        assert!(out.success);

        assert_eq!(hasher.last_add_node.lock().as_deref(), Some("m1"));
        assert_eq!(hasher.last_node_exists.lock().as_deref(), Some("m1"));
        assert_eq!(net.last_add_master.lock().as_deref(), Some("m1"));
    }

    #[tokio::test]
    async fn master_insert_fails_if_hasher_reports_not_exists() {
        let hasher = Arc::new(MockHasher::with_exists(false));
        let net = Arc::new(MockNetwork::new());
        let uc = AssignNodeUseCase::new(hasher, net);

        let input = AssignNodeUseCaseInput {
            node_id: "m2".into(),
            node_type: NodeType::Master,
        };
        let err = uc.execute(input).await.unwrap_err();
        match err {
            AppError::ConnectionError(msg) => assert_eq!(msg, "No se pudo agregar el nodo"),
            _ => panic!("Esperaba ConnectionError(\"No se pudo agregar el nodo\")"),
        }
    }

    #[tokio::test]
    async fn replica_insert_happy_path() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        net.set_next_master(Some("m1")); // el servicio dirá que m1 es el master con menos réplicas

        let uc = AssignNodeUseCase::new(hasher, net.clone());
        let input = AssignNodeUseCaseInput {
            node_id: "r1".into(),
            node_type: NodeType::Replica,
        };

        let out = uc.execute(input).await.expect("no debería fallar");
        assert!(out.success);

        assert_eq!(
            net.last_add_replica
                .lock()
                .as_ref()
                .map(|(m, r)| (m.as_str(), r.as_str())),
            Some(("m1", "r1"))
        );
    }

    #[tokio::test]
    async fn replica_insert_fails_when_no_master_available() {
        let hasher = Arc::new(MockHasher::new());
        let net = Arc::new(MockNetwork::new());
        net.set_next_master(None); // sin masters

        let uc = AssignNodeUseCase::new(hasher, net);
        let input = AssignNodeUseCaseInput {
            node_id: "rX".into(),
            node_type: NodeType::Replica,
        };

        let err = uc.execute(input).await.unwrap_err();
        match err {
            AppError::ConnectionError(msg) => assert_eq!(msg, "No hay nodos en la red"),
            _ => panic!("Esperaba ConnectionError(\"No hay nodos en la red\")"),
        }
    }
}
