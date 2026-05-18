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
        pub created_at:   DateTime<Utc>,
        pub updated_at:   DateTime<Utc>,
    }

    // ── Queries ───────────────────────────────────────────────────
    pub const GET_BY_ID: &str = "
        SELECT id, firebase_uid, email, display_name, avatar_url,
               created_at, updated_at
        FROM users WHERE id = $1
    ";

    pub const GET_BY_FIREBASE_UID: &str = "
        SELECT id, firebase_uid, email, display_name, avatar_url,
               created_at, updated_at
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
        RETURNING id, firebase_uid, email, display_name, avatar_url,
                  created_at, updated_at
    ";

    pub const UPDATE: &str = "
        UPDATE users
        SET email        = COALESCE($2, email),
            display_name = COALESCE($3, display_name),
            avatar_url   = COALESCE($4, avatar_url),
            updated_at   = now()
        WHERE id = $1
        RETURNING id, firebase_uid, email, display_name, avatar_url,
                  created_at, updated_at
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