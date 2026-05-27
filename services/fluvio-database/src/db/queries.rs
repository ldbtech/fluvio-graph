// db/queries.rs

pub mod users {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    // ── The "dataclass" ──────────────────────────────────────────
    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct User {
        pub id:           Uuid,
        pub firebase_uid: String,
        pub email:        Option<String>,
        pub display_name: Option<String>,
        pub avatar_url:   Option<String>,
        pub company_email: Option<String>,
        pub company_id:   Option<Uuid>,
        pub role:         String,
        pub must_change_password: bool,
        pub policies:     Vec<String>,
        pub assigned_agent_roles: Vec<String>,
        pub twin_manifest: Option<String>,
        pub created_at:   DateTime<Utc>,
        pub updated_at:   DateTime<Utc>,
    }

    // ── Queries ───────────────────────────────────────────────────
    pub const GET_BY_ID: &str = "
        SELECT id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
               role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at
        FROM users WHERE id = $1
    ";

    pub const GET_BY_FIREBASE_UID: &str = "
        SELECT id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
               role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at
        FROM users WHERE firebase_uid = $1
    ";

    pub const UPSERT: &str = "
        INSERT INTO users (firebase_uid, email, display_name, avatar_url)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (firebase_uid) DO UPDATE
          SET email        = EXCLUDED.email,
              display_name = EXCLUDED.display_name,
              avatar_url   = EXCLUDED.avatar_url,
              updated_at   = now()
        RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                  role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at
    ";

    pub const UPDATE: &str = "
        UPDATE users
        SET email         = COALESCE($2, email),
            display_name  = COALESCE($3, display_name),
            avatar_url    = COALESCE($4, avatar_url),
            company_email = COALESCE($5, company_email),
            company_id    = COALESCE($6, company_id),
            role          = COALESCE($7, role),
            must_change_password = COALESCE($8, must_change_password),
            policies      = COALESCE($9, policies),
            assigned_agent_roles = COALESCE($10, assigned_agent_roles),
            twin_manifest = COALESCE($11, twin_manifest),
            updated_at    = now()
        WHERE id = $1
        RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                  role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at
    ";

    pub const GET_BY_EMAIL: &str = "
        SELECT id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
               role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at
        FROM users WHERE LOWER(email) = LOWER($1) OR LOWER(company_email) = LOWER($1)
    ";
}

pub mod groups {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct Group {
        pub id:          Uuid,
        pub name:        String,
        pub description: Option<String>,
        pub graph_id:    Uuid,
        pub created_by:  Uuid,
        pub created_at:  DateTime<Utc>,
        pub updated_at:  DateTime<Utc>,
    }

    pub const CREATE: &str = "
        INSERT INTO groups (name, description, created_by)
        VALUES ($1, $2, $3)
        RETURNING id, name, description, graph_id, created_by,
                  created_at, updated_at
    ";

    pub const GET_BY_ID: &str = "
        SELECT id, name, description, graph_id, created_by,
               created_at, updated_at
        FROM groups WHERE id = $1
    ";

    pub const GET_USER_GROUPS: &str = "
        SELECT g.id, g.name, g.description, g.graph_id, g.created_by,
               g.created_at, g.updated_at
        FROM groups g
        INNER JOIN group_members m ON m.group_id = g.id
        WHERE m.user_id = $1
        ORDER BY g.created_at DESC
    ";

    pub const UPDATE: &str = "
        UPDATE groups
        SET name        = COALESCE($2, name),
            description = COALESCE($3, description),
            updated_at  = now()
        WHERE id = $1
        RETURNING id, name, description, graph_id, created_by,
                  created_at, updated_at
    ";
}

pub mod members {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct Member {
        pub id:         Uuid,
        pub group_id:   Uuid,
        pub user_id:    Uuid,
        pub role:       String,
        pub invited_by: Option<Uuid>,
        pub joined_at:  DateTime<Utc>,
    }

    pub const ADD: &str = "
        INSERT INTO group_members (group_id, user_id, role, invited_by)
        VALUES ($1, $2, $3::member_role, $4)
        ON CONFLICT (group_id, user_id) DO UPDATE
          SET role = EXCLUDED.role
        RETURNING id, group_id, user_id, role::text, invited_by, joined_at
    ";

    pub const GET: &str = "
        SELECT id, group_id, user_id, role::text, invited_by, joined_at
        FROM group_members
        WHERE group_id = $1 AND user_id = $2
    ";

