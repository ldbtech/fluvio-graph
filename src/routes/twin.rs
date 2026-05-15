//! routes/twin.rs
//!
//! Production twin API — replaces fluvio_mock.rs
//!
//! URL structure (frontend updated to match):
//!   GET  /twin/me                    — owner profile (Bearer session)
//!   POST /twin/me/profile            — upsert name, email, phone
//!   POST /twin/setup                 — create account + NFC card
//!   GET  /twin/tap/{card_id}         — NFC tap → connect two users
//!   GET  /twin/network               — social graph (nodes + edges)
//!   GET  /twin/network/{id}          — mini graph for one connection (document nodes from SurrealDB)
//!   POST /twin/ingest                — ingest notes/docs into graph
//!   POST /twin/chat                  — streaming twin chat (Claude)
//!   PUT  /twin/zone/{user_id}        — update zone for a connection

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post, put},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::authentication::extract_bearer_headers;
use crate::graph::structs::{Node, NodeId};
use crate::graph::enums::{Domain, NodeKind};
use crate::graph::fluvio_graph::FluvioGraph;
use crate::database::{
    auth::get_session_by_token,
    users::{create_user, get_user_by_id, update_user, user_physical_scope, CreateUser, User},
    user_uploads::list_user_uploads_for_user,
    cards::{create_card, get_card_by_id, get_cards_by_user},
    connections::{
        create_connection, get_connections, get_connected_user_ids,
        get_connection, update_zone,
    },
};
use crate::storage::surreal::SurrealNodeRow;

// ── Owner resolution ──────────────────────────────────────────────────────────
// `Authorization: Bearer <session>` only — session from POST /twin/auth/verify.
// (`require_logged_in_session` guards the twin router globally; callers still use this for lookups.)

pub(crate) async fn resolve_owner(state: &AppState, headers: &axum::http::HeaderMap) -> Option<User> {
    let token = extract_bearer_headers(headers)?;
    let session = get_session_by_token(&state.pg_pool, &token).await.ok()??;
    get_user_by_id(&state.pg_pool, session.user_id).await.ok()?
}

// ── POST /twin/setup ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SetupBody {
    pub name:  String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Serialize)]
pub struct SetupResponse {
    pub user_id:     Uuid,
    pub physical_id: Uuid,
    pub card_id:     Uuid,
    pub name:        String,
}

pub async fn post_twin_setup(
    State(state): State<AppState>,
    Json(body):   Json<SetupBody>,
) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    if body.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name is required".into()));
    }

    let user = create_user(&state.pg_pool, &CreateUser {
        name:  body.name.trim().to_string(),
        email: body.email.map(|e| e.trim().to_string()),
        phone: body.phone.map(|p| p.trim().to_string()),
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create default NFC card
    let card = create_card(&state.pg_pool, user.id, "nfc")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("[Twin] setup: {} ({})", user.name, user.id);

    Ok(Json(SetupResponse {
        user_id:     user.id,
        physical_id: user.physical_id.unwrap_or(user.id),
        card_id:     card.id,
        name:        user.name,
    }))
}

// ── GET /twin/me ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TwinAccount {
    pub user_id:    Uuid,
    pub owner_slug: String,
    pub display_name: String,
    pub tagline:    String,
    pub email:      String,
    pub phone:      String,
    /// NFC card programmed on `GET /twin/tap/{id}` flows (wallet QR, physical tags).
    pub nfc_card_id: Option<Uuid>,
    /// Stable physical / NFC scope UUID (Postgres `users.physical_id`).
    pub physical_id: Option<Uuid>,
    pub documents:  Vec<TwinDocument>,
    pub connections: Vec<TwinConnection>,
}

#[derive(Serialize)]
pub struct TwinDocument {
    pub id:      String,
    pub title:   String,
    pub kind:    String,
    pub status:  String,
    pub excerpt: String,
}

#[derive(Serialize)]
pub struct TwinConnection {
    pub id:                String,
    pub name:              String,
    pub role:              String,
    pub how_we_met:        String,
    pub relation_strength: f32,
    pub ingested_summary:  String,
}

