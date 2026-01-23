use askama::Template;


#[derive(Template)]
#[template(path = "xhtml_wrapper.html")]
pub struct XhtmlWrapper<'a> {
    pub title: &'a str,
    pub content: &'a str,
}


pub struct TocEntry {
    pub toc_filename: String,
    pub name: String,
}


#[derive(Template)]
#[template(path = "master_toc.html")]
pub struct MasterToc {
    pub sources: Vec<TocEntry>,
}


pub struct ArticleEntry {
    pub filename: String,
    pub title: String,
}


#[derive(Template)]
#[template(path = "source_toc.html")]
pub struct SourceToc {
    pub source_name: String,
    pub articles: Vec<ArticleEntry>,
}


#[derive(Template)]
#[template(path = "article.html")]
pub struct ArticleTemplate<'a> {
    pub title: &'a str,
    pub source: &'a str,
    pub pub_date: String,
    pub content: &'a str,
    pub original_link: &'a str,
    pub back_link: String,
}
