use std::collections::{BTreeMap, BTreeSet, HashMap};
use crate::models::epub_message::EpubPart;
use crate::models::{CoverTextColor, CoverTextPosition, CoverTextSize};
use crate::feed::{Article, ArticleSource};
use crate::image::process_images;
use crate::templates::{XhtmlWrapper, MasterToc, TocEntry, SourceToc, ArticleEntry, ArticleTemplate, CategoryGroup, CoverTemplate};
use anyhow::Result;
use askama::Template;
use chrono::Utc;
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ReferenceType, ZipLibrary};
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use image::{load_from_memory, DynamicImage, GenericImage, GenericImageView, ImageFormat, Rgba};
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use tokio::task::JoinSet;
use tracing::info;

const EPUB_LANGUAGE_ENV: &str = "RSSPUB_EPUB_LANGUAGE";

fn resolve_epub_language(value: Option<String>) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "en".to_string())
}

fn epub_language() -> String {
    resolve_epub_language(env::var(EPUB_LANGUAGE_ENV).ok())
}

#[derive(Debug, Clone)]
pub struct CoverTextConfig {
    pub enabled: bool,
    pub color: CoverTextColor,
    pub position: CoverTextPosition,
    pub size: CoverTextSize,
    pub context: Option<String>,
}

impl Default for CoverTextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: CoverTextColor::default(),
            position: CoverTextPosition::default(),
            size: CoverTextSize::default(),
            context: None,
        }
    }
}