pub async fn get_twin_me(
    State(state):  State<AppState>,
    headers:       axum::http::HeaderMap,
) -> Result<Json<TwinAccount>, (StatusCode, String)> {
    let user = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required — send Authorization: Bearer <session>".into()))?;

    // Get connections from PostgreSQL
    let db_conns = get_connections(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // For each connection fetch the other user's info
    let mut twin_conns = Vec::new();
    for conn in &db_conns {
        let other_id = if conn.user_a == user.id { conn.user_b } else { conn.user_a };
        if let Ok(Some(other)) = get_user_by_id(&state.pg_pool, other_id).await {
            twin_conns.push(TwinConnection {
                id:                other.id.to_string(),
                name:              other.name.clone(),
                role:              "Connection".to_string(),
                how_we_met:        "NFC tap".to_string(),
                relation_strength: 0.8,
                ingested_summary:  format!(
                    "Connected via NFC. Zone {}.",
                    conn.zone
                ),
            });
        }
    }

    // Get documents from graph — nodes tagged with this user's physical scope
    let docs = get_twin_documents(&state, &user).await;

    let user_cards = get_cards_by_user(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let nfc_card_id = if let Some(c) = user_cards.iter().find(|c| c.card_type == "nfc") {
        Some(c.id)
    } else {
        let c = create_card(&state.pg_pool, user.id, "nfc")
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        tracing::info!("[Twin] issued default NFC card {} for {}", c.id, user.name);
        Some(c.id)
    };

    Ok(Json(TwinAccount {
        user_id:      user.id,
        owner_slug:   slugify(&user.name),
        display_name: user.name.clone(),
        tagline:      "Digital Twin — powered by Fluvio".to_string(),
        email:        user.email.clone().unwrap_or_default(),
        phone:        user.phone.clone().unwrap_or_default(),
        nfc_card_id,
        physical_id: user.physical_id,
        documents:    docs,
        connections:  twin_conns,
    }))
}

// ── POST /twin/me/profile ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProfileBody {
    pub name:  Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

pub async fn post_twin_profile(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
    Json(body):   Json<ProfileBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required — send Authorization: Bearer <session>".into()))?;

    update_user(
        &state.pg_pool,
        user.id,
        body.name.as_deref(),
        body.email.as_deref(),
        body.phone.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "ok": true })))
}

// ── GET /twin/tap/{card_id} ───────────────────────────────────────────────────
// Called when Person B taps Person A's NFC card.
// Creates a connection instantly — no approval needed.

#[derive(Serialize)]
pub struct TapResponse {
    pub connected:    bool,
    pub tapped_user:  TwinPublicProfile,
    pub connection_id: Uuid,
    pub zone:         i16,
}

#[derive(Serialize)]
pub struct TwinPublicProfile {
    pub user_id:      Uuid,
    pub name:         String,
    pub tagline:      String,
    pub physical_id: Option<Uuid>,
    pub card_id:      Uuid,
}

pub async fn get_twin_tap(
    State(state):   State<AppState>,
    headers:        axum::http::HeaderMap,
    Path(card_id):  Path<Uuid>,
) -> Result<Json<TapResponse>, (StatusCode, String)> {
    // Look up who owns this card
    let card = get_card_by_id(&state.pg_pool, card_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, format!("card {card_id} not found")))?;

    let tapped_user = get_user_by_id(&state.pg_pool, card.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".into()))?;

    // Who is tapping? (the person holding the phone)
    let tapper = resolve_owner(&state, &headers).await;

    // Create connection if tapper is known
    let connection = if let Some(tapper) = &tapper {
        if tapper.id != tapped_user.id {
            // Zone 2 = contact + CV tier — matches Dashboard “full share” so peer chat can read zone‑1 ingests.
            let conn = create_connection(&state.pg_pool, tapper.id, tapped_user.id, 2)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Some(conn)
        } else {
            None
        }
    } else {
        None
    };

    tracing::info!(
        "[Twin] tap: card={card_id} owner={} tapper={:?}",
        tapped_user.name,
        tapper.as_ref().map(|u| u.name.as_str())
    );

    Ok(Json(TapResponse {
        connected:    connection.is_some(),
        tapped_user: TwinPublicProfile {
            user_id:  tapped_user.id,
            name:     tapped_user.name,
            tagline:  "Digital Twin — powered by Fluvio".to_string(),
            physical_id: tapped_user.physical_id,
            card_id:  card.id,
        },
        connection_id: connection.as_ref().map(|c| c.id).unwrap_or(Uuid::nil()),
        zone:          connection.as_ref().map(|c| c.zone).unwrap_or(1),
    }))
}

