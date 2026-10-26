use std::{collections::BTreeMap, ops::Range};

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

use crate::{
    Error, Result,
    model::{CLASSES, STATUSES, TEAM_TAGS, TermSummary},
};

const MAX_SOURCE_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: usize,
    pub heading: String,
    pub level: u8,
    pub markdown: String,
    pub editable: bool,
    pub rich_safe: bool,
    #[serde(skip)]
    pub content_range: Range<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub sections: Vec<Section>,
    pub warnings: Vec<String>,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Edit {
    Section {
        id: usize,
        markdown: String,
    },
    Metadata {
        description: String,
        tags: Vec<String>,
    },
    AddDefinition,
}

fn format_error(message: impl Into<String>) -> Error {
    Error::new("source", message)
}

pub fn validate_path(path: &str) -> Result<()> {
    let slug = path
        .strip_prefix("docs/terms/")
        .and_then(|p| p.strip_suffix(".md"))
        .ok_or_else(|| format_error("A term must be a Markdown file inside docs/terms."))?;
    if slug.is_empty()
        || slug.len() > 160
        || !slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
    {
        return Err(format_error(
            "Term filenames may contain lowercase letters, digits, underscores, and hyphens only.",
        ));
    }
    Ok(())
}

/// Frontmatter delimiters must occupy a complete, unindented line.
fn frontmatter(source: &str) -> Result<(Range<usize>, usize)> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format_error("Term exceeds the 1 MiB source limit."));
    }
    let mut lines = source.split_inclusive('\n');
    let first = lines
        .next()
        .ok_or_else(|| format_error("Empty term file."))?;
    if first.trim_end() != "---" {
        return Err(format_error("Term is missing YAML frontmatter."));
    }
    let start = first.len();
    let mut offset = start;
    for line in lines {
        if line.trim_end() == "---" {
            return Ok((start..offset, offset + line.len()));
        }
        offset += line.len();
    }
    Err(format_error("Frontmatter closing delimiter is missing."))
}

fn metadata(source: &str) -> Result<(Value, Range<usize>, usize)> {
    let (range, body) = frontmatter(source)?;
    let value: Value = serde_yaml_ng::from_str(&source[range.clone()])
        .map_err(|e| format_error(format!("Could not parse term metadata: {e}")))?;
    if !value.is_mapping() {
        return Err(format_error("Frontmatter must be a YAML mapping."));
    }
    Ok((value, range, body))
}

pub fn parse(source: &str) -> Result<Document> {
    let (meta, _, body_start) = metadata(source)?;
    let title = meta["title"]
        .as_str()
        .ok_or_else(|| format_error("Term title is missing."))?
        .to_owned();
    let description = meta["description"].as_str().unwrap_or("").to_owned();
    let tags: Vec<String> = meta["tags"]
        .as_sequence()
        .map(|v| {
            v.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let body = &source[body_start..];
    let mut boundaries: Vec<(Range<usize>, String, u8)> = Vec::new();
    let mut current: Option<(Range<usize>, String, u8)> = None;
    for (event, range) in Parser::new_ext(body, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. })
                if matches!(
                    level,
                    HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                ) =>
            {
                current = Some((range.clone(), String::new(), level as u8));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, heading, _)) = &mut current {
                    heading.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(heading) = current.take() {
                    boundaries.push(heading);
                }
            }
            Event::Rule => boundaries.push((range, String::new(), 0)),
            _ => {}
        }
    }
    let mut sections = Vec::new();
    for (i, (range, heading, level)) in boundaries.iter().enumerate() {
        if *level == 0 {
            continue;
        }
        let end = boundaries.get(i + 1).map_or(body.len(), |b| b.0.start);
        let content_range = body_start + range.end..body_start + end;
        let markdown = source[content_range.clone()].trim().to_owned();
        let editable = *level != 1;
        let rich_safe = !Parser::new_ext(&markdown, Options::all()).any(|e| {
            matches!(
                e,
                Event::Html(_)
                    | Event::InlineHtml(_)
                    | Event::InlineMath(_)
                    | Event::DisplayMath(_)
            )
        }) && !markdown.contains("][")
            && !markdown.lines().any(|l| l.trim_start().starts_with("[^"));
        sections.push(Section {
            id: sections.len(),
            heading: heading.clone(),
            level: *level,
            markdown,
            editable,
            rich_safe,
            content_range,
        });
    }
    let mut warnings = Vec::new();
    if !tags.iter().any(|t| TEAM_TAGS.contains(&t.as_str())) {
        warnings.push("No review team is assigned.".into());
    }
    if tags
        .iter()
        .filter(|t| CLASSES.contains(&t.as_str()))
        .count()
        != 1
    {
        warnings.push(
            "Choose one term class; existing tags are preserved until you change them.".into(),
        );
    }
    if tags
        .iter()
        .filter(|t| STATUSES.contains(&t.as_str()))
        .count()
        != 1
    {
        warnings.push("Choose one discussion status.".into());
    }
    if !sections
        .iter()
        .any(|s| s.level == 2 && s.heading.ends_with("Definition"))
    {
        warnings.push("A numbered Definition section is missing.".into());
    }
    for section in &sections {
        if section.editable
            && section.markdown.is_empty()
            && (section.heading.ends_with("Definition") || section.heading == "Sources")
        {
            warnings.push(format!("{} is empty.", section.heading));
        }
    }
    Ok(Document {
        title,
        description,
        tags,
        sections,
        warnings,
        source: source.to_owned(),
    })
}

