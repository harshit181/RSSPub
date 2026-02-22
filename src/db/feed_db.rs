use chrono::Utc;
use rusqlite::{params, Connection};
use crate::models::{ContentProcessor, Feed, FeedPosition, ProcessorType};

pub fn add_feed(
    conn: &Connection,
    url: &str,
    name: Option<&str>,
    concurrency_limit: usize,
) -> rusqlite::Result<i64> {
    let next_position: i64 = conn
        .query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM feeds", [], |row| row.get(0))
        .unwrap_or(0);

    conn.execute(
        "INSERT OR IGNORE INTO feeds (url, name, concurrency_limit, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![url, name, concurrency_limit, next_position, Utc::now().to_rfc3339()],
    )?;
    let x = conn.last_insert_rowid();
    Ok(x)
}

pub fn update_feed(
    conn: &Connection,
    id: i64,
    url: &str,
    name: Option<&str>,
    concurrency_limit: usize,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE feeds SET url = ?1, name = ?2, concurrency_limit = ?3 WHERE id = ?4",
        params![url, name, concurrency_limit, id],
    )?;
    Ok(())
}

pub fn get_feeds(conn: &Connection) -> rusqlite::Result<Vec<Feed>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.url, f.name, f.concurrency_limit, f.position, fp.processor, fp.custom_config,
                fc.category_id, c.name
         FROM feeds f
         LEFT JOIN feed_processor fp ON f.id = fp.feed_id
         LEFT JOIN feed_category fc ON f.id = fc.feed_id
         LEFT JOIN categories c ON fc.category_id = c.id
         ORDER BY f.position ASC"
    )?;
    let feed_iter = stmt.query_map([], |row| {
        let feed_id: i64 = row.get(0)?;
        let processor_int: Option<i32> = row.get(5)?;
        let processor = processor_int
            .map(ProcessorType::from_i32)
            .unwrap_or(ProcessorType::Default);
        let custom_config: Option<String> = row.get(6)?;

        let category_id: Option<i64> = row.get(7)?;
        let category: Option<String> = row.get(8)?;

        Ok(Feed {
            id: Some(feed_id),
            url: row.get(1)?,
            name: row.get(2)?,
            concurrency_limit: row.get(3)?,
            position: row.get(4)?,
            feed_processor: ContentProcessor {
                id: Some(feed_id),
                processor,
                custom_config,
            },
            category_id,
            category,
        })
    })?;

    let mut feeds = Vec::new();
    for feed in feed_iter {
        feeds.push(feed?);
    }
    Ok(feeds)
}

pub fn get_feeds_by_category(conn: &Connection, search_cat_id: i64) -> rusqlite::Result<Vec<Feed>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.url, f.name, f.concurrency_limit, f.position, fp.processor, fp.custom_config,
                fc.category_id, c.name
         FROM feeds f
         LEFT JOIN feed_processor fp ON f.id = fp.feed_id
         JOIN feed_category fc ON f.id = fc.feed_id
         LEFT JOIN categories c ON fc.category_id = c.id
         WHERE fc.category_id = ?1
         ORDER BY f.position ASC"
    )?;
    let feed_iter = stmt.query_map(params![search_cat_id], |row| {
        let feed_id: i64 = row.get(0)?;
        let processor_int: Option<i32> = row.get(5)?;
        let processor = processor_int
            .map(ProcessorType::from_i32)
            .unwrap_or(ProcessorType::Default);
        let custom_config: Option<String> = row.get(6)?;

        let category_id: Option<i64> = row.get(7)?;
        let category: Option<String> = row.get(8)?;

        Ok(Feed {
            id: Some(feed_id),
            url: row.get(1)?,
            name: row.get(2)?,
            concurrency_limit: row.get(3)?,
            position: row.get(4)?,
            feed_processor: ContentProcessor {
                id: Some(feed_id),
                processor,
                custom_config,
            },
            category_id,
            category,
        })
    })?;

    let mut feeds = Vec::new();
    for feed in feed_iter {
        feeds.push(feed?);
    }
    Ok(feeds)
}

pub fn reorder_feeds(conn: &Connection, feed_positions: &Vec<FeedPosition>) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("UPDATE feeds SET position = ?1 WHERE id = ?2")?;
    for x in feed_positions {
        stmt.execute(params![x.position, x.id])?;
    }
    Ok(())
}

pub fn delete_feed(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM feeds WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn save_feed_processor(
    conn: &Connection,
    feed_id: i64,
    processor: ProcessorType,
    custom_config: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO feed_processor (feed_id, processor, custom_config) VALUES (?1, ?2, ?3)",
        params![feed_id, processor.to_i32(), custom_config],
    )?;
    Ok(())
}

pub fn delete_feed_processor(conn: &Connection, feed_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM feed_processor WHERE feed_id = ?1", params![feed_id])?;
    Ok(())
}