//! routes/twin.rs
//!
//! Production twin API — replaces fluvio_mock.rs
//!
//! URL structure (frontend updated to match):
//!   GET  /twin/me                    — get owner profile + docs + connections
//!   POST /twin/me/profile            — upsert name, email, phone
//!   POST /twin/setup                 — create account + NFC card
//!   GET  /twin/tap/{card_id}         — NFC tap → connect two users
//!   GET  /twin/network               — social graph (nodes + edges)
//!   GET  /twin/network/{id}          — mini graph for one connection
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
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::database::{
    users::{create_user, get_user_by_id, update_user, CreateUser, User},
    cards::{create_card, get_card_by_id, get_cards_by_user},
    connections::{
        create_connection, get_connections, get_connected_user_ids,
        update_zone,
    },
};

// ── Owner resolution ──────────────────────────────────────────────────────────
// Owner UUID is stored in PostgreSQL. The browser keeps it in localStorage after
// POST /twin/setup and sends `X-Owner-ID` on subsequent requests.
// There is no anonymous fallback — without a valid header you cannot read another user's dashboard.

async fn resolve_owner(state: &AppState, owner_id: Option<Uuid>) -> Option<User> {
    let id = owner_id?;
    get_user_by_id(&state.pg_pool, id).await.ok().flatten()
}

