//! Contribution workflow — submit knowledge to a group.

use uuid::Uuid;
use crate::clients::{DatabaseClient, GraphClient, IngestionClient};
use crate::policy::{access, approval};

#[derive(Debug, Clone)]
pub struct Contribution {
    pub surreal_node_id: String,
    pub status:          String,
    pub queue_id:        Option<String>,
    pub duplicate_of:    Option<String>,
}

#[derive(Debug, Clone)]
pub enum ContributionInput {
    Text {
        text:       String,
        source_uri: String,
    },
}

/// Submit a knowledge contribution to a group.
///
/// Flow:
///   1. Verify caller is a member who can contribute
///   2. Check for duplicates (cosine > 0.92)
///   3. Ingest content → embed → store in graph
///   4. Tag node with group_id + status
///   5. If pending → add to approval queue
pub async fn contribute(
    group_id:  &str,
    caller_id: Uuid,
    input:     ContributionInput,
    db:        &DatabaseClient,
    graph:     &GraphClient,
    ingestion: &IngestionClient,
) -> anyhow::Result<Contribution> {
    let caller_id_str = caller_id.to_string();

    // 1. Verify membership + role
    let member = db.get_member(group_id, &caller_id_str).await?
        .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    access::can_contribute(&member.role)?;

    // 2. Determine approval routing
    let status = approval::route(&member.role);

    // 3. Process content
    let (node_id, kind) = match &input {
        ContributionInput::Text { text, source_uri } => {
            // Check for duplicates first
            let similar = graph.search_group(
                caller_id,
                text,
                group_id,
                5,
            ).await.unwrap_or_default();

            if let Some(dup) = similar.iter().find(|n| approval::is_duplicate(n.score)) {
                tracing::warn!(
                    group_id = %group_id,
                    score    = %dup.score,
                    "duplicate contribution detected"
                );
                return Ok(Contribution {
                    surreal_node_id: dup.id.clone(),
                    status:          "duplicate".to_string(),
                    queue_id:        None,
                    duplicate_of:    Some(dup.id.clone()),
                });
            }

            // Ingest the text
            let _result = ingestion.ingest_raw(
                caller_id,
                text,
                source_uri,
                "custom",
            ).await?;

            // Store collab node tagged with group metadata
            let node_id = graph.upsert_group_node(
                caller_id,
                text,
                source_uri,
                group_id,
                &caller_id_str,
                status,
                0,
            ).await?;

            (node_id, "knowledge")
        }
    };

    // 4. If pending → add to approval queue
    let queue_id = if status == approval::PENDING {
        let item = db.submit_to_queue(
            group_id,
            &caller_id_str,
            kind,
            &node_id,
        ).await?;

        tracing::info!(
            group_id = %group_id,
            node_id  = %node_id,
            queue_id = %item.id,
            "contribution submitted to approval queue"
        );

        Some(item.id)
    } else {
        tracing::info!(
            group_id = %group_id,
            node_id  = %node_id,
            role     = %member.role,
            "contribution auto-approved"
        );
        None
    };

    Ok(Contribution {
        surreal_node_id: node_id,
        status:          status.to_string(),
        queue_id,
        duplicate_of:    None,
    })
}