// ── GET /twin/network ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct NetworkGraph {
    pub nodes: Vec<NetworkNode>,
    pub edges: Vec<NetworkEdge>,
}

#[derive(Serialize, Clone)]
pub struct NetworkNode {
    pub id:     String,
    pub label:  String,
    pub page:   String,
    pub source: String,
}

#[derive(Serialize)]
pub struct NetworkEdge {
    pub from:        String,
    pub to:          String,
    pub token:       u32,
    pub probability: f32,
    pub label:       String,
}

pub async fn get_twin_network(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
) -> Result<Json<NetworkGraph>, (StatusCode, String)> {
    let user = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required".into()))?;

    let connected = get_connected_user_ids(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut nodes = vec![NetworkNode {
        id:     user.id.to_string(),
        label:  format!("{} (you)", user.name),
        page:   "profile".to_string(),
        source: "owner".to_string(),
    }];
    let mut edges = vec![];

    for (other_id, zone) in connected {
        if let Ok(Some(other)) = get_user_by_id(&state.pg_pool, other_id).await {
            nodes.push(NetworkNode {
                id:     other.id.to_string(),
                label:  other.name.clone(),
                page:   "connection".to_string(),
                source: "relationship".to_string(),
            });
            edges.push(NetworkEdge {
                from:        user.id.to_string(),
                to:          other.id.to_string(),
                token:       100,
                probability: if zone == 2 { 0.9 } else { 0.75 },
                label:       "knows".to_string(),
            });
        }
    }

    Ok(Json(NetworkGraph { nodes, edges }))
}

// ── GET /twin/network/{id} ────────────────────────────────────────────────────

pub async fn get_twin_network_user(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<Json<NetworkGraph>, (StatusCode, String)> {
    let viewer = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required".into()))?;

    let user = get_user_by_id(&state.pg_pool, user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, format!("user {user_id} not found")))?;

    let hub = NetworkNode {
        id:     user.id.to_string(),
        label:  user.name.clone(),
        page:   "person".to_string(),
        source: "connection".to_string(),
    };

    // Same zone rule as twin chat / status: self sees all own rows; connections see zone <= connection.zone.
    let max_zone: i16 = if viewer.id == user_id {
        2
    } else {
        match get_connection(&state.pg_pool, viewer.id, user_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            Some(c) if (1..=2).contains(&c.zone) => c.zone,
            _ => {
                return Ok(Json(NetworkGraph {
                    nodes: vec![hub],
                    edges: vec![],
                }));
            }
        }
    };

    let surreal_rows = state
        .surreal_storage
        .get_user_nodes(user_id, None, max_zone)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let doc_nodes: Vec<NetworkNode> = surreal_rows
        .iter()
        .filter(|r| twin_document_visible_surreal(r))
        .take(5)
        .map(|r| {
            let n = r.to_node();
            NetworkNode {
                id:     n.id.to_string(),
                label:  n.source_text.chars().take(40).collect(),
                page:   format!("{:?}", n.domain),
                source: "ingest".to_string(),
            }
        })
        .collect();

    let mut nodes = vec![hub.clone()];
    nodes.extend(doc_nodes.clone());

    let edges: Vec<NetworkEdge> = doc_nodes.iter().map(|n| NetworkEdge {
        from:        hub.id.clone(),
        to:          n.id.clone(),
        token:       60,
        probability: 0.85,
        label:       "has".to_string(),
    }).collect();

    Ok(Json(NetworkGraph { nodes, edges }))
}

#[derive(Serialize)]
pub struct PeerGraphStatus {
    pub viewer_user_id:             Uuid,
    pub peer_user_id:               Uuid,
    pub peer_name:                  String,
    pub connected:                  bool,
    pub zone:                       Option<i16>,
    pub surreal_rows_in_zone:       usize,
    pub surreal_workspace_rows:     usize,
    pub pg_user_upload_rows:        usize,
    pub pg_user_upload_kinds:       Vec<String>,
    pub card_based_connection:      bool,
}

/// Debug/status endpoint: verify whether selected peer has persisted Surreal rows visible to viewer's zone.
pub async fn get_twin_network_user_status(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
    Path(peer_id): Path<Uuid>,
) -> Result<Json<PeerGraphStatus>, (StatusCode, String)> {
    let viewer = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required".into()))?;

    let peer = get_user_by_id(&state.pg_pool, peer_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "peer not found".into()))?;

    let conn = get_connection(&state.pg_pool, viewer.id, peer_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let zone = conn.as_ref().map(|c| c.zone);
    let max_zone = zone.unwrap_or(0);
    let connected = conn.is_some();

    let surreal_rows = if connected {
        state
            .surreal_storage
            .get_user_nodes(peer_id, None, max_zone)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };
    let surreal_workspace_rows = surreal_rows
        .iter()
        .filter(|r| surreal_row_workspace_library(r))
        .count();

    let uploads = list_user_uploads_for_user(&state.pg_pool, peer_id, 120)
        .await
        .unwrap_or_default();
    let mut kinds: Vec<String> = uploads.iter().map(|u| u.kind.clone()).collect();
    kinds.sort();
    kinds.dedup();

    Ok(Json(PeerGraphStatus {
        viewer_user_id:        viewer.id,
        peer_user_id:          peer.id,
        peer_name:             peer.name,
        connected,
        zone,
        surreal_rows_in_zone:  surreal_rows.len(),
        surreal_workspace_rows,
        pg_user_upload_rows:   uploads.len(),
        pg_user_upload_kinds:  kinds,
        card_based_connection: connected,
    }))
}

// ── POST /twin/ingest ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngestBody {
    pub title: Option<String>,
    pub body:  Option<String>,
    pub kind:  Option<String>,
}