pub async fn generate_epub_data<W: Write + Seek + Send + 'static>(
    articles: &[Article],
    output: W,
    image_timeout_seconds: i32,
    cover_text: CoverTextConfig,
) -> Result<()> {
    use crate::models::epub_message::{CompletionMessage, EpubPart};
    use crate::util;
    use std::collections::HashMap;
    //TODO: Refactor the code,rather than passing Articles,pass a Map <FeedWrapper,Article>,remove ArticleSource
    let mut articles_by_source: HashMap<String, Vec<&Article>> = HashMap::new();
    let mut articles_sorted:BTreeSet<&ArticleSource> = BTreeSet::new();
    for article in articles {
        articles_by_source
            .entry(article.article_source.source.clone())
            .or_default()
            .push(article);
        articles_sorted.insert(&article.article_source);
    }

    let sources: Vec<_> = {
        let mut seen = std::collections::HashSet::new();
        articles_sorted.iter().map(|x| x.source.clone()).filter(|s| seen.insert(s.clone())).collect()
    };

    let mut article_filenames = HashMap::new();
    for (i, _article) in articles.iter().enumerate() {
        article_filenames.insert(i, format!("chapter_{}.xhtml", i));
    }

    let mut next_seq_id = 0;

    let master_toc_seq_id = 0;
    next_seq_id += 1;

    let mut source_toc_seq_ids = HashMap::new();
    let mut article_seq_ids = HashMap::new();

    for source in &sources {
        source_toc_seq_ids.insert(source.clone(), next_seq_id);
        next_seq_id += 1;

        for article in &articles_by_source[source] {
            let index = articles
                .iter()
                .position(|a| std::ptr::eq(a, *article))
                .unwrap();
            article_seq_ids.insert(index, next_seq_id);
            next_seq_id += 1;
        }
    }

    let total_parts = next_seq_id;
    info!("Total EPUB parts to write: {}", total_parts);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<CompletionMessage>(32);
    let (tx_m, mut rx_m) = tokio::sync::mpsc::channel::<CompletionMessage>(32);
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_again = Arc::clone(&counter);
    let builder_handle = tokio::task::spawn_blocking(move || -> Result<()> {
        let mut builder =
            EpubBuilder::new(ZipLibrary::new().map_err(|e| anyhow::anyhow!("{}", e))?)
                .map_err(|e| anyhow::anyhow!("{}", e))?;

        builder.epub_version(EpubVersion::V33);
        builder
            .metadata("author", "RSSPub RSS Book")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        builder
            .metadata("lang", epub_language())
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        builder
            .metadata(
                "title",
                format!("RSS Digest - {}", Utc::now().format("%Y-%m-%d")),
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let cover_path = util::COVER_LOCATION;
        if std::path::Path::new(cover_path).exists() {
            match std::fs::read(cover_path) {
                Ok(cover_data) => {
                    let cover_data = generate_cover_image(&cover_data, &cover_text);
                    builder
                        .add_cover_image("cover.jpg", cover_data.as_slice(), "image/jpeg")
                        .map_err(|e| anyhow::anyhow!("Failed to add cover image: {}", e))?;

                    let cover_template = CoverTemplate { image_path: "cover.jpg" };
                    let cover_html = cover_template.render().map_err(|e| anyhow::anyhow!("Failed to render cover template: {}", e))?;
                    let cover_xhtml = XhtmlWrapper { title: "Cover", content: &cover_html };
                    let cover_content = cover_xhtml.render().map_err(|e| anyhow::anyhow!("Failed to render cover XHTML: {}", e))?;
                    let cover_page = EpubContent::new("cover.xhtml", cover_content.as_bytes())
                        .title("Cover")
                        .reftype(ReferenceType::Cover);
                    builder.add_content(cover_page).map_err(|e| anyhow::anyhow!("Failed to add cover page: {}", e))?;
                }
                Err(e) => info!("Failed to read cover image: {}", e),
            }
        }

        let mut current_seq = 0;
        let mut buffer: HashMap<usize, Vec<EpubPart>> = HashMap::new();

        let pb = ProgressBar::new(total_parts as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) Articles")
            .unwrap()
            .progress_chars("#>-"));

        while let Some(msg) = rx.blocking_recv() {
            buffer.insert(msg.sequence_id, msg.parts);
            while let Some(parts) = buffer.remove(&current_seq) {
                //info!("Writing sequence {} to EPUB", current_seq);
                populate_epub_data(&mut builder, parts)?;
                current_seq += 1;
                pb.inc(1);
            }

            if current_seq >= total_parts {
                pb.finish_with_message("Articles processed");
                info!("All parts received. Moving to images");
                break;
            }
        }
        current_seq = 0;
        let total_images = &counter_again.load(Ordering::Relaxed);
        info!("Total images are {}", &total_images);
        if total_images> &0{
        let pb_images = ProgressBar::new(*total_images as u64);
        pb_images.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) Images")
            .unwrap()
            .progress_chars("#>-"));

        while let Some(msg) = rx_m.blocking_recv() {
            //info!("Got image with seq id {} {}", msg.sequence_id, &current_seq);
            let parts = msg.parts;
            populate_epub_data(&mut builder, parts)?;
            current_seq += 1;
            pb_images.inc(1);
            if current_seq >= *total_images {
                pb_images.finish_with_message("Images processed");
                info!("All images received. Finishing EPUB.");
                break;
            }
        }
        }

        builder
            .generate(output)
            .map_err(|e| anyhow::anyhow!("Failed to generate EPUB: {}", e))?;

        Ok(())
    });

    let mut category_map:BTreeMap<String, Vec<TocEntry>> =BTreeMap::new();
    
    for source in sources.iter() {
        let source_slug = source
            .replace(|c: char| !c.is_alphanumeric(), "_")
            .to_lowercase();
        
        let mut category = "Uncategorized".to_string();
        if let Some(article) = articles_by_source[source].first() {
            if let Some(cat) = &article.article_source.category {
                category = cat.clone();
            }
        }
        
        let entry = TocEntry {
            toc_filename: format!("toc_{}.xhtml", source_slug),
            name: source.clone(),
        };
        
        category_map.entry(category).or_default().push(entry);
    }

    let groups: Vec<CategoryGroup> = category_map.into_iter().map(|(category, sources)| {
        CategoryGroup { category, sources }
    }).collect();

    let master_toc_template = MasterToc { groups };
    let master_toc_html = master_toc_template.render().map_err(|e| anyhow::anyhow!("Failed to render master TOC: {}", e))?;
    let xhtml_wrapper = XhtmlWrapper { title: "Table of Contents", content: &master_toc_html };
    let master_toc_content = xhtml_wrapper.render().map_err(|e| anyhow::anyhow!("Failed to render XHTML wrapper: {}", e))?;

    tx.send(CompletionMessage {
        sequence_id: master_toc_seq_id,
        parts: vec![EpubPart::Content {
            filename: "toc.xhtml".to_string(),
            title: "Table of Contents".to_string(),
            content: master_toc_content,
            reftype: Some(ReferenceType::Toc),
        }],
    })
    .await
    .map_err(|_| anyhow::anyhow!("Failed to send Master TOC"))?;

    for (idx, source) in sources.iter().enumerate() {
        let source_slug = source
            .replace(|c: char| !c.is_alphanumeric(), "_")
            .to_lowercase();
        let source_toc_filename = format!("toc_{}.xhtml", source_slug);
        let source_articles = &articles_by_source[source];

        let article_entries: Vec<ArticleEntry> = source_articles.iter().map(|article| {
            let index = articles
                .iter()
                .position(|a| std::ptr::eq(a, *article))
                .unwrap();
            ArticleEntry {
                filename: article_filenames[&index].clone(),
                title: article.title.clone(),
            }
        }).collect();

        let next_toc_link = if idx + 1 < sources.len() {
            let next_source = &sources[idx + 1];
            let next_slug = next_source
                .replace(|c: char| !c.is_alphanumeric(), "_")
                .to_lowercase();
            Some((format!("toc_{}.xhtml", next_slug), next_source.clone()))
        } else {
            None
        };

        let source_toc_template = SourceToc {
            source_name: source.clone(),
            articles: article_entries,
            next_toc_link,
        };
        let source_toc_html = source_toc_template.render().map_err(|e| anyhow::anyhow!("Failed to render source TOC: {}", e))?;
        let xhtml_wrapper = XhtmlWrapper { title: source, content: &source_toc_html };
        let source_toc_content = xhtml_wrapper.render().map_err(|e| anyhow::anyhow!("Failed to render XHTML wrapper: {}", e))?;

        let seq_id = source_toc_seq_ids[source];
        tx.send(CompletionMessage {
            sequence_id: seq_id,
            parts: vec![EpubPart::Content {
                filename: source_toc_filename,
                title: source.clone(),
                content: source_toc_content,
                reftype: None,
            }],
        })
        .await
        .map_err(|_| anyhow::anyhow!("Failed to send Source TOC"))?;
    }

    let (prev_links, next_links) = generate_prev_next_links(articles, &mut articles_by_source, &sources, &mut article_filenames);

    let mut join_set = JoinSet::new();
    for (i, article) in articles.iter().enumerate() {
        let article = article.clone();
        let chapter_filename = article_filenames[&i].clone();
        let temp_log = article_filenames[&i].clone();
        let seq_id = article_seq_ids[&i];
        let tx = tx.clone();

        let source_slug = article
            .article_source.source
            .replace(|c: char| !c.is_alphanumeric(), "_")
            .to_lowercase();
        let back_link = format!("toc_{}.xhtml", source_slug);
        let prev_link = prev_links.get(&i).cloned();
        let next_link = next_links.get(&i).cloned();
        let tx_m = tx_m.clone();
        let counter_ref = Arc::clone(&counter);
        join_set.spawn(async move {
            let cleaned_content = util::clean_html(&article.content);
            let (processed_content,total_images_for_seq) = process_images(&cleaned_content,&tx_m,&seq_id, image_timeout_seconds as u64).await;
            counter_ref.fetch_add(total_images_for_seq, Ordering::Relaxed);
            let fixed_content = util::fix_xhtml(&processed_content);

            let article_template = ArticleTemplate {
                title: &article.title,
                source: &article.article_source.source,
                pub_date: article.pub_date.format("%Y-%m-%d %H:%M").to_string(),
                content: &fixed_content,
                original_link: &article.link,
                back_link,
                prev_link,
                next_link,
            };
            let content_html = article_template.render().unwrap_or_else(|e| {
                format!("<p>Failed to render article: {}</p>", e)
            });

            let xhtml_wrapper = XhtmlWrapper { title: &article.title, content: &content_html };
            let final_content = xhtml_wrapper.render().unwrap_or_else(|e| {
                format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><html><body><p>Failed to render: {}</p></body></html>", e)
            });

            let mut parts = Vec::new();

            parts.push(EpubPart::Content {
                filename: chapter_filename,
                title: article.title,
                content: final_content,
                reftype: None,
            });
                info!("Sending Completed Part {}", temp_log);
            if let Err(_) = tx.send(CompletionMessage {
                sequence_id: seq_id,
                parts,
            }).await {
                info!("Failed to send article {} (receiver might be closed)", i);
            }
        });
    }
    drop(tx);

    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res {
            info!("Article processing task failed: {}", e);
        }
    }

    builder_handle
        .await
        .map_err(|e| anyhow::anyhow!("Builder task joined error: {}", e))??;

    info!("EPUB generated successfully");
    Ok(())
}

