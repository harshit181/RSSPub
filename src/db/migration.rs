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
        conn.execute("ALTER TABLE schedules ADD COLUMN category_id INTEGER", [])?;
    }

    Ok(())
}

pub fn migrate_schedule_timezone(conn: &Connection) -> Result<(), Error> {
    let count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('schedules') WHERE name='timezone'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        conn.execute(
            "ALTER TABLE schedules ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC'",
            [],
        )?;
    }

    Ok(())
}

pub fn migrate_schedule_email_override(conn: &Connection) -> Result<(), Error> {
    let count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('schedules') WHERE name='override_to_email'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        conn.execute(
            "ALTER TABLE schedules ADD COLUMN override_to_email TEXT",
            [],
        )?;
    }

    Ok(())
}

pub fn migrate_schedule_fetch_since_hours_override(conn: &Connection) -> Result<(), Error> {
    let count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('schedules') WHERE name='fetch_since_hours_override'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        conn.execute(
            "ALTER TABLE schedules ADD COLUMN fetch_since_hours_override INTEGER",
            [],
        )?;
    }

    Ok(())
}

pub fn migrate_schedule_categories(conn: &Connection) -> Result<(), Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schedule_category (
            schedule_id INTEGER NOT NULL,
            category_id INTEGER NOT NULL,
            PRIMARY KEY (schedule_id, category_id),
            FOREIGN KEY (schedule_id) REFERENCES schedules(id) ON DELETE CASCADE,
            FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO schedule_category (schedule_id, category_id)
         SELECT id, category_id FROM schedules WHERE category_id IS NOT NULL",
        [],
    )?;

    conn.execute(
        "UPDATE schedules SET category_id = NULL WHERE category_id IS NOT NULL",
        [],
    )?;

    Ok(())
}

pub fn migrate_general_config_cover_text(conn: &Connection) -> Result<(), Error> {
    if has_column(conn, "general_config", "add_date_in_cover")
        && !has_column(conn, "general_config", "cover_text_enabled")
    {
        conn.execute(
            "ALTER TABLE general_config RENAME COLUMN add_date_in_cover TO cover_text_enabled",
            [],
        )?;
    }

    if has_column(conn, "general_config", "cover_date_color")
        && !has_column(conn, "general_config", "cover_text_color")
    {
        conn.execute(
            "ALTER TABLE general_config RENAME COLUMN cover_date_color TO cover_text_color",
            [],
        )?;
    }

    if !has_column(conn, "general_config", "cover_text_enabled") {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN cover_text_enabled BOOLEAN NOT NULL DEFAULT 0",
            [],
        )?;
    }

    if !has_column(conn, "general_config", "cover_text_color") {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN cover_text_color TEXT NOT NULL DEFAULT 'white'",
            [],
        )?;
    }

    if !has_column(conn, "general_config", "cover_text_position") {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN cover_text_position TEXT NOT NULL DEFAULT 'bottom-right'",
            [],
        )?;
    }

    if !has_column(conn, "general_config", "cover_text_size") {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN cover_text_size TEXT NOT NULL DEFAULT 'small'",
            [],
        )?;
    }

    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    conn.query_row(
        &format!(
            "SELECT count(*) FROM pragma_table_info('{}') WHERE name=?1",
            table
        ),
        [column],
        |row| row.get::<_, i32>(0),
    )
    .unwrap_or(0)
        > 0
}

pub fn migrate_email_config_smtp_username(conn: &Connection) -> Result<(), Error> {
    let count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('email_config') WHERE name='smtp_username'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        conn.execute(
            "ALTER TABLE email_config ADD COLUMN smtp_username TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }

    Ok(())
}
