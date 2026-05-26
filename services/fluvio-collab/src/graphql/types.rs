//! GraphQL types for fluvio-collab subgraph.

use async_graphql::*;

#[derive(SimpleObject, Clone)]
pub struct GqlGroup {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub graph_id:    String,
    pub created_by:  String,
}

#[derive(SimpleObject, Clone)]
pub struct GqlMember {
    pub id:         String,
    pub group_id:   String,
    pub user_id:    String,
    pub role:       String,
    pub invited_by: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlInvite {
    pub id:         String,
    pub group_id:   String,
    pub token:      String,
    pub role:       String,
    pub email:      Option<String>,
    pub expires_at: String,
}

#[derive(SimpleObject, Clone)]
pub struct GqlContribution {
    pub surreal_node_id: String,
    pub status:          String,
    pub queue_id:        Option<String>,
    pub duplicate_of:    Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlQueueItem {
    pub id:              String,
    pub group_id:        String,
    pub contributed_by:  String,
    pub kind:            String,
    pub surreal_node_id: String,
    pub status:          String,
    pub review_note:     Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlSearchResult {
    pub id:    String,
    pub text:  String,
    pub score: f64,
}

#[derive(SimpleObject, Clone)]
pub struct GqlChatResponse {
    pub answer:  String,
    pub sources: Vec<GqlChatSource>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlChatSource {
    pub id:    String,
    pub score: f64,
    pub text:  String,
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct GqlContributionInput {
    /// "text" | "pdf" (pdf coming soon)
    pub kind:       String,
    pub text:       Option<String>,
    pub source_uri: Option<String>,
}

#[derive(InputObject)]
pub struct GqlChatMessage {
    pub role:    String,
    pub content: String,
}

// ── Company & Teams ──────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlCompany {
    pub id:           String,
    pub name:         String,
    pub website:      String,
    pub linkedin_url: String,
    pub twitter_url:  Option<String>,
    pub github_url:   Option<String>,
    pub created_by:   String,
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
}

#[derive(SimpleObject, Clone)]
pub struct GqlTeam {
    pub id:          String,
    pub company_id:  String,
    pub name:        String,
    pub description: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTeamMember {
    pub id:        String,
    pub team_id:   String,
    pub user_id:   String,
    pub role:      String,
    pub joined_at: String,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTeamWorkflow {
    pub id:          String,
    pub team_id:     String,
    pub name:        String,
    pub description: Option<String>,
    pub steps:       String,
    pub created_by:  String,
}