fn generate_prev_next_links(articles: &[Article], articles_by_source: &mut HashMap<String, Vec<&Article>>, sources: &Vec<String>, article_filenames:  &HashMap<usize, String>) -> (HashMap<usize, String>, HashMap<usize, String>) {
    let mut prev_links: HashMap<usize, String> = HashMap::new();
    let mut next_links: HashMap<usize, String> = HashMap::new();
    for source in sources {
        let source_articles = &articles_by_source[source];
        let indices: Vec<usize> = source_articles.iter().map(|a| {
            articles.iter().position(|x| std::ptr::eq(x, *a)).unwrap()
        }).collect();
        for (pos, &idx) in indices.iter().enumerate() {
            if pos > 0 {
                prev_links.insert(idx, article_filenames[&indices[pos - 1]].clone());
            }
            if pos + 1 < indices.len() {
                next_links.insert(idx, article_filenames[&indices[pos + 1]].clone());
            }
        }
    }
    (prev_links, next_links)
}

#[cfg(test)]
mod tests {
    use super::resolve_epub_language;

    #[test]
    fn uses_default_epub_language_when_env_missing() {
        assert_eq!(resolve_epub_language(None), "en");
    }

    #[test]
    fn trims_configured_epub_language() {
        assert_eq!(resolve_epub_language(Some(" de ".to_string())), "de");
    }