fn owner_id_from_header(headers: &axum::http::HeaderMap) -> Option<Uuid> {
    headers
        .get("x-owner-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
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
    pub user_id:  Uuid,
    pub graph_id: Uuid,
    pub card_id:  Uuid,
    pub name:     String,
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
        user_id:  user.id,
        graph_id: user.graph_id.unwrap_or(user.id),
        card_id:  card.id,
        name:     user.name,
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
    pub graph_id:   Option<Uuid>,
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
    let owner_id = owner_id_from_header(&headers);
    let user     = resolve_owner(&state, owner_id).await
        .ok_or((StatusCode::NOT_FOUND, "no account found — POST /twin/setup first".into()))?;

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

    // Get documents from graph — nodes tagged with this user's graph_id
    let docs = get_twin_documents(&state, &user).await;

    let user_cards = get_cards_by_user(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let nfc_card_id = user_cards.into_iter().find(|c| c.card_type == "nfc").map(|c| c.id);

    Ok(Json(TwinAccount {
        user_id:      user.id,
        owner_slug:   slugify(&user.name),
        display_name: user.name.clone(),
        tagline:      "Digital Twin — powered by Fluvio".to_string(),
        email:        user.email.clone().unwrap_or_default(),
        phone:        user.phone.clone().unwrap_or_default(),
        nfc_card_id,
        graph_id:     user.graph_id,
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
    let owner_id = owner_id_from_header(&headers);
    let user     = resolve_owner(&state, owner_id).await
        .ok_or((StatusCode::NOT_FOUND, "no account — POST /twin/setup first".into()))?;

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
    pub graph_id:     Option<Uuid>,
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
    let tapper = resolve_owner(&state, owner_id_from_header(&headers)).await;

    // Create connection if tapper is known
    let connection = if let Some(tapper) = &tapper {
        if tapper.id != tapped_user.id {
            let conn = create_connection(&state.pg_pool, tapper.id, tapped_user.id, 1)
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
            graph_id: tapped_user.graph_id,
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
    let owner_id = owner_id_from_header(&headers);
    let user     = resolve_owner(&state, owner_id).await
        .ok_or((StatusCode::NOT_FOUND, "no account".into()))?;

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
    Path(user_id): Path<Uuid>,
) -> Result<Json<NetworkGraph>, (StatusCode, String)> {
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

    // Show their graph nodes as mini-graph
    let pipeline = state.pipeline.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let graph_id_str = user.graph_id
        .map(|g| g.to_string())
        .unwrap_or_default();

    let doc_nodes: Vec<NetworkNode> = pipeline.graph.nodes.values()
        .filter(|n| {
            n.metadata.get("owner_graph_id")
                .map(|v| v == &graph_id_str)
                .unwrap_or(false)
        })
        .take(5)
        .map(|n| NetworkNode {
            id:     n.id.to_string(),
            label:  n.source_text.chars().take(40).collect(),
            page:   format!("{:?}", n.domain),
            source: "ingest".to_string(),
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
    let owner_id = owner_id_from_header(&headers);
    let user     = resolve_owner(&state, owner_id).await
        .ok_or((StatusCode::NOT_FOUND, "no account".into()))?;

    let title   = body.title.unwrap_or_else(|| "Untitled note".to_string());
    let content = body.body.unwrap_or_default();
    let kind    = body.kind.unwrap_or_else(|| "note".to_string());

    if content.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "body is required".into()));
    }

    // Ingest into graph as a note node
    let graph_id = user.graph_id.unwrap_or(user.id).to_string();
    let source_uri = format!("twin://{}/note/{}", user.id, Uuid::new_v4());

    use crate::graph::enums::{Domain, NodeKind};
    use crate::graph::structs::{Node, NodeId};
    use crate::graph::fluvio_graph::FluvioGraph;
    use std::collections::HashMap;

    let mut metadata = HashMap::new();
    metadata.insert("kind".into(),           kind.clone());
    metadata.insert("title".into(),          title.clone());
    metadata.insert("owner_id".into(),       user.id.to_string());
    metadata.insert("owner_graph_id".into(), graph_id.clone());
    metadata.insert("zone".into(),           "1".into());

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

    pipeline.graph.insert_node(node)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("[Twin] ingested {} note for {}", kind, user.name);

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

    let owner_id = owner_id_from_header(&headers);
    let user     = resolve_owner(&state, owner_id).await
        .ok_or((StatusCode::NOT_FOUND, "no account".into()))?;

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
}

pub async fn post_twin_chat(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
    Json(req):    Json<TwinChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let owner_id = owner_id_from_header(&headers);
    let user     = resolve_owner(&state, owner_id).await;

    let msgs: Vec<serde_json::Value> = req.messages.iter()
        .filter(|m| (m.role == "user" || m.role == "assistant") && !m.content.trim().is_empty())
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    if msgs.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "messages required".into()));
    }

    // Build system prompt from real graph data
    let knowledge = build_knowledge_context(&state, user.as_ref()).await;
    let name      = user.as_ref().map(|u| u.name.as_str()).unwrap_or("this person");

    let mut system = format!(
        "You are {name}'s AI twin. You speak in first person as {name}.\n\
         Answer questions naturally and conversationally as if you are them.\n\
         Ground your answers in the KNOWLEDGE CONTEXT below.\n\
         If something is not in the context, say so honestly in their voice.\n\n\
         KNOWLEDGE CONTEXT:\n{knowledge}"
    );

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

/// Build knowledge context string from the user's graph nodes.
async fn build_knowledge_context(state: &AppState, user: Option<&User>) -> String {
    let Some(user) = user else {
        return "No profile data available.".to_string();
    };

    let graph_id = user.graph_id
        .map(|g| g.to_string())
        .unwrap_or_default();

    let pipeline = match state.pipeline.lock() {
        Ok(p)  => p,
        Err(_) => return "Graph unavailable.".to_string(),
    };

    let nodes: Vec<String> = pipeline.graph.nodes.values()
        .filter(|n| {
            n.metadata.get("owner_graph_id")
                .map(|v| v == &graph_id)
                .unwrap_or(false)
        })
        .map(|n| format!("- [{}] {}", n.metadata.get("kind").map(|s| s.as_str()).unwrap_or("note"), n.source_text.chars().take(300).collect::<String>()))
        .collect();

    if nodes.is_empty() {
        return format!(
            "## Profile\n{}\n\nEmail: {}\nPhone: {}",
            user.name,
            user.email.as_deref().unwrap_or("not provided"),
            user.phone.as_deref().unwrap_or("not provided"),
        );
    }

    format!(
        "## Profile\n{}\nEmail: {}\nPhone: {}\n\n## Knowledge\n{}",
        user.name,
        user.email.as_deref().unwrap_or("not provided"),
        user.phone.as_deref().unwrap_or("not provided"),
        nodes.join("\n")
    )
}

/// Get documents from graph for a user.
async fn get_twin_documents(state: &AppState, user: &User) -> Vec<TwinDocument> {
    let graph_id = user.graph_id.map(|g| g.to_string()).unwrap_or_default();

    let pipeline = match state.pipeline.lock() {
        Ok(p)  => p,
        Err(_) => return vec![],
    };

    pipeline.graph.nodes.values()
        .filter(|n| {
            n.metadata.get("owner_graph_id")
                .map(|v| v == &graph_id)
                .unwrap_or(false)
        })
        .map(|n| TwinDocument {
            id:      n.source_uri.clone(),
            title:   n.metadata.get("title").cloned().unwrap_or_else(|| "Note".to_string()),
            kind:    n.metadata.get("kind").cloned().unwrap_or_else(|| "note".to_string()),
            status:  "indexed".to_string(),
            excerpt: n.source_text.chars().take(200).collect(),
        })
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

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/twin/setup",              post(post_twin_setup))
        .route("/twin/me",                 get(get_twin_me))
        .route("/twin/me/profile",         post(post_twin_profile))
        .route("/twin/tap/{card_id}",      get(get_twin_tap))
        .route("/twin/network",            get(get_twin_network))
        .route("/twin/network/{id}",       get(get_twin_network_user))
        .route("/twin/ingest",             post(post_twin_ingest))
        .route("/twin/chat",               post(post_twin_chat))
        .route("/twin/zone/{user_id}",     put(put_twin_zone))
}