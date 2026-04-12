use crate::models::EmailConfig;
use anyhow::{Context, Result};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::AsyncSmtpTransport;
use lettre::Tokio1Executor;
use lettre::{AsyncTransport, Message};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};
use rusqlite::Connection;
use crate::db;
use uuid::Uuid;

fn build_epub_message(config: &EmailConfig, filename: &str, filebody: Vec<u8>) -> Result<Message> {
    let content_type = ContentType::parse("application/epub+zip").unwrap();
    let attachment = Attachment::new(String::from(filename)).body(filebody, content_type);

    Message::builder()
        .date_now()
        .message_id(None)
        .user_agent(format!("RSSPub/{}", env!("CARGO_PKG_VERSION")))
        .from(
            config
                .email_address
                .parse()
                .context("Invalid 'from' email")?,
        )
        .to(config.to_email.parse().context("Invalid 'to' email")?)
        .subject(format!("RSS Digest: {}", filename))
        .multipart(
            MultiPart::mixed()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(String::from("Here is your requested RSS Digest EPUB.")),
                )
                .singlepart(attachment),
        )
        .context("Failed to build email")
}

fn debug_dump_email(email: &Message) {
    if env::var_os("RSSPUB_DEBUG_EMAIL_DUMP").as_deref() != Some("1".as_ref()) {
        return;
    }

    let dump_path = env::temp_dir().join(format!("rsspub-email-{}.eml", Uuid::new_v4()));
    match fs::write(&dump_path, email.formatted()) {
        Ok(()) => info!("Wrote debug email dump to {}", dump_path.display()),
        Err(err) => error!("Failed to write debug email dump: {}", err),
    }
}

pub async fn send_epub(config: &EmailConfig, epub_path: &Path) -> Result<()> {
    info!("Preparing to send email to {}", config.to_email);

    let smtp_username = if config.smtp_username.trim().is_empty() {
        config.email_address.as_str()
    } else {
        config.smtp_username.as_str()
    };

    let filename = epub_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("digest.epub");

    let filebody = fs::read(epub_path).context("Failed to read EPUB file")?;
    let email = build_epub_message(config, filename, filebody)?;
    debug_dump_email(&email);

    let creds = Credentials::new(smtp_username.to_string(), config.smtp_password.clone());

    info!(
        "Sending email via {}:{}...",
        config.smtp_host, config.smtp_port
    );

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            .context("Failed to create SMTP transport")?
            .port(config.smtp_port)
            .credentials(creds)
            .timeout(Some(Duration::from_mins(3)))
            .build();

    //mailer.send(email).await.context("Failed to send email")?;
    match mailer.send(email).await {
        Ok(_x) => {
            info!("Email sent successfully!");
        }
        Err(y) => {
            error!("Failed to send email: {}", y);
        }
    }

    Ok(())
}

pub async fn check_and_send_email(db: Arc<Mutex<Connection>>, filename: &String) -> Result<()> {
    let send_email = {
        let conn = db.lock().map_err(|_| anyhow::anyhow!("DB lock failed"))?;
        match db::get_email_config(&conn)? {
            Some(config) => config.enable_auto_send,
            None => false,
        }
    };

    if send_email {
        info!("Auto-send enabled. Sending email...");

        let config_opt = {
            let conn = db.lock().map_err(|_| anyhow::anyhow!("DB lock failed"))?;
            db::get_email_config(&conn)?
        };

        if let Some(config) = config_opt {
            let epub_path = std::path::Path::new(crate::util::EPUB_OUTPUT_DIR).join(&filename);
            if let Err(e) = send_epub(&config, &epub_path).await {
                error!("Failed to auto-send email: {}", e);
            } else {
                info!("Auto-send email sent successfully.");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_epub_message;
    use crate::models::EmailConfig;

    fn test_email_config() -> EmailConfig {
        EmailConfig {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            smtp_password: "secret".to_string(),
            email_address: "sender@example.com".to_string(),
            smtp_username: "sender@example.com".to_string(),
            to_email: "kindle@example.com".to_string(),
            enable_auto_send: false,
        }
    }

    #[test]
    fn serializes_kindle_friendly_epub_email_headers() {
        let email = build_epub_message(
            &test_email_config(),
            "digest.epub",
            b"dummy epub bytes".to_vec(),
        )
        .expect("message should build");

        let raw = String::from_utf8(email.formatted()).expect("message should serialize as utf-8");

        assert!(raw.contains("Date:"), "raw message should include Date header");
        assert!(
            raw.contains("Message-ID:"),
            "raw message should include Message-ID header"
        );
        assert!(
            raw.contains(&format!("User-Agent: RSSPub/{}", env!("CARGO_PKG_VERSION"))),
            "raw message should include RSSPub user agent"
        );
        assert!(
            raw.contains("MIME-Version: 1.0"),
            "raw message should include MIME-Version header"
        );
        assert!(
            raw.contains("Content-Type: multipart/mixed"),
            "raw message should be multipart/mixed"
        );
        assert!(
            raw.contains("filename=\"digest.epub\"") || raw.contains("filename=digest.epub"),
            "raw message should include attachment filename"
        );
        assert!(
            raw.contains("application/epub+zip"),
            "raw message should include EPUB content type"
        );
    }
}
