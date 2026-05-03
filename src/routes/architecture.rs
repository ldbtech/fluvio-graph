// POST /architecture/generate
// Takes a brief, runs the full pipeline, returns scene + ingests into graph

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::AppState;
use crate::ingestion_registry::architecture::{
    extract_space_program, space_program_to_chunks,
    compute_layout, generate_scene, merge_llm_artifacts_into_scene, ArchitectureScene, NaturalLight,
    PrivacyLevel, RelationshipKind,
    Room, RoomRelationship, SpaceProgram, Zone,
};

const ARCH_DESIGNS_DIR: &str = "fluvio_graph/workspace/architecture";

// ---- Helper Functions --------------------------------------------------------
fn design_path(design_id: &str) -> String {
    format!("{ARCH_DESIGNS_DIR}/{design_id}.json")
}

fn save_design(program: &SpaceProgram) -> anyhow::Result<()> {
    std::fs::create_dir_all(ARCH_DESIGNS_DIR)?;
    let json = serde_json::to_string_pretty(program)?;
    std::fs::write(design_path(&program.design_id), json)?;
    Ok(())
}

fn load_design(design_id: &str) -> Result<SpaceProgram, (StatusCode, String)> {
    let path = design_path(design_id);

    if !std::path::Path::new(&path).exists() {
        return Err((StatusCode::NOT_FOUND, format!("design '{}' not found", design_id)));
    }

    let json = std::fs::read_to_string(&path)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    serde_json::from_str(&json)
         .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}



#[derive(Deserialize)]
pub struct GenerateBody {
    pub brief: String,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub design_id: String,
    pub scene:     ArchitectureScene,
    pub chunks:    usize,
    pub nodes:     usize,
    pub edges:     usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomPatchOp {
    SetName { value: String },
    SetAreaSqm { value: f32 },
    SetZone { value: Zone },
    SetNaturalLight { value: NaturalLight },
    SetPrivacy { value: PrivacyLevel },
    SetNotes { value: Option<String> },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignEdit {
    AddRoom { room: Room },
    UpdateRoom { room_id: String, ops: Vec<RoomPatchOp> },
    RemoveRoom { room_id: String },
    AddRelationship { relationship: RoomRelationship },
    RemoveRelationship { from_id: String, to_id: String, kind: RelationshipKind },
}

#[derive(Deserialize)]
pub struct ModifyBody {
    pub design_id: String,
    pub edits: Vec<DesignEdit>,
}

#[derive(Serialize)]
pub struct ModifyResponse {
    pub design_id: String,
    pub scene: ArchitectureScene,
    pub rooms: usize,
    pub relationships: usize,
}

fn apply_room_ops(room: &mut Room, ops: &[RoomPatchOp]) {
    for op in ops {
        match op {
            RoomPatchOp::SetName { value } => room.name = value.trim().to_string(),
            RoomPatchOp::SetAreaSqm { value } => room.area_sqm = *value,
            RoomPatchOp::SetZone { value } => room.zone = value.clone(),
            RoomPatchOp::SetNaturalLight { value } => room.natural_light = value.clone(),
            RoomPatchOp::SetPrivacy { value } => room.privacy = value.clone(),
            RoomPatchOp::SetNotes { value } => room.notes = value.clone(),
        }
    }
}

fn validate_program(program: &SpaceProgram) -> Result<(), (StatusCode, String)> {
    if program.rooms.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "design must contain at least one room".to_string()));
    }
    let mut ids = std::collections::HashSet::new();
    for room in &program.rooms {
        if room.id.trim().is_empty() {
            return Err((StatusCode::BAD_REQUEST, "room id cannot be empty".to_string()));
        }
        if room.area_sqm <= 0.0 {
            return Err((StatusCode::BAD_REQUEST, format!("room '{}' area_sqm must be > 0", room.id)));
        }
        if !ids.insert(room.id.clone()) {
            return Err((StatusCode::BAD_REQUEST, format!("duplicate room id '{}'", room.id)));
        }
    }
    for rel in &program.relationships {
        if !ids.contains(&rel.from_id) || !ids.contains(&rel.to_id) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("relationship references missing room: {} -> {}", rel.from_id, rel.to_id),
            ));
        }
    }
    Ok(())
}

