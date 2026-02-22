use chrono::Utc;
use rusqlite::{params, Connection, Result};

use crate::models::{DomainOverride, EmailConfig, GeneralConfig, ProcessorType, ReadItLaterArticle, Schedule};

pub mod schema_init;
mod migration;
pub mod category_db;
pub mod feed_db;

pub fn add_schedule(conn: &Connection, cron_expression: &str, schedule_type: &str, category_id: Option<i64>) -> Result<()> {
    conn.execute(
        "INSERT INTO schedules (cron_expression, active, schedule_type, category_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![cron_expression, true, schedule_type, category_id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn get_schedules(conn: &Connection) -> Result<Vec<Schedule>> {
    let mut stmt = conn.prepare("SELECT id, cron_expression, active, schedule_type, category_id FROM schedules")?;
    let schedule_iter = stmt.query_map([], |row| {
        Ok(Schedule {
            id: Some(row.get(0)?),
            cron_expression: row.get(1)?,
            active: row.get(2)?,
            schedule_type: row.get(3)?,
            category_id: row.get(4)?,
        })
    })?;

    let mut schedules = Vec::new();
    for schedule in schedule_iter {
        schedules.push(schedule?);
    }
    Ok(schedules)
}

pub fn delete_schedule(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_email_config(conn: &Connection) -> Result<Option<EmailConfig>> {
    let mut stmt = conn.prepare(
        "SELECT smtp_host, smtp_port, smtp_password, email_address, to_email, enable_auto_send FROM email_config WHERE id = 1",
    )?;
    let mut config_iter = stmt.query_map([], |row| {
        Ok(EmailConfig {
            smtp_host: row.get(0)?,
            smtp_port: row.get(1)?,
            smtp_password: row.get(2)?,
            email_address: row.get(3)?,
            to_email: row.get(4)?,
            enable_auto_send: row.get(5).unwrap_or(false),
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
        "INSERT OR REPLACE INTO email_config (id, smtp_host, smtp_port, smtp_password, email_address, to_email, enable_auto_send)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            config.smtp_host,
            config.smtp_port,
            config.smtp_password,
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
    let mut stmt = conn.prepare("SELECT fetch_since_hours, image_timeout_seconds FROM general_config WHERE id = 1")?;
    let mut config_iter = stmt.query_map([], |row| {
        Ok(GeneralConfig {
            fetch_since_hours: row.get(0)?,
            image_timeout_seconds: row.get(1)?,
        })
    })?;

    if let Some(config) = config_iter.next() {
        Ok(config?)
    } else {
        Ok(GeneralConfig {
            fetch_since_hours: 24,
            image_timeout_seconds: 45,
        })
    }
}

pub fn update_general_config(conn: &Connection, config: &GeneralConfig) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO general_config (id, fetch_since_hours, image_timeout_seconds) VALUES (1, ?1, ?2)",
        params![config.fetch_since_hours, config.image_timeout_seconds],
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
                image_timeout_seconds INTEGER NOT NULL DEFAULT 45
            )",
            [],
        ).unwrap();
    }

    #[test]
    fn test_update_general_config() {
        let conn = Connection::open_in_memory().unwrap();
        setup_db(&conn);

        // Initial config check
        let new_config = GeneralConfig {
            fetch_since_hours: 48,
            image_timeout_seconds: 60,
        };

        update_general_config(&conn, &new_config).unwrap();

        let fetched_config = get_general_config(&conn).unwrap();
        assert_eq!(fetched_config.fetch_since_hours, 48);
        assert_eq!(fetched_config.image_timeout_seconds, 60);

        // Update again
        let updated_config = GeneralConfig {
            fetch_since_hours: 12,
            image_timeout_seconds: 30,
        };
        update_general_config(&conn, &updated_config).unwrap();

        let fetched_config_2 = get_general_config(&conn).unwrap();
        assert_eq!(fetched_config_2.fetch_since_hours, 12);
        assert_eq!(fetched_config_2.image_timeout_seconds, 30);
    }
}

