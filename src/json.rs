use crate::app::{Dashboard, FollowUpDraft, TrackedTweet};

pub fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

pub fn dashboard(data: &Dashboard) -> String {
    format!(
        "{{\"counts\":{{\"WAITING\":{},\"ANSWERED\":{},\"EXPIRED\":{},\"CLOSED\":{}}},\"tweets\":[{}],\"drafts\":[{}]}}",
        data.waiting,
        data.answered,
        data.expired,
        data.closed,
        data.tweets.iter().map(tweet).collect::<Vec<_>>().join(","),
        data.drafts.iter().map(draft).collect::<Vec<_>>().join(",")
    )
}

pub fn tweet(tweet: &TrackedTweet) -> String {
    format!(
        "{{\"id\":{},\"tweet_id\":{},\"author\":{},\"target\":{},\"tag\":{},\"status\":{},\"text\":{},\"created_at\":{},\"expire_at\":{},\"answered_at\":{},\"answer_tweet_id\":{},\"notes\":{}}}",
        tweet.id,
        string(&tweet.tweet_id),
        string(&tweet.author),
        string(&tweet.target),
        string(&tweet.tag),
        string(tweet.status.as_str()),
        string(&tweet.text),
        string(&tweet.created_at),
        string(&tweet.expire_at),
        option(&tweet.answered_at),
        option(&tweet.answer_tweet_id),
        option(&tweet.notes)
    )
}

fn draft(draft: &FollowUpDraft) -> String {
    format!(
        "{{\"tweet_id\":{},\"target\":{},\"days_waited\":{},\"text\":{}}}",
        string(&draft.tweet_id),
        string(&draft.target),
        draft.days_waited,
        string(&draft.text)
    )
}

fn option(value: &Option<String>) -> String {
    value
        .as_ref()
        .map(|value| string(value))
        .unwrap_or_else(|| "null".to_string())
}

pub fn parse_object(input: &str) -> Result<Vec<(String, String)>, String> {
    let bytes = input.as_bytes();
    let mut i = skip_ws(bytes, 0);
    expect(bytes, &mut i, b'{')?;
    let mut pairs = Vec::new();
    loop {
        i = skip_ws(bytes, i);
        if peek(bytes, i) == Some(b'}') {
            break;
        }
        let key = parse_string(bytes, &mut i)?;
        i = skip_ws(bytes, i);
        expect(bytes, &mut i, b':')?;
        i = skip_ws(bytes, i);
        let value = if peek(bytes, i) == Some(b'"') {
            parse_string(bytes, &mut i)?
        } else if input[i..].starts_with("null") {
            i += 4;
            String::new()
        } else {
            let start = i;
            while let Some(ch) = peek(bytes, i) {
                if ch == b',' || ch == b'}' {
                    break;
                }
                i += 1;
            }
            input[start..i].trim().to_string()
        };
        pairs.push((key, value));
        i = skip_ws(bytes, i);
        if peek(bytes, i) == Some(b',') {
            i += 1;
            continue;
        }
        if peek(bytes, i) == Some(b'}') {
            break;
        }
        return Err("expected comma or closing brace".to_string());
    }
    Ok(pairs)
}

fn parse_string(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    expect(bytes, i, b'"')?;
    let mut out = String::new();
    while let Some(ch) = peek(bytes, *i) {
        *i += 1;
        match ch {
            b'"' => return Ok(out),
            b'\\' => {
                let escaped = peek(bytes, *i).ok_or("incomplete escape")?;
                *i += 1;
                out.push(match escaped {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'b' => '\u{08}',
                    b'f' => '\u{0c}',
                    _ => return Err("unsupported escape".to_string()),
                });
            }
            _ => out.push(ch as char),
        }
    }
    Err("unterminated string".to_string())
}

fn expect(bytes: &[u8], i: &mut usize, expected: u8) -> Result<(), String> {
    if peek(bytes, *i) == Some(expected) {
        *i += 1;
        Ok(())
    } else {
        Err(format!("expected {}", expected as char))
    }
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while matches!(peek(bytes, i), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        i += 1;
    }
    i
}

fn peek(bytes: &[u8], i: usize) -> Option<u8> {
    bytes.get(i).copied()
}
