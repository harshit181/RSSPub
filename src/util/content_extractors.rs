use crate::models::{CustomExtractorConfig, ContentProcessor, OutputMode, ProcessorType};
use arc_swap::ArcSwap;
use dom_query::Document;
use dom_smoothie::{CandidateSelectMode, Config, TextMode};
use reqwest::Client;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

static DOMAIN_OVERRIDES: OnceLock<ArcSwap<HashMap<String, Arc<ContentProcessor>>>> = OnceLock::new();

pub fn refresh_domain_overrides(overrides: Vec<(String, ContentProcessor)>) {
    let map: HashMap<String, Arc<ContentProcessor>> = overrides
        .into_iter()
        .map(|(k, v)| (k, Arc::new(v)))
        .collect();
    
    match DOMAIN_OVERRIDES.get() {
        Some(swap) => swap.store(Arc::new(map)),
        None => { let _ = DOMAIN_OVERRIDES.set(ArcSwap::from_pointee(map)); }
    }
}

pub fn get_domain_override(url: &str) -> Option<Arc<ContentProcessor>> {
    let domain = extract_domain(url)?;
    DOMAIN_OVERRIDES.get()?.load().get(&domain).cloned()
}

pub trait ContentExtractor: Send + Sync {
    fn extract(&self, html: &str, url: &str) -> anyhow::Result<(String, String)>;
}

pub struct DefaultExtractor;

impl ContentExtractor for DefaultExtractor {
    fn extract(&self, html: &str, url: &str) -> anyhow::Result<(String, String)> {
        let cfg = Config {
            text_mode: TextMode::Markdown,
            ..Default::default()
        };
        let mut readability = dom_smoothie::Readability::new(html, Some(url), Some(cfg))?;
        let extracted = readability
            .parse()
            .map_err(|e| anyhow::anyhow!("DomSmoothie error: {:?}", e))?;
        Ok((extracted.title, extracted.content.to_string()))
    }
}

pub struct DomSmoothieExtractor;

impl ContentExtractor for DomSmoothieExtractor {
    fn extract(&self, html: &str, url: &str) -> anyhow::Result<(String, String)> {
        let cfg = Config {
            text_mode: TextMode::Markdown,
            candidate_select_mode: CandidateSelectMode::DomSmoothie,
            ..Default::default()
        };
        let mut readability = dom_smoothie::Readability::new(html, Some(url), Some(cfg))?;
        let extracted = readability
            .parse()
            .map_err(|e| anyhow::anyhow!("DomSmoothie error: {:?}", e))?;
        Ok((extracted.title, extracted.content.to_string()))
    }
}

pub struct TextOnlyExtractor;

impl ContentExtractor for TextOnlyExtractor {
    fn extract(&self, html: &str, url: &str) -> anyhow::Result<(String, String)> {
        let cfg = Config {
            text_mode: TextMode::Markdown,
            candidate_select_mode: CandidateSelectMode::DomSmoothie,
            ..Default::default()
        };
        let mut readability = dom_smoothie::Readability::new(html, Some(url), Some(cfg))?;
        let extracted = readability
            .parse()
            .map_err(|e| anyhow::anyhow!("TextOnly error: {:?}", e))?;
        
        let content_html = extracted.content.to_string();
        let doc = Document::from(content_html.as_str());
        if let Some(images) = doc.try_select("img") {
            images.remove();
        }
        
        Ok((extracted.title, doc.html().to_string()))
    }
}

pub struct CustomExtractor {
    pub config: CustomExtractorConfig,
}

impl CustomExtractor {
    pub fn new(yaml_config: &str) -> anyhow::Result<Self> {
        let config: CustomExtractorConfig = serde_yaml::from_str(yaml_config)
            .map_err(|e| anyhow::anyhow!("Invalid YAML config: {}", e))?;
        Ok(Self { config })
    }
}

