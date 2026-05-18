//! Approval workflow — owner reviews pending contributions.
use uuid::Uuid;
use crate::clients::dbtypes::DbQueueItem;
use crate::clients::{DatabaseClient, GraphClient};
use crate::policy::{access, approval};

pub async fn get_pending(
    group_id: &str,
    caller_id: &str, 
    db: &DatabaseClient,
) -> anyhow::Result<Vec<DbQueueItem>> {
    let member = db.get_member(group_id, caller_id).await?
         .ok_or_else(|| anyhow::anyhow!("You are not a member of this group"))?;

    access::can_approve(&member.role)?;

    db.get_pending_queue(group_id).await
}

pub async fn approve(
    group_id: &str,
    caller_id: Uuid,
    contribution_id: &str,
    graph: &GraphClient,
    db: &DatabaseClient,
) -> anyhow::Result<DbQueueItem> {
    let caller_id_str = caller_id.to_string();

    // verify owner.
    let member = db.get_member(group_id, &caller_id_str).await?
         .ok_or_else(|| anyhow::anyhow!("You are not a member of this group"))?;

    access::can_approve(&member.role)?;

    // Get queue item to find the surrealDB nodeID.
    let pending = db.get_pending_queue(group_id).await?;
    let item = pending.iter()
        .find(|i| i.id == contribution_id)
        .ok_or_else(|| anyhow::anyhow!("Contribution not found in pending queue"))?;

    if approval::is_terminal(&item.status) {
        anyhow::bail!("Contribution is already in a terminal state");
    }

    // Update surrealDB node -> approved.
    graph.update_node_status(caller_id, &item.surreal_node_id, approval::APPROVED).await?;

    // Update postgres queue -> approved.
    let updated = db.update_queue_status(
                     contribution_id, 
             approval::APPROVED, 
        &caller_id_str, 
               None).await?;

    tracing::info!(
                group_id        = %group_id,
                contribution_id = %contribution_id,
                node_id         = %item.surreal_node_id,
                "contribution approved"
    );

    Ok(updated)
}

// Reject a pending contribution with an optional note.
pub async fn reject(
    group_id: &str,
    caller_id: Uuid,
    contribution_id: &str,
    note: Option<&str>,
    graph: &GraphClient,
    db: &DatabaseClient,
) -> anyhow::Result<DbQueueItem> {
    let caller_id_str = caller_id.to_string();

    let member = db.get_member(group_id, &caller_id_str).await?
               .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    access::can_approve(&member.role)?;

    let pending = db.get_pending_queue(group_id).await?;
    let item = pending.iter()
        .find(|i| i.id == contribution_id)
        .ok_or_else(|| anyhow::anyhow!("Contribution not found in pending queue"))?;

    if approval::is_terminal(&item.status) {
        anyhow::bail!("Contribution is already in a terminal state");
    }

    // Update surrealDB node -> rejected.
    graph.update_node_status(caller_id, &item.surreal_node_id, approval::REJECTED).await?;

    // Update postgres queue -> rejected.
    let updated = db.update_queue_status(
        contribution_id, 
        approval::REJECTED, 
        &caller_id_str, 
        note,).await?;


    Ok(updated)
}


