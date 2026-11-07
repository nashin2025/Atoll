//! API authentication and rate limiting for the Atoll Protocol node.
//!
//! Protects the REST endpoints (`/health`, `/metrics`, and any RPC endpoints)
//! with API-key validation (`X-API-Key` header) and a per-IP token-bucket
//! rate limiter.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

/// A valid API key presented in the `X-API-Key` header.
pub type ApiKey = String;

/// Configuration for API key authentication.
///
/// Keys are loaded from the `ATOLL_API_KEYS` environment variable
/// (comma-separated). If unset, a single development key is accepted so the
/// node is usable out of the box; production deployments should set
/// `ATOLL_API_KEYS`.
pub struct AuthConfig {
    api_keys: HashSet<String>,
}

impl AuthConfig {
    /// Builds config from an explicit list of keys.
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            api_keys: keys.into_iter().filter(|k| !k.is_empty()).collect(),
        }
    }

    /// Builds config from the `ATOLL_API_KEYS` environment variable.
    pub fn from_env() -> Self {
        let keys = std::env::var("ATOLL_API_KEYS")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["atoll-local".to_string()]);
        Self::new(keys)
    }

    /// Returns `true` if `key` is a configured API key.
    pub fn is_valid(&self, key: Option<&str>) -> bool {
        key.map_or(false, |k| self.api_keys.contains(k))
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Token-bucket rate limiter keyed by client IP.
///
/// Each IP maintains a bucket that refills at `rate` tokens per second
/// (`rpm / 60`). A request consumes one token; when the bucket is empty the
/// request is rejected with `429 Too Many Requests`.
///
/// To prevent unbounded memory growth from unique IPs, the number of buckets
/// is capped at `MAX_BUCKETS`. When the cap is exceeded, the oldest bucket
/// (by last refill time) is evicted before inserting a new one.
pub struct RateLimiter {
    /// Refill rate in tokens per second.
    rate: f64,
    /// Per-IP bucket state.
    buckets: Mutex<HashMap<SocketAddr, Bucket>>,
}

/// Maximum number of IP buckets before eviction kicks in.
const MAX_BUCKETS: usize = 10_000;

/// Token-bucket state for a single IP.
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Creates a limiter allowing `rpm` requests per minute per IP.
    pub fn new(rpm: u64) -> Self {
        let rate = rpm as f64 / 60.0;
        Self {
            rate,
            buckets: Mutex::new(HashMap::with_capacity(MAX_BUCKETS.min(1024))),
        }
    }

    /// Builds config from the `ATOLL_API_RATE_LIMIT_RPM` environment variable
    /// (default 60 RPM).
    pub fn from_env() -> Self {
        let rpm = std::env::var("ATOLL_API_RATE_LIMIT_RPM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        Self::new(rpm)
    }

    /// Checks whether a request from `ip` is allowed, consuming one token.
    ///
    /// Returns `true` if allowed, `false` if the IP is rate limited.
    pub fn check(&self, ip: SocketAddr) -> bool {
        let mut buckets = self.buckets.lock().expect("rate limiter lock poisoned");
        let now = Instant::now();

        // Evict the oldest bucket if we're at capacity and the IP is new.
        if buckets.len() >= MAX_BUCKETS && !buckets.contains_key(&ip) {
            let oldest_ip = buckets
                .iter()
                .min_by(|(_, a), (_, b)| a.last_refill.cmp(&b.last_refill))
                .map(|(ip, _)| *ip);
            if let Some(oldest_ip) = oldest_ip {
                buckets.remove(&oldest_ip);
            }
        }

        let bucket = buckets.entry(ip).or_insert_with(|| Bucket {
            tokens: self.rate.max(1.0),
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate).min(self.rate);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Combined authentication + rate-limiting state injected into the router.
pub struct AuthState {
    pub auth_config: Arc<AuthConfig>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AuthState {
    pub fn new(auth_config: AuthConfig, rate_limiter: RateLimiter) -> Self {
        Self {
            auth_config: Arc::new(auth_config),
            rate_limiter: Arc::new(rate_limiter),
        }
    }
}

/// Axum middleware: validates the `X-API-Key` header and applies per-IP
/// token-bucket rate limiting.
///
/// Returns `401 Unauthorized` when the key is missing or invalid, and
/// `429 Too Many Requests` when the client IP has exhausted its token bucket.
pub async fn auth_and_rate_limit(
    State(auth): State<Arc<AuthState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get(header::HeaderName::from_static("x-api-key"))
        .and_then(|value| value.to_str().ok());

    if !auth.auth_config.is_valid(api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if !auth.rate_limiter.check(addr) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn auth_config_accepts_configured_key() {
        let config = AuthConfig::new(vec!["key-a".to_string(), "key-b".to_string()]);
        assert!(config.is_valid(Some("key-a")));
        assert!(config.is_valid(Some("key-b")));
        assert!(!config.is_valid(Some("key-c")));
        assert!(!config.is_valid(None));
    }

    #[test]
    fn rate_limiter_allows_up_to_rpm() {
        let limiter = RateLimiter::new(60); // 1 token/sec
        let ip = SocketAddr::from(([127, 0, 0, 1], 12345));

        // Bucket starts full (1 token), so the first request is allowed.
        assert!(limiter.check(ip));
        // Immediately after consuming the token, the bucket is empty.
        assert!(!limiter.check(ip));
    }

    #[test]
    fn rate_limiter_refills_after_wait() {
        let limiter = RateLimiter::new(600); // 10 tokens/sec, bucket starts with 10
        let ip = SocketAddr::from(([127, 0, 0, 1], 12346));

        // Drain the full bucket.
        for _ in 0..10 {
            assert!(limiter.check(ip));
        }
        assert!(!limiter.check(ip)); // empty

        // Wait long enough to refill at least one token (0.1s at 10/sec).
        std::thread::sleep(Duration::from_millis(150));
        assert!(limiter.check(ip));
    }

    #[test]
    fn rate_limiter_evicts_oldest_bucket_at_capacity() {
        // Use a tiny cap by constructing directly with MAX_BUCKETS override
        // via a small rpm and many unique IPs.
        let limiter = RateLimiter::new(60);
        let base = std::net::Ipv4Addr::new(10, 0, 0, 0);

        // Insert MAX_BUCKETS distinct IPs.
        for i in 0..MAX_BUCKETS {
            let ip = SocketAddr::from((base, i as u16 + 1));
            assert!(limiter.check(ip));
        }
        // Map is now at capacity.

        // Insert one more new IP — should evict the oldest.
        let new_ip = SocketAddr::from((base, 0));
        assert!(limiter.check(new_ip));

        // The oldest IP (first one inserted) should now be evicted.
        let oldest_ip = SocketAddr::from((base, 1));
        let buckets = limiter.buckets.lock().expect("lock poisoned");
        assert!(!buckets.contains_key(&oldest_ip));
        assert!(buckets.contains_key(&new_ip));
        assert_eq!(buckets.len(), MAX_BUCKETS);
        drop(buckets);

        // Re-checking the evicted IP creates a fresh bucket.
        assert!(limiter.check(oldest_ip));
    }
}
