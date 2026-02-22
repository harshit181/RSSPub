use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::fs;
use askama::Template;

#[derive(Template)]
#[template(path = "opds.html", escape = "xml")]
pub struct OpdsTemplate<'a> {
    pub updated: &'a str,
    pub base_url: &'a str,
    pub entries: Vec<OpdsEntry<'a>>,
}

pub struct OpdsEntry<'a> {
    pub filename: &'a str,
    pub modified: String,
    pub date_str: String,
    pub download_url: String,
}

pub async fn generate_opds_feed(base_url: &str, dir_path: &str) -> Result<String> {
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(dir_path).await?;

    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("epub") {
            if let Ok(metadata) = fs::metadata(&path).await {
                if let Ok(modified) = metadata.modified() {
                    let modified: DateTime<Utc> = modified.into();
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    entries.push((filename, modified));
                }
            }
        }
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let updated = if let Some(first) = entries.first() {
        first.1.to_rfc3339()
    } else {
        Utc::now().to_rfc3339()
    };

    let template_entries: Vec<OpdsEntry> = entries.iter().map(|(filename, modified)| {
        let date_str = modified.format("%Y-%m-%d").to_string();
        let download_url = format!("{}/epubs/{}", base_url, filename);
        OpdsEntry {
            filename,
            modified: modified.to_rfc3339(),
            date_str,
            download_url,
        }
    }).collect();

    let template = OpdsTemplate {
        updated: &updated,
        base_url,
        entries: template_entries,
    };

    Ok(template.render()?)
}
