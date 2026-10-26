//! Presentation-only Markdown rendering. It never serializes a term back to source.
use crate::{Result, source};
use pulldown_cmark::{Options, Parser, html};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub sections: Vec<Section>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub key: String,
    pub heading: String,
    pub level: u8,
    pub html: String,
}
pub fn page(text: &str) -> Result<Page> {
    let document = source::parse(text)?;
    let mut definition = 0;
    let mut occurrences = BTreeMap::new();
    let mut sections = Vec::new();
    for section in document.sections {
        if section.level == 1 && section.markdown.is_empty() {
            continue;
        }
        if section.level == 2 {
            definition += 1;
        }
        let key = format!("{definition}:{}:{}", section.level, section.heading);
        let occurrence = occurrences.entry(key.clone()).or_insert(0);
        *occurrence += 1;
        let mut rendered = String::new();
        html::push_html(
            &mut rendered,
            Parser::new_ext(&section.markdown, Options::all()),
        );
        sections.push(Section {
            key: format!("{key}:{occurrence}"),
            heading: if section.level == 1 {
                String::new()
            } else {
                section.heading
            },
            level: section.level,
            html: rendered,
        });
    }
    Ok(Page {
        title: document.title,
        description: document.description,
        tags: document.tags,
        sections,
    })
}