impl ContentExtractor for CustomExtractor {
    fn extract(&self, html: &str, _url: &str) -> anyhow::Result<(String, String)> {
        let use_text_mode = self.config.output_mode == OutputMode::Text;

        let document = Document::from(html);

        let title = match document.try_select("title") {
            None => {"Untitled".to_string()},
            Some(x) => {x.text().to_string()}
        };


        let cleaned_html = html.to_string();
        let doc = Document::from(cleaned_html.as_str());
        let discard_selector =&self.config.discard.join(", ");
        let selector =&self.config.selector.join(", ");
        match doc.try_select(discard_selector) {
            None => {}
            Some(dd) => {dd.remove()}
        };
        let mut selected_content =doc.try_select(selector);
        let mut content=String::new();

        while let Some(selected) =selected_content{
            if use_text_mode {
                content=selected.text().to_string();
            }
            else{
                let temp=selected.html().to_string();
                content=format!("{} {}",content,temp);
            }
            selected.select(selector).remove();
            selected_content=selected.try_select(selector);
        }
        let content=content.to_string();

        Ok((title, content))
    }
}
pub fn create_extractor(processor: Option<&ContentProcessor>) -> anyhow::Result<Box<dyn ContentExtractor>> {
    let processor_type = processor.map(|p| p.processor).unwrap_or(ProcessorType::Default);

    match processor_type {
        ProcessorType::DomSmoothie => Ok(Box::new(DomSmoothieExtractor)),
        ProcessorType::TextOnly => Ok(Box::new(TextOnlyExtractor)),
        ProcessorType::Custom => {
            let custom_config = processor
                .and_then(|p| p.custom_config.as_ref())
                .ok_or_else(|| anyhow::anyhow!("Custom processor requires custom_config"))?;
            Ok(Box::new(CustomExtractor::new(custom_config)?))
        }
        ProcessorType::Default => Ok(Box::new(DefaultExtractor)),
    }
}

pub fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
}

pub async fn fetch_full_content(client: &Client, url: &str) -> anyhow::Result<(String, String)> {
    fetch_full_content_with_processor(client, url, None).await
}

/// Markdown Ninja ships the page body as JSON instead of rendering it in HTML.
/// Materialize that body before applying the configured extractor, without
/// executing JavaScript or fetching any additional resources.
fn prepare_article_html(html: &str) -> anyhow::Result<Cow<'_, str>> {
    if !html.contains("window.__markdown_ninja_data") {
        return Ok(Cow::Borrowed(html));
    }

    #[derive(serde::Deserialize)]
    struct PageData {
        page: Page,
    }
    #[derive(serde::Deserialize)]
    struct Page {
        body: String,
    }

    let document = Document::from(html);
    let Some(container) = document.try_select("#markdown-ninja-website") else {
        return Ok(Cow::Borrowed(html));
    };
    for script in document.select("script").iter() {
        let text = script.text();
        let Some(json) = text.trim().strip_prefix("window.__markdown_ninja_data") else {
            continue;
        };
        let Some(json) = json.trim_start().strip_prefix('=') else {
            continue;
        };
        let data: PageData = serde_json::from_str(json.trim().trim_end_matches(';'))
            .map_err(|e| anyhow::anyhow!("Invalid Markdown Ninja page data: {}", e))?;
        if data.page.body.trim().is_empty() {
            anyhow::bail!("Markdown Ninja page contains no article body");
        }
        container.set_html(format!("<article>{}</article>", data.page.body));
        script.remove();
        document.select("noscript").remove();
        return Ok(Cow::Owned(document.html().to_string()));
    }
    Ok(Cow::Borrowed(html))
}

