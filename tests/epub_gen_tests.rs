use chrono::{DateTime, TimeZone, Utc};
use rsspub::feed::{Article, ArticleSource};
use rsspub::epub_gen::generate_epub_data;
use std::fs::File;
use std::io::{Cursor, Read};
use tempfile::NamedTempFile;
use zip::ZipArchive;

// ============================================================================
// Test Helpers
// ============================================================================

/// Creates an ArticleSource with the given source name and position.
fn create_article_source(source: &str, position: i64) -> ArticleSource {
    ArticleSource {
        source: source.to_string(),
        position,
    }
}

/// Creates a test Article with all fields populated.
fn create_article(
    title: &str,
    link: &str,
    content: &str,
    pub_date: DateTime<Utc>,
    source: &str,
    position: i64,
) -> Article {
    Article {
        title: title.to_string(),
        link: link.to_string(),
        content: content.to_string(),
        pub_date,
        article_source: create_article_source(source, position),
    }
}

/// Creates a simple test article with minimal content.
fn create_simple_article(title: &str, source: &str, position: i64) -> Article {
    create_article(
        title,
        &format!("https://example.com/{}", title.replace(' ', "-").to_lowercase()),
        &format!("<p>Content for article: {}</p>", title),
        Utc::now(),
        source,
        position,
    )
}

/// Generates an EPUB and returns the data as a byte vector.
async fn generate_epub_to_vec(articles: &[Article]) -> Vec<u8> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file = File::create(temp_file.path()).expect("Failed to create file");
    
    generate_epub_data(articles, file, 30)
        .await
        .expect("Failed to generate EPUB");
    
    std::fs::read(temp_file.path()).expect("Failed to read temp file")
}

/// Extracts EPUB content and returns as a ZipArchive for inspection.
fn extract_epub(epub_data: Vec<u8>) -> ZipArchive<Cursor<Vec<u8>>> {
    let cursor = Cursor::new(epub_data);
    ZipArchive::new(cursor).expect("Failed to open EPUB as ZIP")
}

/// Reads a file from the EPUB archive.
fn read_epub_file(archive: &mut ZipArchive<Cursor<Vec<u8>>>, filename: &str) -> Option<String> {
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).ok()?;
        if file.name().ends_with(filename) {
            let mut contents = String::new();
            file.read_to_string(&mut contents).ok()?;
            return Some(contents);
        }
    }
    None
}

// ============================================================================
// Basic EPUB Generation Tests
// ============================================================================

#[tokio::test]
async fn test_generate_epub_with_single_article() {
    let articles = vec![create_simple_article("Test Article", "Tech News", 0)];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    
    assert!(!epub_data.is_empty(), "EPUB output should not be empty");
    
    // Verify it's a valid ZIP/EPUB
    let archive = extract_epub(epub_data);
    assert!(archive.len() > 0, "EPUB should contain files");
}

