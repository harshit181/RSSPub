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
                fc.category_id, CASE WHEN c.name IS NOT NULL THEN (c.position + 1) || '.' || c.name ELSE NULL END
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
                fc.category_id, CASE WHEN c.name IS NOT NULL THEN (c.position + 1) || '.' || c.name ELSE NULL END
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::category_db;
    use crate::db::schema_init;

    #[test]
    fn test_get_feeds_appends_category_order() {
        let conn = schema_init::init_db(":memory:").unwrap();

        let cat1_id = category_db::add_category(&conn, "cat1").unwrap();
        let cat2_id = category_db::add_category(&conn, "cat2").unwrap();

        let feed1_id = add_feed(&conn, "https://example.com/feed1", Some("Feed 1"), 0).unwrap();
        let feed2_id = add_feed(&conn, "https://example.com/feed2", Some("Feed 2"), 0).unwrap();
        let _feed3_id = add_feed(&conn, "https://example.com/feed3", Some("Feed 3"), 0).unwrap();

        category_db::update_feed_category(&conn, feed1_id, Some(cat1_id)).unwrap();
        category_db::update_feed_category(&conn, feed2_id, Some(cat2_id)).unwrap();

        let feeds = get_feeds(&conn).unwrap();
        assert_eq!(feeds.len(), 3);
        assert_eq!(feeds[0].category.as_deref(), Some("1.cat1"));
        assert_eq!(feeds[1].category.as_deref(), Some("2.cat2"));
        assert_eq!(feeds[2].category, None);

        let feeds_cat1 = get_feeds_by_category(&conn, cat1_id).unwrap();
        assert_eq!(feeds_cat1.len(), 1);
        assert_eq!(feeds_cat1[0].category.as_deref(), Some("1.cat1"));

        let feeds_cat2 = get_feeds_by_category(&conn, cat2_id).unwrap();
        assert_eq!(feeds_cat2.len(), 1);
        assert_eq!(feeds_cat2[0].category.as_deref(), Some("2.cat2"));
    }
}
