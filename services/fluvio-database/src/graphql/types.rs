//! GraphQL types for fluvio-database subgraph.

use async_graphql::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::queries::{users::User, groups::Group, members::Member,
                invites::Invite, queue::QueueItem,
                workspaces::{Workspace, WorkspaceShare, WorkspaceShareWithUser}};
use crate::db::companies::{Company, CompanyInvite};
use crate::db::teams::{Team, TeamMember as DbTeamMember, TeamWorkflow};

// ── User ─────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlUser {
    pub id:            String,
    pub firebase_uid:  String,
    pub email:         Option<String>,
    pub display_name:  Option<String>,
    pub avatar_url:    Option<String>,
    pub company_email: Option<String>,
    pub company_id:    Option<String>,
    pub created_at:    String,
}

impl From<User> for GqlUser {
    fn from(u: User) -> Self {
        Self {
            id:            u.id.to_string(),
            firebase_uid:  u.firebase_uid,
            email:         u.email,
            display_name:  u.display_name,
            avatar_url:    u.avatar_url,
            company_email: u.company_email,
            company_id:    u.company_id.map(|id| id.to_string()),
            created_at:    u.created_at.to_rfc3339(),
        }
    }
}

// ── Group ─────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlGroup {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub graph_id:    String,
    pub created_by:  String,
    pub created_at:  String,
}

impl From<Group> for GqlGroup {
    fn from(g: Group) -> Self {
        Self {
            id:          g.id.to_string(),
            name:        g.name,
            description: g.description,
            graph_id:    g.graph_id.to_string(),
            created_by:  g.created_by.to_string(),
            created_at:  g.created_at.to_rfc3339(),
        }
    }
}

// ── Member ────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlMember {
    pub id:         String,
    pub group_id:   String,
    pub user_id:    String,
    pub role:       String,
    pub invited_by: Option<String>,
    pub joined_at:  String,
}

impl From<Member> for GqlMember {
    fn from(m: Member) -> Self {
        Self {
            id:         m.id.to_string(),
            group_id:   m.group_id.to_string(),
            user_id:    m.user_id.to_string(),
            role:       m.role,
            invited_by: m.invited_by.map(|u| u.to_string()),
            joined_at:  m.joined_at.to_rfc3339(),
        }
    }
}

// ── Invite ────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlInvite {
    pub id:          String,
    pub group_id:    String,
    pub invited_by:  String,
    pub token:       String,
    pub role:        String,
    pub email:       Option<String>,
    pub expires_at:  String,
    pub accepted_at: Option<String>,
    pub created_at:  String,
}

impl From<Invite> for GqlInvite {
    fn from(i: Invite) -> Self {
        Self {
            id:          i.id.to_string(),
            group_id:    i.group_id.to_string(),
            invited_by:  i.invited_by.to_string(),
            token:       i.token,
            role:        i.role,
            email:       i.email,
            expires_at:  i.expires_at.to_rfc3339(),
            accepted_at: i.accepted_at.map(|t| t.to_rfc3339()),
            created_at:  i.created_at.to_rfc3339(),
        }
    }
}

// ── QueueItem ─────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlQueueItem {
    pub id:              String,
    pub group_id:        String,
    pub contributed_by:  String,
    pub kind:            String,
    pub surreal_node_id: String,
    pub status:          String,
    pub reviewed_by:     Option<String>,
    pub review_note:     Option<String>,
    pub created_at:      String,
    pub reviewed_at:     Option<String>,
}

