//! layout_engine.rs
//!
//! Assigns 2D positions and dimensions to rooms in a SpaceProgram.
//!
//! Architecture:
//!   `LayoutStrategy` trait — stable public interface, never changes.
//!   `ZoneGridLayout`       — Option A: deterministic zone-based grid (current).
//!   `ForceLayout`          — Option B: force-directed (future drop-in).
//!
//! To migrate to Option B later:
//!   1. Implement `LayoutStrategy` for `ForceLayout`
//!   2. Change one line in `compute_layout()` — nothing else changes
//!
//! Output: `Vec<PositionedRoom>` — room positions in meters from origin (0,0).
//! The frontend converts these directly to Three.js Box geometry positions.
//!
//! Zone grid strategy:
//!   ┌─────────────────────────────────────┐
//!   │  PUBLIC zone (left column)          │
//!   │  entry, living, kitchen, dining     │
//!   ├─────────────────────────────────────┤
//!   │  PRIVATE zone (right column)        │
//!   │  bedrooms, bathrooms, office        │
//!   ├─────────────────────────────────────┤
//!   │  SERVICE zone (bottom strip)        │
//!   │  laundry, storage, garage, utility  │
//!   ├─────────────────────────────────────┤
//!   │  OUTDOOR zone (outer boundary)      │
//!   │  garden, terrace, balcony           │
//!   └─────────────────────────────────────┘

use serde::{Deserialize, Serialize};
use super::space_program::{SpaceProgram, Zone};

// ── Output type ───────────────────────────────────────────────────────────────

/// A room with computed 2D position and dimensions.
/// x, y are the room's bottom-left corner in meters from origin.
/// width = east-west extent, depth = north-south extent, height = floor-to-ceiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedRoom {
    pub room_id:  String,
    pub name:     String,
    pub x:        f32,
    pub y:        f32,
    pub width:    f32,
    pub depth:    f32,
    pub height:   f32,
    pub zone:     String,
    /// Computed center point (convenience for Three.js positioning).
    pub center_x: f32,
    pub center_y: f32,
}

impl PositionedRoom {
    fn new(
        room_id: &str,
        name:    &str,
        zone:    &str,
        x: f32, y: f32,
        width: f32, depth: f32,
        height: f32,
    ) -> Self {
        Self {
            room_id:  room_id.to_string(),
            name:     name.to_string(),
            x,
            y,
            width,
            depth,
            height,
            zone:     zone.to_string(),
            center_x: x + width  / 2.0,
            center_y: y + depth  / 2.0,
        }
    }
}

// ── Layout strategy trait ─────────────────────────────────────────────────────

/// Stable interface for all layout algorithms.
/// Option A and Option B both implement this — the rest of the codebase
/// only ever calls `compute_layout()` and never touches this trait directly.
pub trait LayoutStrategy {
    fn place(&self, program: &SpaceProgram) -> Vec<PositionedRoom>;
}

// ── Option A — Zone grid layout ───────────────────────────────────────────────

pub struct ZoneGridLayout {
    /// Default floor-to-ceiling height in meters.
    pub default_height: f32,
    /// Padding between rooms in meters.
    pub padding: f32,
}

impl Default for ZoneGridLayout {
    fn default() -> Self {
        Self {
            default_height: 3.0,
            padding:        0.2,
        }
    }
}

impl ZoneGridLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute width and depth from area, using a sensible aspect ratio.
    /// Returns (width, depth) in meters.
    fn dimensions_from_area(&self, area_sqm: f32) -> (f32, f32) {
        // Target aspect ratio ~1.4:1 (width:depth) — feels like a real room.
        // width * depth = area, width = depth * 1.4
        // depth^2 * 1.4 = area → depth = sqrt(area / 1.4)
        let depth = (area_sqm / 1.4_f32).sqrt().max(2.0);
        let width = (area_sqm / depth).max(2.0);
        (
            (width  * 10.0).round() / 10.0,  // round to 0.1m
            (depth  * 10.0).round() / 10.0,
        )
    }
}