pub async fn fetch_full_content_with_processor(
    client: &Client,
    url: &str,
    processor: Option<&ContentProcessor>,
) -> anyhow::Result<(String, String)> {
    let html = client.get(url).send().await?.text().await?;
    let html = prepare_article_html(&html)?;

    let extractor = if let Some(content_processor) = get_domain_override(url) {
        create_extractor(Some(&*content_processor))?
    } else {
        create_extractor(processor)?
    };
    
    extractor.extract(&html, url)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic fixture matching kerkour.com's JavaScript-only page structure.
    const MARKDOWN_NINJA_HTML: &str = r#"<!DOCTYPE html><html><head>
        <title>Embedded article</title>
        <script>console.log("unrelated script");</script>
        <script>
          window.__markdown_ninja_data = {"website":{"ad":"Advertisement"},"page":{
            "title":"Embedded article",
            "body":"\u003ch2\u003eMemory management\u003c/h2\u003e\n\u003cp\u003eAllocations affect performance. A reusable buffer can reduce the number of allocations in a program. This article explains how to measure memory usage and compare different strategies before changing an implementation. It also describes how fragmentation affects programs that run for a long time.\u003c/p\u003e\u003cp\u003eFixed capacity buffers help when the maximum size is known. Shared buffers can be useful when data must be cloned or sliced. Every strategy comes with tradeoffs, so applications should measure the effects on their own workloads. These techniques apply to both servers and embedded systems.\u003c/p\u003e\u003cpre\u003e\u003ccode\u003elet text = \"Grüße &amp; buffers\";\u003c/code\u003e\u003c/pre\u003e\u003cp\u003e\u003ca href=\"/related\"\u003eRelated article\u003c/a\u003e\u003cimg src=\"/memory.png\" alt=\"Memory diagram\"\u003e\u003c/p\u003e"
          },"contact":null};
        </script></head><body>
        <noscript>Please enable JavaScript to continue.</noscript>
        <div id="markdown-ninja-website"></div>
        </body></html>"#;

    #[test]
    fn materializes_embedded_html_without_executing_javascript() {
        assert!(DefaultExtractor.extract(MARKDOWN_NINJA_HTML, "https://example.com/post").is_err());
        let html = prepare_article_html(MARKDOWN_NINJA_HTML).unwrap();
        let doc = Document::from(html.as_ref());
        assert_eq!(doc.select("title").text().as_ref(), "Embedded article");
        assert_eq!(doc.select("pre code").text().as_ref(), "let text = \"Grüße & buffers\";");
        assert_eq!(doc.select("img").attr("src").unwrap().as_ref(), "/memory.png");
        assert!(!html.contains("Please enable JavaScript"));
        assert!(!html.contains("Advertisement"));
    }

    #[test]
    fn extracts_embedded_article_with_each_processor() {
        let url = "https://example.com/post";
        let html = prepare_article_html(MARKDOWN_NINJA_HTML).unwrap();
        for processor in [ProcessorType::Default, ProcessorType::DomSmoothie,
                          ProcessorType::TextOnly, ProcessorType::Custom] {
            let config = ContentProcessor {
                id: None,
                processor,
                custom_config: Some("selector: [article]\noutput_mode: html".to_string()),
            };
            let (title, content) = create_extractor(Some(&config)).unwrap()
                .extract(&html, url).unwrap();
            assert_eq!(title, "Embedded article");
            assert!(content.contains("Allocations affect performance"), "{processor:?}: {content}");
            assert!(content.contains("Grüße &amp; buffers"));
            assert!(!content.contains("Please enable JavaScript"));
            assert!(!content.contains("Advertisement"));
            assert_eq!(content.contains("<img"), processor != ProcessorType::TextOnly);
            if processor != ProcessorType::Custom {
                assert!(content.contains(&format!("href=\"{}related\"", url.trim_end_matches("post"))));
            }
        }
    }

    #[test]
    fn preserves_regular_html_and_rejects_invalid_embedded_data() {
        let regular = "<html><body><article>Ordinary content</article></body></html>";
        assert!(matches!(prepare_article_html(regular).unwrap(), Cow::Borrowed(_)));
        for data in ["not JSON", r#"{"page":{"body":""}}"#, r#"{"page":{}}"#,
                     r#"{"page":{"body":"<p>Content</p>"}};alert('do not execute')"#] {
            let html = format!("<script>window.__markdown_ninja_data = {data};</script><div id=\"markdown-ninja-website\"></div>");
            assert!(prepare_article_html(&html).is_err());
        }
    }
}