    pub const GET_ALL: &str = "
        SELECT id, group_id, user_id, role::text, invited_by, joined_at
        FROM group_members
        WHERE group_id = $1
        ORDER BY joined_at ASC
    ";

    pub const UPDATE_ROLE: &str = "
        UPDATE group_members
        SET role = $3::member_role
        WHERE group_id = $1 AND user_id = $2
        RETURNING id, group_id, user_id, role::text, invited_by, joined_at
    ";

    pub const REMOVE: &str = "
        DELETE FROM group_members WHERE group_id = $1 AND user_id = $2
    ";

    pub const OWNER_COUNT: &str = "
        SELECT COUNT(*) as count FROM group_members
        WHERE group_id = $1 AND role = 'owner'
    ";
}

pub mod invites {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct Invite {
        pub id:          Uuid,
        pub group_id:    Uuid,
        pub invited_by:  Uuid,
        pub token:       String,
        pub role:        String,
        pub email:       Option<String>,
        pub expires_at:  DateTime<Utc>,
        pub accepted_at: Option<DateTime<Utc>>,
        pub accepted_by: Option<Uuid>,
        pub created_at:  DateTime<Utc>,
    }

    pub const CREATE: &str = "
        INSERT INTO invites (group_id, invited_by, token, role, email, expires_at)
        VALUES ($1, $2, $3, $4::member_role, $5, $6)
        RETURNING id, group_id, invited_by, token, role::text,
                  email, expires_at, accepted_at, accepted_by, created_at
    ";

    pub const GET_BY_TOKEN: &str = "
        SELECT id, group_id, invited_by, token, role::text,
               email, expires_at, accepted_at, accepted_by, created_at
        FROM invites WHERE token = $1
    ";

    pub const ACCEPT: &str = "
        UPDATE invites
        SET accepted_at = now(), accepted_by = $2
        WHERE token = $1
          AND accepted_at IS NULL
          AND expires_at > now()
        RETURNING id, group_id, invited_by, token, role::text,
                  email, expires_at, accepted_at, accepted_by, created_at
    ";

    pub const GET_GROUP_INVITES: &str = "
        SELECT id, group_id, invited_by, token, role::text,
               email, expires_at, accepted_at, accepted_by, created_at
        FROM invites WHERE group_id = $1
        ORDER BY created_at DESC
    ";
}

pub mod queue {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct QueueItem {
        pub id:              Uuid,
        pub group_id:        Uuid,
        pub contributed_by:  Uuid,
        pub kind:            String,
        pub surreal_node_id: String,
        pub status:          String,
        pub reviewed_by:     Option<Uuid>,
        pub review_note:     Option<String>,
        pub created_at:      DateTime<Utc>,
        pub reviewed_at:     Option<DateTime<Utc>>,
    }

    pub const SUBMIT: &str = "
        INSERT INTO approval_queue
            (group_id, contributed_by, kind, surreal_node_id)
        VALUES ($1, $2, $3::contribution_kind, $4)
        RETURNING id, group_id, contributed_by,
                  kind::text, surreal_node_id, status::text,
                  reviewed_by, review_note, created_at, reviewed_at
    ";

    pub const GET_PENDING: &str = "
        SELECT id, group_id, contributed_by,
               kind::text, surreal_node_id, status::text,
               reviewed_by, review_note, created_at, reviewed_at
        FROM approval_queue
        WHERE group_id = $1 AND status = 'pending'
        ORDER BY created_at ASC
    ";

    pub const GET_BY_ID: &str = "
        SELECT id, group_id, contributed_by,
               kind::text, surreal_node_id, status::text,
               reviewed_by, review_note, created_at, reviewed_at
        FROM approval_queue WHERE id = $1
    ";

    pub const UPDATE_STATUS: &str = "
        UPDATE approval_queue
        SET status      = $2::approval_status,
            reviewed_by = $3,
            review_note = $4,
            reviewed_at = now()
        WHERE id = $1
        RETURNING id, group_id, contributed_by,
                  kind::text, surreal_node_id, status::text,
                  reviewed_by, review_note, created_at, reviewed_at
    ";

    pub const GET_USER_CONTRIBUTIONS: &str = "
        SELECT id, group_id, contributed_by,
               kind::text, surreal_node_id, status::text,
               reviewed_by, review_note, created_at, reviewed_at
        FROM approval_queue
        WHERE group_id = $1 AND contributed_by = $2
        ORDER BY created_at DESC
    ";
}