impl LayoutStrategy for ZoneGridLayout {
    fn place(&self, program: &SpaceProgram) -> Vec<PositionedRoom> {
        let pad = self.padding;
        let mut positioned: Vec<PositionedRoom> = Vec::new();

        // Separate rooms by zone.
        let public_rooms:  Vec<_> = program.rooms.iter()
            .filter(|r| r.zone == Zone::Public).collect();
        let private_rooms: Vec<_> = program.rooms.iter()
            .filter(|r| r.zone == Zone::Private).collect();
        let service_rooms: Vec<_> = program.rooms.iter()
            .filter(|r| r.zone == Zone::Service).collect();
        let outdoor_rooms: Vec<_> = program.rooms.iter()
            .filter(|r| r.zone == Zone::Outdoor).collect();

        // ── Public zone: left column, stacked vertically ──────────────────────
        let public_x  = 0.0_f32;
        let mut public_y  = 0.0_f32;
        let mut max_public_width = 0.0_f32;

        for room in &public_rooms {
            let (w, d) = self.dimensions_from_area(room.area_sqm);
            positioned.push(PositionedRoom::new(
                &room.id, &room.name,
                Zone::Public.as_str(),
                public_x, public_y,
                w, d,
                self.default_height,
            ));
            public_y += d + pad;
            if w > max_public_width { max_public_width = w; }
        }

        let public_total_height = public_y;

        // ── Private zone: right column (offset by public width + gap) ─────────
        let private_x_start = max_public_width + pad * 3.0;
        let private_x   = private_x_start;
        let mut private_y   = 0.0_f32;
        let mut max_private_width = 0.0_f32;

        for room in &private_rooms {
            let (w, d) = self.dimensions_from_area(room.area_sqm);
            positioned.push(PositionedRoom::new(
                &room.id, &room.name,
                Zone::Private.as_str(),
                private_x, private_y,
                w, d,
                self.default_height,
            ));
            private_y += d + pad;
            if w > max_private_width { max_private_width = w; }
        }

        // ── Service zone: bottom strip below both columns ─────────────────────
        let service_y_start = public_total_height.max(private_y) + pad * 2.0;
        let mut service_x   = 0.0_f32;

        for room in &service_rooms {
            let (w, d) = self.dimensions_from_area(room.area_sqm);
            positioned.push(PositionedRoom::new(
                &room.id, &room.name,
                Zone::Service.as_str(),
                service_x, service_y_start,
                w, d,
                self.default_height,
            ));
            service_x += w + pad;
        }

        // ── Outdoor zone: right side of the whole layout ──────────────────────
        let outdoor_x_start = private_x_start + max_private_width + pad * 3.0;
        let mut outdoor_y   = 0.0_f32;

        for room in &outdoor_rooms {
            // Outdoor spaces get larger dimensions (squarer aspect ratio).
            let side = (room.area_sqm).sqrt().max(3.0);
            let (w, d) = (
                (side * 1.2 * 10.0).round() / 10.0,
                (side * 10.0).round() / 10.0,
            );
            positioned.push(PositionedRoom::new(
                &room.id, &room.name,
                Zone::Outdoor.as_str(),
                outdoor_x_start, outdoor_y,
                w, d,
                0.0, // outdoor has no ceiling
            ));
            outdoor_y += d + pad;
        }

        positioned
    }
}

// ── Option B stub — future force-directed layout ───────────────────────────────

/// Force-directed layout — future implementation.
/// Rooms with `adjacent_to` edges attract, `separated_from` edges repel.
/// Iterate until kinetic energy drops below threshold.
///
/// NOT IMPLEMENTED — placeholder for migration path.
/// When ready: implement `LayoutStrategy for ForceLayout` and change
/// one line in `compute_layout()`.
pub struct ForceLayout {
    pub iterations:  usize,
    pub spring_k:    f32,
    pub repulsion_k: f32,
    pub damping:     f32,
}

impl Default for ForceLayout {
    fn default() -> Self {
        Self {
            iterations:  500,
            spring_k:    0.1,
            repulsion_k: 50.0,
            damping:     0.85,
        }
    }
}

// When implementing Option B, add:
// impl LayoutStrategy for ForceLayout { fn place(...) { ... } }

// ── Public API ────────────────────────────────────────────────────────────────

/// Compute room positions for a SpaceProgram.
///
/// Currently uses ZoneGridLayout (Option A).
/// To switch to Option B: change `ZoneGridLayout::new()` to `ForceLayout::default()`.
pub fn compute_layout(program: &SpaceProgram) -> Vec<PositionedRoom> {
    ZoneGridLayout::new().place(program)
}

