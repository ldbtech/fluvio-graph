//! space_program.rs
//!
//! Extracts a structured architectural space program from a natural language brief.
//! Produces NormalizedChunks that plug directly into the existing IngestionPipeline —
//! same infrastructure as PDF, email, and codebase domains.
//!
//! Flow:
//!   User brief (natural language)
//!     → LLM extracts structured rooms + relationships
//!     → SpaceProgram (rooms, zones, adjacencies)
//!     → Vec<NormalizedChunk> with pre_defined_edges
//!     → IngestionPipeline.ingest_normalized_chunks()
//!     → DomainGraph (same graph as everything else)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::graph::enums::Domain;
use crate::ingestion_registry::connector::{NormalizedChunk, PreDefinedEdge};

// ── Domain constant ───────────────────────────────────────────────────────────

pub fn architecture_domain() -> Domain {
    Domain::Architecture
}

// ── Node types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Public,    // living, dining, kitchen, entry
    #[serde(alias = "semi", alias = "semi_private", alias = "semi-public", alias = "semi_public")]
    Private,   // bedrooms, bathrooms, office
    #[serde(alias = "utility")]
    Service,   // laundry, storage, garage, utility
    #[serde(alias = "external")]
    Outdoor,   // garden, terrace, balcony, patio
}

impl Zone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Zone::Public   => "public",
            Zone::Private  => "private",
            Zone::Service  => "service",
            Zone::Outdoor  => "outdoor",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NaturalLight {
    High,
    Medium,
    Low,
}

impl NaturalLight {
    pub fn as_str(&self) -> &'static str {
        match self {
            NaturalLight::High   => "high",
            NaturalLight::Medium => "medium",
            NaturalLight::Low    => "low",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevel {
    Public,
    Semi,
    Private,
}

impl PrivacyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrivacyLevel::Public  => "public",
            PrivacyLevel::Semi    => "semi",
            PrivacyLevel::Private => "private",
        }
    }
}

/// A single room or space in the architectural program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id:            String,
    pub name:          String,
    pub area_sqm:      f32,
    pub zone:          Zone,
    pub natural_light: NaturalLight,
    pub privacy:       PrivacyLevel,
    pub notes:         Option<String>,
}

/// Directional relationship between two rooms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    AdjacentTo,
    ConnectsVia,
    OrientedToward,
    VisibleFrom,
    FlowsThrough,
    SeparatedFrom,
}

impl RelationshipKind {
    pub fn as_label(&self) -> &'static str {
        match self {
            RelationshipKind::AdjacentTo     => "adjacent_to",
            RelationshipKind::ConnectsVia    => "connects_via",
            RelationshipKind::OrientedToward => "oriented_toward",
            RelationshipKind::VisibleFrom    => "visible_from",
            RelationshipKind::FlowsThrough   => "flows_through",
            RelationshipKind::SeparatedFrom  => "separated_from",
        }
    }

    pub fn probability(&self) -> f64 {
        match self {
            RelationshipKind::AdjacentTo     => 0.95,
            RelationshipKind::ConnectsVia    => 0.90,
            RelationshipKind::OrientedToward => 0.85,
            RelationshipKind::VisibleFrom    => 0.80,
            RelationshipKind::FlowsThrough   => 0.85,
            RelationshipKind::SeparatedFrom  => 0.90,
        }
    }
}

/// A spatial relationship between two rooms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRelationship {
    pub from_id: String,
    pub to_id:   String,
    pub kind:    RelationshipKind,
    pub notes:   Option<String>,
}

/// Design style extracted from the brief.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignStyle {
    pub aesthetic:  String,
    pub priority:   String,
    pub total_area: Option<f32>,
    pub stories:    Option<u8>,
}

/// Complete structured space program from a brief.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceProgram {
    pub brief:         String,
    pub design_id:     String,
    pub design_style:  DesignStyle,
    pub rooms:         Vec<Room>,
    pub relationships: Vec<RoomRelationship>,
}

impl SpaceProgram {
    pub fn total_area(&self) -> f32 {
        self.rooms.iter().map(|r| r.area_sqm).sum()
    }

    pub fn rooms_in_zone(&self, zone: &Zone) -> Vec<&Room> {
        self.rooms.iter().filter(|r| &r.zone == zone).collect()
    }