pub async fn post_twin_ingest(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
    Json(body):   Json<IngestBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required".into()))?;

    let title   = body.title.unwrap_or_else(|| "Untitled note".to_string());
    let content = body.body.unwrap_or_default();
    let kind    = body.kind.unwrap_or_else(|| "note".to_string());

    if content.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "body is required".into()));
    }

    let source_uri = format!("twin://{}/note/{}", user.id, Uuid::new_v4());

    let scope_str = user_physical_scope(&user);
    let mut metadata = HashMap::new();
    metadata.insert("kind".into(),           kind.clone());
    metadata.insert("title".into(),          title.clone());
    metadata.insert("owner_id".into(),       user.id.to_string());
    metadata.insert("owner_physical_id".into(), scope_str);
    metadata.insert("zone".into(),           "1".into());

    // Embed + insert into in-memory graph in a short scope so no MutexGuard crosses await.
    let node_for_surreal = {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let embeddings = pipeline.embed_ctx.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .embed(&content)
            .unwrap_or_default();

        let node = Node {
            id:          NodeId::from_content("twin_note", &source_uri),
            domain:      Domain::Custom("twin".to_string()),
            source_uri:  source_uri.clone(),
            source_text: format!("{title}\n\n{content}"),
            embeddings,
            metadata,
            kind:        NodeKind::Artifcat,
        };

        pipeline
            .graph
            .insert_node(node.clone())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        node
    };

    // Surreal is authoritative for chat context; fail ingest if persistence fails.
    state
        .surreal_storage
        .upsert_node(user.id, &node_for_surreal, 1)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("SurrealDB upsert_node failed: {e}"),
            )
        })?;

    tracing::info!("[Twin] ingested {kind} note for {}", user.name);

    Ok(Json(json!({
        "ok": true,
        "document": {
            "id":      source_uri,
            "title":   title,
            "kind":    kind,
            "status":  "indexed",
            "excerpt": content.chars().take(200).collect::<String>(),
        }
    })))
}

// ── PUT /twin/zone/{user_id} ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ZoneBody {
    pub zone: i16,
}

pub async fn put_twin_zone(
    State(state):  State<AppState>,
    headers:       axum::http::HeaderMap,
    Path(other_id): Path<Uuid>,
    Json(body):    Json<ZoneBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.zone < 1 || body.zone > 2 {
        return Err((StatusCode::BAD_REQUEST, "zone must be 1 or 2".into()));
    }

    let user = resolve_owner(&state, &headers)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "sign in required".into()))?;

    let updated = update_zone(&state.pg_pool, user.id, other_id, body.zone)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "ok": updated, "zone": body.zone })))
}

