use chrono::Utc;
use rusqlite::{Connection, Result, Transaction, params};

use crate::models::{CoverTextColor, CoverTextPosition, CoverTextSize, DomainOverride, EmailConfig, GeneralConfig, ProcessorType, ReadItLaterArticle, Schedule};

pub mod category_db;
pub mod feed_db;
mod migration;
pub mod schema_init;

pub fn add_schedule(
    conn: &Connection,
    cron_expression: &str,
    schedule_type: &str,
    timezone: &str,
    category_ids: &[i64],
    override_to_email: Option<&str>,
    fetch_since_hours_override: Option<i32>,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO schedules (cron_expression, active, schedule_type, timezone, override_to_email, fetch_since_hours_override, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![cron_expression, true, schedule_type, timezone, override_to_email, fetch_since_hours_override, Utc::now().to_rfc3339()],
    )?;
    let schedule_id = tx.last_insert_rowid();
    save_schedule_categories(&tx, schedule_id, category_ids)?;
    tx.commit()?;
    Ok(())
}

pub fn get_schedules(conn: &Connection) -> Result<Vec<Schedule>> {
    let mut stmt = conn.prepare(
        "SELECT id, cron_expression, active, schedule_type, timezone, override_to_email, fetch_since_hours_override FROM schedules",
    )?;
    let schedule_iter = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        Ok(Schedule {
            id: Some(id),
            cron_expression: row.get(1)?,
            active: row.get(2)?,
            schedule_type: row.get(3)?,
            timezone: row.get(4)?,
            category_ids: get_schedule_category_ids(conn, id)?,
            override_to_email: row.get(5)?,
            fetch_since_hours_override: row.get(6)?,
        })
    })?;

    let mut schedules = Vec::new();
    for schedule in schedule_iter {
        schedules.push(schedule?);
    }
    Ok(schedules)
}

pub fn update_schedule(
    conn: &Connection,
    id: i64,
    cron_expression: &str,
    schedule_type: &str,
    timezone: &str,
    category_ids: &[i64],
    override_to_email: Option<&str>,
    fetch_since_hours_override: Option<i32>,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE schedules SET cron_expression = ?1, schedule_type = ?2, timezone = ?3, override_to_email = ?4, fetch_since_hours_override = ?5, category_id = NULL WHERE id = ?6",
        params![cron_expression, schedule_type, timezone, override_to_email, fetch_since_hours_override, id],
    )?;
    save_schedule_categories(&tx, id, category_ids)?;
    tx.commit()?;
    Ok(())
}

pub fn delete_schedule(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM schedule_category WHERE schedule_id = ?1",
        params![id],
    )?;
    conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])?;
    Ok(())
}

fn get_schedule_category_ids(conn: &Connection, schedule_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT category_id FROM schedule_category WHERE schedule_id = ?1 ORDER BY category_id ASC",
    )?;
    let rows = stmt.query_map(params![schedule_id], |row| row.get(0))?;

    let mut category_ids = Vec::new();
    for row in rows {
        category_ids.push(row?);
    }
    Ok(category_ids)
}

fn save_schedule_categories(
    tx: &Transaction<'_>,
    schedule_id: i64,
    category_ids: &[i64],
) -> Result<()> {
    tx.execute(
        "DELETE FROM schedule_category WHERE schedule_id = ?1",
        params![schedule_id],
    )?;

    let mut stmt =
        tx.prepare("INSERT INTO schedule_category (schedule_id, category_id) VALUES (?1, ?2)")?;
    for category_id in category_ids {
        stmt.execute(params![schedule_id, category_id])?;
    }

    Ok(())
}