    pub fn room_by_id(&self, id: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    pub fn relationships_for(&self, room_id: &str) -> Vec<&RoomRelationship> {
        self.relationships.iter()
            .filter(|r| r.from_id == room_id || r.to_id == room_id)
            .collect()
    }
}

// ── URI helpers ───────────────────────────────────────────────────────────────

/// `architecture://design-id/room/living`
pub fn room_uri(design_id: &str, room_id: &str) -> String {
    format!("architecture://{design_id}/room/{room_id}")
}

/// `architecture://design-id/design` — the top-level design node
pub fn design_uri(design_id: &str) -> String {
    format!("architecture://{design_id}/design")
}

// ── NormalizedChunk conversion ────────────────────────────────────────────────

/// Convert a `SpaceProgram` into `NormalizedChunk`s for the IngestionPipeline.
///
/// Produces:
///   1. One design-level chunk (the whole brief as context)
///   2. One chunk per room (embeddable, with pre_defined_edges to related rooms)
///
/// Edges on each room chunk:
///   adjacent_to / connects_via / oriented_toward / visible_from /
///   flows_through / separated_from   → target room chunk
///
/// The design chunk gets `contains` edges to every room chunk.
pub fn space_program_to_chunks(
    program:     &SpaceProgram,
    start_index: usize,
) -> Vec<NormalizedChunk> {
    let mut chunks  = Vec::new();
    let design_id   = &program.design_id;

    // ── 1. Design-level chunk ─────────────────────────────────────────────────
    let design_source_uri = design_uri(design_id);
    let design_text = format!(
        "architecture design: {design_id}\n\
         brief: {brief}\n\
         aesthetic: {aesthetic}\n\
         priority: {priority}\n\
         total_area: {area}sqm\n\
         rooms: {room_names}",
        brief     = program.brief,
        aesthetic = program.design_style.aesthetic,
        priority  = program.design_style.priority,
        area      = program.total_area(),
        room_names = program.rooms.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join(", "),
    );

    let mut design_meta = HashMap::new();
    design_meta.insert("source".to_string(),    "architecture".to_string());
    design_meta.insert("kind".to_string(),      "design".to_string());
    design_meta.insert("design_id".to_string(), design_id.clone());
    design_meta.insert("aesthetic".to_string(), program.design_style.aesthetic.clone());
    design_meta.insert("room_count".to_string(), program.rooms.len().to_string());
    design_meta.insert("total_area".to_string(), program.total_area().to_string());

    // Design chunk gets contains edges to every room.
    let design_contains_edges: Vec<PreDefinedEdge> = program.rooms.iter()
        .map(|r| PreDefinedEdge {
            to_uri:                  room_uri(design_id, &r.id),
            label:                   "contains".to_string(),
            relationship_probability: 1.0,
            token_cost:              0,
        })
        .collect();

    chunks.push(NormalizedChunk {
        text:              design_text,
        metadata:          design_meta,
        chunk_index:       start_index,
        source_uri:        design_source_uri,
        domain:            architecture_domain(),
        pre_defined_edges: design_contains_edges,
    });

    // ── 2. Room chunks ────────────────────────────────────────────────────────
    // Build a room_id → source_uri map for fast edge lookup.
    let uri_map: HashMap<&str, String> = program.rooms.iter()
        .map(|r| (r.id.as_str(), room_uri(design_id, &r.id)))
        .collect();

    for (i, room) in program.rooms.iter().enumerate() {
        let source_uri = room_uri(design_id, &room.id);

        // Embeddable text — rich context for semantic search and LLM.
        let relationships_text = program.relationships.iter()
            .filter(|rel| rel.from_id == room.id || rel.to_id == room.id)
            .map(|rel| {
                let other_id = if rel.from_id == room.id { &rel.to_id } else { &rel.from_id };
                let other_name = program.room_by_id(other_id)
                    .map(|r| r.name.as_str())
                    .unwrap_or(other_id.as_str());
                format!("  {} {}{}", rel.kind.as_label(), other_name,
                    rel.notes.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default())
            })
            .collect::<Vec<_>>()
            .join("\n");

        let text = format!(
            "room: {name}\ndesign: {design_id}\nzone: {zone}\n\
             area: {area}sqm\nnatural_light: {light}\nprivacy: {privacy}\n\
             aesthetic: {aesthetic}{notes}{rels}",
            name      = room.name,
            zone      = room.zone.as_str(),
            area      = room.area_sqm,
            light     = room.natural_light.as_str(),
            privacy   = room.privacy.as_str(),
            aesthetic = program.design_style.aesthetic,
            notes     = room.notes.as_ref()
                .map(|n| format!("\nnotes: {n}"))
                .unwrap_or_default(),
            rels      = if relationships_text.is_empty() { String::new() }
                        else { format!("\nrelationships:\n{relationships_text}") },
        );

        // Metadata.
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(),       "architecture".to_string());
        metadata.insert("kind".to_string(),         "room".to_string());
        metadata.insert("design_id".to_string(),    design_id.clone());
        metadata.insert("room_id".to_string(),      room.id.clone());
        metadata.insert("name".to_string(),         room.name.clone());
        metadata.insert("area_sqm".to_string(),     room.area_sqm.to_string());
        metadata.insert("zone".to_string(),         room.zone.as_str().to_string());
        metadata.insert("natural_light".to_string(), room.natural_light.as_str().to_string());
        metadata.insert("privacy".to_string(),      room.privacy.as_str().to_string());
        metadata.insert("aesthetic".to_string(),    program.design_style.aesthetic.clone());
        metadata.insert("source_uri".to_string(),   source_uri.clone());
        if let Some(notes) = &room.notes {
            metadata.insert("notes".to_string(), notes.clone());
        }

        // Pre-defined edges from this room's relationships.
        let mut edges: Vec<PreDefinedEdge> = Vec::new();

        // defined_in edge back to design.
        edges.push(PreDefinedEdge {
            to_uri:                  design_uri(design_id),
            label:                   "defined_in".to_string(),
            relationship_probability: 1.0,
            token_cost:              0,
        });

        // Relationship edges.
        for rel in &program.relationships {
            if rel.from_id == room.id {
                // Outgoing edge from this room.
                if let Some(to_uri) = uri_map.get(rel.to_id.as_str()) {
                    edges.push(PreDefinedEdge {
                        to_uri:                  to_uri.clone(),
                        label:                   rel.kind.as_label().to_string(),
                        relationship_probability: rel.kind.probability(),
                        token_cost:              1,
                    });
                }
            }
        }

        chunks.push(NormalizedChunk {
            text,
            metadata,
            chunk_index:       start_index + 1 + i,
            source_uri,
            domain:            architecture_domain(),
            pre_defined_edges: edges,
        });
    }

