use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::app::{FollowUpService, IncomingTweet, Status};
use crate::json;
use crate::store::TweetStore;

pub fn serve<S: TweetStore>(
    host: &str,
    port: u16,
    mut service: FollowUpService<S>,
    api_token: Option<String>,
) -> Result<(), String> {
    let listener = TcpListener::bind((host, port)).map_err(|err| err.to_string())?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle(&mut stream, &mut service, api_token.as_deref()) {
                    let _ = response(
                        &mut stream,
                        "500 Internal Server Error",
                        "application/json",
                        &format!("{{\"error\":{}}}", json::string(&err)),
                    );
                }
            }
            Err(err) => eprintln!("connection error: {err}"),
        }
    }
    Ok(())
}

fn handle<S: TweetStore>(
    stream: &mut TcpStream,
    service: &mut FollowUpService<S>,
    api_token: Option<&str>,
) -> Result<(), String> {
    let mut buffer = vec![0_u8; 64 * 1024];
    let read = stream.read(&mut buffer).map_err(|err| err.to_string())?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let mut parts = request.split("\r\n\r\n");
    let head = parts.next().unwrap_or_default();
    let body = parts.next().unwrap_or_default();
    let mut lines = head.lines();
    let request_line = lines.next().ok_or("empty request")?;
    let headers = lines.collect::<Vec<_>>();
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default();
    let path = request_parts.next().unwrap_or("/");

    match (method, path.split('?').next().unwrap_or(path)) {
        ("GET", "/") => {
            let dashboard = service.dashboard(None)?;
            response(
                stream,
                "200 OK",
                "text/html; charset=utf-8",
                &render_dashboard(&dashboard),
            )
        }
        ("GET", "/health") => response(stream, "200 OK", "application/json", "{\"ok\":true}"),
        ("GET", "/api/tweets") => {
            if !authorized(api_token, &headers) {
                return unauthorized(stream);
            }
            let status = query_value(path, "status").and_then(|value| Status::parse(&value));
            let body = service.dashboard_json(status)?;
            response(stream, "200 OK", "application/json", &body)
        }
        ("POST", "/api/check") => {
            if !authorized(api_token, &headers) {
                return unauthorized(stream);
            }
            let expired = service.expire_due()?;
            response(
                stream,
                "200 OK",
                "application/json",
                &format!("{{\"expired\":{expired}}}"),
            )
        }
        ("POST", "/api/ingest") => {
            if !authorized(api_token, &headers) {
                return unauthorized(stream);
            }
            let input = parse_tweet_body(body)?;
            if input.in_reply_to_tweet_id.is_some() {
                let answered = service.ingest_reply(input.clone())?;
                if answered {
                    return response(
                        stream,
                        "200 OK",
                        "application/json",
                        "{\"answered\":true,\"tracked\":null}",
                    );
                }
            }
            let tracked = service.ingest_tweet(input)?;
            let payload = tracked
                .as_ref()
                .map(|tweet| format!("{{\"tracked\":{}}}", json::tweet(tweet)))
                .unwrap_or_else(|| "{\"tracked\":null}".to_string());
            response(stream, "200 OK", "application/json", &payload)
        }
        ("POST", route) if route.starts_with("/api/tweets/") && route.ends_with("/close") => {
            if !authorized(api_token, &headers) {
                return unauthorized(stream);
            }
            let tweet_id = route
                .trim_start_matches("/api/tweets/")
                .trim_end_matches("/close");
            let closed = service.close_manually(tweet_id, None)?;
            response(
                stream,
                "200 OK",
                "application/json",
                &format!("{{\"closed\":{closed}}}"),
            )
        }
        _ => response(
            stream,
            "404 Not Found",
            "application/json",
            "{\"error\":\"not found\"}",
        ),
    }
}

fn authorized(api_token: Option<&str>, headers: &[&str]) -> bool {
    let Some(expected) = api_token else {
        return true;
    };

    headers.iter().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.eq_ignore_ascii_case("authorization") && value.trim() == format!("Bearer {expected}")
    })
}

fn unauthorized(stream: &mut TcpStream) -> Result<(), String> {
    response(
        stream,
        "401 Unauthorized",
        "application/json",
        "{\"error\":\"unauthorized\"}",
    )
}

fn response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .map_err(|err| err.to_string())
}

fn parse_tweet_body(body: &str) -> Result<IncomingTweet, String> {
    let fields = json::parse_object(body)?;
    let get = |name: &str| -> Option<String> {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .filter(|value| !value.is_empty())
    };
    Ok(IncomingTweet {
        tweet_id: get("tweet_id").ok_or("missing tweet_id")?,
        author: get("author").ok_or("missing author")?,
        text: get("text").ok_or("missing text")?,
        created_at: get("created_at").unwrap_or_else(crate::app::now_iso),
        in_reply_to_tweet_id: get("in_reply_to_tweet_id"),
    })
}

fn query_value(path: &str, name: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

fn render_dashboard(data: &crate::app::Dashboard) -> String {
    let rows = if data.tweets.is_empty() {
        "<tr><td colspan=\"6\" class=\"empty\">Henüz takip edilen tweet yok.</td></tr>".to_string()
    } else {
        data.tweets
            .iter()
            .map(|tweet| {
                format!(
                    "<tr><td><b class=\"{}\">{}</b></td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    tweet.status.as_str(),
                    tweet.status.as_str(),
                    escape(&tweet.target),
                    escape(&tweet.tag),
                    escape(&tweet.expire_at),
                    escape(&tweet.text),
                    escape(tweet.notes.as_deref().unwrap_or(""))
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    format!(
        "<!doctype html><html lang=\"tr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>FollowUp</title><style>body{{margin:0;font-family:system-ui,-apple-system,Segoe UI,sans-serif;background:#f6f8fb;color:#1f2933}}header,main{{padding:24px max(18px,5vw)}}header{{background:#fff;border-bottom:1px solid #d8dee8}}h1{{margin:0;font-size:28px}}.stats{{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin:22px 0}}.stat{{background:#fff;border:1px solid #d8dee8;border-radius:8px;padding:14px}}.stat b{{display:block;font-size:26px}}table{{width:100%;border-collapse:collapse;background:#fff;border:1px solid #d8dee8}}td,th{{padding:11px;border-bottom:1px solid #d8dee8;text-align:left;vertical-align:top}}th{{font-size:12px;color:#667085;background:#fbfcfe}}.WAITING{{color:#0f766e}}.ANSWERED,.CLOSED{{color:#087443}}.EXPIRED{{color:#b54708}}.empty{{text-align:center;color:#667085;padding:34px}}</style></head><body><header><h1>FollowUp</h1><div>#fu tag'li sorular icin Velo-Lite destekli takip paneli</div></header><main><section class=\"stats\"><div class=\"stat\"><b>{}</b>WAITING</div><div class=\"stat\"><b>{}</b>ANSWERED</div><div class=\"stat\"><b>{}</b>EXPIRED</div><div class=\"stat\"><b>{}</b>CLOSED</div></section><table><thead><tr><th>Status</th><th>Target</th><th>Tag</th><th>Expire</th><th>Tweet</th><th>Answer</th></tr></thead><tbody>{}</tbody></table></main></body></html>",
        data.waiting, data.answered, data.expired, data.closed, rows
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
