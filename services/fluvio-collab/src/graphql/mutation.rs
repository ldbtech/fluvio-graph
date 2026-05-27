//! Mutation resolvers for fluvio-collab.

use async_graphql::*;
use uuid::Uuid;

use crate::server::AppState;
use crate::graphql::types::*;
use crate::graphql::query::extract_user_id;
use crate::workflows::{group, invite, contribution, approval, company, team_workflow};
use crate::workflows::contribution::{ContributionInput};

pub struct MutationRoot;

#[Object(name = "Mutation")]
impl MutationRoot {

    /// Create a new group. Caller becomes the first owner.
    async fn create_group(
        &self,
        ctx:         &Context<'_>,
        name:        String,
        description: Option<String>,
    ) -> Result<GqlGroup> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let g = group::create_group(
            &caller_id.to_string(),
            &name,
            description.as_deref(),
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlGroup {
            id:          g.id,
            name:        g.name,
            description: g.description,
            graph_id:    g.graph_id,
            created_by:  g.created_by,
        })
    }

    /// Promote or demote a member's role. Caller must be an owner.
    async fn promote_member(
        &self,
        ctx:       &Context<'_>,
        group_id:  String,
        user_id:   String,
        new_role:  String,
    ) -> Result<GqlMember> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let m = group::promote_member(
            &group_id,
            &caller_id.to_string(),
            &user_id,
            &new_role,
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlMember {
            id:         m.id,
            group_id:   m.group_id,
            user_id:    m.user_id,
            role:       m.role,
            invited_by: m.invited_by,
        })
    }

    /// Create an invite token. Caller must be an owner.
    async fn invite(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        email:    Option<String>,
        role:     String,
    ) -> Result<GqlInvite> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let inv = invite::invite(
            &group_id,
            &caller_id.to_string(),
            email.as_deref(),
            &role,
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlInvite {
            id:         inv.id,
            group_id:   inv.group_id,
            token:      inv.token,
            role:       inv.role,
            email:      inv.email,
            expires_at: inv.expires_at,
        })
    }

    /// Accept an invite token and join the group.
    async fn accept_invite(
        &self,
        ctx:   &Context<'_>,
        token: String,
    ) -> Result<GqlMember> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let m = invite::accept_invite(
            &token,
            &caller_id.to_string(),
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlMember {
            id:         m.id,
            group_id:   m.group_id,
            user_id:    m.user_id,
            role:       m.role,
            invited_by: m.invited_by,
        })
    }

    /// Contribute knowledge to a group.
    async fn contribute(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        input:    GqlContributionInput,
    ) -> Result<GqlContribution> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let contrib_input = match input.kind.as_str() {
            "text" => ContributionInput::Text {
                text:       input.text.ok_or_else(|| Error::new("text required for kind=text"))?,
                source_uri: input.source_uri.unwrap_or_else(|| {
                    format!("collab://{group_id}/{}", uuid::Uuid::new_v4())
                }),
            },
            k => return Err(Error::new(format!("unsupported kind: {k}"))),
        };

        let c = contribution::contribute(
            &group_id,
            caller_id,
            contrib_input,
            &state.db,
            &state.graph,
            &state.ingestion,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlContribution {
            surreal_node_id: c.surreal_node_id,
            status:          c.status,
            queue_id:        c.queue_id,
            duplicate_of:    c.duplicate_of,
        })
    }

    /// Approve a pending contribution. Caller must be an owner.
    async fn approve(
        &self,
        ctx:             &Context<'_>,
        group_id:        String,
        contribution_id: String,
    ) -> Result<GqlQueueItem> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let item = approval::approve(
            &group_id,
            caller_id,
            &contribution_id,
            &state.graph,
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlQueueItem {
            id:              item.id,
            group_id:        item.group_id,
            contributed_by:  item.contributed_by,
            kind:            item.kind,
            surreal_node_id: item.surreal_node_id,
            status:          item.status,
            review_note:     item.review_note,
        })
    }

    /// Reject a pending contribution. Caller must be an owner.
    async fn reject(
        &self,
        ctx:             &Context<'_>,
        group_id:        String,
        contribution_id: String,
        note:            Option<String>,
    ) -> Result<GqlQueueItem> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let item = approval::reject(
            &group_id,
            caller_id,
            &contribution_id,
            note.as_deref(),
            &state.graph,
            &state.db,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlQueueItem {
            id:              item.id,
            group_id:        item.group_id,
            contributed_by:  item.contributed_by,
            kind:            item.kind,
            surreal_node_id: item.surreal_node_id,
            status:          item.status,
            review_note:     item.review_note,
        })
    }

    // ── Company & Teams Mutations ──────────────────────────────────────────────────

    /// Set/link the company email for the authenticated user.
    async fn update_company_email(
        &self,
        ctx:   &Context<'_>,
        email: String,
    ) -> Result<bool> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        company::link_company_email(
            &state.db,
            &caller_id.to_string(),
            &email,
        ).await.map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        Ok(true)
    }

    /// Create a new company. Sets caller's company_id and creates a General team.
    async fn create_company(
        &self,
        ctx:          &Context<'_>,
        name:         String,
        website:      String,
        linkedin_url: String,
        twitter_url:  Option<String>,
        github_url:   Option<String>,
    ) -> Result<GqlCompany> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let c = company::create_company(
            &state.db,
            &name,
            &website,
            &linkedin_url,
            twitter_url.as_deref(),
            github_url.as_deref(),
            &caller_id.to_string(),
        ).await.map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        Ok(GqlCompany {
            id:           c.id,
            name:         c.name,
            website:      c.website,
            linkedin_url: c.linkedin_url,
            twitter_url:  c.twitter_url,
            github_url:   c.github_url,
            created_by:   c.created_by,
        })
    }

    /// Accept a company invitation and join the company.
    async fn accept_company_invite(
        &self,
        ctx:       &Context<'_>,
        invite_id: String,
    ) -> Result<GqlCompanyInvite> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let inv = company::accept_company_invite(
            &state.db,
            &invite_id,
            &caller_id.to_string(),
        ).await.map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        Ok(GqlCompanyInvite {
            id:          inv.id,
            company_id:  inv.company_id,
            invited_by:  inv.invited_by,
            email:       inv.email,
            token:       inv.token,
            role:        inv.role,
            expires_at:  inv.expires_at,
            accepted_at: inv.accepted_at,
        })
    }

    /// Create a team/squad inside the company.
    async fn create_team(
        &self,
        ctx:         &Context<'_>,
        company_id:  String,
        name:        String,
        description: Option<String>,
    ) -> Result<GqlTeam> {
        let state = ctx.data::<AppState>()?;
        let t = state.db.create_team(
            &company_id,
            &name,
            description.as_deref(),
        ).await.map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        Ok(GqlTeam {
            id:          t.id,
            company_id:  t.company_id,
            name:        t.name,
            description: t.description,
        })
    }

    /// Create a custom AI automation workflow for a team.
    async fn create_workflow(
        &self,
        ctx:         &Context<'_>,
        team_id:     String,
        name:        String,
        description: Option<String>,
        steps:       String,
    ) -> Result<GqlTeamWorkflow> {
        let state     = ctx.data::<AppState>()?;
        let caller_id = extract_user_id(ctx)?;

        let w = team_workflow::create_team_workflow(
            &state.db,
            &team_id,
            &name,
            description.as_deref(),
            &steps,
            &caller_id.to_string(),
        ).await.map_err(|e: anyhow::Error| Error::new(e.to_string()))?;

        Ok(GqlTeamWorkflow {
            id:          w.id,
            team_id:     w.team_id,
            name:        w.name,
            description: w.description,
            steps:       w.steps,
            created_by:  w.created_by,
            is_enabled:  w.is_enabled,
        })
    }
}