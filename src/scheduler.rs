use crate::{db, email, processor};
use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};
use crate::db::category_db;
use crate::db::feed_db;

const RSS: &'static str = "rss";
const READ_IT_LATER: &'static str = "read_it_later";

pub async fn init_scheduler(db_conn: Arc<Mutex<Connection>>) -> Result<JobScheduler> {
    let sched = JobScheduler::new().await?;

    let cleanup_job = Job::new_async("0 0 * * * *", |_uuid, _l| {
        Box::pin(async {
            info!("Running cleanup task...");
            if let Err(e) = cleanup_old_files().await {
                error!("Cleanup failed: {}", e);
            }
        })
    })?;
    sched.add(cleanup_job).await?;

    let schedules = {
        let conn = db_conn
            .lock()
            .map_err(|_| anyhow::anyhow!("DB lock failed"))?;
        db::get_schedules(&conn)?
    };

    for schedule in schedules {
        if schedule.active {
            let db_clone = db_conn.clone();
            let job_type = schedule.schedule_type.clone();
            let category_ids = schedule.category_ids.clone();
            let override_to_email = schedule.override_to_email.clone();
            let fetch_since_hours_override = schedule.fetch_since_hours_override;
            info!("Adding schedule: {}", schedule.cron_expression);

            match Job::new_async(schedule.cron_expression.as_str(), move |_uuid, _l| {
                let db = db_clone.clone();
                let job_type = job_type.clone();
                let category_ids = category_ids.clone();
                let override_to_email = override_to_email.clone();
                Box::pin(async move {
                    info!("Running scheduled generation for type: {}", job_type);
                    if job_type == RSS {
                        if let Err(e) = run_scheduled_generation(
                            db,
                            category_ids,
                            override_to_email,
                            fetch_since_hours_override,
                        ).await
                        {
                            error!("Scheduled generation (RSS) failed: {}", e);
                        }
                    } else if job_type == READ_IT_LATER {
                        if let Err(e) = run_read_it_later_generation(db, override_to_email).await {
                            error!("Scheduled generation (Read It Later) failed: {}", e);
                        }
                    } else {
                        error!("Unknown schedule type: {}", job_type);
                    }
                })
            }) {
                Ok(job) => {
                    sched.add(job).await?;
                }
                Err(e) => error!(
                    "Failed to create job for schedule {}: {}",
                    schedule.cron_expression, e
                ),
            }
        }
    }

    sched.start().await?;
    Ok(sched)
}

async fn run_scheduled_generation(
    db: Arc<Mutex<Connection>>,
    category_ids: Vec<i64>,
    override_to_email: Option<String>,
    fetch_since_hours_override: Option<i32>,
) -> Result<()> {
    let (feeds, cover_text_context) = {
        let conn = db.lock().map_err(|_| anyhow::anyhow!("DB lock failed"))?;
        let cover_text_context = schedule_category_context(&conn, &category_ids)?;
        if category_ids.is_empty() {
            (feed_db::get_feeds(&conn)?, cover_text_context)
        } else {
            let mut deduped_feeds = Vec::new();
            let mut seen_feed_ids = HashSet::new();

            for category_id in &category_ids {
                for feed in feed_db::get_feeds_by_category(&conn, *category_id)? {
                    let feed_id = feed.id.unwrap_or_default();
                    if seen_feed_ids.insert(feed_id) {
                        deduped_feeds.push(feed);
                    }
                }
            }

            deduped_feeds.sort_by_key(|feed| feed.position);
            (deduped_feeds, cover_text_context)
        }
    };

    if feeds.is_empty() {
        info!("No feeds to generate.");
        return Ok(());
    }

    let filename = processor::generate_and_save(
        feeds,
        &db,
        crate::util::EPUB_OUTPUT_DIR,
        fetch_since_hours_override,
        Some(cover_text_context),
    ).await?;
    info!("Scheduled generation completed: {}", filename);
    email::check_and_send_email(db, &filename, override_to_email.as_deref()).await?;

    Ok(())
}

async fn run_read_it_later_generation(
    db: Arc<Mutex<Connection>>,
    override_to_email: Option<String>,
) -> Result<()> {
    let (articles, image_timeout, cover_text) = {
        let conn = db.lock().map_err(|_| anyhow::anyhow!("DB lock failed"))?;
        let articles = db::get_read_it_later_articles(&conn, true)?;
        let config = db::get_general_config(&conn)?;
        let cover_text = processor::cover_text_config_from_general_config(
            &config,
            Some(processor::READ_IT_LATER_COVER_TEXT_CONTEXT.to_string()),
        );

        (articles, config.image_timeout_seconds, cover_text)
    };

    if articles.is_empty() {
        info!("No unread articles to deliver.");
        return Ok(());
    }
    let article_ids: Vec<i64> = articles.iter().filter_map(|a| a.id).collect();

    let filename = processor::generate_read_it_later_epub(
        articles,
        crate::util::EPUB_OUTPUT_DIR,
        image_timeout,
        cover_text,
    )
    .await?;
    info!("Read It Later generation completed: {}", filename);

    if !article_ids.is_empty() {
        match db.lock() {
            Ok(conn) => {
                if let Err(e) = db::mark_articles_as_read(&conn, &article_ids) {
                    error!("Failed to mark articles as read: {}", e);
                } else {
                    info!("Marked {} articles as read", article_ids.len());
                }
            }
            Err(_) => error!("Failed to lock DB to mark articles as read"),
        }
    }

    email::check_and_send_email(db, &filename, override_to_email.as_deref()).await?;
    Ok(())
}

fn schedule_category_context(conn: &Connection, category_ids: &[i64]) -> Result<String> {
    if category_ids.is_empty() {
        return Ok("All categories".to_string());
    }

    let names = category_db::get_category_names_by_ids(conn, category_ids)?;
    if names.is_empty() {
        Ok("All categories".to_string())
    } else {
        Ok(names.join(", "))
    }
}

pub async fn cleanup_old_files() -> Result<()> {
    let output_dir = crate::util::EPUB_OUTPUT_DIR;
    if !Path::new(output_dir).exists() {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(output_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        if let Ok(modified) = metadata.modified() {
            if modified.elapsed().unwrap_or_default() > Duration::from_secs(48 * 3600) {
                info!("Deleting old file: {:?}", entry.path());
                tokio::fs::remove_file(entry.path()).await?;
            }
        }
    }
    Ok(())
}
