//! # fluvio-types
//!
//! Single source of truth for every domain type used across Fluvio
//! microservices. No service should define its own Node, Edge, Group,
//! or ToolManifest — import from here.
//!
//! ## Module map
//!
//! ```text
//! fluvio_types
//! ├── graph          — core graph primitives (Node, Edge, DomainGraph, …)
//! │   ├── ids        — NodeId, EdgeId, GraphId
//! │   ├── node       — Node, ExternalRef
//! │   ├── edge       — Edge
//! │   ├── domain     — DomainGraph (in-memory graph engine)
//! │   └── enums      — Domain, NodeKind, GraphEvent, GraphError, …
//! ├── collab         — collaborative graph types (Group, Member, Invite, …)
//! │   ├── group      — Group, GroupId, Role
//! │   ├── member     — Member, MemberStatus
//! │   ├── invite     — Invite, InviteToken
//! │   └── approval   — Contribution, ApprovalStatus
//! └── connector      — connector types (ConnectorId, ConnectorStatus, ToolManifest)
//!     ├── connector  — ConnectorId, ConnectorStatus, ConnectorConfig
//!     └── tool       — ToolDraft, ToolManifest, ToolStatus, ToolRunResult
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::all)]

// ── Core graph primitives ─────────────────────────────────────────────────────
pub mod graph;

// ── Collaborative graph types ─────────────────────────────────────────────────
pub mod collab;

// ── Connector + tool types ────────────────────────────────────────────────────
pub mod connector;

// ── Flat re-exports for convenience ──────────────────────────────────────────
// Services can `use fluvio_types::*` or pick specific paths.
pub use graph::ids::{EdgeId, GraphId, NodeId};
pub use graph::node::{ExternalRef, Node};
pub use graph::edge::Edge;
pub use graph::domain::DomainGraph;
pub use graph::enums::{
    Domain, GraphError, GraphEvent, GraphQuery, GraphResult,
    NodeKind, NodeKindFilter, NodePredicate,
};

pub use collab::group::{Group, GroupId, Role};
pub use collab::member::{Member, MemberStatus};
pub use collab::invite::{Invite, InviteToken};
pub use collab::approval::{ApprovalStatus, Contribution};

pub use connector::connector::{ConnectorConfig, ConnectorId, ConnectorStatus};
pub use connector::tool::{ToolDraft, ToolManifest, ToolRunResult, ToolStatus};