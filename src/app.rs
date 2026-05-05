use std::time::{SystemTime, UNIX_EPOCH};

use crate::json;
use crate::parser::parse_followup;
use crate::store::TweetStore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Answered,
    Expired,
    Closed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "WAITING",
            Self::Answered => "ANSWERED",
            Self::Expired => "EXPIRED",
            Self::Closed => "CLOSED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "WAITING" => Some(Self::Waiting),
            "ANSWERED" => Some(Self::Answered),
            "EXPIRED" => Some(Self::Expired),
            "CLOSED" => Some(Self::Closed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncomingTweet {
    pub tweet_id: String,
    pub author: String,
    pub text: String,
    pub created_at: String,
    pub in_reply_to_tweet_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedTweet {
    pub id: u64,
    pub tweet_id: String,
    pub author: String,
    pub target: String,
    pub tag: String,
    pub status: Status,
    pub text: String,
    pub created_at: String,
    pub expire_at: String,
    pub answered_at: Option<String>,
    pub answer_tweet_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FollowUpDraft {
    pub tweet_id: String,
    pub target: String,
    pub days_waited: u64,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Dashboard {
    pub waiting: usize,
    pub answered: usize,
    pub expired: usize,
    pub closed: usize,
    pub tweets: Vec<TrackedTweet>,
    pub drafts: Vec<FollowUpDraft>,
}

pub struct FollowUpService<S: TweetStore> {
    store: S,
}

impl<S: TweetStore> FollowUpService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn ingest_tweet(&mut self, tweet: IncomingTweet) -> Result<Option<TrackedTweet>, String> {
        let parsed = match parse_followup(&tweet.text) {
            Some(parsed) => parsed,
            None => return Ok(None),
        };
        let tracked = TrackedTweet {
            id: self.store.next_id()?,
            tweet_id: tweet.tweet_id,
            author: normalize_handle(&tweet.author),
            target: parsed.target,
            tag: parsed.tag,
            status: Status::Waiting,
            text: tweet.text,
            expire_at: add_days_iso(&tweet.created_at, parsed.days as u64),
            created_at: tweet.created_at,
            answered_at: None,
            answer_tweet_id: None,
            notes: None,
        };
        self.store.add_tracked(tracked).map(Some)
    }

    pub fn ingest_reply(&mut self, tweet: IncomingTweet) -> Result<bool, String> {
        let Some(parent_id) = &tweet.in_reply_to_tweet_id else {
            return Ok(false);
        };
        let Some(mut tracked) = self.store.get_by_tweet_id(parent_id)? else {
            return Ok(false);
        };
        if tracked.status != Status::Waiting {
            return Ok(false);
        }
        let author = normalize_handle(&tweet.author);
        if !author.eq_ignore_ascii_case(&tracked.target) {
            return Ok(false);
        }
        tracked.status = Status::Answered;
        tracked.answered_at = Some(tweet.created_at);
        tracked.answer_tweet_id = Some(tweet.tweet_id);
        tracked.notes = Some(format!("Answered by {author}"));
        self.store.save_tracked(tracked)?;
        Ok(true)
    }

    pub fn expire_due(&mut self) -> Result<usize, String> {
        let now = now_epoch();
        let mut expired = 0;
        for mut tweet in self.store.list_tracked(Some(Status::Waiting))? {
            if iso_to_epoch(&tweet.expire_at) <= now {
                tweet.status = Status::Expired;
                self.store.save_tracked(tweet)?;
                expired += 1;
            }
        }
        Ok(expired)
    }

    pub fn close_manually(
        &mut self,
        tweet_id: &str,
        notes: Option<String>,
    ) -> Result<bool, String> {
        let Some(mut tweet) = self.store.get_by_tweet_id(tweet_id)? else {
            return Ok(false);
        };
        if tweet.status != Status::Waiting {
            return Ok(false);
        }
        tweet.status = Status::Closed;
        tweet.answered_at = Some(now_iso());
        tweet.notes = Some(notes.unwrap_or_else(|| "Closed manually".to_string()));
        self.store.save_tracked(tweet)?;
        Ok(true)
    }

    pub fn dashboard(&mut self, status: Option<Status>) -> Result<Dashboard, String> {
        self.expire_due()?;
        let tweets = self.store.list_tracked(status)?;
        let all = self.store.list_tracked(None)?;
        let mut dashboard = Dashboard {
            waiting: all
                .iter()
                .filter(|tweet| tweet.status == Status::Waiting)
                .count(),
            answered: all
                .iter()
                .filter(|tweet| tweet.status == Status::Answered)
                .count(),
            expired: all
                .iter()
                .filter(|tweet| tweet.status == Status::Expired)
                .count(),
            closed: all
                .iter()
                .filter(|tweet| tweet.status == Status::Closed)
                .count(),
            tweets,
            drafts: Vec::new(),
        };
        dashboard.drafts = dashboard
            .tweets
            .iter()
            .filter(|tweet| tweet.status == Status::Expired)
            .map(|tweet| FollowUpDraft {
                tweet_id: tweet.tweet_id.clone(),
                target: tweet.target.clone(),
                days_waited: now_epoch().saturating_sub(iso_to_epoch(&tweet.created_at)) / 86_400,
                text: format!("{} bu konu hakkında bir güncelleme var mı?", tweet.target),
            })
            .collect();
        Ok(dashboard)
    }

    pub fn dashboard_json(&mut self, status: Option<Status>) -> Result<String, String> {
        Ok(json::dashboard(&self.dashboard(status)?))
    }
}

pub fn normalize_handle(handle: &str) -> String {
    let trimmed = handle.trim();
    if trimmed.starts_with('@') {
        trimmed.to_string()
    } else {
        format!("@{trimmed}")
    }
}

pub fn now_iso() -> String {
    epoch_to_iso(now_epoch())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn add_days_iso(iso: &str, days: u64) -> String {
    epoch_to_iso(iso_to_epoch(iso).saturating_add(days * 86_400))
}

pub fn iso_to_epoch(iso: &str) -> u64 {
    let date = iso.get(0..10).unwrap_or("1970-01-01");
    let time = iso.get(11..19).unwrap_or("00:00:00");
    let year = date
        .get(0..4)
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1970);
    let month = date
        .get(5..7)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let day = date
        .get(8..10)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let hour = time
        .get(0..2)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let minute = time
        .get(3..5)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let second = time
        .get(6..8)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    days_from_civil(year, month, day).saturating_mul(86_400) + hour * 3600 + minute * 60 + second
}

fn epoch_to_iso(epoch: u64) -> String {
    let days = (epoch / 86_400) as i64;
    let secs = epoch % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn days_from_civil(year: i32, month: u32, day: u32) -> u64 {
    let mut y = year as i64;
    let m = month as i64;
    let d = day as i64;
    y -= (m <= 2) as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468).max(0) as u64
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    ((y + (m <= 2) as i64) as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TweetStore;

    #[derive(Default)]
    struct MemoryStore {
        next: u64,
        tweets: Vec<TrackedTweet>,
    }

    impl TweetStore for MemoryStore {
        fn next_id(&mut self) -> Result<u64, String> {
            self.next += 1;
            Ok(self.next)
        }

        fn add_tracked(&mut self, tweet: TrackedTweet) -> Result<TrackedTweet, String> {
            self.tweets.push(tweet.clone());
            Ok(tweet)
        }

        fn save_tracked(&mut self, tweet: TrackedTweet) -> Result<(), String> {
            if let Some(existing) = self
                .tweets
                .iter_mut()
                .find(|item| item.tweet_id == tweet.tweet_id)
            {
                *existing = tweet;
            }
            Ok(())
        }

        fn get_by_tweet_id(&self, tweet_id: &str) -> Result<Option<TrackedTweet>, String> {
            Ok(self
                .tweets
                .iter()
                .find(|tweet| tweet.tweet_id == tweet_id)
                .cloned())
        }

        fn list_tracked(&self, status: Option<Status>) -> Result<Vec<TrackedTweet>, String> {
            Ok(self
                .tweets
                .iter()
                .filter(|tweet| status.is_none_or(|wanted| tweet.status == wanted))
                .cloned()
                .collect())
        }
    }

    #[test]
    fn date_round_trip_and_add_days() {
        assert_eq!(iso_to_epoch("1970-01-01T00:00:00Z"), 0);
        assert_eq!(epoch_to_iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            add_days_iso("2026-05-05T10:00:00Z", 7),
            "2026-05-12T10:00:00Z"
        );
    }

    #[test]
    fn tracks_and_marks_answered() {
        let mut service = FollowUpService::new(MemoryStore::default());
        let tracked = service
            .ingest_tweet(IncomingTweet {
                tweet_id: "1".to_string(),
                author: "@me".to_string(),
                text: "@unity update? #fu30".to_string(),
                created_at: "2026-05-05T00:00:00Z".to_string(),
                in_reply_to_tweet_id: None,
            })
            .unwrap()
            .unwrap();
        assert_eq!(tracked.target, "@unity");
        assert_eq!(tracked.expire_at, "2026-06-04T00:00:00Z");

        assert!(service
            .ingest_reply(IncomingTweet {
                tweet_id: "2".to_string(),
                author: "@unity".to_string(),
                text: "yes".to_string(),
                created_at: "2026-05-06T00:00:00Z".to_string(),
                in_reply_to_tweet_id: Some("1".to_string()),
            })
            .unwrap());

        let dashboard = service.dashboard(None).unwrap();
        assert_eq!(dashboard.answered, 1);
        assert_eq!(dashboard.waiting, 0);
    }

    #[test]
    fn expires_old_waiting_tweets() {
        let mut service = FollowUpService::new(MemoryStore::default());
        service
            .ingest_tweet(IncomingTweet {
                tweet_id: "old".to_string(),
                author: "@me".to_string(),
                text: "@company stale #fu1".to_string(),
                created_at: "2020-01-01T00:00:00Z".to_string(),
                in_reply_to_tweet_id: None,
            })
            .unwrap();
        assert_eq!(service.expire_due().unwrap(), 1);
        let dashboard = service.dashboard(None).unwrap();
        assert_eq!(dashboard.expired, 1);
        assert_eq!(dashboard.drafts.len(), 1);
    }
}