// Add these two modules to your existing db/queries.rs file
// Place them after the existing queue module

pub mod connectors {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct Connector {
        pub id:               Uuid,
        pub user_id:          Uuid,
        pub group_id:         Option<Uuid>,
        pub kind:             String,
        pub auth_method:      String,
        pub access_token:     String,
        pub refresh_token:    Option<String>,
        pub token_expires_at: Option<DateTime<Utc>>,
        pub status:           String,
        pub error_message:    Option<String>,
        pub created_at:       DateTime<Utc>,
        pub updated_at:       DateTime<Utc>,
        pub last_sync_at:     Option<DateTime<Utc>>,
    }

    pub const CREATE: &str = "
        INSERT INTO connectors
            (user_id, group_id, kind, auth_method, access_token,
            refresh_token, token_expires_at)
        VALUES ($1, $2, $3::connector_kind, $4::auth_method, $5, $6, $7)
        ON CONFLICT (user_id, kind) WHERE group_id IS NULL
            DO UPDATE SET
                access_token  = EXCLUDED.access_token,
                auth_method   = EXCLUDED.auth_method,
                status        = 'connected'::connector_status,
                error_message = NULL,
                updated_at    = now()
        RETURNING
            id, user_id, group_id,
            kind::text, auth_method::text,
            access_token, refresh_token, token_expires_at,
            status::text, error_message,
            created_at, updated_at, last_sync_at
    ";

    pub const GET_BY_ID: &str = "
        SELECT id, user_id, group_id,
               kind::text, auth_method::text,
               access_token, refresh_token, token_expires_at,
               status::text, error_message,
               created_at, updated_at, last_sync_at
        FROM connectors WHERE id = $1
    ";

    pub const GET_USER_CONNECTORS: &str = "
        SELECT id, user_id, group_id,
               kind::text, auth_method::text,
               access_token, refresh_token, token_expires_at,
               status::text, error_message,
               created_at, updated_at, last_sync_at
        FROM connectors
        WHERE user_id = $1
        ORDER BY created_at DESC
    ";

    pub const GET_GROUP_CONNECTORS: &str = "
        SELECT id, user_id, group_id,
               kind::text, auth_method::text,
               access_token, refresh_token, token_expires_at,
               status::text, error_message,
               created_at, updated_at, last_sync_at
        FROM connectors
        WHERE group_id = $1
        ORDER BY created_at DESC
    ";

    pub const UPDATE_STATUS: &str = "
        UPDATE connectors
        SET status        = $2::connector_status,
            error_message = $3,
            updated_at    = now()
        WHERE id = $1
        RETURNING
            id, user_id, group_id,
            kind::text, auth_method::text,
            access_token, refresh_token, token_expires_at,
            status::text, error_message,
            created_at, updated_at, last_sync_at
    ";

    pub const UPDATE_LAST_SYNC: &str = "
        UPDATE connectors
        SET last_sync_at = now(),
            status       = 'connected'::connector_status,
            updated_at   = now()
        WHERE id = $1
        RETURNING
            id, user_id, group_id,
            kind::text, auth_method::text,
            access_token, refresh_token, token_expires_at,
            status::text, error_message,
            created_at, updated_at, last_sync_at
    ";

    pub const UPDATE_TOKENS: &str = "
        UPDATE connectors
        SET access_token     = $2,
            refresh_token    = $3,
            token_expires_at = $4,
            updated_at       = now()
        WHERE id = $1
        RETURNING
            id, user_id, group_id,
            kind::text, auth_method::text,
            access_token, refresh_token, token_expires_at,
            status::text, error_message,
            created_at, updated_at, last_sync_at
    ";

    pub const DELETE: &str = "
        DELETE FROM connectors WHERE id = $1
    ";
}