/// Find top-level YAML field spans while respecting multiline quoted scalars.
fn field_spans(yaml: &str) -> BTreeMap<String, Range<usize>> {
    let mut keys: Vec<(String, usize)> = Vec::new();
    let mut quote: Option<char> = None;
    let mut offset = 0;
    for line in yaml.split_inclusive('\n') {
        if quote.is_none()
            && !line.starts_with(char::is_whitespace)
            && let Some((key, value)) = line.split_once(':')
            && !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            keys.push((key.to_owned(), offset));
            let value = value.trim_start();
            if value.starts_with('"') || value.starts_with('\'') {
                quote = value.chars().next();
            } else {
                offset += line.len();
                continue;
            }
            // Skip the scalar's opening quote; scan its remainder below.
            let start = line.len() - value.len() + 1;
            scan_quote(&line[start..], &mut quote);
            offset += line.len();
            continue;
        }
        if quote.is_some() {
            scan_quote(line, &mut quote);
        }
        offset += line.len();
    }
    keys.iter()
        .enumerate()
        .map(|(i, (key, start))| {
            (
                key.clone(),
                *start..keys.get(i + 1).map_or(yaml.len(), |k| k.1),
            )
        })
        .collect()
}

fn scan_quote(text: &str, quote: &mut Option<char>) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match *quote {
            Some('"') if c == '\\' => {
                chars.next();
            }
            Some(q) if c == q => {
                if q == '\'' && chars.peek() == Some(&'\'') {
                    chars.next();
                } else {
                    *quote = None;
                    return;
                }
            }
            _ => {}
        }
    }
}

fn replace_metadata(source: &str, description: &str, tags: &[String]) -> Result<String> {
    if description.len() > 20_000
        || tags.len() > 64
        || tags
            .iter()
            .any(|t| t.len() > 160 || t.contains(['\n', '\r']))
    {
        return Err(format_error("Metadata exceeds the allowed size."));
    }
    let (_, range, _) = metadata(source)?;
    let yaml = &source[range.clone()];
    let spans = field_spans(yaml);
    let doc = parse(source)?;
    let mut replacements = Vec::new();
    if doc.description != description {
        let text = format!("description: {}\n", serde_json::to_string(description)?);
        replacements.push((
            spans
                .get("description")
                .cloned()
                .unwrap_or(yaml.len()..yaml.len()),
            text,
        ));
    }
    if doc.tags != tags {
        let mut unique = std::collections::HashSet::new();
        if tags.iter().any(|t| !unique.insert(t)) {
            return Err(format_error("Duplicate tags are not allowed."));
        }
        let text = if tags.is_empty() {
            "tags: []\n".to_owned()
        } else {
            format!(
                "tags:\n{}",
                tags.iter()
                    .map(|t| serde_json::to_string(t).map(|s| format!("- {s}\n")))
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .join("")
            )
        };
        replacements.push((
            spans.get("tags").cloned().unwrap_or(yaml.len()..yaml.len()),
            text,
        ));
    }
    let mut result = source.to_owned();
    replacements.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    for (span, text) in replacements {
        result.replace_range(range.start + span.start..range.start + span.end, &text);
    }
    parse(&result)?;
    Ok(result)
}

