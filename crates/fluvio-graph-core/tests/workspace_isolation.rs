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

/// Documents the constraint that shapes the tenancy mechanism (ADR 0002 §2.2
/// Fork B): the embedded surrealkv store is **single-connection per path**. A
/// second connection to the same path — which "one connection per workspace
/// database" would require — fails with a datastore LOCK error. Therefore
/// database-per-workspace-via-separate-connections cannot be the isolation
/// mechanism for the embedded backend; scoping must work over a single shared
/// connection (metadata filter, or per-op use_db under a lock).
#[tokio::test]
async fn embedded_surrealkv_is_single_connection_per_path() {
    let dir = std::env::temp_dir().join(format!("fluvio-dbprobe-{}", Uuid::new_v4()));
    let url = format!("surrealkv://{}", dir.display());

    let mk = |db: &str| SurrealConfig {
        url: url.clone(),
        namespace: "test".to_string(),
        database: db.to_string(),
        ..SurrealConfig::default()
    };

    let _store_a = SurrealStorage::connect(&mk("ws_alpha")).await.expect("first connection opens");
    let second = SurrealStorage::connect(&mk("ws_beta")).await;
    assert!(
        second.is_err(),
        "expected the embedded store to reject a second connection to the same path; \
         if this ever succeeds, connection-per-workspace becomes viable for embedded"
    );
    let msg = format!("{:#}", second.err().unwrap());
    assert!(msg.contains("locked"), "expected a datastore lock error, got: {msg}");
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
        .get_user_nodes(owner, None, 0, &ws_a)
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
        .get_user_nodes(owner, None, 0, &ws_b)
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
        .similarity_search_nodes(owner, &vec![0.1_f32; 384], 10, 0, &ws_a)
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

/// The workspace filter is BOUND, not string-interpolated (ADR 0002 §2.6). A
/// workspace id crafted to break out of the SurrealQL string literal
/// (`x' OR '1'='1`) must be treated as an opaque value — it matches its own
/// (empty) tenant and can never widen the scope to another tenant's data.
#[tokio::test]
async fn crafted_workspace_id_cannot_escape_the_filter() {
    let store = embedded_store().await;
    let owner = Uuid::new_v4();

    let victim_ws = WorkspaceId::new("victim").unwrap();
    let victim = node_in_workspace("victim secret", &victim_ws);
    let victim_id = victim.id;
    store.upsert_node(owner, &victim, 0).await.expect("write victim");

    // A classic injection payload as the workspace id. WorkspaceId accepts it as
    // an opaque string; the query binds it, so it matches only a tenant literally
    // named that — i.e. nothing — rather than OR-ing the WHERE clause to true.
    let attack = WorkspaceId::new("x' OR '1'='1").unwrap();
    let leaked = store
        .get_user_nodes(owner, None, 0, &attack)
        .await
        .expect("query with crafted workspace id must not error");

    let ids: Vec<String> = leaked.iter().map(|r| format!("{:?}", r.id)).collect();
    assert!(
        !ids.iter().any(|id| id.contains(&victim_id.to_string())),
        "INJECTION: a crafted workspace_id widened the filter and leaked another tenant's node"
    );
    assert!(leaked.is_empty(), "crafted tenant should own no rows, got {}", leaked.len());
}

/// Fork A migration: nodes written before tenancy have no workspace tag and
/// would vanish once reads require a scope. `backfill_default_workspace` stamps
/// them into the default workspace, and it is idempotent.
#[tokio::test]
async fn backfill_moves_untagged_nodes_into_default_workspace() {
    let store = embedded_store().await;
    let owner = Uuid::new_v4();
    let default = WorkspaceId::default_workspace();

    // A legacy, untagged node (no metadata.workspace_id).
    let mut legacy = Node::new(
        NodeId::random(), Domain::Web, "test://legacy", "pre-tenancy data", NodeKind::Topic,
    );
    legacy.metadata.clear();
    let legacy_id = legacy.id;
    store.upsert_node(owner, &legacy, 0).await.expect("write legacy");

    // Before backfill: invisible under the required default scope.
    let before = store.get_user_nodes(owner, None, 0, &default).await.expect("read default");
    assert!(
        !before.iter().any(|r| format!("{:?}", r.id).contains(&legacy_id.to_string())),
        "untagged node should not yet be in the default workspace"
    );

    let stamped = store.backfill_default_workspace(&default).await.expect("backfill");
    assert_eq!(stamped, 1, "exactly the one untagged node should be stamped");

    // After backfill: visible under the default scope.
    let after = store.get_user_nodes(owner, None, 0, &default).await.expect("read default again");
    assert!(
        after.iter().any(|r| format!("{:?}", r.id).contains(&legacy_id.to_string())),
        "backfilled node should now be readable in the default workspace"
    );

    // Idempotent: a second run stamps nothing.
    let again = store.backfill_default_workspace(&default).await.expect("backfill again");
    assert_eq!(again, 0, "backfill must be idempotent");
}
