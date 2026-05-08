//! auth/email.rs
//!
//! Send OTP codes via email.
//! Uses Resend API (resend.com) — free tier: 3000 emails/month.
//!
//! Set RESEND_API_KEY in .env to enable real email sending.
//! If not set — returns code in response (demo mode).

/// Send an OTP code to an email address.
/// Returns Ok(true) if email was sent, Ok(false) if demo mode.
pub async fn send_otp_email(
    email:   &str,
    code:    &str,
    api_key: Option<&str>,
) -> anyhow::Result<bool> {
    let Some(key) = api_key else {
        // Demo mode — no email sent, code returned in API response
        tracing::info!("[Auth] Demo mode — OTP for {email}: {code}");
        return Ok(false);
    };

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "from":    "Fluvio <noreply@fluvio.io>",
            "to":      [email],
            "subject": format!("{code} — your Fluvio login code"),
            "html": format!(
                r#"
                <div style="font-family:system-ui,sans-serif;max-width:400px;margin:40px auto;background:#04040c;color:#fff;border-radius:12px;padding:40px;border:1px solid rgba(255,255,255,0.08)">
                  <div style="margin-bottom:32px">
                    <span style="font-size:20px;font-weight:700;letter-spacing:2px">FLUVIO</span>
                  </div>
                  <p style="color:rgba(255,255,255,0.5);font-size:14px;margin-bottom:16px">Your login code:</p>
                  <div style="font-size:42px;font-weight:700;letter-spacing:8px;color:#00d17a;margin-bottom:24px">{code}</div>
                  <p style="color:rgba(255,255,255,0.35);font-size:12px">Expires in 10 minutes. Do not share this code.</p>
                </div>
                "#,
                code = code
            ),
        }))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("email send failed: {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let body   = res.text().await.unwrap_or_default();
        anyhow::bail!("Resend API {status}: {body}");
    }

    tracing::info!("[Auth] OTP sent to {email}");
    Ok(true)
}