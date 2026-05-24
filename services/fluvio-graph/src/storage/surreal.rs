//! storage/surreal.rs
//!
//! SurrealDB persistence layer for the Fluvio knowledge graph.
//! Written for surrealdb 3.x API.
//!
//! Key insight for 3.x:
//!   - Use format!() queries with JSON values embedded directly
//!   - Avoid `.bind()` for complex types — it requires `SurrealValue`
//!   - `.take::<Vec<serde_json::Value>>(0)` then `serde_json::from_value` for row structs

use std::collections::HashMap;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::engine::any;
use surrealdb::engine::any::Any;
use surrealdb::types::{RecordId, RecordIdKey, ToSql};
use uuid::Uuid;

use fluvio_types::{Node, NodeId, Edge};
use fluvio_types::{Domain, NodeKind};

/// For ws/http URLs only: map `localhost` → `127.0.0.1` when resolution is flaky.
fn normalize_remote_endpoint(url: &str) -> String {
    let t = url.trim();
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("ws://") || lower.starts_with("wss://") || lower.starts_with("http://") || lower.starts_with("https://")
    {
        t.replacen("localhost", "127.0.0.1", 1)
    } else {
        t.to_string()
    }
}

// ── SurrealNodeRow — what SELECT * FROM nodes returns ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RecordIdCompat {
    Record(RecordId),
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealNodeRow {
    pub id:          RecordIdCompat,
    pub owner_id:    String,
    pub domain:      String,
    pub source_uri:  String,
    pub source_text: String,
    pub embeddings:  Vec<f32>,
    pub metadata:    HashMap<String, String>,
    pub kind:        String,
    pub zone:        i16,
}

impl SurrealNodeRow {
    /// Convert back to DomainGraph Node for in-memory use.
    pub fn to_node(&self) -> Node {
        let uuid = node_uuid_from_record_id_compat(&self.id);
        Node {
            id:          NodeId(uuid),
            domain: match self.domain.as_str() {
                "Pdf"          => Domain::Pdf,
                "Email"        => Domain::Email,
                "Whatsapp"     => Domain::Whatsapp,
                "Calendar"     => Domain::Calendar,
                "Codebase"     => Domain::Codebase,
                "Web"          => Domain::Web,
                other          => Domain::Custom(other.to_string()),
            },
            source_uri:  self.source_uri.clone(),
            source_text: self.source_text.clone(),
            embeddings:  self.embeddings.clone(),
            metadata:    self.metadata.clone(),
            kind:        NodeKind::Artifcat,
            zone:        self.zone,
        }
    }
}

// ── SurrealStorage ────────────────────────────────────────────────────────────

pub struct SurrealStorage {
    pub db: Surreal<Any>,
}

impl SurrealStorage {
    /// Connect to SurrealDB and select namespace + database.
    ///
    /// **`SURREAL_URL`** selects the backend at runtime (`Surreal<Any>`):
    /// - **Unset** → standalone SurrealDB at `ws://127.0.0.1:8000` (matches `surreal sql` against a local server).
    /// - `embedded` → embedded file store `surrealkv://./fluvio_surreal_data` (no separate Surreal process).
    /// - Any other URL → `ws://…` / `wss://` / `http://…` / `surrealkv://…` / `mem://` as supported by Surreal.
    ///   On some hosts the remote WebSocket stack surfaces broken `localhost` resolution; prefer `127.0.0.1`.
    pub async fn connect() -> anyhow::Result<Self> {
        let raw = std::env::var("SURREAL_URL").unwrap_or_default();
        let endpoint = match raw.trim() {
            "" => normalize_remote_endpoint("ws://127.0.0.1:8000"),
            "embedded" => "surrealkv://./fluvio_surreal_data".to_string(),
            url => normalize_remote_endpoint(url),
        };

        let user = std::env::var("SURREAL_USER")
            .unwrap_or_else(|_| "root".to_string());
        let pass = std::env::var("SURREAL_PASS")
            .unwrap_or_else(|_| "root".to_string());
        let ns   = std::env::var("SURREAL_NS")
            .unwrap_or_else(|_| "fluvio".to_string());
        let db_name = std::env::var("SURREAL_DB")
            .unwrap_or_else(|_| "graph".to_string());

        let surreal = any::connect(endpoint.as_str())
            .await
            .map_err(|e| anyhow::anyhow!("SurrealDB connect failed at {endpoint}: {e}"))?;

        let ep = endpoint.to_ascii_lowercase();
        let remote = ep.starts_with("ws://")
            || ep.starts_with("wss://")
            || ep.starts_with("http://")
            || ep.starts_with("https://");

        if remote {
            surreal
                .signin(surrealdb::opt::auth::Root {
                    username: user,
                    password: pass,
                })
                .await
                .map_err(|e| anyhow::anyhow!("SurrealDB auth failed: {e}"))?;
        }

        surreal
            .use_ns(ns.as_str())
            .use_db(db_name.as_str())
            .await
            .map_err(|e| anyhow::anyhow!("SurrealDB use ns/db failed: {e}"))?;

        tracing::info!("[SurrealDB] Connected to {endpoint} ns={ns} db={db_name}");

        Ok(Self { db: surreal })
    }

