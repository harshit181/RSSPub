use rusqlite::{Connection, Error};

pub fn migrate_constraint(conn: &Connection) -> Result<(), Error> {
    // Migration: in case sqllite db has check constraint from previous version (will delete this in future)
    let has_check_constraint: bool = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='feed_processor'",
            [],
            |row| {
                let sql: String = row.get(0)?;
                Ok(sql.contains("CHECK"))
            },
        )
        .unwrap_or(false);

    if has_check_constraint {
        conn.execute_batch(
            "CREATE TABLE feed_processor_new (
                feed_id INTEGER PRIMARY KEY,
                processor INTEGER NOT NULL DEFAULT 1,
                custom_config TEXT,
                FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
            );
            INSERT INTO feed_processor_new SELECT * FROM feed_processor;
            DROP TABLE feed_processor;
            ALTER TABLE feed_processor_new RENAME TO feed_processor;",
        )?;
    }
    Ok(())
}

pub fn migrate_position(conn: &Connection) -> Result<(), Error> {
    let has_position: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('feeds') WHERE name='position'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if has_position == 0 {
        conn.execute(
            "ALTER TABLE feeds ADD COLUMN position INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

pub fn migrate_feed_schedule(conn: &Connection) -> Result<(), Error> {

    let count_category_id: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('schedules') WHERE name='category_id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count_category_id == 0 {
        conn.execute(
            "ALTER TABLE schedules ADD COLUMN category_id INTEGER",
            [],
        )?;
    }

    Ok(())
}

pub fn migrate_general_config_cover_date(conn: &Connection) -> Result<(), Error> {
    let count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('general_config') WHERE name='add_date_in_cover'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN add_date_in_cover BOOLEAN NOT NULL DEFAULT 0",
            [],
        )?;
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN cover_date_color TEXT NOT NULL DEFAULT 'white'",
            [],
        )?;
    }
    Ok(())
}