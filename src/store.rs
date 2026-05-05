use crate::app::{Status, TrackedTweet};
use crate::json;
use crate::velo::VeloDb;

const INDEX_KEY: &str = "followup:index:tracked_tweets";
const NEXT_ID_KEY: &str = "followup:meta:next_id";

pub trait TweetStore {
    fn next_id(&mut self) -> Result<u64, String>;
    fn add_tracked(&mut self, tweet: TrackedTweet) -> Result<TrackedTweet, String>;
    fn save_tracked(&mut self, tweet: TrackedTweet) -> Result<(), String>;
    fn get_by_tweet_id(&self, tweet_id: &str) -> Result<Option<TrackedTweet>, String>;
    fn list_tracked(&self, status: Option<Status>) -> Result<Vec<TrackedTweet>, String>;
}

pub struct VeloTweetStore {
    db: VeloDb,
}

impl VeloTweetStore {
    pub fn open(path: &str) -> Result<Self, String> {
        let db = VeloDb::open(path)?;
        let store = Self { db };
        store.ensure_meta()?;
        Ok(store)
    }

    fn ensure_meta(&self) -> Result<(), String> {
        if self.db.get(INDEX_KEY)?.is_none() {
            self.db.set(INDEX_KEY, "")?;
        }
        if self.db.get(NEXT_ID_KEY)?.is_none() {
            self.db.set(NEXT_ID_KEY, "1")?;
        }
        Ok(())
    }

    fn index(&self) -> Result<Vec<String>, String> {
        Ok(self
            .db
            .get(INDEX_KEY)?
            .unwrap_or_default()
            .split('\n')
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect())
    }

    fn save_index(&self, index: &[String]) -> Result<(), String> {
        self.db.set(INDEX_KEY, &index.join("\n"))
    }

    fn key(tweet_id: &str) -> String {
        format!("followup:tweet:{tweet_id}")
    }
}

impl TweetStore for VeloTweetStore {
    fn next_id(&mut self) -> Result<u64, String> {
        let current = self
            .db
            .get(NEXT_ID_KEY)?
            .unwrap_or_else(|| "1".to_string())
            .parse::<u64>()
            .map_err(|_| "invalid next id in Velo-Lite".to_string())?;
        self.db.set(NEXT_ID_KEY, &(current + 1).to_string())?;
        Ok(current)
    }

    fn add_tracked(&mut self, tweet: TrackedTweet) -> Result<TrackedTweet, String> {
        if let Some(existing) = self.get_by_tweet_id(&tweet.tweet_id)? {
            return Ok(existing);
        }
        self.db
            .set(&Self::key(&tweet.tweet_id), &encode_tweet(&tweet))?;
        let mut index = self.index()?;
        if !index.iter().any(|id| id == &tweet.tweet_id) {
            index.push(tweet.tweet_id.clone());
            self.save_index(&index)?;
        }
        Ok(tweet)
    }

    fn save_tracked(&mut self, tweet: TrackedTweet) -> Result<(), String> {
        self.db
            .set(&Self::key(&tweet.tweet_id), &encode_tweet(&tweet))
    }

    fn get_by_tweet_id(&self, tweet_id: &str) -> Result<Option<TrackedTweet>, String> {
        self.db
            .get(&Self::key(tweet_id))?
            .map(|raw| decode_tweet(&raw))
            .transpose()
    }

    fn list_tracked(&self, status: Option<Status>) -> Result<Vec<TrackedTweet>, String> {
        let mut tweets = Vec::new();
        for tweet_id in self.index()? {
            let Some(tweet) = self.get_by_tweet_id(&tweet_id)? else {
                continue;
            };
            if status.is_some_and(|wanted| wanted != tweet.status) {
                continue;
            }
            tweets.push(tweet);
        }
        tweets.sort_by(|a, b| a.expire_at.cmp(&b.expire_at).then(a.id.cmp(&b.id)));
        Ok(tweets)
    }
}

fn encode_tweet(tweet: &TrackedTweet) -> String {
    json::tweet(tweet)
}

fn decode_tweet(raw: &str) -> Result<TrackedTweet, String> {
    let pairs = json::parse_object(raw)?;
    let get = |name: &str| -> String {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    Ok(TrackedTweet {
        id: get("id").parse::<u64>().unwrap_or_default(),
        tweet_id: get("tweet_id"),
        author: get("author"),
        target: get("target"),
        tag: get("tag"),
        status: Status::parse(&get("status")).unwrap_or(Status::Waiting),
        text: get("text"),
        created_at: get("created_at"),
        expire_at: get("expire_at"),
        answered_at: none_if_empty(get("answered_at")),
        answer_tweet_id: none_if_empty(get("answer_tweet_id")),
        notes: none_if_empty(get("notes")),
    })
}

fn none_if_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tweet_codec_round_trips() {
        let tweet = TrackedTweet {
            id: 1,
            tweet_id: "100".to_string(),
            author: "@me".to_string(),
            target: "@you".to_string(),
            tag: "#fu7".to_string(),
            status: Status::Waiting,
            text: "@you hello #fu7".to_string(),
            created_at: "2026-05-05T00:00:00Z".to_string(),
            expire_at: "2026-05-12T00:00:00Z".to_string(),
            answered_at: None,
            answer_tweet_id: None,
            notes: None,
        };
        assert_eq!(decode_tweet(&encode_tweet(&tweet)).unwrap(), tweet);
    }
}