fn apply_edits(mut program: SpaceProgram, edits: &[DesignEdit]) -> Result<SpaceProgram, (StatusCode, String)> {
    for edit in edits {
        match edit {
            DesignEdit::AddRoom { room } => {
                if program.rooms.iter().any(|r| r.id == room.id) {
                    return Err((StatusCode::BAD_REQUEST, format!("room '{}' already exists", room.id)));
                }
                program.rooms.push(room.clone());
            }
            DesignEdit::UpdateRoom { room_id, ops } => {
                let Some(room) = program.rooms.iter_mut().find(|r| r.id == *room_id) else {
                    return Err((StatusCode::NOT_FOUND, format!("room '{}' not found", room_id)));
                };
                apply_room_ops(room, ops);
            }
            DesignEdit::RemoveRoom { room_id } => {
                let before = program.rooms.len();
                program.rooms.retain(|r| r.id != *room_id);
                if before == program.rooms.len() {
                    return Err((StatusCode::NOT_FOUND, format!("room '{}' not found", room_id)));
                }
                program.relationships.retain(|rel| rel.from_id != *room_id && rel.to_id != *room_id);
            }
            DesignEdit::AddRelationship { relationship } => {
                program.relationships.push(relationship.clone());
            }
            DesignEdit::RemoveRelationship { from_id, to_id, kind } => {
                let before = program.relationships.len();
                program.relationships.retain(|rel| {
                    !(rel.from_id == *from_id && rel.to_id == *to_id && rel.kind == *kind)
                });
                if before == program.relationships.len() {
                    return Err((
                        StatusCode::NOT_FOUND,
                        format!("relationship not found: {} -> {} ({})", from_id, to_id, kind.as_label()),
                    ));
                }
            }
        }
    }
    validate_program(&program)?;
    Ok(program)
}

pub async fn post_architecture_generate(
    State(state): State<AppState>,
    Json(body):   Json<GenerateBody>,
) -> Result<Json<GenerateResponse>, (StatusCode, String)> {
    let brief     = body.brief.trim().to_string();
    let design_id = Uuid::new_v4().to_string();
    let api_key   = state.api_key.clone();
    let did       = design_id.clone();

    if brief.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "brief is empty".to_string()));
    }

    // Extract space program via LLM.
    let program = extract_space_program(&brief, &did, &api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Compute layout.
    let positions = compute_layout(&program);

    // Generate Three.js scene.
    let scene = generate_scene(&program, &positions);

    {
        let mut design_store = state
            .architecture_designs
            .lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        design_store.insert(design_id.clone(), program.clone());
        save_design(&program)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Convert to NormalizedChunks and ingest into graph.
    let chunks = space_program_to_chunks(&program, 0);
    let chunk_count = chunks.len();

    let (nodes, edges) = {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let (_nodes_added, _edges_added) = pipeline
            .ingest_normalized_chunks(&chunks)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        pipeline.wire_edges(0.35);
        let total_nodes = pipeline.graph.nodes.len();
        let total_edges: usize = pipeline.graph.adj.values().map(|e| e.len()).sum();
        (state.presist)(&pipeline.graph)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        (total_nodes, total_edges)
    };

    Ok(Json(GenerateResponse {
        design_id,
        scene,
        chunks: chunk_count,
        nodes,
        edges,
    }))
}

pub async fn post_architecture_modify(
    State(state): State<AppState>,
    Json(body): Json<ModifyBody>,
) -> Result<Json<ModifyResponse>, (StatusCode, String)> {
    let design_id = body.design_id.trim().to_string();
    if design_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "design_id is empty".to_string()));
    }
    if body.edits.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "edits cannot be empty".to_string()));
    }

    let mut design_store = state
        .architecture_designs
        .lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let current = design_store
        .get(&design_id)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, format!("unknown design_id '{}'", design_id)))?;

    let updated = apply_edits(current, &body.edits)?;
    let positions = compute_layout(&updated);
    let scene = generate_scene(&updated, &positions);
    let rooms = updated.rooms.len();
    let relationships = updated.relationships.len();

   // design_store.insert(design_id.clone(), updated.clone());
    save_design(&updated)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    {
        let mut store = state.architecture_designs.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        store.insert(design_id.clone(), updated.clone());
    }

    Ok(Json(ModifyResponse {
        design_id,
        scene,
        rooms,
        relationships,
    }))
}

