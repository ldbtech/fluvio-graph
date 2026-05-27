//! Query resolvers for fluvio-collab.

use async_graphql::*;
use uuid::Uuid;

use crate::server::AppState;
use crate::graphql::types::*;
use crate::workflows::{group, approval, search, company, team_workflow};

pub struct QueryRoot;

#[Object(name = "Query")]
impl QueryRoot {

    /// All groups the authenticated user belongs to.
    async fn my_groups(&self, ctx: &Context<'_>) -> Result<Vec<GqlGroup>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let groups = group::get_my_groups(
            &caller_id.to_string(),
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(groups.into_iter().map(|g| GqlGroup {
            id:          g.id,
            name:        g.name,
            description: g.description,
            graph_id:    g.graph_id,
            created_by:  g.created_by,
        }).collect())
    }

    /// Members of a group — caller must be a member.
    async fn group_members(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
    ) -> Result<Vec<GqlMember>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let members = group::get_group_members(
            &group_id,
            &caller_id.to_string(),
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(members.into_iter().map(|m| GqlMember {
            id:         m.id,
            group_id:   m.group_id,
            user_id:    m.user_id,
            role:       m.role,
            invited_by: m.invited_by,
        }).collect())
    }

    /// Pending contributions awaiting owner approval.
    async fn pending_contributions(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
    ) -> Result<Vec<GqlQueueItem>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let items = approval::get_pending(
            &group_id,
            &caller_id.to_string(),
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(items.into_iter().map(|i| GqlQueueItem {
            id:              i.id,
            group_id:        i.group_id,
            contributed_by:  i.contributed_by,
            kind:            i.kind,
            surreal_node_id: i.surreal_node_id,
            status:          i.status,
            review_note:     i.review_note,
        }).collect())
    }

    /// Semantic search over approved group knowledge.
    async fn search_group(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        query:    String,
        top_k:    Option<i32>,
    ) -> Result<Vec<GqlSearchResult>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let results = search::search_group(
            &group_id,
            caller_id,
            &query,
            top_k.unwrap_or(10) as usize,
            &state.db,
            &state.graph,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(results.into_iter().map(|r| GqlSearchResult {
            id:    r.node.id,
            text:  r.node.source_text,
            score: r.score as f64,
        }).collect())
    }

    /// Chat over the group's approved knowledge graph.
    async fn group_chat(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        question: String,
        history:  Option<Vec<GqlChatMessage>>,
    ) -> Result<GqlChatResponse> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let history_pairs: Vec<(String, String)> = history
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.role, m.content))
            .collect();

        let resp = search::group_chat(
            &group_id,
            caller_id,
            &question,
            history_pairs,
            &state.anthropic_key,
            &state.db,
            &state.graph,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlChatResponse {
            answer: resp.answer,
            sources: resp.sources.into_iter().map(|s| GqlChatSource {
                id:    s.id,
                score: s.score as f64,
                text:  s.text,
            }).collect(),
        })
    }

    // ── Company & Teams ──────────────────────────────────────────────────────────

    /// The company the authenticated user belongs to.
    async fn my_company(&self, ctx: &Context<'_>) -> Result<Option<GqlCompany>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let user = state.db.get_user(&caller_id.to_string()).await
            .map_err(|e| Error::new(e.to_string()))?;

        if let Some(u) = user {
            if let Some(company_id) = u.company_id {
                let company = state.db.get_company(&company_id).await
                    .map_err(|e| Error::new(e.to_string()))?;
                return Ok(company.map(|c| GqlCompany {
                    id:           c.id,
                    name:         c.name,
                    website:      c.website,
                    linkedin_url: c.linkedin_url,
                    twitter_url:  c.twitter_url,
                    github_url:   c.github_url,
                    created_by:   c.created_by,
                }));
            }
        }
        Ok(None)
    }

    /// Retrieve squads/teams for the user's company
    async fn my_company_teams(&self, ctx: &Context<'_>) -> Result<Vec<GqlTeam>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let user = state.db.get_user(&caller_id.to_string()).await
            .map_err(|e| Error::new(e.to_string()))?;

        if let Some(u) = user {
            if let Some(company_id) = u.company_id {
                let teams = state.db.get_company_teams(&company_id).await
                    .map_err(|e| Error::new(e.to_string()))?;
                return Ok(teams.into_iter().map(|t| GqlTeam {
                    id:          t.id,
                    company_id:  t.company_id,
                    name:        t.name,
                    description: t.description,
                }).collect());
            }
        }
        Ok(Vec::new())
    }

    /// Retrieve a single squad/team by ID, ensuring it belongs to the user's company
    async fn my_company_team(&self, ctx: &Context<'_>, id: String) -> Result<Option<GqlTeam>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let user = state.db.get_user(&caller_id.to_string()).await
            .map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        if let Some(u) = user {
            if let Some(company_id) = u.company_id {
                if let Some(team) = state.db.get_team(&id).await.map_err(|e: anyhow::Error| Error::new(e.to_string()))? {
                    if team.company_id == company_id {
                        return Ok(Some(GqlTeam {
                            id:          team.id,
                            company_id:  team.company_id,
                            name:        team.name,
                            description: team.description,
                        }));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Retrieve pending company invites for user's emails
    async fn my_pending_company_invites(&self, ctx: &Context<'_>) -> Result<Vec<GqlCompanyInvite>> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let user = state.db.get_user(&caller_id.to_string()).await
            .map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        if let Some(u) = user {
            let mut invites = Vec::new();
            if let Some(ref email) = u.email {
                let mut res = state.db.get_pending_company_invites(email).await
                    .map_err(|e: anyhow::Error| Error::new(e.to_string()))?;
                invites.append(&mut res);
            }
            if let Some(ref company_email) = u.company_email {
                if Some(company_email.clone()) != u.email {
                    let mut res = state.db.get_pending_company_invites(company_email).await
                        .map_err(|e: anyhow::Error| Error::new(e.to_string()))?;
                    invites.append(&mut res);
                }
            }
            // De-duplicate by invite ID
            invites.sort_by_key(|i| i.id.clone());
            invites.dedup_by_key(|i| i.id.clone());

            return Ok(invites.into_iter().map(|i| GqlCompanyInvite {
                id:          i.id,
                company_id:  i.company_id,
                invited_by:  i.invited_by,
                email:       i.email,
                token:       i.token,
                role:        i.role,
                expires_at:  i.expires_at,
                accepted_at: i.accepted_at,
            }).collect());
        }
        Ok(Vec::new())
    }

    /// Retrieve custom workflows for a squad/team
    async fn my_team_workflows(
        &self,
        ctx:     &Context<'_>,
        team_id: String,
    ) -> Result<Vec<GqlTeamWorkflow>> {
        let state = ctx.data::<AppState>()?;
        let workflows = team_workflow::get_team_workflows(&state.db, &team_id).await
            .map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        Ok(workflows.into_iter().map(|w| GqlTeamWorkflow {
            id:          w.id,
            team_id:     w.team_id,
            name:        w.name,
            description: w.description,
            steps:       w.steps,
            created_by:  w.created_by,
            is_enabled:  w.is_enabled,
        }).collect())
    }
}

pub fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new(
            "x-user-id header missing — request must go through the gateway"
        ))
}