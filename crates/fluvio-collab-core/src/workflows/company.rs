//! Company lifecycle workflows.
use crate::clients::database_client::DatabaseClient;
use crate::clients::dbtypes::{DbCompany, DbCompanyInvite, DbUser};
use uuid::Uuid;

pub fn is_valid_company_email(email: &str) -> bool {
    let email = email.trim().to_lowercase();
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let domain = parts[1];
    let banned = [
        "gmail.com", "yahoo.com", "hotmail.com", "outlook.com",
        "live.com", "aol.com", "protonmail.com", "proton.me",
        "icloud.com", "mail.com", "zoho.com"
    ];
    for b in banned.iter() {
        if domain == *b || domain.ends_with(&format!(".{}", b)) {
            return false;
        }
    }
    true
}

pub async fn link_company_email(
    db:      &DatabaseClient,
    user_id: &str,
    email:   &str,
) -> anyhow::Result<DbUser> {
    if !is_valid_company_email(email) {
        anyhow::bail!("Generic public email providers are not allowed. Please use your company or institutional email (.edu, company.com, etc.)");
    }

    db.update_user_company_email(user_id, email).await
}

pub async fn create_company(
    db:           &DatabaseClient,
    name:         &str,
    website:      &str,
    linkedin_url: &str,
    twitter_url:  Option<&str>,
    github_url:   Option<&str>,
    created_by:   &str,
) -> anyhow::Result<DbCompany> {
    // 1. Retrieve the creator's profile
    let user = db.get_user(created_by).await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    // 2. Validate email presence and domain suffix
    let comp_email = user.company_email.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Please link your company or student email address first."))?;

    if !is_valid_company_email(comp_email) {
        anyhow::bail!("Banned email domain suffix. Please update your company email to a corporate or academic address.");
    }

    // 3. Create company
    let company = db.create_company(name, website, linkedin_url, twitter_url, github_url, created_by).await?;

    // 4. Update creator's company_id
    db.update_user_company(created_by, &company.id).await?;

    // 5. Create default "General" team
    let team = db.create_team(&company.id, "General", Some("General squad for all company members.")).await?;
    db.add_team_member(&team.id, created_by, "owner").await?;

    Ok(company)
}

pub async fn accept_company_invite(
    db:          &DatabaseClient,
    invite_id:   &str,
    accepted_by: &str,
) -> anyhow::Result<DbCompanyInvite> {
    // 1. Fetch user to check email matches
    let user = db.get_user(accepted_by).await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    // 2. Accept invite in database
    let invite = db.accept_company_invite(invite_id).await?;

    // Verify invite target matches user's email
    let user_comp_email = user.company_email.as_deref().unwrap_or("");
    let user_primary_email = user.email.as_deref().unwrap_or("");

    if invite.email.to_lowercase() != user_comp_email.to_lowercase() && invite.email.to_lowercase() != user_primary_email.to_lowercase() {
        anyhow::bail!("This invitation was sent to {} which does not match your linked company email ({}) or personal email.", invite.email, user_comp_email);
    }

    // 3. Update user company_id
    db.update_user_company(accepted_by, &invite.company_id).await?;

    // 4. Find General team in company to add member to
    if let Ok(teams) = db.get_company_teams(&invite.company_id).await {
        if let Some(general_team) = teams.iter().find(|t| t.name == "General") {
            let _ = db.add_team_member(&general_team.id, accepted_by, "member").await;
        }
    }

    Ok(invite)
}
