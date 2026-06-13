use poise::serenity_prelude as serenity;
use redis::AsyncCommands;

/// Optional Redis-backed cache of author display data, keyed by author id (`d:{user_id}`).
#[derive(Clone)]
pub struct AuthorCache {
    conn: Option<redis::aio::ConnectionManager>,
}

impl AuthorCache {
    /// Connect if a URL is provided; otherwise return a no-op cache.
    pub async fn connect(url: Option<&str>) -> Self {
        let conn = match url {
            Some(url) => match redis::Client::open(url) {
                Ok(client) => match client.get_connection_manager().await {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("redis: failed to connect ({e}); author cache disabled");
                        None
                    }
                },
                Err(e) => {
                    eprintln!("redis: invalid REDIS_URL ({e}); author cache disabled");
                    None
                }
            },
            None => None,
        };
        AuthorCache { conn }
    }

    /// Cache a user's username/nickname/avatar. Silently no-ops when Redis is unavailable.
    pub async fn store(&self, user: &serenity::User, nick: Option<&str>) {
        let Some(mut conn) = self.conn.clone() else {
            return;
        };
        let key = format!("author:d:{}", user.id);
        let fields = vec![
            ("username", user.name.clone()),
            ("nickname", nick.unwrap_or(&user.name).to_string()),
            ("avatar", user.face()),
        ];
        if let Err(e) = conn.hset_multiple::<_, _, _, ()>(&key, &fields).await {
            eprintln!("redis: failed to cache {key} ({e})");
        }
    }
}