/// ---- Chat Endpint: ------------------------------------------------------------
#[derive(Deserialize)]
pub struct ArchChatBody {
    pub design_id:        String,
    pub selected_room_id: Option<String>,
    pub message:          String,
}

#[derive(Serialize)]
pub struct ArchChatResponse {
    pub design_id: String,
    pub scene:     ArchitectureScene,
    pub answer:    String,
    pub changes:   Vec<serde_json::Value>,
}

pub async fn post_architecture_chat(
    State(state): State<AppState>,
    Json(body):   Json<ArchChatBody>,
) -> Result<Json<ArchChatResponse>, (StatusCode, String)> {
    let design_id = body.design_id.trim().to_string();
    let message   = body.message.trim().to_string();

    if design_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "design_id is empty".to_string()));
    }
    if message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is empty".to_string()));
    }

    // Load current design.
    let program = {
        let store = state.architecture_designs.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        store.get(&design_id).cloned()
    };
    let program = match program {
        Some(p) => p,
        None    => load_design(&design_id)?,
    };

    // Build room context for the LLM.
    let selected_room = body.selected_room_id.as_deref()
        .and_then(|id| program.room_by_id(id));

    let room_context = if let Some(room) = selected_room {
        let rels = program.relationships_for(&room.id);
        format!(
            "Selected room: {} ({})\nArea: {}sqm\nZone: {}\nRelationships:\n{}",
            room.name, room.id, room.area_sqm,
            room.zone.as_str(),
            rels.iter().map(|r| {
                let other = if r.from_id == room.id { &r.to_id } else { &r.from_id };
                format!("  {} {}", r.kind.as_label(), other)
            }).collect::<Vec<_>>().join("\n")
        )
    } else {
        let room_list = program.rooms.iter()
            .map(|r| format!("  {} ({}, {}sqm, {})",
                r.name, r.id, r.area_sqm, r.zone.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("All rooms:\n{room_list}")
    };

    let room_id_hint = program.rooms.iter()
        .map(|r| format!("{} ({})", r.id, r.name))
        .collect::<Vec<_>>()
        .join(", ");

    // LLM prompt — ask Claude to output structured edits as JSON.
    let prompt = format!(
        r#"You are an architectural AI assistant modifying a house design.

CURRENT DESIGN: {brief}
AESTHETIC: {aesthetic}

{room_context}

USER REQUEST: {message}

Room ids for tools (use exact `room_id` in artifacts): {room_id_hint}

The backend also syncs TypeScript Three.js tools from fluvio-tools/src/tools (e.g. parking_lot_car_parked.ts, chair.ts, sofa.ts, table.ts, bed.ts, desk.ts). When the user asks for a visible object (car, furniture, fixture) that should appear in the 3D viewer, add an "artifacts" array. Each entry places one catalog mesh inside a room:
{{ "tool_file": "parking_lot_car_parked.ts", "tool_name": "Parked Car", "room_id": "<exact room id>", "offset_xz": [0, 0], "rotation_y": 0, "scale": 1.0, "style": "modern", "material": "matte_black" }}
offset_xz are meters relative to the room center on the floor plane (x, z).

Respond with a JSON object:
{{
  "answer": "brief conversational response to the user",
  "edits": [
    // Use these edit types:
    // {{"type": "update_room", "room_id": "...", "area_sqm": 30.0}}
    // {{"type": "update_room", "room_id": "...", "name": "new name"}}
    // {{"type": "add_room", "id": "...", "name": "...", "area_sqm": 20.0, "zone": "public|private|service|outdoor"}}
    // {{"type": "remove_room", "room_id": "..."}}
    // {{"type": "add_relationship", "from_id": "...", "to_id": "...", "kind": "adjacent_to|connects_via|oriented_toward|visible_from|separated_from"}}
    // {{"type": "remove_relationship", "from_id": "...", "to_id": "...", "kind": "..."}}
  ],
  "artifacts": [
    // optional — catalog meshes for the 3D viewer; omit or [] if not needed
  ]
}}

Only include edits that directly respond to the user request.
Respond with JSON only, no markdown."#,
        brief     = program.brief,
        aesthetic = program.design_style.aesthetic,
    );

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &state.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 2048,
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let text = res["content"][0]["text"]
        .as_str()
        .unwrap_or("{}")
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or(serde_json::json!({"answer": "I couldn't process that request.", "edits": []}));

    let answer  = parsed["answer"].as_str().unwrap_or("Done.").to_string();
    let edits   = parsed["edits"].as_array().cloned().unwrap_or_default();
    let artifacts = parsed["artifacts"].as_array().cloned().unwrap_or_default();

    // Apply LLM edits to the program.
    let mut updated = program.clone();
    let mut changes = Vec::new();

    for edit in &edits {
        let kind = edit["type"].as_str().unwrap_or("");
        match kind {
            "update_room" => {
                let room_id = edit["room_id"].as_str().unwrap_or("");
                if let Some(room) = updated.rooms.iter_mut().find(|r| r.id == room_id) {
                    if let Some(area) = edit["area_sqm"].as_f64() {
                        room.area_sqm = area as f32;
                    }
                    if let Some(name) = edit["name"].as_str() {
                        room.name = name.to_string();
                    }
                    if let Some(notes) = edit["notes"].as_str() {
                        room.notes = Some(notes.to_string());
                    }
                    changes.push(edit.clone());
                }
            }
            "add_room" => {
                use crate::ingestion_registry::architecture::space_program::{
                    NaturalLight, PrivacyLevel,
                };
                let new_room = Room {
                    id:            edit["id"].as_str().unwrap_or("new_room").to_string(),
                    name:          edit["name"].as_str().unwrap_or("New Room").to_string(),
                    area_sqm:      edit["area_sqm"].as_f64().unwrap_or(15.0) as f32,
                    zone:          match edit["zone"].as_str().unwrap_or("public") {
                        "private" => Zone::Private,
                        "service" => Zone::Service,
                        "outdoor" => Zone::Outdoor,
                        _         => Zone::Public,
                    },
                    natural_light: NaturalLight::Medium,
                    privacy:       PrivacyLevel::Semi,
                    notes:         edit["notes"].as_str().map(|s| s.to_string()),
                };
                if !updated.rooms.iter().any(|r| r.id == new_room.id) {
                    updated.rooms.push(new_room);
                    changes.push(edit.clone());
                }
            }
            "remove_room" => {
                let room_id = edit["room_id"].as_str().unwrap_or("");
                updated.rooms.retain(|r| r.id != room_id);
                updated.relationships.retain(|rel|
                    rel.from_id != room_id && rel.to_id != room_id);
                changes.push(edit.clone());
            }
            "add_relationship" => {
                let from = edit["from_id"].as_str().unwrap_or("").to_string();
                let to   = edit["to_id"].as_str().unwrap_or("").to_string();
                let kind_str = edit["kind"].as_str().unwrap_or("adjacent_to");
                let kind = match kind_str {
                    "connects_via"    => RelationshipKind::ConnectsVia,
                    "oriented_toward" => RelationshipKind::OrientedToward,
                    "visible_from"    => RelationshipKind::VisibleFrom,
                    "flows_through"   => RelationshipKind::FlowsThrough,
                    "separated_from"  => RelationshipKind::SeparatedFrom,
                    _                 => RelationshipKind::AdjacentTo,
                };
                // Avoid duplicate relationships.
                let exists = updated.relationships.iter().any(|r|
                    r.from_id == from && r.to_id == to && r.kind == kind);
                if !exists && !from.is_empty() && !to.is_empty() {
                    updated.relationships.push(RoomRelationship {
                        from_id: from, to_id: to, kind, notes: None,
                    });
                    changes.push(edit.clone());
                }
            }
            "remove_relationship" => {
                let from     = edit["from_id"].as_str().unwrap_or("");
                let to       = edit["to_id"].as_str().unwrap_or("");
                let kind_str = edit["kind"].as_str().unwrap_or("");
                updated.relationships.retain(|r|
                    !(r.from_id == from && r.to_id == to
                      && r.kind.as_label() == kind_str));
                changes.push(edit.clone());
            }
            _ => {}
        }
    }

    // Recompute layout + scene.
    let positions = compute_layout(&updated);
    let mut scene = generate_scene(&updated, &positions);
    merge_llm_artifacts_into_scene(&mut scene, &artifacts);

    // Persist updated design.
    save_design(&updated)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    {
        let mut store = state.architecture_designs.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        store.insert(design_id.clone(), updated);
    }

    Ok(Json(ArchChatResponse {
        design_id,
        scene,
        answer,
        changes,
    }))
}