// ── POST /twin/chat ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TwinChatMessage {
    pub role:    String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct TwinChatRequest {
    pub messages:      Vec<TwinChatMessage>,
    #[serde(default)]
    pub graph_context: Option<String>,
    /// When set, load Surreal graph for this user (must be you or an NFC connection).
    #[serde(default)]
    pub graph_owner_id: Option<Uuid>,
}

// ── Chat knowledge scope (Surreal-backed) ────────────────────────────────────

#[derive(Clone)]
struct ChatKnowledgeScope {
    /// Whose `nodes.owner_id` rows to read from SurrealDB.
    pub owner_id:     Uuid,
    pub max_zone:     i16,
    /// Person the context describes (prompt copy).
    pub subject_name: String,
    pub is_self:      bool,
}

async fn resolve_chat_knowledge_scope(
    state:              &AppState,
    viewer:             Option<&User>,
    graph_owner_param:  Option<Uuid>,
) -> Result<Option<ChatKnowledgeScope>, (StatusCode, String)> {
    let Some(viewer) = viewer else {
        if graph_owner_param.is_some() {
            return Err((
                StatusCode::UNAUTHORIZED,
                "sign in required to load another user's graph".into(),
            ));
        }
        return Ok(None);
    };

    let target = graph_owner_param.unwrap_or(viewer.id);
    if target == viewer.id {
        return Ok(Some(ChatKnowledgeScope {
            owner_id:     viewer.id,
            max_zone:     2,
            subject_name: viewer.name.clone(),
            is_self:      true,
        }));
    }

    let other_id = target;
    let conn = get_connection(&state.pg_pool, viewer.id, other_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::FORBIDDEN,
            "not connected — only NFC connections can load each other's graphs".into(),
        ))?;
    let other = get_user_by_id(&state.pg_pool, other_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".into()))?;
    Ok(Some(ChatKnowledgeScope {
        owner_id:     other_id,
        max_zone:     conn.zone,
        subject_name: other.name,
        is_self:      false,
    }))
}

