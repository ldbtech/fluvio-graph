//! Group lifecycle workflows.

use uuid::Uuid;
use crate::clients::{DatabaseClient, GraphClient};
use crate::clients::dbtypes::{DbGroup, DbMember};
use crate::policy::access;

/// Create a new group and add the creator as owner.
pub async fn create_group(
    caller_id:   &str,
    name:        &str,
    description: Option<&str>,
    db:          &DatabaseClient,
) -> anyhow::Result<DbGroup> {
    // Create group in Postgres
    let group = db.create_group(name, description, caller_id).await?;

    // Creator becomes first owner
    db.add_member(&group.id, caller_id, access::OWNER, None).await?;

    tracing::info!(
        group_id = %group.id,
        owner_id = %caller_id,
        name     = %name,
        "group created"
    );

    Ok(group)
}

/// Get all groups the caller belongs to.
pub async fn get_my_groups(
    caller_id: &str,
    db:        &DatabaseClient,
) -> anyhow::Result<Vec<DbGroup>> {
    db.get_user_groups(caller_id).await
}

/// Get all members of a group.
/// Caller must be a member to see the member list.
pub async fn get_group_members(
    group_id:  &str,
    caller_id: &str,
    db:        &DatabaseClient,
) -> anyhow::Result<Vec<DbMember>> {
    // Verify caller is a member
    db.get_member(group_id, caller_id).await?
        .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    db.get_group_members(group_id).await
}

/// Promote or demote a member's role.
/// Only owners can do this.
pub async fn promote_member(
    group_id:  &str,
    caller_id: &str,
    target_id: &str,
    new_role:  &str,
    db:        &DatabaseClient,
) -> anyhow::Result<DbMember> {
    // Verify caller is owner
    let caller = db.get_member(group_id, caller_id).await?
        .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    access::can_promote(&caller.role)?;

    // Get current members to count owners
    let members = db.get_group_members(group_id).await?;
    let owner_count = members.iter().filter(|m| m.role == access::OWNER).count() as i64;
    let is_self = caller_id == target_id;

    access::can_set_role(new_role, owner_count, is_self)?;

    let updated = db.update_member_role(group_id, target_id, new_role).await?;

    tracing::info!(
        group_id  = %group_id,
        target_id = %target_id,
        new_role  = %new_role,
        "member role updated"
    );

    Ok(updated)
}