pub fn get_email_config(conn: &Connection) -> Result<Option<EmailConfig>> {
    let mut stmt = conn.prepare(
        "SELECT smtp_host, smtp_port, smtp_password, smtp_username, email_address, to_email, enable_auto_send FROM email_config WHERE id = 1",
    )?;
    let mut config_iter = stmt.query_map([], |row| {
        Ok(EmailConfig {
            smtp_host: row.get(0)?,
            smtp_port: row.get(1)?,
            smtp_password: row.get(2)?,
            smtp_username: row.get(3).unwrap_or_default(),
            email_address: row.get(4)?,
            to_email: row.get(5)?,
            enable_auto_send: row.get(6).unwrap_or(false),
        })
    })?;

    if let Some(config) = config_iter.next() {
        Ok(Some(config?))
    } else {
        Ok(None)
    }
}

pub fn save_email_config(conn: &Connection, config: &EmailConfig) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO email_config (id, smtp_host, smtp_port, smtp_password, smtp_username, email_address, to_email, enable_auto_send)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            config.smtp_host,
            config.smtp_port,
            config.smtp_password,
            config.smtp_username,
            config.email_address,
            config.to_email,
            config.enable_auto_send
        ],
    )?;
    Ok(())
}

pub fn add_read_it_later_article(conn: &Connection, url: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO read_it_later (url, created_at) VALUES (?1, ?2)",
        params![url, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn get_read_it_later_articles(
    conn: &Connection,
    unread_only: bool,
) -> Result<Vec<ReadItLaterArticle>> {
    let mut query = "SELECT id, url, read, created_at FROM read_it_later".to_string();
    if unread_only {
        query.push_str(" WHERE read = 0");
    }
    query.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&query)?;
    let article_iter = stmt.query_map([], |row| {
        Ok(ReadItLaterArticle {
            id: Some(row.get(0)?),
            url: row.get(1)?,
            read: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;

    let mut articles = Vec::new();
    for article in article_iter {
        articles.push(article?);
    }
    Ok(articles)
}

pub fn update_read_it_later_status(conn: &Connection, id: i64, read: bool) -> Result<()> {
    conn.execute(
        "UPDATE read_it_later SET read = ?1 WHERE id = ?2",
        params![read, id],
    )?;
    Ok(())
}

pub fn delete_read_it_later_article(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM read_it_later WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn mark_articles_as_read(conn: &Connection, ids: &[i64]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    // Use a transaction for better performance and atomicity
    let mut stmt = conn.prepare("UPDATE read_it_later SET read = 1 WHERE id = ?")?;

    for id in ids {
        stmt.execute(params![id])?;
    }

    Ok(())
}

pub fn get_general_config(conn: &Connection) -> Result<GeneralConfig> {
    let mut stmt = conn.prepare("SELECT fetch_since_hours, image_timeout_seconds, cover_text_enabled, cover_text_color, cover_text_position, cover_text_size FROM general_config WHERE id = 1")?;
    let mut config_iter = stmt.query_map([], |row| {
        let cover_text_color = row.get::<_, String>(3).unwrap_or_else(|_| "white".to_string());
        let cover_text_position = row
            .get::<_, String>(4)
            .unwrap_or_else(|_| "bottom-right".to_string());
        let cover_text_size = row.get::<_, String>(5).unwrap_or_else(|_| "small".to_string());

        Ok(GeneralConfig {
            fetch_since_hours: row.get(0)?,
            image_timeout_seconds: row.get(1)?,
            cover_text_enabled: row.get(2).unwrap_or(false),
            cover_text_color: CoverTextColor::from_db(&cover_text_color),
            cover_text_position: CoverTextPosition::from_db(&cover_text_position),
            cover_text_size: CoverTextSize::from_db(&cover_text_size),
        })
    })?;

    if let Some(config) = config_iter.next() {
        Ok(config?)
    } else {
        Ok(GeneralConfig {
            fetch_since_hours: 24,
            image_timeout_seconds: 45,
            cover_text_enabled: false,
            cover_text_color: CoverTextColor::default(),
            cover_text_position: CoverTextPosition::default(),
            cover_text_size: CoverTextSize::default(),
        })
    }
}

pub fn update_general_config(conn: &Connection, config: &GeneralConfig) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO general_config (id, fetch_since_hours, image_timeout_seconds, cover_text_enabled, cover_text_color, cover_text_position, cover_text_size) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
        params![config.fetch_since_hours, config.image_timeout_seconds, config.cover_text_enabled, config.cover_text_color.as_str(), config.cover_text_position.as_str(), config.cover_text_size.as_str()],
    )?;
    Ok(())
}

pub fn add_domain_override(
    conn: &Connection,
    domain: &str,
    processor: ProcessorType,
    custom_config: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT OR REPLACE INTO domain_override (domain, processor, custom_config, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![domain.to_lowercase(), processor.to_i32(), custom_config, Utc::now().to_rfc3339()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_domain_overrides(conn: &Connection) -> Result<Vec<DomainOverride>> {
    let mut stmt = conn.prepare("SELECT id, domain, processor, custom_config, created_at FROM domain_override ORDER BY created_at DESC")?;
    let iter = stmt.query_map([], |row| {
        let processor_int: i32 = row.get(2)?;
        Ok(DomainOverride {
            id: Some(row.get(0)?),
            domain: row.get(1)?,
            processor: ProcessorType::from_i32(processor_int),
            custom_config: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;

    let mut overrides = Vec::new();
    for item in iter {
        overrides.push(item?);
    }
    Ok(overrides)
}

pub fn delete_domain_override(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM domain_override WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db(conn: &Connection) {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS general_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                fetch_since_hours INTEGER NOT NULL DEFAULT 24,
                image_timeout_seconds INTEGER NOT NULL DEFAULT 45,
                cover_text_enabled BOOLEAN NOT NULL DEFAULT 0,
                cover_text_color TEXT NOT NULL DEFAULT 'white',
                cover_text_position TEXT NOT NULL DEFAULT 'bottom-right',
                cover_text_size TEXT NOT NULL DEFAULT 'small'
            )",
            [],
        )
        .unwrap();
    }

    #[test]
    fn test_update_general_config() {
        let conn = Connection::open_in_memory().unwrap();
        setup_db(&conn);

        // Initial config check
        let new_config = GeneralConfig {
            fetch_since_hours: 48,
            image_timeout_seconds: 60,
            cover_text_enabled: true,
            cover_text_color: CoverTextColor::Black,
            cover_text_position: CoverTextPosition::TopLeft,
            cover_text_size: CoverTextSize::Large,
        };

        update_general_config(&conn, &new_config).unwrap();

        let fetched_config = get_general_config(&conn).unwrap();
        assert_eq!(fetched_config.fetch_since_hours, 48);
        assert_eq!(fetched_config.image_timeout_seconds, 60);
        assert_eq!(fetched_config.cover_text_enabled, true);
        assert_eq!(fetched_config.cover_text_color, CoverTextColor::Black);
        assert_eq!(fetched_config.cover_text_position, CoverTextPosition::TopLeft);
        assert_eq!(fetched_config.cover_text_size, CoverTextSize::Large);

        // Update again
        let updated_config = GeneralConfig {
            fetch_since_hours: 12,
            image_timeout_seconds: 30,
            cover_text_enabled: false,
            cover_text_color: CoverTextColor::White,
            cover_text_position: CoverTextPosition::BottomRight,
            cover_text_size: CoverTextSize::Small,
        };
        update_general_config(&conn, &updated_config).unwrap();

        let fetched_config_2 = get_general_config(&conn).unwrap();
        assert_eq!(fetched_config_2.fetch_since_hours, 12);
        assert_eq!(fetched_config_2.image_timeout_seconds, 30);
        assert_eq!(fetched_config_2.cover_text_enabled, false);
        assert_eq!(fetched_config_2.cover_text_color, CoverTextColor::White);
        assert_eq!(fetched_config_2.cover_text_position, CoverTextPosition::BottomRight);
        assert_eq!(fetched_config_2.cover_text_size, CoverTextSize::Small);
    }
}