impl From<QueueItem> for GqlQueueItem {
    fn from(q: QueueItem) -> Self {
        Self {
            id:              q.id.to_string(),
            group_id:        q.group_id.to_string(),
            contributed_by:  q.contributed_by.to_string(),
            kind:            q.kind,
            surreal_node_id: q.surreal_node_id,
            status:          q.status,
            reviewed_by:     q.reviewed_by.map(|u| u.to_string()),
            review_note:     q.review_note,
            created_at:      q.created_at.to_rfc3339(),
            reviewed_at:     q.reviewed_at.map(|t| t.to_rfc3339()),
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateUserInput {
    pub firebase_uid: String,
    pub email:        Option<String>,
    pub display_name: Option<String>,
    pub avatar_url:   Option<String>,
}

#[derive(InputObject)]
pub struct CreateGroupInput {
    pub name:        String,
    pub description: Option<String>,
    pub created_by:  String,
}

#[derive(InputObject)]
pub struct AddMemberInput {
    pub group_id:   String,
    pub user_id:    String,
    pub role:       String,
    pub invited_by: Option<String>,
}

#[derive(InputObject)]
pub struct CreateInviteInput {
    pub group_id:   String,
    pub invited_by: String,
    pub role:       String,
    pub email:      Option<String>,
    /// Hours until expiry — default 72
    pub expires_in_hours: Option<i32>,
}

#[derive(InputObject)]
pub struct SubmitToQueueInput {
    pub group_id:        String,
    pub contributed_by:  String,
    pub kind:            String,
    pub surreal_node_id: String,
}

#[derive(InputObject)]
pub struct UpdateQueueStatusInput {
    pub id:          String,
    pub status:      String,
    pub reviewed_by: String,
    pub review_note: Option<String>,
}

// ── Workspace ─────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlWorkspace {
    pub id:          String,
    pub owner_id:    String,
    pub name:        String,
    pub is_public:   bool,
    pub created_at:  String,
}

impl From<Workspace> for GqlWorkspace {
    fn from(w: Workspace) -> Self {
        Self {
            id:          w.id.to_string(),
            owner_id:    w.owner_id.to_string(),
            name:        w.name,
            is_public:   w.is_public,
            created_at:  w.created_at.to_rfc3339(),
        }
    }
}

// ── WorkspaceShare ─────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlWorkspaceShare {
    pub id:           String,
    pub workspace_id: String,
    pub user_id:      String,
    pub shared_at:    String,
    pub email:        Option<String>,
    pub display_name: Option<String>,
}

impl From<WorkspaceShareWithUser> for GqlWorkspaceShare {
    fn from(s: WorkspaceShareWithUser) -> Self {
        Self {
            id:           s.id.to_string(),
            workspace_id: s.workspace_id.to_string(),
            user_id:      s.user_id.to_string(),
            shared_at:    s.shared_at.to_rfc3339(),
            email:        s.email,
            display_name: s.display_name,
        }
    }
}

#[derive(InputObject)]
pub struct CreateWorkspaceInput {
    pub owner_id:  String,
    pub name:      String,
    pub is_public: bool,
}

#[derive(InputObject)]
pub struct UpdateWorkspaceInput {
    pub id:        String,
    pub name:      Option<String>,
    pub is_public: Option<bool>,
}

// ── Company ───────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlCompany {
    pub id:           String,
    pub name:         String,
    pub website:      String,
    pub linkedin_url: String,
    pub twitter_url:  Option<String>,
    pub github_url:   Option<String>,
    pub created_by:   String,
    pub created_at:   String,
}

