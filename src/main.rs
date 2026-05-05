use std::env;
use std::process;

mod app;
mod http;
mod json;
mod parser;
mod store;
mod velo;

use app::{FollowUpService, IncomingTweet, Status};
use store::VeloTweetStore;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    let db_path = env::var("FOLLOWUP_DB").unwrap_or_else(|_| "./data/followup.velo".to_string());
    let store = VeloTweetStore::open(&db_path)?;
    let mut service = FollowUpService::new(store);

    match args.remove(0).as_str() {
        "serve" => {
            let host = env::var("FOLLOWUP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port = env::var("FOLLOWUP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(8000);
            let api_token = env::var("FOLLOWUP_API_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty());
            println!("FollowUp running at http://{host}:{port}");
            http::serve(&host, port, service, api_token)
        }
        "add" => {
            let tweet = IncomingTweet {
                tweet_id: required(&args, "--tweet-id")?,
                author: required(&args, "--author")?,
                text: required(&args, "--text")?,
                created_at: optional(&args, "--created-at").unwrap_or_else(|| app::now_iso()),
                in_reply_to_tweet_id: None,
            };
            let tracked = service.ingest_tweet(tweet)?;
            println!(
                "{{\"tracked\":{}}}",
                tracked
                    .map(|tweet| json::string(&tweet.tweet_id))
                    .unwrap_or_else(|| "null".to_string())
            );
            Ok(())
        }
        "reply" => {
            let tweet = IncomingTweet {
                tweet_id: required(&args, "--tweet-id")?,
                author: required(&args, "--author")?,
                text: required(&args, "--text")?,
                created_at: optional(&args, "--created-at").unwrap_or_else(|| app::now_iso()),
                in_reply_to_tweet_id: Some(required(&args, "--in-reply-to")?),
            };
            println!("{{\"answered\":{}}}", service.ingest_reply(tweet)?);
            Ok(())
        }
        "list" => {
            let status = optional(&args, "--status").and_then(|value| Status::parse(&value));
            service.expire_due()?;
            println!("{}", service.dashboard_json(status)?);
            Ok(())
        }
        "check" => {
            println!("{{\"expired\":{}}}", service.expire_due()?);
            Ok(())
        }
        "close" => {
            let tweet_id = args
                .first()
                .cloned()
                .ok_or("missing tweet id".to_string())?;
            println!(
                "{{\"closed\":{}}}",
                service.close_manually(&tweet_id, None)?
            );
            Ok(())
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn required(args: &[String], name: &str) -> Result<String, String> {
    optional(args, name).ok_or_else(|| format!("missing {name}"))
}

fn optional(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn print_help() {
    println!(
        "FollowUp\n\nCommands:\n  serve\n  add --tweet-id ID --author @you --text TEXT [--created-at ISO]\n  reply --tweet-id ID --author @target --text TEXT --in-reply-to ID [--created-at ISO]\n  list [--status WAITING|ANSWERED|EXPIRED|CLOSED]\n  check\n  close TWEET_ID"
    );
}
