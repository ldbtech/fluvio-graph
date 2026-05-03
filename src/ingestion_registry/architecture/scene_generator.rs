//! scene_generator.rs
//!
//! Converts a `SpaceProgram` + `Vec<PositionedRoom>` into a Three.js-ready
//! scene JSON that the frontend renders directly.
//!
//! Output shape (consumed by @react-three/fiber):
//! {
//!   "design_id": "...",
//!   "rooms": [
//!     {
//!       "id": "living", "name": "Living Room",
//!       "position": [x, y, z],       ← Three.js position (center)
//!       "dimensions": [w, h, d],     ← width, height, depth
//!       "material": "plaster_white",
//!       "zone": "public",
//!       "node_id": "architecture://design-id/room/living"
//!     }
//!   ],
//!   "walls": [...],      ← wall segments between rooms
//!   "openings": [...],   ← doors and windows derived from relationships
//!   "camera": {          ← suggested initial camera position
//!     "position": [x, y, z],
//!     "target": [x, y, z]
//!   }
//! }
//!
//! Coordinate system:
//!   x = east (+) / west (-)
//!   y = up (+) / down (-)    ← Three.js Y-up
//!   z = south (+) / north (-) ← Three.js convention
//!
//! Room footprints from layout_engine are in (x, y) floor plan space.
//! scene_generator maps these to Three.js (x, 0, z) with y=height/2 for center.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::layout_engine::PositionedRoom;
use super::space_program::{RelationshipKind, RoomRelationship, SpaceProgram};

// ── Material mapping ──────────────────────────────────────────────────────────

/// Maps zone + room name hints to a material name the frontend recognises.
fn material_for_room(zone: &str, name: &str) -> &'static str {
    let name_lower = name.to_lowercase();
    if name_lower.contains("bath") || name_lower.contains("toilet") || name_lower.contains("wc") {
        return "tile_white";
    }
    if name_lower.contains("kitchen") {
        return "tile_light";
    }
    if name_lower.contains("garden") || name_lower.contains("terrace") || name_lower.contains("balcony") || name_lower.contains("patio") {
        return "grass";
    }
    if name_lower.contains("garage") {
        return "concrete_grey";
    }
    match zone {
        "public"  => "plaster_white",
        "private" => "plaster_warm",
        "service" => "concrete_grey",
        "outdoor" => "grass",
        _         => "plaster_white",
    }
}

// ── Scene types ───────────────────────────────────────────────────────────────

/// A room ready for Three.js rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneRoom {
    pub id:         String,
    pub name:       String,
    /// Three.js position — center of the room box [x, y, z].
    /// y = height / 2 so the box sits on the floor plane.
    pub position:   [f32; 3],
    /// Three.js box dimensions [width, height, depth].
    pub dimensions: [f32; 3],
    pub material:   String,
    pub zone:       String,
    /// Source URI — maps back to the knowledge graph node.
    /// Frontend uses this as the nodeId when user clicks the room.
    pub node_id:    String,
}

/// A wall segment between two rooms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneWall {
    pub from_room:  String,
    pub to_room:    String,
    /// Wall center position in Three.js coords.
    pub position:   [f32; 3],
    /// Wall dimensions [width, height, thickness].
    pub dimensions: [f32; 3],
    /// Rotation around Y axis in radians (0 = east-west, PI/2 = north-south).
    pub rotation_y: f32,
    pub material:   String,
}

/// An opening (door or window) in a wall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneOpening {
    pub kind:       String,   // "door" | "window"
    pub from_room:  String,
    pub to_room:    String,
    pub position:   [f32; 3],
    pub dimensions: [f32; 3], // [width, height, depth=wall_thickness]
    pub label:      String,   // relationship label e.g. "connects_via"
}

/// Suggested camera setup for the initial view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCamera {
    /// Camera position in Three.js coords.
    pub position: [f32; 3],
    /// Look-at target (scene center).
    pub target:   [f32; 3],
    /// Suggested field of view in degrees.
    pub fov:      f32,
}

/// A catalog mesh from `fluvio-tools/src/tools/*.ts` placed in a room (Three.js client renders it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneArtifact {
    pub tool_file:   String,
    #[serde(default)]
    pub tool_name:   String,
    pub room_id:     String,
    pub position:    [f32; 3],
    #[serde(default)]
    pub rotation_y:  f32,
    #[serde(default = "default_artifact_scale")]
    pub scale:       f32,
    #[serde(default)]
    pub style:       String,
    #[serde(default)]
    pub material:    String,
}