pub fn apply_edit(source: &str, edit: Edit) -> Result<String> {
    let doc = parse(source)?;
    let result = match edit {
        Edit::Metadata { description, tags } => replace_metadata(source, &description, &tags)?,
        Edit::Section { id, markdown } => {
            let section = doc
                .sections
                .get(id)
                .ok_or_else(|| format_error("This section no longer exists. Reload the term."))?;
            if !section.editable {
                return Err(Error::new(
                    "protected_heading",
                    "The term title and required section headings are protected.",
                ));
            }
            if Parser::new_ext(&markdown, Options::all()).any(|event| {
                matches!(
                    event,
                    Event::Start(Tag::Heading {
                        level: HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3,
                        ..
                    }) | Event::Rule
                )
            }) {
                return Err(Error::new(
                    "protected_heading",
                    "Required headings and definition separators are managed by the editor. Use Add alternative definition to extend the skeleton.",
                ));
            }
            if markdown.trim() == section.markdown {
                return Ok(source.to_owned());
            }
            let raw = &source[section.content_range.clone()];
            let leading = raw.len() - raw.trim_start().len();
            let trailing = raw.len() - raw.trim_end().len();
            let text = if raw.trim().is_empty() {
                format!("\n{}\n\n", markdown.trim())
            } else {
                format!(
                    "{}{}{}",
                    &raw[..leading],
                    markdown.trim(),
                    &raw[raw.len() - trailing..]
                )
            };
            let mut result = source.to_owned();
            result.replace_range(section.content_range.clone(), &text);
            result
        }
        Edit::AddDefinition => {
            let next = doc
                .sections
                .iter()
                .filter(|s| s.level == 2 && s.heading.ends_with("Definition"))
                .count()
                + 1;
            format!(
                "{}\n\n___\n\n## {next} Definition\n\n### Notes\n\n### Examples\n\n### Sources\n",
                source.trim_end()
            )
        }
    };
    parse(&result)?;
    Ok(result)
}

pub fn skeleton(title: &str, tags: &[String]) -> Result<String> {
    let title = title.trim();
    if title.is_empty() || title.len() > 200 || title.contains(['\n', '\r', '#']) {
        return Err(format_error(
            "Enter a term title, without Markdown headings or line breaks.",
        ));
    }
    let tags = if tags.is_empty() {
        vec!["core".to_owned(), "to be defined".to_owned()]
    } else {
        tags.to_vec()
    };
    let source = format!(
        "---\ntitle: {}\ndescription: \"\"\ntags:\n{}---\n\n# {title}\n\n## 1 Definition\n\n### Notes\n\n### Examples\n\n### Sources\n",
        serde_json::to_string(title)?,
        tags.iter()
            .map(|t| serde_json::to_string(t).map(|s| format!("- {s}\n")))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("")
    );
    parse(&source)?;
    Ok(source)
}

pub fn summary(
    path: &str,
    source: &str,
    changed: bool,
    pending: bool,
    is_new: bool,
) -> Result<TermSummary> {
    let doc = parse(source)?;
    Ok(TermSummary {
        path: path.to_owned(),
        title: doc.title,
        description: doc.description,
        tags: doc.tags,
        search_text: source.to_owned(),
        changed,
        pending,
        is_new,
    })
}

pub fn slug(title: &str) -> String {
    use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};
    let mut result = String::new();
    for c in title
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
    {
        if c.is_ascii_alphanumeric() {
            result.push(c);
        } else if !result.is_empty() && !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_owned()
}
