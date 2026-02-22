use chrono::Utc;
use rusqlite::{params, Connection};
use crate::models::{Category, CategoryPosition};

pub fn add_category(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    let next_position: i64 = conn
        .query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM categories", [], |row| row.get(0))
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO categories (name, position, created_at) VALUES (?1, ?2, ?3)",
        params![name, next_position, Utc::now().to_rfc3339()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_category(conn: &Connection, id: i64, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE categories SET name = ?1 WHERE id = ?2",
        params![name, id],
    )?;
    Ok(())
}

pub fn get_categories(conn: &Connection) -> rusqlite::Result<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT id, name, position FROM categories ORDER BY position ASC")?;
    let iter = stmt.query_map([], |row| {
        Ok(Category {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            position: row.get(2)?,
        })
    })?;

    let mut cats = Vec::new();
    for curr in iter { cats.push(curr?); }
    Ok(cats)
}

pub fn reorder_categories(conn: &Connection, positions: &Vec<CategoryPosition>) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("UPDATE categories SET position = ?1 WHERE id = ?2")?;
    for x in positions {
        stmt.execute(params![x.position, x.id])?;
    }
    Ok(())
}

pub fn delete_category(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn update_feed_category(conn: &Connection, feed_id: i64, category_id: Option<i64>) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM feed_category WHERE feed_id = ?1", params![feed_id])?;
    if let Some(cat_id) = category_id {
        tx.execute("INSERT INTO feed_category (feed_id, category_id) VALUES (?1, ?2)", params![feed_id, cat_id])?;
    }
    tx.commit()?;
    Ok(())
}