pub async fn post_twin_chat(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
    Json(req):    Json<TwinChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let user = resolve_owner(&state, &headers).await;

    let msgs: Vec<serde_json::Value> = req.messages.iter()
        .filter(|m| (m.role == "user" || m.role == "assistant") && !m.content.trim().is_empty())
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    if msgs.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "messages required".into()));
    }

    let scope = resolve_chat_knowledge_scope(&state, user.as_ref(), req.graph_owner_id)
        .await?;

    let query_hint = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.as_str());

    let knowledge = build_knowledge_context(
        &state,
        user.as_ref(),
        scope.as_ref(),
        query_hint,
    )
    .await;

    let name = user
        .as_ref()
        .map(|u| u.name.as_str())
        .unwrap_or("this person");

    let subject = scope
        .as_ref()
        .map(|s| s.subject_name.as_str())
        .unwrap_or(name);

    let mut system = if scope.as_ref().map(|s| s.is_self).unwrap_or(true) {
        format!(
            "You are {subject}'s AI twin. You speak in first person as {subject}.\n\
             Answer questions naturally and conversationally as if you are them.\n\
             Ground your answers in the KNOWLEDGE CONTEXT below (persisted knowledge graph for {subject}).\n\
             If something is not in the context, say so honestly in their voice.\n\n\
             KNOWLEDGE CONTEXT:\n{knowledge}"
        )
    } else {
        format!(
            "The user you are assisting ({name}) is asking about {subject}.\n\
             KNOWLEDGE CONTEXT below is {subject}'s ingested materials (PDF, video, notes, etc.) \
             shared with {name} per your NFC connection zone — not {name}'s own uploads.\n\
             Describe {subject} in third person. Do not pretend to be {subject} unless the user asks for a role-play.\n\
             If the context does not support a claim, say so clearly.\n\n\
             KNOWLEDGE CONTEXT:\n{knowledge}"
        )
    };

    if let Some(gc) = req.graph_context.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        system.push_str("\n\nGRAPH VIEW (user's current selection):\n");
        system.push_str(gc);
    }

    let (tx, rx) = mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(64);
    let api_key  = state.api_key.clone();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let res = match client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key",         &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type",      "application/json")
            .json(&json!({
                "model":      "claude-sonnet-4-20250514",
                "max_tokens": 4096,
                "stream":     true,
                "system":     system,
                "messages":   msgs,
            }))
            .send().await
        {
            Ok(r)  => r,
            Err(e) => {
                let _ = tx.send(Err(std::io::Error::new(
                    std::io::ErrorKind::Other, e.to_string()
                ))).await;
                return;
            }
        };

        if !res.status().is_success() {
            let txt = res.text().await.unwrap_or_default();
            let _ = tx.send(Err(std::io::Error::new(
                std::io::ErrorKind::Other, txt
            ))).await;
            return;
        }

        let mut stream = res.bytes_stream();
        let mut carry  = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(b)  => b,
                Err(e) => {
                    let _ = tx.send(Err(std::io::Error::new(
                        std::io::ErrorKind::Other, e.to_string()
                    ))).await;
                    return;
                }
            };

            carry.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = carry.find('\n') {
                let line = carry[..pos].trim_end_matches('\r').to_string();
                carry.drain(..=pos);
                if line.is_empty() { continue; }

                let payload = match line.strip_prefix("data:") {
                    Some(p) => p.trim(),
                    None    => continue,
                };
                if payload == "[DONE]" { drop(tx); return; }

                let v: serde_json::Value = match serde_json::from_str(payload) {
                    Ok(v)  => v,
                    Err(_) => continue,
                };

                if v.get("type").and_then(|x| x.as_str()) == Some("content_block_delta") {
                    if let Some(t) = v.pointer("/delta/text").and_then(|x| x.as_str()) {
                        if tx.send(Ok(bytes::Bytes::copy_from_slice(t.as_bytes()))).await.is_err() {
                            return;
                        }
                    }
                }
            }
        }
        drop(tx);
    });

    let body = Body::from_stream(ReceiverStream::new(rx));
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type",  "text/plain; charset=utf-8")
        .header("cache-control", "no-store")
        .body(body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
}

// ── Helpers ───────────────────────────────────────────────────────────────────


/// PDF / Map ingests etc. — heuristic on persisted Surreal row (`domain` is `Debug` from ingest).
fn surreal_row_workspace_library(n: &SurrealNodeRow) -> bool {
    let src = n
        .metadata
        .get("source")
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(
        src.as_str(),
        "pdf" | "email" | "gmail" | "codebase" | "video" | "architecture" | "tools"
    ) {
        return true;
    }
    let d = n.domain.as_str();
    matches!(d, "Pdf" | "Email" | "Codebase" | "Architecture") || d == r#"Custom("video")"#
}

fn knowledge_context_line_surreal(n: &SurrealNodeRow, max_chars: usize) -> String {
    let kind = n
        .metadata
        .get("kind")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| n.metadata.get("source").map(|s| s.as_str()))
        .unwrap_or("chunk");
    let tag = n
        .metadata
        .get("filename")
        .or_else(|| n.metadata.get("title"))
        .map(|s| format!(" `{}`", s.replace('`', "'")))
        .unwrap_or_default();
    let excerpt: String = n.source_text.chars().take(max_chars).collect();
    format!("- [{kind}]{tag}\n  {excerpt}")
}

fn surreal_row_dedup_key(r: &SurrealNodeRow) -> String {
    format!("{}|{}", r.source_uri, r.domain)
}