    // ── Schema init ───────────────────────────────────────────────────────────

    pub async fn init_schema(&self) -> anyhow::Result<()> {
        self.db.query(
            "DEFINE TABLE IF NOT EXISTS nodes SCHEMALESS;
             DEFINE INDEX IF NOT EXISTS nodes_owner  ON nodes FIELDS owner_id;
             DEFINE INDEX IF NOT EXISTS nodes_uri    ON nodes FIELDS source_uri;
             DEFINE INDEX IF NOT EXISTS nodes_domain ON nodes FIELDS domain;"
        ).await
        .map_err(|e| anyhow::anyhow!("schema init failed: {e}"))?;

        tracing::info!("[SurrealDB] Schema initialized");
        Ok(())
    }

    // ── Node operations ───────────────────────────────────────────────────────

    /// Insert or update a node.
    /// Uses format!() with embedded JSON to avoid SurrealValue trait requirement.
    pub async fn upsert_node(
        &self,
        owner_id: Uuid,
        node:     &Node,
        zone:     i16,
    ) -> anyhow::Result<()> {
        let _node_id = format!("nodes:{}", node.id);

        let content = serde_json::json!({
            "owner_id":    owner_id.to_string(),
            "domain":      format!("{:?}", node.domain),
            "source_uri":  node.source_uri,
            "source_text": node.source_text,
            "embeddings":  node.embeddings,
            "metadata":    node.metadata,
            "kind":        format!("{:?}", node.kind),
            "zone":        zone,
        });

        self.db
            .query("UPSERT type::record('nodes', $id) CONTENT $content")
            .bind(("id",      node.id.to_string()))
            .bind(("content", content))
            .await
            .map_err(|e| anyhow::anyhow!("upsert_node {}: {e}", node.id))?;

        Ok(())
    }

    /// Insert or update an edge between two nodes.
    pub async fn upsert_edge(
        &self,
        owner_id: Uuid,
        edge:     &Edge,
    ) -> anyhow::Result<()> {
        let rel = sanitize_label(&edge.label);
        let query = format!(
            "LET $from = type::record('nodes', $from_id); \
             LET $to = type::record('nodes', $to_id); \
             RELATE $from->{rel}->$to \
             SET owner_id = $owner_id, label = $label, edge_token = $edge_token, probability = $probability"
        );

        self.db
            .query(query)
            .bind(("from_id", edge.from.to_string()))
            .bind(("to_id", edge.to.to_string()))
            .bind(("owner_id", owner_id.to_string()))
            .bind(("label", edge.label.clone()))
            .bind(("edge_token", edge.token))
            .bind(("probability", edge.relationship_probability))
            .await
            .map_err(|e| anyhow::anyhow!("upsert_edge: {e}"))?;

        Ok(())
    }

    /// Get a node by NodeId.
    pub async fn get_node(
        &self,
        node_id: &NodeId,
    ) -> anyhow::Result<Option<SurrealNodeRow>> {
        let mut result = self.db
                .query(format!("SELECT * FROM nodes:`{node_id}`"))
        .await
            .map_err(|e| anyhow::anyhow!("get_node: {e}"))?;

        let rows: Vec<SurrealNodeRow> = rows_from_json(&mut result, "get_node")?;

        Ok(rows.into_iter().next())
    }