    #[test]
    fn ignores_empty_configured_epub_language() {
        assert_eq!(resolve_epub_language(Some("   ".to_string())), "en");
    }
}

fn generate_cover_image(cover_data: &Vec<u8>, cover_text: &CoverTextConfig) -> Vec<u8> {
    let mut final_cover_data = cover_data.clone();

    if cover_text.enabled {
        if let Ok(mut img) = load_from_memory(&cover_data) {
            let font_data: &[u8] = include_bytes!("../static/Roboto-Regular.ttf");
            if let Ok(font) = FontRef::try_from_slice(font_data) {
                let size_ratio = match cover_text.size {
                    CoverTextSize::Medium => 0.08,
                    CoverTextSize::Large => 0.12,
                    CoverTextSize::Small => 0.05,
                };
                let height = img.height() as f32 * size_ratio;
                let height = if height < 20.0 { 20.0 } else { height };
                let scale = PxScale::from(height);
                let mut lines = Vec::new();
                if let Some(context) = cover_text.context.as_deref() {
                    let context = context.trim();
                    if !context.is_empty() {
                        lines.push(context.to_string());
                    }
                }
                lines.push(Utc::now().format("%Y-%m-%d %H:%M").to_string());

                let scaled_font = font.as_scaled(scale);
                let line_widths: Vec<u32> = lines
                    .iter()
                    .map(|line| measure_text_width(&font, &scaled_font, line))
                    .collect();
                let max_text_width = line_widths.iter().copied().max().unwrap_or(0);

                let img_height = img.height();
                let img_width = img.width();
                let padding = 20;
                let line_gap = height * 0.25;
                let block_height =
                    height * lines.len() as f32 + line_gap * (lines.len().saturating_sub(1) as f32);

                let block_x = match cover_text.position {
                    CoverTextPosition::TopLeft | CoverTextPosition::BottomLeft => padding,
                    CoverTextPosition::Center if img_width > max_text_width => {
                        (img_width - max_text_width) / 2
                    }
                    _ if img_width > max_text_width + padding => {
                        img_width - max_text_width - padding
                    }
                    _ => 10,
                };
                let alignment_width =
                    max_text_width.min(img_width.saturating_sub(block_x.saturating_add(padding)));
                let block_y = match cover_text.position {
                    CoverTextPosition::TopLeft | CoverTextPosition::TopRight => padding,
                    CoverTextPosition::Center if img_height > block_height as u32 => {
                        (img_height - block_height as u32) / 2
                    }
                    _ if img_height > (block_height as u32) + padding => {
                        img_height - (block_height as u32) - padding
                    }
                    _ => 10,
                };

                let is_right_aligned = matches!(
                    cover_text.position,
                    CoverTextPosition::TopRight | CoverTextPosition::BottomRight
                );
                let is_center_aligned = matches!(cover_text.position, CoverTextPosition::Center);
                let text_color = match cover_text.color {
                    CoverTextColor::Black => 0,
                    CoverTextColor::White => 255,
                };

                for (line_index, line) in lines.iter().enumerate() {
                    let line_x = if is_right_aligned && alignment_width > line_widths[line_index] {
                        block_x + alignment_width - line_widths[line_index]
                    } else if is_center_aligned && alignment_width > line_widths[line_index] {
                        block_x + (alignment_width - line_widths[line_index]) / 2
                    } else {
                        block_x
                    };
                    let line_y = block_y as f32 + (line_index as f32 * (height + line_gap));
                    let mut current_x = line_x as f32;
                    let mut last_glyph_id = None;

                    for c in line.chars() {
                        let id = font.glyph_id(c);
                        if let Some(last_id) = last_glyph_id {
                            current_x += scaled_font.kern(last_id, id);
                        }

                        let glyph = id.with_scale_and_position(
                            scale,
                            point(current_x, line_y + scaled_font.ascent()),
                        );

                        if let Some(outlined) = font.outline_glyph(glyph) {
                            let bounds = outlined.px_bounds();
                            outlined.draw(|gx, gy, v| {
                                let px = bounds.min.x as i32 + gx as i32;
                                let py = bounds.min.y as i32 + gy as i32;

                                if px >= 0
                                    && px < img.width() as i32
                                    && py >= 0
                                    && py < img.height() as i32
                                {
                                    let pixel = img.get_pixel(px as u32, py as u32);
                                    let blend = |old: u8, new: u8, alpha: f32| -> u8 {
                                        ((old as f32) * (1.0 - alpha) + (new as f32) * alpha) as u8
                                    };
                                    let r = blend(pixel[0], text_color, v);
                                    let g = blend(pixel[1], text_color, v);
                                    let b = blend(pixel[2], text_color, v);
                                    let a = blend(pixel[3], 255, v);
                                    img.put_pixel(px as u32, py as u32, Rgba([r, g, b, a]));
                                }
                            });
                        }

                        current_x += scaled_font.h_advance(id);
                        last_glyph_id = Some(id);
                    }
                }

                let rgb_img = DynamicImage::ImageRgb8(img.into_rgb8());
                let mut cursor = Cursor::new(Vec::new());
                if rgb_img.write_to(&mut cursor, ImageFormat::Jpeg).is_ok() {
                    final_cover_data = cursor.into_inner();
                } else {
                    tracing::warn!("Failed to write cover text image to JPEG");
                }
            }
        }
    }
    final_cover_data
}

fn measure_text_width<F, S>(font: F, scaled_font: &S, text: &str) -> u32
where
    F: Font,
    S: ScaleFont<F>,
{
    let mut text_width = 0.0;
    let mut last_glyph_id = None;
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(last_id) = last_glyph_id {
            text_width += scaled_font.kern(last_id, id);
        }
        text_width += scaled_font.h_advance(id);
        last_glyph_id = Some(id);
    }
    text_width as u32
}

fn populate_epub_data(builder: &mut EpubBuilder<ZipLibrary>, parts: Vec<EpubPart>) -> Result<()> {
    for part in parts {
        match part {
            EpubPart::Content {
                filename,
                title,
                content,
                reftype,
            } => {
                let mut content = EpubContent::new(filename, content.as_bytes()).title(title);
                if let Some(rt) = reftype {
                    content = content.reftype(rt);
                }
                builder
                    .add_content(content)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }
            EpubPart::Resource {
                filename,
                mut content,
                mime_type,
            } => {
                content.seek(SeekFrom::Start(0))?;
                builder
                    .add_resource(filename, content, mime_type)
                    .map_err(|e| anyhow::anyhow!("Failed to add resource: {}", e))?;
            }
        }
    }
    Ok(())
}
