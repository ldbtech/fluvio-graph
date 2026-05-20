//! GraphQL types for connector operations.

use async_graphql::*;
use crate::db::queries::connectors::Connector;
use crate::db::queries::resources::ConnectorResource;

#[derive(SimpleObject, Clone)]
pub struct GqlConnector {
    pub id:            String,
    pub user_id:       String,
    pub group_id:      Option<String>,
    pub kind:          String,
    pub auth_method:   String,
    pub status:        String,
    pub error_message: Option<String>,
    pub access_token:  String, 
    pub created_at:    String,
    pub updated_at:    String,
    pub last_sync_at:  Option<String>,
}

impl From<Connector> for GqlConnector {
    fn from(c: Connector) -> Self {
        Self {
            id:            c.id.to_string(),
            user_id:       c.user_id.to_string(),
            group_id:      c.group_id.map(|g| g.to_string()),
            kind:          c.kind,
            auth_method:   c.auth_method,
            status:        c.status,
            error_message: c.error_message,
            access_token:  c.access_token,
            created_at:    c.created_at.to_rfc3339(),
            updated_at:    c.updated_at.to_rfc3339(),
            last_sync_at:  c.last_sync_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlConnectorResource {
    pub id:            String,
    pub connector_id:  String,
    pub resource_kind: String,
    pub external_id:   String,
    pub name:          String,
    pub description:   Option<String>,
    pub selected:      bool,
    pub last_sync_at:  Option<String>,
    pub node_count:    i32,
    pub created_at:    String,
}

impl From<ConnectorResource> for GqlConnectorResource {
    fn from(r: ConnectorResource) -> Self {
        Self {
            id:            r.id.to_string(),
            connector_id:  r.connector_id.to_string(),
            resource_kind: r.resource_kind,
            external_id:   r.external_id,
            name:          r.name,
            description:   r.description,
            selected:      r.selected,
            last_sync_at:  r.last_sync_at.map(|t| t.to_rfc3339()),
            node_count:    r.node_count,
            created_at:    r.created_at.to_rfc3339(),
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateConnectorInput {
    pub kind:         String,
    pub auth_method:  String,
    pub access_token: String,
    pub group_id:     Option<String>,
}

#[derive(InputObject)]
pub struct UpsertResourceInput {
    pub connector_id:  String,
    pub resource_kind: String,
    pub external_id:   String,
    pub name:          String,
    pub description:   Option<String>,
}

#[derive(InputObject)]
pub struct SelectResourcesInput {
    pub connector_id: String,
    /// List of external_ids to select — all others deselected
    pub external_ids: Vec<String>,
}