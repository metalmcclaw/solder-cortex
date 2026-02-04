use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

use crate::config::CacheConfig;
use crate::db::models::*;

/// Cache key types for different query patterns
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheKey {
    MarketTrend { slug: String, interval: String },
    VolumeProfile { slug: String },
    SearchMemory { query: String },
    Anomalies { slug: String },
}

/// Cached value wrapper
#[derive(Debug, Clone)]
pub enum CacheValue {
    MarketTrend(Vec<OhlcvRow>),
    VolumeProfile(Option<VolumeProfileRow>),
    SearchMemory(Vec<MarketSearchResult>),
    Anomalies(Vec<AnomalyRow>),
}

/// Query cache for reducing Clickhouse load on frequent queries
#[derive(Clone)]
pub struct QueryCache {
    cache: Cache<CacheKey, CacheValue>,
}

impl QueryCache {
    pub fn new(config: &CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .build();

        Self { cache }
    }

    /// Get a cached market trend result
    pub async fn get_market_trend(
        &self,
        slug: &str,
        interval: &str,
    ) -> Option<Vec<OhlcvRow>> {
        let key = CacheKey::MarketTrend {
            slug: slug.to_string(),
            interval: interval.to_string(),
        };

        self.cache.get(&key).await.and_then(|v| {
            if let CacheValue::MarketTrend(data) = v {
                Some(data)
            } else {
                None
            }
        })
    }

    /// Cache a market trend result
    pub async fn set_market_trend(
        &self,
        slug: &str,
        interval: &str,
        data: Vec<OhlcvRow>,
    ) {
        let key = CacheKey::MarketTrend {
            slug: slug.to_string(),
            interval: interval.to_string(),
        };
        self.cache.insert(key, CacheValue::MarketTrend(data)).await;
    }

    /// Get cached volume profile
    pub async fn get_volume_profile(&self, slug: &str) -> Option<Option<VolumeProfileRow>> {
        let key = CacheKey::VolumeProfile {
            slug: slug.to_string(),
        };

        self.cache.get(&key).await.and_then(|v| {
            if let CacheValue::VolumeProfile(data) = v {
                Some(data)
            } else {
                None
            }
        })
    }

    /// Cache a volume profile result
    pub async fn set_volume_profile(&self, slug: &str, data: Option<VolumeProfileRow>) {
        let key = CacheKey::VolumeProfile {
            slug: slug.to_string(),
        };
        self.cache.insert(key, CacheValue::VolumeProfile(data)).await;
    }

    /// Get cached search results
    pub async fn get_search_memory(&self, query: &str) -> Option<Vec<MarketSearchResult>> {
        let key = CacheKey::SearchMemory {
            query: query.to_lowercase(),
        };

        self.cache.get(&key).await.and_then(|v| {
            if let CacheValue::SearchMemory(data) = v {
                Some(data)
            } else {
                None
            }
        })
    }

    /// Cache search results
    pub async fn set_search_memory(&self, query: &str, data: Vec<MarketSearchResult>) {
        let key = CacheKey::SearchMemory {
            query: query.to_lowercase(),
        };
        self.cache.insert(key, CacheValue::SearchMemory(data)).await;
    }

    /// Get cached anomalies
    pub async fn get_anomalies(&self, slug: &str) -> Option<Vec<AnomalyRow>> {
        let key = CacheKey::Anomalies {
            slug: slug.to_string(),
        };

        self.cache.get(&key).await.and_then(|v| {
            if let CacheValue::Anomalies(data) = v {
                Some(data)
            } else {
                None
            }
        })
    }

    /// Cache anomaly results
    pub async fn set_anomalies(&self, slug: &str, data: Vec<AnomalyRow>) {
        let key = CacheKey::Anomalies {
            slug: slug.to_string(),
        };
        self.cache.insert(key, CacheValue::Anomalies(data)).await;
    }

    /// Invalidate cache for a specific slug (useful after data updates)
    pub async fn invalidate_slug(&self, slug: &str) {
        // Invalidate all cache entries related to this slug
        // Note: moka doesn't have a prefix-based invalidation, so we track specific keys
        self.cache
            .invalidate(&CacheKey::VolumeProfile {
                slug: slug.to_string(),
            })
            .await;
        self.cache
            .invalidate(&CacheKey::Anomalies {
                slug: slug.to_string(),
            })
            .await;

        // Invalidate common intervals for market trends
        for interval in &["1m", "5m", "15m", "1h", "4h", "24h"] {
            self.cache
                .invalidate(&CacheKey::MarketTrend {
                    slug: slug.to_string(),
                    interval: interval.to_string(),
                })
                .await;
        }
    }

    /// Get cache statistics for monitoring
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: u64,
    pub weighted_size: u64,
}

/// Shared cache instance
pub type SharedCache = Arc<QueryCache>;

pub fn create_cache(config: &CacheConfig) -> SharedCache {
    Arc::new(QueryCache::new(config))
}
