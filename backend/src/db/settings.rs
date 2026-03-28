use crate::models::FeatureFlag;
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SettingsRepository {
    pool: SqlitePool,
}

// SQLite-native row type (bool stored as INTEGER)
#[derive(sqlx::FromRow)]
struct FlagRow {
    key: String,
    enabled: i64,
    updated_at: String,
}

impl From<FlagRow> for FeatureFlag {
    fn from(r: FlagRow) -> Self {
        FeatureFlag {
            key: r.key,
            enabled: r.enabled != 0,
            updated_at: r.updated_at,
        }
    }
}

impl SettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<Vec<FeatureFlag>> {
        let rows = sqlx::query_as::<_, FlagRow>(
            "SELECT key, enabled, updated_at FROM feature_flags ORDER BY key",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_flag(&self, key: &str) -> Result<bool> {
        let row = sqlx::query_as::<_, (i64,)>("SELECT enabled FROM feature_flags WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(v,)| v != 0).unwrap_or(false))
    }

    pub async fn set_flag(&self, key: &str, enabled: bool) -> Result<FeatureFlag> {
        let enabled_int: i64 = if enabled { 1 } else { 0 };
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO feature_flags (key, enabled, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET enabled = excluded.enabled, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(enabled_int)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(FeatureFlag {
            key: key.to_string(),
            enabled,
            updated_at: now,
        })
    }
}
