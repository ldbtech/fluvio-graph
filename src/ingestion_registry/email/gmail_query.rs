//! Gmail `q` string for inbox preview + optional sender allow-list.

/// Build `users.messages.list` search: always scoped to Inbox.
/// With an empty `focus` list, all inbox messages match. Otherwise only senders in the list
/// (full `user@domain` or domain-only `@domain.com` → `*@domain.com`).
pub fn inbox_recent_list_q(normalized_focus: &[String]) -> String {
    const BASE: &str = "in:inbox";
    if normalized_focus.is_empty() {
        return BASE.to_string();
    }
    let mut parts = Vec::new();
    for f in normalized_focus {
        if let Some(term) = gmail_from_search_term(f) {
            parts.push(term);
        }
    }
    if parts.is_empty() {
        return BASE.to_string();
    }
    format!("{BASE} ({})", parts.join(" OR "))
}

fn gmail_from_search_term(normalized: &str) -> Option<String> {
    if normalized.starts_with('@') {
        let dom = &normalized[1..];
        if dom.is_empty() {
            return None;
        }
        // Gmail: wildcard local-part for a domain
        Some(format!("from:*@{dom}"))
    } else {
        if normalized.chars().any(|c| matches!(c, ' ' | '\t' | '"' | '(' | ')')) {
            return None;
        }
        Some(format!("from:{normalized}"))
    }
}

/// Parse `user@domain` from a raw `From` / `Sender` header.
pub fn extract_email_address(from_header_value: &str) -> Option<String> {
    if let (Some(i), Some(j)) = (from_header_value.rfind('<'), from_header_value.rfind('>')) {
        if i < j {
            let inner = from_header_value[i + 1..j].trim();
            if inner.contains('@') {
                return Some(inner.to_ascii_lowercase());
            }
        }
    }
    let t = from_header_value.trim();
    if t.contains('@') && !t.contains(' ') {
        return Some(t.to_ascii_lowercase());
    }
    None
}

/// Lowercase mailbox with Gmail / GoogleMail host aliased so the same inbox compares equal.
pub fn normalize_email_identity(addr: &str) -> String {
    let s = addr.trim().to_ascii_lowercase();
    let Some((local, domain)) = s.split_once('@') else {
        return s;
    };
    let dom = if domain == "googlemail.com" {
        "gmail.com"
    } else {
        domain
    };
    format!("{local}@{dom}")
}

/// Whether two mailbox strings refer to the same inbox (parses angles, Gmail alias hosts).
pub fn email_identities_equivalent(addr_a: &str, addr_b: &str) -> bool {
    let ea = extract_email_address(addr_a)
        .unwrap_or_else(|| addr_a.trim().to_ascii_lowercase());
    let eb = extract_email_address(addr_b)
        .unwrap_or_else(|| addr_b.trim().to_ascii_lowercase());
    normalize_email_identity(&ea) == normalize_email_identity(&eb)
}

/// When `focus` is empty, any sender matches. Otherwise the parsed address must match an entry.
pub fn from_header_matches_focus(from_header: Option<&str>, normalized_focus: &[String]) -> bool {
    if normalized_focus.is_empty() {
        return true;
    }
    let Some(h) = from_header.map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    let Some(addr) = extract_email_address(h) else {
        return false;
    };
    for f in normalized_focus {
        if f.starts_with('@') {
            let dom = &f[1..];
            if addr.ends_with(&format!("@{dom}")) {
                return true;
            }
        } else if addr == *f {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_q_all_when_no_focus() {
        assert_eq!(inbox_recent_list_q(&[]), "in:inbox");
    }

    #[test]
    fn inbox_q_or_from_terms() {
        let f = vec!["a@b.com".to_string(), "@c.org".to_string()];
        let q = inbox_recent_list_q(&f);
        assert!(q.starts_with("in:inbox ("));
        assert!(q.contains("from:a@b.com"));
        assert!(q.contains("from:*@c.org"));
        assert!(q.contains(" OR "));
    }

    #[test]
    fn matches_focus_domain() {
        let focus = vec!["@acme.com".to_string()];
        assert!(from_header_matches_focus(Some("Bot <x@acme.com>"), &focus));
        assert!(!from_header_matches_focus(Some("Other <y@nope.net>"), &focus));
    }

    #[test]
    fn matches_focus_exact() {
        let focus = vec!["alice@z.com".to_string()];
        assert!(from_header_matches_focus(Some("Alice <alice@z.com>"), &focus));
    }

    #[test]
    fn gmail_googlemail_normalize_equal() {
        assert!(email_identities_equivalent("a@gmail.com", "a@googlemail.com"));
        assert!(email_identities_equivalent("Bob <x@googlemail.com>", "x@gmail.com"));
        assert_eq!(normalize_email_identity("x@GoogleMail.Com"), "x@gmail.com");
    }
}