    chunks
}

// ── LLM extraction ────────────────────────────────────────────────────────────

/// Extract a structured SpaceProgram from a natural language brief via Claude.
pub async fn extract_space_program(
    brief:     &str,
    design_id: &str,
    api_key:   &str,
) -> anyhow::Result<SpaceProgram> {
    let prompt = format!(
        r#"You are an architectural space planner. Extract a structured space program from this brief.

BRIEF:
{brief}

Respond with ONLY a JSON object — no markdown, no explanation, no backticks.

{{
  "brief": "<original brief>",
  "design_id": "{design_id}",
  "design_style": {{
    "aesthetic": "<modern|traditional|minimalist|industrial|scandinavian|mediterranean|other>",
    "priority": "<what user emphasized most>",
    "total_area": <sqm or null>,
    "stories": <number or null>
  }},
  "rooms": [
    {{
      "id": "<snake_case unique id>",
      "name": "<display name>",
      "area_sqm": <number>,
      "zone": "<public|private|service|outdoor>",
      "natural_light": "<high|medium|low>",
      "privacy": "<public|semi|private>",
      "notes": "<optional or null>"
    }}
  ],
  "relationships": [
    {{
      "from_id": "<room_id>",
      "to_id": "<room_id>",
      "kind": "<adjacent_to|connects_via|oriented_toward|visible_from|flows_through|separated_from>",
      "notes": "<optional or null>"
    }}
  ]
}}

Rules:
- Always include an entry/foyer room
- Every bedroom needs adjacent_to or connects_via to a bathroom
- Kitchen and dining should be adjacent_to or connects_via
- Living room connects_via to entry
- Master bedroom separated_from street noise
- Add oriented_toward for any views mentioned
- Estimate areas from typical residential sizes if not given
- 6-15 relationships minimum
- All room IDs must be unique snake_case"#
    );

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 2000,
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let text = res["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no text in LLM response"))?
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let program: SpaceProgram = serde_json::from_str(text)
        .map_err(|e| anyhow::anyhow!("failed to parse space program: {e}\nRaw: {text}"))?;

    Ok(program)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_program() -> SpaceProgram {
        SpaceProgram {
            brief:     "modern 3 bedroom house, open kitchen, garden views".to_string(),
            design_id: "design-001".to_string(),
            design_style: DesignStyle {
                aesthetic:  "modern".to_string(),
                priority:   "natural light".to_string(),
                total_area: Some(180.0),
                stories:    Some(1),
            },
            rooms: vec![
                Room {
                    id:            "entry".to_string(),
                    name:          "Entry".to_string(),
                    area_sqm:      8.0,
                    zone:          Zone::Public,
                    natural_light: NaturalLight::Medium,
                    privacy:       PrivacyLevel::Public,
                    notes:         None,
                },
                Room {
                    id:            "living".to_string(),
                    name:          "Living Room".to_string(),
                    area_sqm:      35.0,
                    zone:          Zone::Public,
                    natural_light: NaturalLight::High,
                    privacy:       PrivacyLevel::Public,
                    notes:         Some("open to kitchen".to_string()),
                },
                Room {
                    id:            "kitchen".to_string(),
                    name:          "Kitchen".to_string(),
                    area_sqm:      22.0,
                    zone:          Zone::Public,
                    natural_light: NaturalLight::High,
                    privacy:       PrivacyLevel::Semi,
                    notes:         None,
                },
                Room {
                    id:            "master_bedroom".to_string(),
                    name:          "Master Bedroom".to_string(),
                    area_sqm:      25.0,
                    zone:          Zone::Private,
                    natural_light: NaturalLight::High,
                    privacy:       PrivacyLevel::Private,
                    notes:         None,
                },
                Room {
                    id:            "garden".to_string(),
                    name:          "Garden".to_string(),
                    area_sqm:      60.0,
                    zone:          Zone::Outdoor,
                    natural_light: NaturalLight::High,
                    privacy:       PrivacyLevel::Semi,
                    notes:         None,
                },
            ],
            relationships: vec![
                RoomRelationship {
                    from_id: "entry".to_string(),
                    to_id:   "living".to_string(),
                    kind:    RelationshipKind::ConnectsVia,
                    notes:   None,
                },
                RoomRelationship {
                    from_id: "living".to_string(),
                    to_id:   "kitchen".to_string(),
                    kind:    RelationshipKind::AdjacentTo,
                    notes:   Some("open plan".to_string()),
                },
                RoomRelationship {
                    from_id: "living".to_string(),
                    to_id:   "garden".to_string(),
                    kind:    RelationshipKind::OrientedToward,
                    notes:   None,
                },
                RoomRelationship {
                    from_id: "master_bedroom".to_string(),
                    to_id:   "garden".to_string(),
                    kind:    RelationshipKind::OrientedToward,
                    notes:   None,
                },
            ],
        }
    }

    #[test]
    fn test_total_area() {
        let p = make_program();
        assert!((p.total_area() - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_rooms_in_zone() {
        let p = make_program();
        assert_eq!(p.rooms_in_zone(&Zone::Public).len(),  3);
        assert_eq!(p.rooms_in_zone(&Zone::Private).len(), 1);
        assert_eq!(p.rooms_in_zone(&Zone::Outdoor).len(), 1);
    }

    #[test]
    fn test_room_by_id() {
        let p = make_program();
        assert_eq!(p.room_by_id("living").unwrap().name, "Living Room");
        assert!(p.room_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_relationships_for_room() {
        let p = make_program();
        // living is in: entry→living, living→kitchen, living→garden
        assert_eq!(p.relationships_for("living").len(), 3);
    }

    #[test]
    fn test_chunk_count() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        // 1 design chunk + 5 room chunks
        assert_eq!(chunks.len(), 6);
    }

    #[test]
    fn test_design_chunk_is_first() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let design = &chunks[0];
        assert_eq!(design.metadata.get("kind").unwrap(), "design");
        assert_eq!(design.source_uri, "architecture://design-001/design");
        assert_eq!(design.domain, architecture_domain());
    }

    #[test]
    fn test_design_chunk_contains_edges() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let design = &chunks[0];
        let contains: Vec<_> = design.pre_defined_edges.iter()
            .filter(|e| e.label == "contains")
            .collect();
        assert_eq!(contains.len(), 5); // one per room
    }

    #[test]
    fn test_room_chunk_metadata() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let living = chunks.iter().find(|c| {
            c.metadata.get("room_id").map(|s| s == "living").unwrap_or(false)
        }).unwrap();
        assert_eq!(living.metadata.get("zone").unwrap(), "public");
        assert_eq!(living.metadata.get("area_sqm").unwrap(), "35");
        assert_eq!(living.metadata.get("kind").unwrap(), "room");
        assert_eq!(living.metadata.get("source").unwrap(), "architecture");
    }

    #[test]
    fn test_room_chunk_text_embeddable() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let master = chunks.iter().find(|c| {
            c.metadata.get("room_id").map(|s| s == "master_bedroom").unwrap_or(false)
        }).unwrap();
        assert!(master.text.contains("room: Master Bedroom"));
        assert!(master.text.contains("zone: private"));
        assert!(master.text.contains("privacy: private"));
        assert!(master.text.contains("aesthetic: modern"));
        assert!(master.text.contains("oriented_toward"));
    }

    #[test]
    fn test_relationship_edges_on_room_chunk() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let living = chunks.iter().find(|c| {
            c.metadata.get("room_id").map(|s| s == "living").unwrap_or(false)
        }).unwrap();
        // living has: defined_in + adjacent_to(kitchen) + oriented_toward(garden)
        let rel_edges: Vec<_> = living.pre_defined_edges.iter()
            .filter(|e| e.label != "defined_in")
            .collect();
        assert!(rel_edges.iter().any(|e| e.label == "adjacent_to"));
        assert!(rel_edges.iter().any(|e| e.label == "oriented_toward"));
    }

    #[test]
    fn test_defined_in_edge_on_every_room() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        // Skip design chunk (index 0), check all room chunks.
        for chunk in chunks.iter().skip(1) {
            let has_defined_in = chunk.pre_defined_edges.iter()
                .any(|e| e.label == "defined_in");
            assert!(has_defined_in,
                "room chunk {} missing defined_in edge",
                chunk.metadata.get("room_id").unwrap_or(&"?".to_string()));
        }
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 10);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, 10 + i);
        }
    }

    #[test]
    fn test_edge_probabilities() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let living = chunks.iter().find(|c| {
            c.metadata.get("room_id").map(|s| s == "living").unwrap_or(false)
        }).unwrap();
        let adj = living.pre_defined_edges.iter()
            .find(|e| e.label == "adjacent_to").unwrap();
        assert_eq!(adj.relationship_probability, 0.95);
        let oriented = living.pre_defined_edges.iter()
            .find(|e| e.label == "oriented_toward").unwrap();
        assert_eq!(oriented.relationship_probability, 0.85);
    }

    #[test]
    fn test_room_uri_format() {
        assert_eq!(
            room_uri("design-001", "living_room"),
            "architecture://design-001/room/living_room"
        );
    }

    #[test]
    fn test_design_uri_format() {
        assert_eq!(
            design_uri("design-001"),
            "architecture://design-001/design"
        );
    }

    #[test]
    fn test_all_uris_unique() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        let uris: Vec<&str> = chunks.iter().map(|c| c.source_uri.as_str()).collect();
        let unique: std::collections::HashSet<&str> = uris.iter().copied().collect();
        assert_eq!(uris.len(), unique.len(), "all URIs must be unique");
    }

    #[test]
    fn test_architecture_domain() {
        let p = make_program();
        let chunks = space_program_to_chunks(&p, 0);
        assert!(chunks.iter().all(|c| c.domain == architecture_domain()));
    }

    #[test]
    fn test_relationship_kind_labels() {
        assert_eq!(RelationshipKind::AdjacentTo.as_label(),     "adjacent_to");
        assert_eq!(RelationshipKind::ConnectsVia.as_label(),    "connects_via");
        assert_eq!(RelationshipKind::OrientedToward.as_label(), "oriented_toward");
        assert_eq!(RelationshipKind::SeparatedFrom.as_label(),  "separated_from");
    }

    #[test]
    fn test_zone_as_str() {
        assert_eq!(Zone::Public.as_str(),  "public");
        assert_eq!(Zone::Private.as_str(), "private");
        assert_eq!(Zone::Service.as_str(), "service");
        assert_eq!(Zone::Outdoor.as_str(), "outdoor");
    }
}