fn default_artifact_scale() -> f32 {
    1.0
}

/// Complete Three.js scene ready for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureScene {
    pub design_id:    String,
    pub rooms:        Vec<SceneRoom>,
    pub walls:        Vec<SceneWall>,
    pub openings:     Vec<SceneOpening>,
    pub camera:       SceneCamera,
    /// Bounding box of the whole scene [min_x, min_z, max_x, max_z].
    pub bounds:       [f32; 4],
    /// Total floor area in sqm.
    pub total_area:   f32,
    /// Optional meshes from the architecture tool catalog (`fluvio-tools/src/tools`).
    #[serde(default)]
    pub artifacts:    Vec<SceneArtifact>,
}

// ── Generator ─────────────────────────────────────────────────────────────────

/// Convert a SpaceProgram + positioned rooms into a Three.js scene.
pub fn generate_scene(
    program:   &SpaceProgram,
    positions: &[PositionedRoom],
) -> ArchitectureScene {
    // ── 1. Build room map for fast lookup ─────────────────────────────────────
    let pos_map: std::collections::HashMap<&str, &PositionedRoom> = positions
        .iter()
        .map(|r| (r.room_id.as_str(), r))
        .collect();

    // ── 2. Scene rooms ────────────────────────────────────────────────────────
    let rooms: Vec<SceneRoom> = positions.iter().map(|pos| {
        // Floor plan (x, y) → Three.js (x, 0, z).
        // Center of box: x + w/2, height/2, y + d/2.
        let height = if pos.height == 0.0 { 0.1 } else { pos.height }; // outdoor gets thin slab
        SceneRoom {
            id:         pos.room_id.clone(),
            name:       pos.name.clone(),
            position:   [
                pos.center_x,
                height / 2.0,
                pos.center_y,
            ],
            dimensions: [pos.width, height, pos.depth],
            material:   material_for_room(&pos.zone, &pos.name).to_string(),
            zone:       pos.zone.clone(),
            node_id:    super::space_program::room_uri(&program.design_id, &pos.room_id),
        }
    }).collect();

    // ── 3. Walls between adjacent rooms ───────────────────────────────────────
    const WALL_THICKNESS: f32 = 0.2;
    const WALL_HEIGHT:    f32 = 3.0;

    let mut walls: Vec<SceneWall> = Vec::new();
    let mut seen_walls = std::collections::HashSet::new();

    for rel in &program.relationships {
        if rel.kind != RelationshipKind::AdjacentTo
            && rel.kind != RelationshipKind::ConnectsVia
        {
            continue;
        }

        let key = if rel.from_id < rel.to_id {
            format!("{}|{}", rel.from_id, rel.to_id)
        } else {
            format!("{}|{}", rel.to_id, rel.from_id)
        };
        if !seen_walls.insert(key) {
            continue; // already generated this wall
        }

        let Some(from_pos) = pos_map.get(rel.from_id.as_str()) else { continue };
        let Some(to_pos)   = pos_map.get(rel.to_id.as_str())   else { continue };

        // Wall sits at the shared boundary between two rooms.
        // Determine orientation from relative positions.
        let from_cx = from_pos.center_x;
        let from_cy = from_pos.center_y;
        let to_cx   = to_pos.center_x;
        let to_cy   = to_pos.center_y;

        let dx = (to_cx - from_cx).abs();
        let dz = (to_cy - from_cy).abs();

        let (wall_cx, wall_cz, wall_w, wall_d, rotation_y) = if dx > dz {
            // Rooms side by side (east-west) — wall runs north-south.
            let wx = (from_pos.x + from_pos.width).min(to_pos.x + to_pos.width)
                .max(from_pos.x).max(to_pos.x);
            let shared_depth = from_pos.depth.min(to_pos.depth);
            let wz = from_pos.center_y.min(to_pos.center_y);
            (wx, wz + shared_depth / 2.0, WALL_THICKNESS, shared_depth, std::f32::consts::FRAC_PI_2)
        } else {
            // Rooms stacked (north-south) — wall runs east-west.
            let wz = (from_pos.y + from_pos.depth).min(to_pos.y + to_pos.depth)
                .max(from_pos.y).max(to_pos.y);
            let shared_width = from_pos.width.min(to_pos.width);
            let wx = from_pos.center_x.min(to_pos.center_x);
            (wx + shared_width / 2.0, wz, shared_width, WALL_THICKNESS, 0.0)
        };

        walls.push(SceneWall {
            from_room:  rel.from_id.clone(),
            to_room:    rel.to_id.clone(),
            position:   [wall_cx, WALL_HEIGHT / 2.0, wall_cz],
            dimensions: [wall_w, WALL_HEIGHT, wall_d],
            rotation_y,
            material:   "plaster_white".to_string(),
        });
    }

    // ── 4. Openings (doors / windows from relationships) ───────────────────────
    let openings: Vec<SceneOpening> = program.relationships.iter()
        .filter_map(|rel| {
            let kind = match rel.kind {
                RelationshipKind::ConnectsVia => "door",
                _                             => return None,
            };

            let from_pos = pos_map.get(rel.from_id.as_str())?;
            let to_pos   = pos_map.get(rel.to_id.as_str())?;

            // Place opening at the wall midpoint between the two rooms.
            let mid_x = (from_pos.center_x + to_pos.center_x) / 2.0;
            let mid_z = (from_pos.center_y + to_pos.center_y) / 2.0;

            Some(SceneOpening {
                kind:      kind.to_string(),
                from_room: rel.from_id.clone(),
                to_room:   rel.to_id.clone(),
                position:  [mid_x, 1.05, mid_z], // door center at 1.05m (half of 2.1m door)
                dimensions: [0.9, 2.1, WALL_THICKNESS],
                label:     rel.kind.as_label().to_string(),
            })
        })
        .collect();

    // ── 5. Bounding box + camera ───────────────────────────────────────────────
    let (min_x, min_z, max_x, max_z) = positions.iter().fold(
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
        |(mnx, mnz, mxx, mxz), r| (
            mnx.min(r.x),
            mnz.min(r.y),
            mxx.max(r.x + r.width),
            mxz.max(r.y + r.depth),
        ),
    );

    let scene_w  = max_x - min_x;
    let scene_d  = max_z - min_z;
    let center_x = min_x + scene_w / 2.0;
    let center_z = min_z + scene_d / 2.0;

    // Camera sits above and slightly back, looking at scene center.
    let cam_distance = (scene_w.max(scene_d) * 1.2).max(15.0);
    let camera = SceneCamera {
        position: [center_x, cam_distance * 0.7, center_z + cam_distance],
        target:   [center_x, 0.0, center_z],
        fov:      50.0,
    };

    let total_area = positions.iter()
        .filter(|r| r.zone != "outdoor")
        .map(|r| r.width * r.depth)
        .sum();

    ArchitectureScene {
        design_id:  program.design_id.clone(),
        rooms,
        walls,
        openings,
        camera,
        bounds:     [min_x, min_z, max_x, max_z],
        total_area,
        artifacts:  Vec::new(),
    }
}