/// Primary path: read nodes for `scope.owner_id` from SurrealDB (embedding-ranked + anchors + breadth).
async fn build_knowledge_from_surreal(
    state:       &AppState,
    scope:       &ChatKnowledgeScope,
    query_hint:  Option<&str>,
) -> Option<String> {
    const WS_MAX_NODES: usize = 64;
    const WS_CHARS_PER_NODE: usize = 720;
    const WS_CTX_CAP: usize = 28_000;
    const SIM_TOP: usize = 36;

    let all_rows = match state
        .surreal_storage
        .get_user_nodes(scope.owner_id, None, scope.max_zone)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[Twin chat] Surreal get_user_nodes failed: {e}");
            return None;
        }
    };

    if all_rows.is_empty() {
        return None;
    }

    let mut ordered: Vec<SurrealNodeRow> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let q = query_hint.map(str::trim).filter(|q| !q.is_empty());
    if let Some(question) = q {
        let qvec = (|| {
            let pipeline = state.pipeline.lock().ok()?;
            let mut ctx = pipeline.embed_ctx.lock().ok()?;
            ctx.embed(question).ok()
        })();
        if let Some(vec) = qvec {
            match state
                .surreal_storage
                .similarity_search_nodes(scope.owner_id, &vec, SIM_TOP, scope.max_zone)
                .await
            {
                Ok(sim) => {
                    for row in sim {
                        let k = surreal_row_dedup_key(&row);
                        if seen.insert(k) {
                            ordered.push(row);
                        }
                    }
                }
                Err(e) => tracing::warn!("[Twin chat] Surreal similarity_search_nodes failed: {e}"),
            }
        }
    }

    for row in &all_rows {
        if row.metadata.get("dashboard_doc_anchor").map(|s| s.as_str()) == Some("1") {
            let k = surreal_row_dedup_key(row);
            if seen.insert(k) {
                ordered.push(row.clone());
            }
        }
    }

    if ordered.len() < 12 {
        let mut rest: Vec<SurrealNodeRow> = all_rows
            .iter()
            .filter(|r| surreal_row_workspace_library(r))
            .cloned()
            .collect();
        rest.sort_by(|a, b| a.source_uri.cmp(&b.source_uri));
        for row in rest {
            let k = surreal_row_dedup_key(&row);
            if seen.insert(k) {
                ordered.push(row);
            }
            if ordered.len() >= WS_MAX_NODES {
                break;
            }
        }
    }

    if ordered.is_empty() {
        ordered = all_rows;
        ordered.sort_by(|a, b| a.source_uri.cmp(&b.source_uri));
        ordered.truncate(WS_MAX_NODES);
    } else {
        ordered.truncate(WS_MAX_NODES);
    }

    let mut user_workspace: Vec<String> = ordered
        .iter()
        .filter(|n| surreal_row_workspace_library(n))
        .map(|n| knowledge_context_line_surreal(n, WS_CHARS_PER_NODE))
        .collect();
    user_workspace.sort();
    user_workspace.truncate(WS_MAX_NODES);
    while user_workspace.join("\n\n").len() > WS_CTX_CAP && user_workspace.len() > 8 {
        user_workspace.pop();
    }

    let twin_lines: Vec<String> = ordered
        .iter()
        .filter(|n| !surreal_row_workspace_library(n))
        .map(|n| knowledge_context_line_surreal(n, 520))
        .collect();

    let workspace_block = user_workspace.join("\n\n");

    if workspace_block.is_empty() && twin_lines.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    if !workspace_block.is_empty() {
        parts.push(format!(
            "## {}'s workspace (PDF, video, code, email — from SurrealDB)\n{}",
            scope.subject_name, workspace_block
        ));
    }
    if !twin_lines.is_empty() {
        parts.push(format!(
            "## {} — notes & twin context (SurrealDB)\n{}",
            scope.subject_name,
            twin_lines.join("\n\n")
        ));
    }

    Some(parts.join("\n\n"))
}


