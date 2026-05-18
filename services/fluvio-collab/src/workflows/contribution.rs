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
///   2. Determine approval status from role
///   3. Check for duplicates (cosine > 0.92)
///   4. Ingest via fluvio-ingestion → creates embedded node in fluvio-graph
///   5. Tag that embedded node with group_id + status metadata
///   6. If pending → add to approval queue in Postgres
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

    // 2. Determine approval routing based on role
    let status = approval::route(&member.role);

    // 3. Process content
    let (node_id, kind) = match &input {
        ContributionInput::Text { text, source_uri } => {

            // 3a. Check for duplicates in this group's approved nodes
            let similar = graph.search_group(caller_id, text, group_id, 5)
                .await
                .unwrap_or_default();

            if let Some(dup) = similar.iter().find(|n| approval::is_duplicate(n.score)) {
                tracing::warn!(
                    group_id = %group_id,
                    dup_id   = %dup.id,
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

            // 3b. Ingest via fluvio-ingestion
            //     This creates an embedded node in fluvio-graph with owner_id metadata
            ingestion.ingest_raw(
                caller_id,
                text,
                source_uri,
                "custom",
            ).await?;

            // 3c. Find that embedded node and tag it with group metadata
            //     find_and_tag_node locates the node by source_text + isEmbedded,
            //     then upserts it with group_id + status + contributed_by added
            let node_id = graph.find_and_tag_node(
                caller_id,
                text,
                group_id,
                &caller_id_str,
                status,
            ).await?;

            tracing::info!(
                group_id = %group_id,
                node_id  = %node_id,
                status   = %status,
                "contribution stored and tagged"
            );

            (node_id, "knowledge")
        }
    };

    // 4. If pending → add to approval queue in Postgres
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
            "contribution queued for owner approval"
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