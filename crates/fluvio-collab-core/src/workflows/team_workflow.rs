//! Team custom workflows.
use crate::clients::database_client::DatabaseClient;
use crate::clients::dbtypes::DbTeamWorkflow;

pub async fn create_team_workflow(
    db:          &DatabaseClient,
    team_id:     &str,
    name:        &str,
    description: Option<&str>,
    steps:       &str,
    created_by:  &str,
) -> anyhow::Result<DbTeamWorkflow> {
    db.create_team_workflow(team_id, name, description, steps, created_by).await
}

pub async fn get_team_workflows(
    db:      &DatabaseClient,
    team_id: &str,
) -> anyhow::Result<Vec<DbTeamWorkflow>> {
    db.get_team_workflows(team_id).await
}