impl From<Company> for GqlCompany {
    fn from(c: Company) -> Self {
        Self {
            id:           c.id.to_string(),
            name:         c.name,
            website:      c.website,
            linkedin_url: c.linkedin_url,
            twitter_url:  c.twitter_url,
            github_url:   c.github_url,
            created_by:   c.created_by.to_string(),
            created_at:   c.created_at.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlCompanyInvite {
    pub id:          String,
    pub company_id:  String,
    pub invited_by:  String,
    pub email:       String,
    pub token:       String,
    pub role:        String,
    pub expires_at:  String,
    pub accepted_at: Option<String>,
    pub created_at:  String,
}

impl From<CompanyInvite> for GqlCompanyInvite {
    fn from(i: CompanyInvite) -> Self {
        Self {
            id:          i.id.to_string(),
            company_id:  i.company_id.to_string(),
            invited_by:  i.invited_by.to_string(),
            email:       i.email,
            token:       i.token,
            role:        i.role,
            expires_at:  i.expires_at.to_rfc3339(),
            accepted_at: i.accepted_at.map(|t| t.to_rfc3339()),
            created_at:  i.created_at.to_rfc3339(),
        }
    }
}

#[derive(InputObject)]
pub struct CreateCompanyInput {
    pub name:         String,
    pub website:      String,
    pub linkedin_url: String,
    pub twitter_url:  Option<String>,
    pub github_url:   Option<String>,
    pub created_by:   String,
}

#[derive(InputObject)]
pub struct CreateCompanyInviteInput {
    pub company_id: String,
    pub invited_by: String,
    pub email:      String,
    pub token:      String,
    pub role:       String,
    pub expires_at: String,
}

// ── Team ──────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlTeam {
    pub id:          String,
    pub company_id:  String,
    pub name:        String,
    pub description: Option<String>,
    pub created_at:  String,
    pub updated_at:  String,
}

impl From<Team> for GqlTeam {
    fn from(t: Team) -> Self {
        Self {
            id:          t.id.to_string(),
            company_id:  t.company_id.to_string(),
            name:        t.name,
            description: t.description,
            created_at:  t.created_at.to_rfc3339(),
            updated_at:  t.updated_at.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlTeamMember {
    pub id:        String,
    pub team_id:   String,
    pub user_id:   String,
    pub role:      String,
    pub joined_at: String,
}

impl From<DbTeamMember> for GqlTeamMember {
    fn from(tm: DbTeamMember) -> Self {
        Self {
            id:        tm.id.to_string(),
            team_id:   tm.team_id.to_string(),
            user_id:   tm.user_id.to_string(),
            role:      tm.role,
            joined_at: tm.joined_at.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlTeamWorkflow {
    pub id:          String,
    pub team_id:     String,
    pub name:        String,
    pub description: Option<String>,
    pub steps:       String, // JSON string
    pub created_by:  String,
    pub created_at:  String,
    pub updated_at:  String,
}

impl From<TeamWorkflow> for GqlTeamWorkflow {
    fn from(tw: TeamWorkflow) -> Self {
        Self {
            id:          tw.id.to_string(),
            team_id:     tw.team_id.to_string(),
            name:        tw.name,
            description: tw.description,
            steps:       tw.steps.to_string(),
            created_by:  tw.created_by.to_string(),
            created_at:  tw.created_at.to_rfc3339(),
            updated_at:  tw.updated_at.to_rfc3339(),
        }
    }
}

#[derive(InputObject)]
pub struct CreateTeamInput {
    pub company_id:  String,
    pub name:        String,
    pub description: Option<String>,
}

#[derive(InputObject)]
pub struct AddTeamMemberInput {
    pub team_id: String,
    pub user_id: String,
    pub role:    String,
}

#[derive(InputObject)]
pub struct CreateTeamWorkflowInput {
    pub team_id:     String,
    pub name:        String,
    pub description: Option<String>,
    pub steps:       String, // JSON string
    pub created_by:  String,
}

#[derive(SimpleObject, sqlx::FromRow, Clone)]
pub struct GqlPlannerChatMessage {
    pub id:           String,
    pub workspace_id: String,
    pub sender:       String,
    pub content:      String,
    pub created_at:   String,
}

// ── Company Brain (fluvio_company) Types ─────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlExecutionLog {
    pub id:                     String,
    pub company_id:             String,
    pub initiated_by_user_id:   String,
    pub initiated_by_twin_id:   Option<String>,
    pub agent_name:             String,
    pub message:                String,
    pub log_level:              String,
    pub timestamp:              String,
}

impl From<crate::db::company_ops::ExecutionLog> for GqlExecutionLog {
    fn from(el: crate::db::company_ops::ExecutionLog) -> Self {
        Self {
            id:                   el.id.to_string(),
            company_id:           el.company_id.to_string(),
            initiated_by_user_id: el.initiated_by_user_id.to_string(),
            initiated_by_twin_id: el.initiated_by_twin_id.map(|t| t.to_string()),
            agent_name:           el.agent_name,
            message:              el.message,
            log_level:            el.log_level,
            timestamp:            el.timestamp.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlActionAuthorization {
    pub id:                    String,
    pub company_id:            String,
    pub action_type:           String,
    pub description:           String,
    pub severity:              String,
    pub initiated_by_user_id:  String,
    pub status:                String,
    pub authorized_by_user_id: Option<String>,
    pub notes:                 Option<String>,
    pub created_at:            String,
    pub resolved_at:           Option<String>,
}

impl From<crate::db::company_ops::ActionAuthorization> for GqlActionAuthorization {
    fn from(aa: crate::db::company_ops::ActionAuthorization) -> Self {
        Self {
            id:                    aa.id.to_string(),
            company_id:            aa.company_id.to_string(),
            action_type:           aa.action_type,
            description:           aa.description,
            severity:              aa.severity,
            initiated_by_user_id:  aa.initiated_by_user_id.to_string(),
            status:                aa.status,
            authorized_by_user_id: aa.authorized_by_user_id.map(|u| u.to_string()),
            notes:                 aa.notes,
            created_at:            aa.created_at.to_rfc3339(),
            resolved_at:           aa.resolved_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlDocumentReconciliation {
    pub id:          String,
    pub company_id:  String,
    pub title:       String,
    pub description: String,
    pub source_a:    String,
    pub source_b:    String,
    pub resolved_to: String,
    pub time_ago:    String,
    pub created_at:  String,
}

impl From<crate::db::company_ops::DocumentReconciliation> for GqlDocumentReconciliation {
    fn from(dr: crate::db::company_ops::DocumentReconciliation) -> Self {
        Self {
            id:          dr.id.to_string(),
            company_id:  dr.company_id.to_string(),
            title:       dr.title,
            description: dr.description,
            source_a:    dr.source_a,
            source_b:    dr.source_b,
            resolved_to: dr.resolved_to,
            time_ago:    dr.time_ago,
            created_at:  dr.created_at.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlPipelineRun {
    pub id:         String,
    pub company_id: String,
    pub name:       String,
    pub agent_name: String,
    pub status:     String,
    pub progress:   i32,
    pub detail:     Option<String>,
    pub started_at: String,
    pub updated_at: String,
}

impl From<crate::db::company_ops::PipelineRun> for GqlPipelineRun {
    fn from(pr: crate::db::company_ops::PipelineRun) -> Self {
        Self {
            id:         pr.id.to_string(),
            company_id: pr.company_id.to_string(),
            name:       pr.name,
            agent_name: pr.agent_name,
            status:     pr.status,
            progress:   pr.progress,
            detail:     pr.detail,
            started_at: pr.started_at.to_rfc3339(),
            updated_at: pr.updated_at.to_rfc3339(),
        }
    }
}