/// Profile + Surreal-backed knowledge only (no RAM fallback).
async fn build_knowledge_context(
    state:        &AppState,
    viewer:       Option<&User>,
    scope:        Option<&ChatKnowledgeScope>,
    query_hint:   Option<&str>,
) -> String {
    let Some(viewer) = viewer else {
        return "No profile data available.".to_string();
    };

    let Some(scope) = scope else {
        return "Sign in to load a knowledge graph from SurrealDB.".to_string();
    };

    let profile_block = format!(
        "## Your profile (signed-in account)\n{}\nEmail: {}\nPhone: {}",
        viewer.name,
        viewer.email.as_deref().unwrap_or("not provided"),
        viewer.phone.as_deref().unwrap_or("not provided"),
    );

    if let Some(body) = build_knowledge_from_surreal(state, scope, query_hint).await {
        return vec![profile_block, body].join("\n\n");
    }

    // Surreal-only mode: if there are no rows, be explicit (do not fall back to RAM graph).
    if let Ok(Some(ref peer)) = get_user_by_id(&state.pg_pool, scope.owner_id).await {
        let about = if scope.is_self {
            format!(
                "## {} — account (from PostgreSQL)\n\
                 No SurrealDB chunks were found for your account yet. Uploads may not have persisted.\n\
                 - Name: {}\n\
                 - Email: {}\n\
                 - Phone: {}",
                scope.subject_name,
                peer.name,
                peer.email.as_deref().unwrap_or("not provided"),
                peer.phone.as_deref().unwrap_or("not provided"),
            )
        } else {
            format!(
                "## {} — account (from PostgreSQL)\n\
                 No SurrealDB chunks were found for them in your current connection zone.\n\
                 - Name: {}\n\
                 - Email: {}\n\
                 - Phone: {}",
                scope.subject_name,
                peer.name,
                peer.email.as_deref().unwrap_or("not provided"),
                peer.phone.as_deref().unwrap_or("not provided"),
            )
        };
        return vec![profile_block, about].join("\n\n");
    }

    let missing = if scope.is_self {
        "No Surreal-backed ingests were found for your account, and your user profile could not be loaded."
            .to_string()
    } else {
        format!(
            "No Surreal-backed ingests from {} were found in this zone, and we could not load their user profile.",
            scope.subject_name
        )
    };
    vec![profile_block, missing].join("\n\n")
}

/// Hide per-page PDF chunks and per-scene video rows from the Documents list (summary rows only).
fn twin_document_visible_surreal(r: &SurrealNodeRow) -> bool {
    if r.metadata.get("kind").map(|s| s.as_str()) == Some("scene") {
        return false;
    }
    if r.metadata.get("source").map(|s| s.as_str()) == Some("pdf") {
        return r.metadata.get("dashboard_doc_anchor").map(|s| s.as_str()) == Some("1");
    }
    true
}

fn twin_document_from_surreal_row(r: &SurrealNodeRow) -> TwinDocument {
    let src = r.metadata.get("source").cloned().unwrap_or_default();
    let kind = r
        .metadata
        .get("kind")
        .cloned()
        .unwrap_or_else(|| src.clone())
        .to_ascii_lowercase();
    let title = r
        .metadata
        .get("title")
        .or_else(|| r.metadata.get("filename"))
        .cloned()
        .unwrap_or_else(|| match kind.as_str() {
            "pdf" => "PDF upload".into(),
            "video" => "Video upload".into(),
            "note" | "" => "Note".into(),
            _ => kind.clone(),
        });
    TwinDocument {
        id:      r.source_uri.clone(),
        title,
        kind,
        status:  "indexed".to_string(),
        excerpt: r.source_text.chars().take(200).collect(),
    }
}

/// Documents for the dashboard: **SurrealDB** rows for this user (`owner_id`), not the shared RAM graph.
async fn get_twin_documents(state: &AppState, user: &User) -> Vec<TwinDocument> {
    const MAX_ZONE_SELF: i16 = 2;
    const MAX_DOCS: usize = 500;

    let rows = match state
        .surreal_storage
        .get_user_nodes(user.id, None, MAX_ZONE_SELF)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[Twin] get_user_nodes for documents list failed: {e}");
            return vec![];
        }
    };

    rows.iter()
        .filter(|r| twin_document_visible_surreal(r))
        .take(MAX_DOCS)
        .map(twin_document_from_surreal_row)
        .collect()
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

// ── Routers ────────────────────────────────────────────────────────────────────

/// `POST /twin/setup` — creates a user outside email OTP (merged **without** `require_logged_in_session`).
pub fn public_onboarding_routes() -> Router<AppState> {
    Router::new().route("/twin/setup", post(post_twin_setup))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/twin/me",                 get(get_twin_me))
        .route("/twin/me/profile",         post(post_twin_profile))
        .route("/twin/tap/{card_id}",      get(get_twin_tap))
        .route("/twin/network",            get(get_twin_network))
        .route("/twin/network/{id}",       get(get_twin_network_user))
        .route("/twin/network/{id}/status", get(get_twin_network_user_status))
        .route("/twin/ingest",             post(post_twin_ingest))
        .route("/twin/chat",               post(post_twin_chat))
        .route("/twin/zone/{user_id}",     put(put_twin_zone))
}