/// Compute layout with a custom strategy — useful for testing.
pub fn compute_layout_with<S: LayoutStrategy>(
    program:  &SpaceProgram,
    strategy: &S,
) -> Vec<PositionedRoom> {
    strategy.place(program)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_registry::architecture::space_program::{
        DesignStyle, NaturalLight, PrivacyLevel, RelationshipKind,
        Room, RoomRelationship, SpaceProgram, Zone,
    };

    fn make_program() -> SpaceProgram {
        SpaceProgram {
            brief:     "modern house".to_string(),
            design_id: "test-design".to_string(),
            design_style: DesignStyle {
                aesthetic:  "modern".to_string(),
                priority:   "light".to_string(),
                total_area: Some(150.0),
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
                    id: "kitchen".to_string(), name: "Kitchen".to_string(),
                    area_sqm: 22.0, zone: Zone::Public,
                    natural_light: NaturalLight::High,
                    privacy: PrivacyLevel::Semi, notes: None,
                },
                Room {
                    id: "master_bed".to_string(), name: "Master Bedroom".to_string(),
                    area_sqm: 25.0, zone: Zone::Private,
                    natural_light: NaturalLight::High,
                    privacy: PrivacyLevel::Private, notes: None,
                },
                Room {
                    id: "master_bath".to_string(), name: "Master Bathroom".to_string(),
                    area_sqm: 8.0, zone: Zone::Private,
                    natural_light: NaturalLight::Low,
                    privacy: PrivacyLevel::Private, notes: None,
                },
                Room {
                    id: "laundry".to_string(), name: "Laundry".to_string(),
                    area_sqm: 6.0, zone: Zone::Service,
                    natural_light: NaturalLight::Low,
                    privacy: PrivacyLevel::Semi, notes: None,
                },
                Room {
                    id: "garden".to_string(), name: "Garden".to_string(),
                    area_sqm: 60.0, zone: Zone::Outdoor,
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
                    from_id: "living".to_string(), to_id: "kitchen".to_string(),
                    kind: RelationshipKind::AdjacentTo, notes: None,
                },
                RoomRelationship {
                    from_id: "master_bed".to_string(), to_id: "master_bath".to_string(),
                    kind: RelationshipKind::ConnectsVia, notes: None,
                },
            ],
        }
    }

    #[test]
    fn test_all_rooms_positioned() {
        let p = make_program();
        let positions = compute_layout(&p);
        assert_eq!(positions.len(), p.rooms.len());
    }

    #[test]
    fn test_every_room_has_position() {
        let p = make_program();
        let positions = compute_layout(&p);
        for room in &p.rooms {
            assert!(
                positions.iter().any(|pos| pos.room_id == room.id),
                "room '{}' not positioned", room.id
            );
        }
    }

    #[test]
    fn test_positive_dimensions() {
        let p = make_program();
        let positions = compute_layout(&p);
        for pos in &positions {
            assert!(pos.width  > 0.0, "{} has zero width",  pos.room_id);
            assert!(pos.depth  > 0.0, "{} has zero depth",  pos.room_id);
            // Outdoor rooms have height 0 (no ceiling).
            if pos.zone != "outdoor" {
                assert!(pos.height > 0.0, "{} has zero height", pos.room_id);
            }
        }
    }

    #[test]
    fn test_center_point_correct() {
        let p = make_program();
        let positions = compute_layout(&p);
        for pos in &positions {
            let expected_cx = pos.x + pos.width / 2.0;
            let expected_cy = pos.y + pos.depth / 2.0;
            assert!(
                (pos.center_x - expected_cx).abs() < 0.01,
                "{} center_x wrong: {} vs {}", pos.room_id, pos.center_x, expected_cx
            );
            assert!(
                (pos.center_y - expected_cy).abs() < 0.01,
                "{} center_y wrong", pos.room_id
            );
        }
    }

    #[test]
    fn test_zones_separated() {
        let p = make_program();
        let positions = compute_layout(&p);

        let public_rooms: Vec<_>  = positions.iter().filter(|r| r.zone == "public").collect();
        let private_rooms: Vec<_> = positions.iter().filter(|r| r.zone == "private").collect();

        // Public rooms should be to the left of private rooms.
        if !public_rooms.is_empty() && !private_rooms.is_empty() {
            let max_public_x  = public_rooms.iter().map(|r| r.x + r.width).fold(f32::NEG_INFINITY, f32::max);
            let min_private_x = private_rooms.iter().map(|r| r.x).fold(f32::INFINITY, f32::min);
            assert!(
                min_private_x >= max_public_x,
                "private rooms overlap public rooms: min_private_x={min_private_x} max_public_x={max_public_x}"
            );
        }
    }

    #[test]
    fn test_outdoor_has_no_ceiling() {
        let p = make_program();
        let positions = compute_layout(&p);
        let garden = positions.iter().find(|r| r.room_id == "garden").unwrap();
        assert_eq!(garden.height, 0.0);
        assert_eq!(garden.zone, "outdoor");
    }

    #[test]
    fn test_service_rooms_below_main() {
        let p = make_program();
        let positions = compute_layout(&p);

        let laundry = positions.iter().find(|r| r.room_id == "laundry").unwrap();
        let living  = positions.iter().find(|r| r.room_id == "living").unwrap();

        // Service rooms start below (higher y) than main living areas.
        assert!(
            laundry.y >= living.y,
            "laundry.y={} should be >= living.y={}", laundry.y, living.y
        );
    }

    #[test]
    fn test_dimensions_from_area() {
        let layout = ZoneGridLayout::new();
        let (w, d) = layout.dimensions_from_area(35.0);
        // Area should be approximately correct.
        assert!((w * d - 35.0).abs() < 3.0, "area mismatch: {}x{}={}", w, d, w*d);
        // Both dimensions should be positive.
        assert!(w > 0.0 && d > 0.0);
    }

    #[test]
    fn test_small_room_minimum_size() {
        let layout = ZoneGridLayout::new();
        let (w, d) = layout.dimensions_from_area(1.0); // tiny room
        assert!(w >= 2.0, "width too small: {w}");
        assert!(d >= 2.0, "depth too small: {d}");
    }

    #[test]
    fn test_compute_layout_with_custom_strategy() {
        // Verify compute_layout_with accepts any LayoutStrategy implementation.
        struct FixedLayout;
        impl LayoutStrategy for FixedLayout {
            fn place(&self, program: &SpaceProgram) -> Vec<PositionedRoom> {
                program.rooms.iter().enumerate().map(|(i, r)| {
                    PositionedRoom::new(
                        &r.id, &r.name, r.zone.as_str(),
                        i as f32 * 10.0, 0.0,
                        5.0, 5.0, 3.0,
                    )
                }).collect()
            }
        }
        let p = make_program();
        let positions = compute_layout_with(&p, &FixedLayout);
        assert_eq!(positions.len(), p.rooms.len());
        // Fixed layout places rooms at x=0, 10, 20...
        assert_eq!(positions[0].x, 0.0);
        assert_eq!(positions[1].x, 10.0);
    }

    #[test]
    fn test_empty_program() {
        let p = SpaceProgram {
            brief:     "empty".to_string(),
            design_id: "empty-design".to_string(),
            design_style: DesignStyle {
                aesthetic: "modern".to_string(),
                priority:  "light".to_string(),
                total_area: None,
                stories:    None,
            },
            rooms:         vec![],
            relationships: vec![],
        };
        let positions = compute_layout(&p);
        assert!(positions.is_empty());
    }

    #[test]
    fn test_single_room() {
        let p = SpaceProgram {
            brief:     "one room".to_string(),
            design_id: "single".to_string(),
            design_style: DesignStyle {
                aesthetic: "modern".to_string(),
                priority:  "light".to_string(),
                total_area: Some(20.0),
                stories:    Some(1),
            },
            rooms: vec![Room {
                id: "studio".to_string(), name: "Studio".to_string(),
                area_sqm: 20.0, zone: Zone::Public,
                natural_light: NaturalLight::High,
                privacy: PrivacyLevel::Public, notes: None,
            }],
            relationships: vec![],
        };
        let positions = compute_layout(&p);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].room_id, "studio");
        assert!(positions[0].width > 0.0);
    }
}