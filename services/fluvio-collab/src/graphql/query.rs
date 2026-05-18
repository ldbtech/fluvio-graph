//! Query resolvers for fluvio-collab.

use async_graphql::*;
use uuid::Uuid;

use crate::server::AppState;
use crate::graphql::types::*;
use crate::workflows::{group, approval, search};

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
}

pub fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new(
            "x-user-id header missing — request must go through the gateway"
        ))
}