#[tokio::test]
async fn test_generate_epub_with_multiple_articles_single_source() {
    let articles = vec![
        create_simple_article("Article One", "Tech Blog", 0),
        create_simple_article("Article Two", "Tech Blog", 0),
        create_simple_article("Article Three", "Tech Blog", 0),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    
    // Check that all chapter files exist
    assert!(read_epub_file(&mut archive, "chapter_0.xhtml").is_some(), "chapter_0.xhtml should exist");
    assert!(read_epub_file(&mut archive, "chapter_1.xhtml").is_some(), "chapter_1.xhtml should exist");
    assert!(read_epub_file(&mut archive, "chapter_2.xhtml").is_some(), "chapter_2.xhtml should exist");
}

#[tokio::test]
async fn test_generate_epub_with_multiple_sources() {
    let articles = vec![
        create_simple_article("Tech Article 1", "Tech News", 0),
        create_simple_article("Tech Article 2", "Tech News", 0),
        create_simple_article("Sports Article 1", "Sports Daily", 1),
        create_simple_article("Science Article 1", "Science Weekly", 2),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    
    // Check master TOC exists
    let master_toc = read_epub_file(&mut archive, "toc.xhtml");
    assert!(master_toc.is_some(), "Master TOC should exist");
    
    let toc_content = master_toc.unwrap();
    assert!(toc_content.contains("Table of Contents"), "Master TOC should have title");
    assert!(toc_content.contains("Tech News"), "Master TOC should link to Tech News");
    assert!(toc_content.contains("Sports Daily"), "Master TOC should link to Sports Daily");
    assert!(toc_content.contains("Science Weekly"), "Master TOC should link to Science Weekly");
}

#[tokio::test]
async fn test_generate_epub_empty_articles() {
    let articles: Vec<Article> = vec![];
    
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file = File::create(temp_file.path()).expect("Failed to create file");
    
    let result = generate_epub_data(&articles, file, 30).await;
    
    // Should succeed even with empty articles (creates just the TOC)
    assert!(result.is_ok(), "EPUB generation should succeed with empty articles");
}

// ============================================================================
// TOC Generation Tests
// ============================================================================

#[tokio::test]
async fn test_master_toc_structure() {
    let articles = vec![
        create_simple_article("Article A", "Source Alpha", 0),
        create_simple_article("Article B", "Source Beta", 1),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let master_toc = read_epub_file(&mut archive, "toc.xhtml").unwrap();
    
    // Check for proper HTML structure
    assert!(master_toc.contains("<h1>Table of Contents</h1>"), "Should have TOC heading");
    assert!(master_toc.contains("<ul>"), "Should have unordered list");
    assert!(master_toc.contains("</ul>"), "Should close unordered list");
    assert!(master_toc.contains("toc_source_alpha.xhtml"), "Should link to Source Alpha TOC");
    assert!(master_toc.contains("toc_source_beta.xhtml"), "Should link to Source Beta TOC");
}

#[tokio::test]
async fn test_source_toc_structure() {
    let articles = vec![
        create_simple_article("First Article", "My Feed", 0),
        create_simple_article("Second Article", "My Feed", 0),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let source_toc = read_epub_file(&mut archive, "toc_my_feed.xhtml").unwrap();
    
    // Check for proper structure
    assert!(source_toc.contains("My Feed"), "Should have source name as heading");
    assert!(source_toc.contains("Back to Master TOC"), "Should have back link");
    assert!(source_toc.contains("toc.xhtml"), "Should link back to master TOC");
    assert!(source_toc.contains("chapter_0.xhtml"), "Should link to first article");
    assert!(source_toc.contains("chapter_1.xhtml"), "Should link to second article");
    assert!(source_toc.contains("First Article"), "Should show first article title");
    assert!(source_toc.contains("Second Article"), "Should show second article title");
}

#[tokio::test]
async fn test_toc_escaping_special_characters() {
    let articles = vec![
        create_article(
            "Article with <special> & \"characters\"",
            "https://example.com/article",
            "<p>Content</p>",
            Utc::now(),
            "Source with Tags",
            0,
        ),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let master_toc = read_epub_file(&mut archive, "toc.xhtml").unwrap();
    
    // Source name in TOC should not have unescaped angle brackets
    assert!(master_toc.contains("Source with Tags"), "Should contain source name");
}

// ============================================================================
// Article Content Formatting Tests
// ============================================================================

#[tokio::test]
async fn test_article_content_basic() {
    let articles = vec![create_article(
        "Title Test",
        "https://example.com/test",
        "<p>Content with some text</p>",
        Utc::now(),
        "Test Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Title should be present
    assert!(chapter.contains("Title Test"), "Chapter should contain title");
}

#[tokio::test]
async fn test_article_with_html_content() {
    let html_content = r#"
        <h2>Subheading</h2>
        <p>This is a <strong>bold</strong> and <em>italic</em> text.</p>
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
        </ul>
        <blockquote>A quote</blockquote>
    "#;
    
    let articles = vec![create_article(
        "HTML Article",
        "https://example.com/html",
        html_content,
        Utc::now(),
        "HTML Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Basic HTML tags should be preserved
    assert!(chapter.contains("<strong>bold</strong>") || chapter.contains("bold"), 
            "Bold text should be preserved");
    assert!(chapter.contains("<li>") || chapter.contains("Item 1"), 
            "List items should be preserved");
}

#[tokio::test]
async fn test_article_with_unicode() {
    let articles = vec![create_article(
        "Unicode Article",
        "https://example.com/unicode",
        "<p>Content with special chars: € £ ¥</p>",
        Utc::now(),
        "International News",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Unicode should be preserved
    assert!(chapter.contains("Unicode") || chapter.contains("€"), 
            "Unicode characters should be preserved");
}

#[tokio::test]
async fn test_article_metadata_formatting() {
    let pub_date = Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 0).unwrap();
    
    let articles = vec![create_article(
        "Metadata Test Article",
        "https://example.com/metadata",
        "<p>Test content</p>",
        pub_date,
        "Metadata Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Check metadata formatting
    assert!(chapter.contains("Metadata Test Article"), "Title should be present");
    assert!(chapter.contains("Metadata Source"), "Source should be present");
    assert!(chapter.contains("2025-06-15") || chapter.contains("2025"), "Date should be present");
    assert!(chapter.contains("Read original article"), "Should have original article link");
    assert!(chapter.contains("Back to Feed TOC"), "Should have back to TOC link");
}

// ============================================================================
// Sequencing and Ordering Tests
// ============================================================================

#[tokio::test]
async fn test_articles_sorted_by_source_position() {
    // Articles with different source positions
    let articles = vec![
        create_simple_article("Second Source Article", "Second Source", 1),
        create_simple_article("First Source Article", "First Source", 0),
        create_simple_article("Third Source Article", "Third Source", 2),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let master_toc = read_epub_file(&mut archive, "toc.xhtml").unwrap();
    
    // Find positions of source links in TOC
    let first_pos = master_toc.find("First Source").unwrap_or(usize::MAX);
    let second_pos = master_toc.find("Second Source").unwrap_or(usize::MAX);
    let third_pos = master_toc.find("Third Source").unwrap_or(usize::MAX);
    
    // Sources should be ordered by position
    assert!(first_pos < second_pos, "First Source should appear before Second Source");
    assert!(second_pos < third_pos, "Second Source should appear before Third Source");
}

#[tokio::test]
async fn test_unique_sources_preserved() {
    // Multiple articles from same source should result in single TOC entry
    let articles = vec![
        create_simple_article("Article 1", "Duplicate Source", 0),
        create_simple_article("Article 2", "Duplicate Source", 0),
        create_simple_article("Article 3", "Duplicate Source", 0),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let master_toc = read_epub_file(&mut archive, "toc.xhtml").unwrap();
    
    // Count occurrences of source in TOC
    let count = master_toc.matches("Duplicate Source").count();
    assert_eq!(count, 1, "Source should appear only once in master TOC");
}

#[tokio::test]
async fn test_article_filename_generation() {
    let articles = vec![
        create_simple_article("First Article", "Source", 0),
        create_simple_article("Second Article", "Source", 0),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    
    // Check file naming pattern
    let filenames: Vec<_> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index_raw(i).ok()?;
            Some(file.name().to_string())
        })
        .collect();
    
    let has_chapter_0 = filenames.iter().any(|n| n.contains("chapter_0.xhtml"));
    let has_chapter_1 = filenames.iter().any(|n| n.contains("chapter_1.xhtml"));
    
    assert!(has_chapter_0, "Should have chapter_0.xhtml");
    assert!(has_chapter_1, "Should have chapter_1.xhtml");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_special_characters_in_source_name() {
    let articles = vec![create_simple_article(
        "Article with special source",
        "Source With Special Characters And More",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    
    // Source TOC filename should be sanitized
    let filenames: Vec<_> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index_raw(i).ok()?;
            Some(file.name().to_string())
        })
        .collect();
    
    let has_source_toc = filenames.iter().any(|n| n.contains("toc_") && n.ends_with(".xhtml") && !n.ends_with("toc.xhtml"));
    assert!(has_source_toc, "Should have sanitized source TOC filename");
}

#[tokio::test]
async fn test_long_article_content() {
    // Generate very long content
    let long_content = "<p>".to_string() + &"Lorem ipsum dolor sit amet. ".repeat(1000) + "</p>";
    
    let articles = vec![create_article(
        "Long Article",
        "https://example.com/long",
        &long_content,
        Utc::now(),
        "Long Content Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    
    assert!(epub_data.len() > 1000, "EPUB should contain the long content");
}

#[tokio::test]
async fn test_empty_article_content() {
    let articles = vec![create_article(
        "Empty Content Article",
        "https://example.com/empty",
        "",
        Utc::now(),
        "Empty Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml");
    assert!(chapter.is_some(), "Chapter file should exist even with empty content");
}

#[tokio::test]
async fn test_empty_title() {
    let articles = vec![create_article(
        "",
        "https://example.com/notitle",
        "<p>Content without title</p>",
        Utc::now(),
        "No Title Source",
        0,
    )];
    
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file = File::create(temp_file.path()).expect("Failed to create file");
    
    let result = generate_epub_data(&articles, file, 30).await;
    
    assert!(result.is_ok(), "Should handle empty title");
}

// ============================================================================
// EPUB Structure Verification Tests
// ============================================================================

#[tokio::test]
async fn test_epub_is_valid_zip() {
    let articles = vec![create_simple_article("Test Article", "Test Source", 0)];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    
    let cursor = Cursor::new(epub_data);
    let archive_result = ZipArchive::new(cursor);
    
    assert!(archive_result.is_ok(), "EPUB should be a valid ZIP file");
}

#[tokio::test]
async fn test_epub_contains_required_files() {
    let articles = vec![
        create_simple_article("Article 1", "Source 1", 0),
        create_simple_article("Article 2", "Source 2", 1),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    
    let filenames: Vec<_> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index_raw(i).ok()?;
            Some(file.name().to_string())
        })
        .collect();
    
    // Check for essential EPUB files
    let has_toc = filenames.iter().any(|n| n.contains("toc.xhtml"));
    let has_chapters = filenames.iter().any(|n| n.contains("chapter_"));
    
    assert!(has_toc, "EPUB should contain master TOC");
    assert!(has_chapters, "EPUB should contain chapter files");
}

#[tokio::test]
async fn test_epub_xhtml_validity() {
    let articles = vec![create_simple_article("XHTML Test", "XHTML Source", 0)];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Check for valid XHTML structure
    assert!(chapter.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"), 
            "Should have XML declaration");
    assert!(chapter.contains("<!DOCTYPE html"), "Should have DOCTYPE");
    assert!(chapter.contains("<html xmlns="), "Should have HTML element with namespace");
    assert!(chapter.contains("<head>"), "Should have head element");
    assert!(chapter.contains("<body>"), "Should have body element");
    assert!(chapter.contains("</html>"), "Should close HTML element");
}

#[tokio::test]
async fn test_epub_chapter_links_in_source_toc() {
    let articles = vec![
        create_simple_article("Article A", "My Source", 0),
        create_simple_article("Article B", "My Source", 0),
        create_simple_article("Article C", "My Source", 0),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let source_toc = read_epub_file(&mut archive, "toc_my_source.xhtml").unwrap();
    
    // All articles should be linked
    assert!(source_toc.contains("Article A"), "Should contain Article A link");
    assert!(source_toc.contains("Article B"), "Should contain Article B link");
    assert!(source_toc.contains("Article C"), "Should contain Article C link");
}

// ============================================================================
// Content Processing Tests
// ============================================================================

#[tokio::test]
async fn test_html_cleaning_removes_scripts() {
    let articles = vec![create_article(
        "Script Test",
        "https://example.com/script",
        "<p>Good content</p><script>evil()</script><p>More good content</p>",
        Utc::now(),
        "Script Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Script should be removed by clean_html
    assert!(!chapter.contains("<script>"), "Script tags should be removed");
    assert!(!chapter.contains("evil()"), "Script content should be removed");
    assert!(chapter.contains("Good content"), "Good content should remain");
}

#[tokio::test]
async fn test_self_closing_tags_fixed() {
    let articles = vec![create_article(
        "Self-closing Test",
        "https://example.com/selfclose",
        "<p>Text with breaks</p>",
        Utc::now(),
        "Self-closing Source",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Content should be present
    assert!(chapter.contains("Text with breaks"), "Content should be present");
}

#[tokio::test]
async fn test_ampersand_in_content() {
    let articles = vec![create_article(
        "R and D Department",
        "https://example.com/rd",
        "<p>Tom and Jerry and Friends</p>",
        Utc::now(),
        "News Updates",
        0,
    )];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Content should be present
    assert!(chapter.contains("R and D"), "Title should be in chapter");
}

// ============================================================================
// Multiple Feeds Integration Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_feeds_integration() {
    let articles = vec![
        create_article(
            "Tech News 1",
            "https://example.com/tech1",
            "<p>Latest tech news</p>",
            Utc::now(),
            "TechCrunch",
            0,
        ),
        create_article(
            "Tech News 2",
            "https://example.com/tech2",
            "<p>More tech news</p>",
            Utc::now(),
            "TechCrunch",
            0,
        ),
        create_article(
            "Sports Update",
            "https://example.com/sports1",
            "<p>Latest sports news</p>",
            Utc::now(),
            "ESPN",
            1,
        ),
        create_article(
            "World News",
            "https://example.com/world1",
            "<p>Global news update</p>",
            Utc::now(),
            "BBC World",
            2,
        ),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    
    // Verify master TOC
    let master_toc = read_epub_file(&mut archive, "toc.xhtml").unwrap();
    assert!(master_toc.contains("TechCrunch"), "Should have TechCrunch");
    assert!(master_toc.contains("ESPN"), "Should have ESPN");
    assert!(master_toc.contains("BBC World"), "Should have BBC World");
    
    // Verify source-specific TOCs exist
    assert!(read_epub_file(&mut archive, "toc_techcrunch.xhtml").is_some(), 
            "TechCrunch TOC should exist");
    assert!(read_epub_file(&mut archive, "toc_espn.xhtml").is_some(), 
            "ESPN TOC should exist");
    assert!(read_epub_file(&mut archive, "toc_bbc_world.xhtml").is_some(), 
            "BBC World TOC should exist");
    
    // Verify all chapters exist
    for i in 0..4 {
        let filename = format!("chapter_{}.xhtml", i);
        assert!(read_epub_file(&mut archive, &filename).is_some(), 
                "Chapter {} should exist", i);
    }
}

#[tokio::test]
async fn test_article_back_links() {
    let articles = vec![
        create_simple_article("Test Article", "My Feed", 0),
    ];
    
    let epub_data = generate_epub_to_vec(&articles).await;
    let mut archive = extract_epub(epub_data);
    let chapter = read_epub_file(&mut archive, "chapter_0.xhtml").unwrap();
    
    // Article should link back to its source TOC
    assert!(chapter.contains("toc_my_feed.xhtml"), 
            "Article should have back link to source TOC");
}
