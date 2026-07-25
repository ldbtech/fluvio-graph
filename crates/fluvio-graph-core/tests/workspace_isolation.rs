//! §9 acceptance test: workspace A cannot read workspace B's nodes.
//!
//! This runs against an **embedded** SurrealDB store (`surrealkv://` in a temp
//! dir), so it needs no Docker and no running SurrealDB — `cargo test` alone
//! proves the isolation. It is the regression oracle the restructure plan (§9)
//! requires: any future change to the isolation *mechanism* (e.g. moving from
//! metadata filters to a namespace/database per workspace) must keep this green.

use fluvio_graph_core::storage::surreal::{SurrealConfig, SurrealStorage};
use fluvio_types::{Domain, Node, NodeId, NodeKind, WorkspaceId};
use uuid::Uuid;

/// A node tagged into a workspace via its metadata (how writes scope today).
fn node_in_workspace(text: &str, workspace: &WorkspaceId) -> Node {
    let mut node = Node::new(
        NodeId::random(),
        Domain::Web,
        format!("test://{text}"),
        text,
        NodeKind::Topic,
    );
    node.metadata
        .insert("workspace_id".to_string(), workspace.as_str().to_string());
    node
}

async fn embedded_store() -> SurrealStorage {
    // Unique temp dir per run so tests don't collide.
    let dir = std::env::temp_dir().join(format!("fluvio-iso-{}", Uuid::new_v4()));
    let cfg = SurrealConfig {
        url: format!("surrealkv://{}", dir.display()),
        namespace: "test".to_string(),
        database: "iso".to_string(),
        ..SurrealConfig::default()
    };
    let store = SurrealStorage::connect(&cfg)
        .await
        .expect("connect embedded surrealkv store");
    store.init_schema().await.expect("init schema");
    store
}

#[tokio::test]
async fn workspace_reads_do_not_leak_across_tenants() {
    let store = embedded_store().await;

    // Same owner, two workspaces — the exact cross-tenant scenario §9 targets.
    let owner = Uuid::new_v4();
    let ws_a = WorkspaceId::new("workspace-a").unwrap();
    let ws_b = WorkspaceId::new("workspace-b").unwrap();

    let a_node = node_in_workspace("secret belonging to A", &ws_a);
    let b_node = node_in_workspace("secret belonging to B", &ws_b);
    let a_id = a_node.id;
    let b_id = b_node.id;

    store.upsert_node(owner, &a_node, 0).await.expect("write A node");
    store.upsert_node(owner, &b_node, 0).await.expect("write B node");

    // Reading workspace A must return A's node and never B's.
    let a_view = store
        .get_user_nodes(owner, None, 0, Some(ws_a.as_str()))
        .await
        .expect("read workspace A");
    let a_ids: Vec<String> = a_view.iter().map(|r| format!("{:?}", r.id)).collect();
    assert!(
        a_ids.iter().any(|id| id.contains(&a_id.to_string())),
        "workspace A should see its own node"
    );
    assert!(
        !a_ids.iter().any(|id| id.contains(&b_id.to_string())),
        "LEAK: workspace A can see workspace B's node"
    );

    // And symmetrically for workspace B.
    let b_view = store
        .get_user_nodes(owner, None, 0, Some(ws_b.as_str()))
        .await
        .expect("read workspace B");
    let b_ids: Vec<String> = b_view.iter().map(|r| format!("{:?}", r.id)).collect();
    assert!(
        b_ids.iter().any(|id| id.contains(&b_id.to_string())),
        "workspace B should see its own node"
    );
    assert!(
        !b_ids.iter().any(|id| id.contains(&a_id.to_string())),
        "LEAK: workspace B can see workspace A's node"
    );
}

#[tokio::test]
async fn similarity_search_is_workspace_scoped() {
    let store = embedded_store().await;

    let owner = Uuid::new_v4();
    let ws_a = WorkspaceId::new("alpha").unwrap();
    let ws_b = WorkspaceId::new("beta").unwrap();

    // Give both nodes a real (identical) embedding so a naive, unscoped
    // similarity search *would* return both — the workspace filter is the only
    // thing keeping them apart.
    let mut a_node = node_in_workspace("alpha doc", &ws_a);
    let mut b_node = node_in_workspace("beta doc", &ws_b);
    a_node.embeddings = vec![0.1_f32; 384];
    b_node.embeddings = vec![0.1_f32; 384];
    let b_id = b_node.id;

    store.upsert_node(owner, &a_node, 0).await.expect("write A");
    store.upsert_node(owner, &b_node, 0).await.expect("write B");

    let hits = store
        .similarity_search_nodes(owner, &vec![0.1_f32; 384], 10, 0, Some(ws_a.as_str()))
        .await
        .expect("similarity search in workspace A");

    let hit_ids: Vec<String> = hits.iter().map(|r| format!("{:?}", r.id)).collect();
    assert!(
        !hit_ids.iter().any(|id| id.contains(&b_id.to_string())),
        "LEAK: similarity search in workspace A returned workspace B's vector"
    );
}

/// Guards the invariant that the isolation key can never be empty — an empty
/// workspace id is how a filter-based scheme silently turns into "match all".
#[test]
fn empty_workspace_id_is_rejected() {
    assert!(WorkspaceId::new("").is_err());
    assert!(WorkspaceId::new("  ").is_err());
}