/// Turn LLM `artifacts` JSON into concrete floor positions using current scene rooms.
pub fn merge_llm_artifacts_into_scene(scene: &mut ArchitectureScene, raw: &[Value]) {
    for v in raw {
        let tool_file = match v.get("tool_file").and_then(|x| x.as_str()) {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => continue,
        };
        let tool_name = v
            .get("tool_name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let room_query = match v.get("room_id").and_then(|x| x.as_str()) {
            Some(s) if !s.trim().is_empty() => s.trim(),
            _ => continue,
        };

        let Some(room) = scene
            .rooms
            .iter()
            .find(|r| r.id == room_query)
            .or_else(|| {
                let needle = room_query.to_lowercase();
                scene.rooms.iter().find(|r| {
                    r.id.to_lowercase() == needle
                        || r.name.to_lowercase().contains(needle.as_str())
                        || needle.contains(&r.name.to_lowercase())
                })
            })
        else {
            continue;
        };

        let ox = v
            .get("offset_xz")
            .and_then(|a| a.as_array())
            .map(|a| {
                let x = a.get(0).and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
                let z = a.get(1).and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
                (x, z)
            })
            .unwrap_or((0.0, 0.0));

        let rotation_y = v.get("rotation_y").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        let scale = v.get("scale").and_then(|n| n.as_f64()).unwrap_or(1.0) as f32;
        let scale = scale.clamp(0.2, 3.0);
        let style = v
            .get("style")
            .and_then(|s| s.as_str())
            .unwrap_or("modern")
            .to_string();
        let material = v
            .get("material")
            .and_then(|s| s.as_str())
            .unwrap_or("matte_black")
            .to_string();

        let floor_y = room.position[1] - room.dimensions[1] * 0.5 + 0.08;
        let px = room.position[0] + ox.0.clamp(-2.5, 2.5);
        let pz = room.position[2] + ox.1.clamp(-2.5, 2.5);

        scene.artifacts.push(SceneArtifact {
            tool_file,
            tool_name,
            room_id: room.id.clone(),
            position: [px, floor_y, pz],
            rotation_y,
            scale,
            style,
            material,
        });
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_registry::architecture::{
        compute_layout,
        space_program::{
            DesignStyle, NaturalLight, PrivacyLevel,
            RelationshipKind, Room, RoomRelationship, SpaceProgram, Zone,
        },
    };

    fn make_program() -> SpaceProgram {
        SpaceProgram {
            brief:     "modern house".to_string(),
            design_id: "scene-test".to_string(),
            design_style: DesignStyle {
                aesthetic:  "modern".to_string(),
                priority:   "light".to_string(),
                total_area: Some(130.0),
                stories:    Some(1),
            },
            rooms: vec![
                Room {
                    id: "entry".to_string(), name: "Entry".to_string(),
                    area_sqm: 8.0, zone: Zone::Public,
                    natural_light: NaturalLight::Medium,
                    privacy: PrivacyLevel::Public, notes: None,
                },
                Room {
                    id: "living".to_string(), name: "Living Room".to_string(),
                    area_sqm: 35.0, zone: Zone::Public,
                    natural_light: NaturalLight::High,
                    privacy: PrivacyLevel::Public, notes: None,
                },
                Room {
                    id: "master_bed".to_string(), name: "Master Bedroom".to_string(),
                    area_sqm: 25.0, zone: Zone::Private,
                    natural_light: NaturalLight::High,
                    privacy: PrivacyLevel::Private, notes: None,
                },
                Room {
                    id: "bathroom".to_string(), name: "Bathroom".to_string(),
                    area_sqm: 8.0, zone: Zone::Private,
                    natural_light: NaturalLight::Low,
                    privacy: PrivacyLevel::Private, notes: None,
                },
                Room {
                    id: "garden".to_string(), name: "Garden".to_string(),
                    area_sqm: 50.0, zone: Zone::Outdoor,
                    natural_light: NaturalLight::High,
                    privacy: PrivacyLevel::Semi, notes: None,
                },
            ],
            relationships: vec![
                RoomRelationship {
                    from_id: "entry".to_string(), to_id: "living".to_string(),
                    kind: RelationshipKind::ConnectsVia, notes: None,
                },
                RoomRelationship {
                    from_id: "living".to_string(), to_id: "garden".to_string(),
                    kind: RelationshipKind::OrientedToward, notes: None,
                },
                RoomRelationship {
                    from_id: "master_bed".to_string(), to_id: "bathroom".to_string(),
                    kind: RelationshipKind::ConnectsVia, notes: None,
                },
            ],
        }
    }

    fn make_scene() -> ArchitectureScene {
        let p = make_program();
        let positions = compute_layout(&p);
        generate_scene(&p, &positions)
    }

    #[test]
    fn test_scene_has_all_rooms() {
        let scene = make_scene();
        let p = make_program();
        assert_eq!(scene.rooms.len(), p.rooms.len());
    }

    #[test]
    fn test_every_room_has_node_id() {
        let scene = make_scene();
        for room in &scene.rooms {
            assert!(
                room.node_id.starts_with("architecture://"),
                "room {} has invalid node_id: {}", room.id, room.node_id
            );
        }
    }

    #[test]
    fn test_room_position_above_floor() {
        let scene = make_scene();
        for room in &scene.rooms {
            // y position should be height/2 (box center above floor).
            assert!(
                room.position[1] > 0.0,
                "room {} y position is at or below floor: {}", room.id, room.position[1]
            );
        }
    }

    #[test]
    fn test_room_dimensions_positive() {
        let scene = make_scene();
        for room in &scene.rooms {
            assert!(room.dimensions[0] > 0.0, "{} width <= 0", room.id);
            assert!(room.dimensions[1] > 0.0, "{} height <= 0", room.id);
            assert!(room.dimensions[2] > 0.0, "{} depth <= 0", room.id);
        }
    }

    #[test]
    fn test_outdoor_gets_thin_slab() {
        let scene = make_scene();
        let garden = scene.rooms.iter().find(|r| r.id == "garden").unwrap();
        // Outdoor rooms get height=0.1 (thin slab, no ceiling).
        assert!(garden.dimensions[1] <= 0.15, "garden height should be thin: {}", garden.dimensions[1]);
    }

    #[test]
    fn test_materials_assigned() {
        let scene = make_scene();
        let bathroom = scene.rooms.iter().find(|r| r.id == "bathroom").unwrap();
        assert_eq!(bathroom.material, "tile_white");
        let garden = scene.rooms.iter().find(|r| r.id == "garden").unwrap();
        assert_eq!(garden.material, "grass");
    }

    #[test]
    fn test_openings_for_connects_via() {
        let scene = make_scene();
        // entry→living and master_bed→bathroom are connects_via → should produce doors.
        assert!(
            scene.openings.iter().any(|o| o.from_room == "entry" && o.to_room == "living"),
            "expected door between entry and living"
        );
        assert!(
            scene.openings.iter().any(|o| o.from_room == "master_bed" && o.to_room == "bathroom"),
            "expected door between master_bed and bathroom"
        );
    }

    #[test]
    fn test_no_opening_for_oriented_toward() {
        let scene = make_scene();
        // living→garden is oriented_toward — should NOT produce a door.
        assert!(
            !scene.openings.iter().any(|o|
                (o.from_room == "living" && o.to_room == "garden") ||
                (o.from_room == "garden" && o.to_room == "living")
            ),
            "oriented_toward should not produce an opening"
        );
    }

    #[test]
    fn test_camera_above_scene() {
        let scene = make_scene();
        assert!(
            scene.camera.position[1] > 0.0,
            "camera y should be above ground: {}", scene.camera.position[1]
        );
    }

    #[test]
    fn test_bounds_valid() {
        let scene = make_scene();
        let [min_x, min_z, max_x, max_z] = scene.bounds;
        assert!(max_x > min_x, "bounds: max_x should be > min_x");
        assert!(max_z > min_z, "bounds: max_z should be > min_z");
    }

    #[test]
    fn test_total_area_excludes_outdoor() {
        let scene = make_scene();
        // Total area should not include outdoor rooms.
        assert!(
            scene.total_area > 0.0,
            "total area should be positive: {}", scene.total_area
        );
        // Garden is 50sqm outdoor — total should be less than full program area.
        assert!(
            scene.total_area < 200.0,
            "total area seems too large (includes outdoor?): {}", scene.total_area
        );
    }

    #[test]
    fn test_room_node_id_matches_uri() {
        let scene = make_scene();
        let living = scene.rooms.iter().find(|r| r.id == "living").unwrap();
        assert_eq!(
            living.node_id,
            "architecture://scene-test/room/living"
        );
    }

    #[test]
    fn test_empty_program_produces_empty_scene() {
        let p = SpaceProgram {
            brief: "empty".to_string(),
            design_id: "empty".to_string(),
            design_style: DesignStyle {
                aesthetic: "modern".to_string(),
                priority: "light".to_string(),
                total_area: None, stories: None,
            },
            rooms: vec![],
            relationships: vec![],
        };
        let positions = compute_layout(&p);
        let scene = generate_scene(&p, &positions);
        assert!(scene.rooms.is_empty());
        assert!(scene.walls.is_empty());
        assert!(scene.openings.is_empty());
    }

    #[test]
    fn test_material_mapping() {
        assert_eq!(material_for_room("private", "Bathroom"),     "tile_white");
        assert_eq!(material_for_room("public",  "Kitchen"),      "tile_light");
        assert_eq!(material_for_room("outdoor", "Garden"),       "grass");
        assert_eq!(material_for_room("public",  "Living Room"),  "plaster_white");
        assert_eq!(material_for_room("private", "Bedroom"),      "plaster_warm");
        assert_eq!(material_for_room("service", "Garage"),       "concrete_grey");
    }

    #[test]
    fn test_merge_llm_artifact_places_mesh() {
        let mut scene = make_scene();
        let raw = vec![serde_json::json!({
            "tool_file": "chair.ts",
            "tool_name": "Chair",
            "room_id": "living",
            "offset_xz": [0.5, -0.3],
            "rotation_y": 0.0,
            "scale": 1.0,
            "style": "modern",
            "material": "fabric_grey"
        })];
        merge_llm_artifacts_into_scene(&mut scene, &raw);
        assert_eq!(scene.artifacts.len(), 1);
        assert_eq!(scene.artifacts[0].tool_file, "chair.ts");
        assert_eq!(scene.artifacts[0].room_id, "living");
    }
}