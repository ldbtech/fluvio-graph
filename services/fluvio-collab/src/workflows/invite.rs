//! Invite flow workflows.

use crate::clients::DatabaseClient;
use crate::clients::dbtypes::{DbInvite, DbMember};
use crate::policy::access;

/// Create an invite token for a new member.
/// Caller must be an owner.
pub async fn invite(
    group_id:   &str,
    caller_id:  &str,
    email:      Option<&str>,
    role:       &str,
    db:         &DatabaseClient,
) -> anyhow::Result<DbInvite> {
    // Verify caller is owner
    let caller = db.get_member(group_id, caller_id).await?
        .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    access::can_invite(&caller.role)?;

    // Validate target role
    match role {
        access::OWNER | access::TRUSTED |
        access::CONTRIBUTOR | access::VIEWER => {}
        r => anyhow::bail!("invalid role: {r}"),
    }

    let invite = db.create_invite(group_id, caller_id, role, email).await?;

    tracing::info!(
        group_id = %group_id,
        role     = %role,
        email    = ?email,
        token    = %invite.token,
        "invite created"
    );

    Ok(invite)
}

/// Accept an invite and join the group.
/// Validates token is not expired or already used.
pub async fn accept_invite(
    token:     &str,
    user_id:   &str,
    db:        &DatabaseClient,
) -> anyhow::Result<DbMember> {
    // Validate token exists
    let invite = db.get_invite_by_token(token).await?
        .ok_or_else(|| anyhow::anyhow!("invite not found or expired"))?;

    // Mark token as accepted + create membership in one transaction
    db.accept_invite(token, user_id).await?;

    let member = db.add_member(
        &invite.group_id,
        user_id,
        &invite.role,
        None,
    ).await?;

    tracing::info!(
        group_id = %invite.group_id,
        user_id  = %user_id,
        role     = %invite.role,
        "invite accepted"
    );

    Ok(member)
}