    /// Get all nodes for a user, optionally filtered by domain.
    pub async fn get_user_nodes(
        &self,
        owner_id: Uuid,
        domain:   Option<&str>,
        zone:     i16,
    ) -> anyhow::Result<Vec<SurrealNodeRow>> {
        let query = match domain {
            Some(d) => format!(
                "SELECT * FROM nodes \
                 WHERE owner_id = '{owner_id}' \
                   AND domain = '{d}' \
                   AND zone <= {zone}"
            ),
            None => format!(
                "SELECT * FROM nodes \
                 WHERE owner_id = '{owner_id}' \
                   AND zone <= {zone}"
            ),
        };

        let mut result = self.db.query(query).await
            .map_err(|e| anyhow::anyhow!("get_user_nodes: {e}"))?;

        let nodes: Vec<SurrealNodeRow> = rows_from_json(&mut result, "get_user_nodes")?;

        Ok(nodes)
    }

    // ── Graph traversal ───────────────────────────────────────────────────────

    /// BFS traversal from a node up to `depth` hops.
    pub async fn bfs(
        &self,
        start_id: &NodeId,
        depth:    usize,
    ) -> anyhow::Result<Vec<SurrealNodeRow>> {
        let id    = format!("nodes:{start_id}");
        let query = format!(
            "SELECT * FROM nodes WHERE id INSIDE \
             (SELECT VALUE ->{{0,{depth}}}->nodes.id FROM {id})"
        );

        let mut result = self.db.query(query).await
            .map_err(|e| anyhow::anyhow!("bfs: {e}"))?;

        let nodes: Vec<SurrealNodeRow> = rows_from_json(&mut result, "bfs")?;

        Ok(nodes)
    }

    // ── Similarity search ─────────────────────────────────────────────────────

    /// Vector similarity search across a user's nodes.
    pub async fn similarity_search(
        &self,
        owner_id:  Uuid,
        query_vec: &[f32],
        top_k:     usize,
        zone:      i16,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let rows = self.similarity_search_nodes(owner_id, query_vec, top_k, zone).await?;
        Ok(rows.into_iter()
            .map(|r| {
                let score  = cosine_sim(query_vec, &r.embeddings);
                let id_str = format!("{:?}", r.id);
                (id_str, score)
            })
            .collect())
    }

    /// Same as [`Self::similarity_search`], but returns full node rows (for LLM context).
    pub async fn similarity_search_nodes(
        &self,
        owner_id:  Uuid,
        query_vec: &[f32],
        top_k:     usize,
        max_zone:  i16,
    ) -> anyhow::Result<Vec<SurrealNodeRow>> {
        let query = format!(
            "SELECT * FROM nodes \
             WHERE owner_id = '{owner_id}' AND zone <= {max_zone}"
        );
    
        let mut result = self.db.query(query).await
            .map_err(|e| anyhow::anyhow!("similarity_search_nodes: {e}"))?;
    
        let rows: Vec<SurrealNodeRow> = rows_from_json(&mut result, "similarity_search_nodes")?;
    
        let mut scored: Vec<(f32, SurrealNodeRow)> = rows
            .into_iter()
            .filter(|r| !r.embeddings.is_empty())
            .map(|r| (cosine_sim(query_vec, &r.embeddings), r))
            .collect();
    
        scored.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
    
        Ok(scored.into_iter().map(|(_, row)| row).collect())
    }

    /// Cross-user similarity search — "who in my network works on X?"
    pub async fn network_similarity_search(
        &self,
        user_ids:  &[Uuid],
        query_vec: &[f32],
        top_k:     usize,
        max_zone:  i16,
    ) -> anyhow::Result<Vec<(String, String, f32)>> {
        if user_ids.is_empty() { return Ok(vec![]); }
    
        let owners = user_ids.iter()
            .map(|u| format!("'{u}'"))
            .collect::<Vec<_>>()
            .join(", ");
    
        let query = format!(
            "SELECT * FROM nodes \
             WHERE owner_id IN [{owners}] AND zone <= {max_zone}"
        );
    
        let mut result = self.db.query(query).await
            .map_err(|e| anyhow::anyhow!("network_similarity_search: {e}"))?;
    
        let rows: Vec<SurrealNodeRow> = rows_from_json(&mut result, "network_similarity_search")?;
    
        let mut scored: Vec<(f32, SurrealNodeRow)> = rows
            .into_iter()
            .filter(|r| !r.embeddings.is_empty())
            .map(|r| (cosine_sim(query_vec, &r.embeddings), r))
            .collect();
    
        scored.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
    
        Ok(scored.into_iter()
            .map(|(score, row)| (row.owner_id.clone(), format!("{:?}", row.id), score))
            .collect())
    }