pub mod resources {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};
    use serde_json::Value;

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct ConnectorResource {
        pub id:            Uuid,
        pub connector_id:  Uuid,
        pub resource_kind: String,
        pub external_id:   String,
        pub name:          String,
        pub description:   Option<String>,
        pub selected:      bool,
        pub last_sync_at:  Option<DateTime<Utc>>,
        pub node_count:    i32,
        pub meta:          Value,
        pub created_at:    DateTime<Utc>,
        pub updated_at:    DateTime<Utc>,
    }

    pub const UPSERT: &str = "
        INSERT INTO connector_resources
            (connector_id, resource_kind, external_id, name, description, meta)
        VALUES ($1, $2::resource_kind, $3, $4, $5, $6)
        ON CONFLICT (connector_id, external_id) DO UPDATE
            SET name        = EXCLUDED.name,
                description = EXCLUDED.description,
                meta        = EXCLUDED.meta,
                updated_at  = now()
        RETURNING
            id, connector_id, resource_kind::text,
            external_id, name, description,
            selected, last_sync_at, node_count, meta,
            created_at, updated_at
    ";

    pub const GET_BY_CONNECTOR: &str = "
        SELECT id, connector_id, resource_kind::text,
               external_id, name, description,
               selected, last_sync_at, node_count, meta,
               created_at, updated_at
        FROM connector_resources
        WHERE connector_id = $1
        ORDER BY name ASC
    ";

    pub const GET_SELECTED: &str = "
        SELECT id, connector_id, resource_kind::text,
               external_id, name, description,
               selected, last_sync_at, node_count, meta,
               created_at, updated_at
        FROM connector_resources
        WHERE connector_id = $1 AND selected = true
        ORDER BY name ASC
    ";

    pub const SET_SELECTED: &str = "
        UPDATE connector_resources
        SET selected   = $2,
            updated_at = now()
        WHERE connector_id = $1 AND external_id = $3
        RETURNING
            id, connector_id, resource_kind::text,
            external_id, name, description,
            selected, last_sync_at, node_count, meta,
            created_at, updated_at
    ";

    pub const UPDATE_SYNC_STATS: &str = "
        UPDATE connector_resources
        SET last_sync_at = now(),
            node_count   = node_count + $2,
            updated_at   = now()
        WHERE connector_id = $1 AND external_id = $3
    ";

    pub const BULK_SET_SELECTED: &str = "
        UPDATE connector_resources
        SET selected   = (external_id = ANY($2)),
            updated_at = now()
        WHERE connector_id = $1
    ";
}

pub mod workspaces {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct Workspace {
        pub id:          Uuid,
        pub owner_id:    Uuid,
        pub name:        String,
        pub is_public:   bool,
        pub created_at:  DateTime<Utc>,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct WorkspaceShare {
        pub id:           Uuid,
        pub workspace_id: Uuid,
        pub user_id:      Uuid,
        pub shared_at:    DateTime<Utc>,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct WorkspaceShareWithUser {
        pub id:           Uuid,
        pub workspace_id: Uuid,
        pub user_id:      Uuid,
        pub shared_at:    DateTime<Utc>,
        pub email:        Option<String>,
        pub display_name: Option<String>,
    }

    pub const CREATE: &str = "
        INSERT INTO workspaces (owner_id, name, is_public)
        VALUES ($1, $2, $3)
        RETURNING id, owner_id, name, is_public, created_at
    ";

    pub const GET_BY_ID: &str = "
        SELECT id, owner_id, name, is_public, created_at
        FROM workspaces WHERE id = $1
    ";

    pub const GET_USER_WORKSPACES: &str = "
        SELECT DISTINCT w.id, w.owner_id, w.name, w.is_public, w.created_at
        FROM workspaces w
        LEFT JOIN workspace_shares s ON s.workspace_id = w.id
        WHERE w.owner_id = $1 OR s.user_id = $1
        ORDER BY w.created_at DESC
    ";

    pub const UPDATE: &str = "
        UPDATE workspaces
        SET name      = COALESCE($2, name),
            is_public = COALESCE($3, is_public)
        WHERE id = $1
        RETURNING id, owner_id, name, is_public, created_at
    ";

    pub const DELETE: &str = "
        DELETE FROM workspaces WHERE id = $1
    ";

    pub const SHARE: &str = "
        INSERT INTO workspace_shares (workspace_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (workspace_id, user_id) DO NOTHING
        RETURNING id, workspace_id, user_id, shared_at
    ";

    pub const UNSHARE: &str = "
        DELETE FROM workspace_shares
        WHERE workspace_id = $1 AND user_id = $2
    ";

    pub const GET_SHARES: &str = "
        SELECT s.id, s.workspace_id, s.user_id, s.shared_at, u.email, u.display_name
        FROM workspace_shares s
        INNER JOIN users u ON u.id = s.user_id
        WHERE s.workspace_id = $1
        ORDER BY s.shared_at ASC
    ";
}