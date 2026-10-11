use rusqlite::{Connection, Error};

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

pub fn migrate_general_config_cleanup_after_hours(conn: &Connection) -> Result<(), Error> {
    if !has_column(conn, "general_config", "cleanup_after_hours") {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN cleanup_after_hours INTEGER NOT NULL DEFAULT 48",
            [],
        )?;
    }

    Ok(())
}

pub fn migrate_general_config_user_agent(conn: &Connection) -> Result<(), Error> {
    if !has_column(conn, "general_config", "user_agent") {
        conn.execute(
            "ALTER TABLE general_config ADD COLUMN user_agent TEXT",
            [],
        )?;
    }

    Ok(())
}