    // ── Bulk operations ───────────────────────────────────────────────────────

    /// Save an entire DomainGraph for a user to SurrealDB.
    pub async fn save_graph(
        &self,
        owner_id: Uuid,
        graph:    &fluvio_types::DomainGraph,
        zone:     i16,
    ) -> anyhow::Result<(usize, usize)> {
        let mut nodes_saved = 0usize;
        let mut edges_saved = 0usize;

        for node in graph.nodes.values() {
            self.upsert_node(owner_id, node, zone).await?;
            nodes_saved += 1;
        }

        for edges in graph.adj.values() {
            for edge in edges {
                self.upsert_edge(owner_id, edge).await?;
                edges_saved += 1;
            }
        }

        tracing::info!(
            "[SurrealDB] Saved graph for {owner_id}: \
             {nodes_saved} nodes, {edges_saved} edges"
        );

        Ok((nodes_saved, edges_saved))
    }

    /// Delete all graph data for a user.
    pub async fn delete_user_graph(&self, owner_id: Uuid) -> anyhow::Result<()> {
        self.db
            .query(format!("DELETE nodes WHERE owner_id = '{owner_id}'"))
            .await
            .map_err(|e| anyhow::anyhow!("delete_user_graph: {e}"))?;

        tracing::info!("[SurrealDB] Deleted graph for {owner_id}");
        Ok(())
    }

    pub async fn delete_node_records(&self, node_ids: &[NodeId]) -> anyhow::Result<()> {
        for id in node_ids {
            self.db
                .query("DELETE type::record('nodes', $id)")
                .bind(("id", id.to_string()))
                .await
                .map_err(|e| anyhow::anyhow!("delete_node_records {id}: {e}"))?;
        }
        if !node_ids.is_empty() {
            tracing::info!("[SurrealDB] Deleted {} node record(s)", node_ids.len());
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// `IndexedResults::take` in 3.x requires `SurrealValue`; row shapes deserialize via JSON.
fn rows_from_json<T: for<'de> Deserialize<'de>>(
    result: &mut surrealdb::IndexedResults,
    ctx: &str,
) -> anyhow::Result<Vec<T>> {
    let raw: Vec<serde_json::Value> = result
        .take(0)
        .map_err(|e| anyhow::anyhow!("{ctx}: {e}"))?;
    raw.into_iter()
        .map(|v| serde_json::from_value(v).map_err(|e| anyhow::anyhow!("{ctx} row: {e}")))
        .collect()
}

fn node_uuid_from_record_id(id: &RecordId) -> Uuid {
    match &id.key {
        RecordIdKey::Uuid(u) => (*u).into(),
        RecordIdKey::String(s) => Uuid::parse_str(s).unwrap_or_default(),
        RecordIdKey::Number(n) => {
            Uuid::parse_str(&n.to_string()).unwrap_or_default()
        }
        _ => Uuid::nil(),
    }
}

fn node_uuid_from_record_id_compat(id: &RecordIdCompat) -> Uuid {
    match id {
        RecordIdCompat::Record(r) => node_uuid_from_record_id(r),
        RecordIdCompat::String(s) => {
            // Handles formats like `nodes:550e8400-e29b-41d4-a716-446655440000`
            // and `nodes:`550e8400-e29b-41d4-a716-446655440000``.
            let raw = s
                .split_once(':')
                .map(|(_, rhs)| rhs)
                .unwrap_or(s.as_str())
                .trim_matches('`')
                .trim();
            Uuid::parse_str(raw).unwrap_or_default()
        }
    }
}

/// Human-oriented `table:key` (not full SQL escaping).
fn record_id_compact(r: &RecordId) -> String {
    let key = match &r.key {
        RecordIdKey::Uuid(u) => u.to_string(),
        RecordIdKey::String(s) => s.clone(),
        RecordIdKey::Number(n) => n.to_string(),
        RecordIdKey::Array(_) | RecordIdKey::Object(_) | RecordIdKey::Range(_) => {
            return r.to_sql();
        }
    };
    format!("{}:{key}", r.table.as_str())
}

/// Sanitize edge label for SurrealDB relation name.
/// Relation names must be alphanumeric + underscore only.
fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na:  f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb:  f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}