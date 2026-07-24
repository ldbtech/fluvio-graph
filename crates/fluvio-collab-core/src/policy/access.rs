//! Access control policy — pure functions, no IO, no async.
//!
//! Every workflow calls these before executing any action.
//! These are the single source of truth for what each role can do.

use anyhow::bail;

// ── Role constants ────────────────────────────────────────────────────────────

pub const OWNER:       &str = "owner";
pub const TRUSTED:     &str = "trusted";
pub const CONTRIBUTOR: &str = "contributor";
pub const VIEWER:      &str = "viewer";

// ── Access rules ──────────────────────────────────────────────────────────────

/// Can this role contribute knowledge to a group?
/// Viewers cannot contribute — they are read-only.
pub fn can_contribute(role: &str) -> anyhow::Result<()> {
    match role {
        OWNER | TRUSTED | CONTRIBUTOR => Ok(()),
        VIEWER => bail!("viewers cannot contribute — ask an owner to promote your role"),
        r => bail!("unknown role: {r}"),
    }
}

/// Can this role approve or reject contributions?
/// Only owners can review the approval queue.
pub fn can_approve(role: &str) -> anyhow::Result<()> {
    match role {
        OWNER => Ok(()),
        _ => bail!("only owners can approve or reject contributions"),
    }
}

/// Can this role invite new members?
/// Only owners can extend invitations.
pub fn can_invite(role: &str) -> anyhow::Result<()> {
    match role {
        OWNER => Ok(()),
        _ => bail!("only owners can invite members"),
    }
}

/// Can this role query the group knowledge graph?
/// All members (including viewers) can query.
pub fn can_query(role: &str) -> anyhow::Result<()> {
    match role {
        OWNER | TRUSTED | CONTRIBUTOR | VIEWER => Ok(()),
        r => bail!("unknown role: {r}"),
    }
}

/// Can this role promote another member?
/// Only owners can change roles.
pub fn can_promote(caller_role: &str) -> anyhow::Result<()> {
    match caller_role {
        OWNER => Ok(()),
        _ => bail!("only owners can promote or demote members"),
    }
}

/// Is it safe to remove this member given the current owner count?
/// A group must always have at least one owner.
pub fn can_remove_member(
    member_role:  &str,
    owner_count:  i64,
) -> anyhow::Result<()> {
    if member_role == OWNER && owner_count <= 1 {
        bail!(
            "cannot remove the last owner — \
             promote another member to owner first"
        );
    }
    Ok(())
}

/// Can the caller promote to the target role?
/// Owners cannot self-demote if they are the last owner.
pub fn can_set_role(
    new_role:    &str,
    owner_count: i64,
    is_self:     bool,
) -> anyhow::Result<()> {
    // Demoting self from owner when last owner
    if is_self && new_role != OWNER && owner_count <= 1 {
        bail!("you are the last owner — promote someone else first");
    }
    // Valid role check
    match new_role {
        OWNER | TRUSTED | CONTRIBUTOR | VIEWER => Ok(()),
        r => bail!("invalid role: {r}"),
    }
}