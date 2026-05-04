use crate::domain::errors::AppError;
use sqlx::PgPool;

pub const ALERT_DELIVERY_SETTING_KEY: &str = "ENABLE_ALERT_DELIVERY";

pub fn parse_enabled_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on" | "enabled"
    )
}

pub async fn is_alert_delivery_enabled(pool: &PgPool) -> bool {
    let value = sqlx::query_scalar::<_, String>(
        "SELECT value_data FROM system_settings WHERE key_name = $1",
    )
    .bind(ALERT_DELIVERY_SETTING_KEY)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    value.as_deref().map(parse_enabled_value).unwrap_or(true)
}

pub async fn set_alert_delivery_enabled(pool: &PgPool, enabled: bool) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO system_settings (key_name, value_data)
         VALUES ($1, $2)
         ON CONFLICT (key_name)
         DO UPDATE SET
            value_data = EXCLUDED.value_data,
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(ALERT_DELIVERY_SETTING_KEY)
    .bind(if enabled { "true" } else { "false" })
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub async fn alert_delivery_status_text(pool: &PgPool) -> String {
    let enabled = is_alert_delivery_enabled(pool).await;

    if enabled {
        "🔔 <b>Alert Delivery Status</b>\n━━━━━━━━━━━━━━━━━━\nStatus: <code>ENABLED</code>\n\nMining alerts are being sent normally.\n\nNote: This does not affect block detection, DAG analysis, or database logging.".to_string()
    } else {
        "🔕 <b>Alert Delivery Status</b>\n━━━━━━━━━━━━━━━━━━\nStatus: <code>DISABLED</code>\n\nThe bot is still detecting blocks and recording events, but Telegram mining alerts are muted.\n\nNew alerts will not be sent until alert delivery is resumed.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enabled_values() {
        assert!(parse_enabled_value("true"));
        assert!(parse_enabled_value("1"));
        assert!(parse_enabled_value("enabled"));
        assert!(!parse_enabled_value("false"));
        assert!(!parse_enabled_value("0"));
    }
}
