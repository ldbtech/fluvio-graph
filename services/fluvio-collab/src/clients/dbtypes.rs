//! Database types for fluvio-collab.
// ── Types returned from fluvio-database ───────────────────────────────────────
 
#[derive(Debug, Clone)]
pub struct DbUser {
    pub id:            String,
    pub firebase_uid:  String,
    pub email:         Option<String>,
    pub display_name:  Option<String>,
    pub company_email: Option<String>,
    pub company_id:    Option<String>,
}
 
#[derive(Debug, Clone)]
pub struct DbGroup {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub graph_id:    String,
    pub created_by:  String,
}
 
#[derive(Debug, Clone)]
pub struct DbMember {
    pub id:         String,
    pub group_id:   String,
    pub user_id:    String,
    pub role:       String,
    pub invited_by: Option<String>,
}
 
#[derive(Debug, Clone)]
pub struct DbInvite {
    pub id:         String,
    pub group_id:   String,
    pub token:      String,
    pub role:       String,
    pub email:      Option<String>,
    pub expires_at: String,
}
 
#[derive(Debug, Clone)]
pub struct DbQueueItem {
    pub id:              String,
    pub group_id:        String,
    pub contributed_by:  String,
    pub kind:            String,
    pub surreal_node_id: String,
    pub status:          String,
    pub review_note:     Option<String>,
}

#[derive(Debug, Clone)]
pub struct DbCompany {
    pub id:           String,
    pub name:         String,
    pub website:      String,
    pub linkedin_url: String,
    pub twitter_url:  Option<String>,
    pub github_url:   Option<String>,
    pub created_by:   String,
}

#[derive(Debug, Clone)]
pub struct DbCompanyInvite {
    pub id:          String,
    pub company_id:  String,
    pub invited_by:  String,
    pub email:       String,
    pub token:       String,
    pub role:        String,
    pub expires_at:  String,
    pub accepted_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DbTeam {
    pub id:          String,
    pub company_id:  String,
    pub name:        String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DbTeamMember {
    pub id:        String,
    pub team_id:   String,
    pub user_id:   String,
    pub role:      String,
    pub joined_at: String,
}

#[derive(Debug, Clone)]
pub struct DbTeamWorkflow {
    pub id:          String,
    pub team_id:     String,
    pub name:        String,
    pub description: Option<String>,
    pub steps:       String, // JSON string
    pub created_by:  String,
}
 