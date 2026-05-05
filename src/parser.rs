#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFollowUp {
    pub tag: String,
    pub days: u16,
    pub target: String,
}

pub fn parse_followup(text: &str) -> Option<ParsedFollowUp> {
    let tag = find_tag(text)?;
    let target = find_mention(text)?;
    Some(ParsedFollowUp {
        tag: tag.0,
        days: tag.1,
        target,
    })
}

fn find_tag(text: &str) -> Option<(String, u16)> {
    for token in text.split_whitespace() {
        let clean = token.trim_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':' | '!' | '?'));
        let lower = clean.to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix("#fu") else {
            continue;
        };
        if !rest.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let days = if rest.is_empty() {
            7
        } else {
            rest.parse::<u16>().ok()?
        };
        if !(1..=365).contains(&days) {
            continue;
        }
        return Some((lower, days));
    }
    None
}

fn find_mention(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let clean = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '.' | ',' | ';' | ':' | '!' | '?' | '(' | ')' | '[' | ']'
            )
        });
        let Some(handle) = clean.strip_prefix('@') else {
            continue;
        };
        if (1..=15).contains(&handle.len())
            && handle
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Some(format!("@{handle}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_tag() {
        let parsed = parse_followup("@unity any update? #fu").unwrap();
        assert_eq!(parsed.days, 7);
        assert_eq!(parsed.target, "@unity");
        assert_eq!(parsed.tag, "#fu");
    }

    #[test]
    fn parses_day_tag_case_insensitive() {
        let parsed = parse_followup("@support ping #FU30").unwrap();
        assert_eq!(parsed.days, 30);
        assert_eq!(parsed.tag, "#fu30");
    }

    #[test]
    fn rejects_missing_target_or_bad_days() {
        assert!(parse_followup("no target #fu7").is_none());
        assert!(parse_followup("@x too long #fu999").is_none());
        assert!(parse_followup("@x zero #fu0").is_none());
    }
}
