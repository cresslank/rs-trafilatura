//! Opt-in extraction of bounded structured facts from the pre-cleaning DOM.
//!
//! These facts are intentionally separate from the main text/HTML/Markdown
//! extraction path. They preserve useful attributes for downstream structured
//! extraction without mutating the default parser output.

use crate::dom::{Document, Selection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredFactsOptions {
    pub max_links: usize,
    pub max_images: usize,
    pub max_metadata_facts: usize,
    pub max_tables: usize,
    #[serde(default)]
    pub max_media: usize,
    #[serde(default)]
    pub max_sections: usize,
    pub max_table_rows: usize,
    pub max_table_cells_per_row: usize,
    pub max_field_chars: usize,
    pub max_json_ld_scripts: usize,
    pub max_json_ld_bytes: usize,
    pub max_json_ld_depth: usize,
    pub max_total_chars: usize,
}

impl Default for StructuredFactsOptions {
    fn default() -> Self {
        Self {
            max_links: 500,
            max_images: 50,
            max_metadata_facts: 80,
            max_tables: 10,
            max_media: 25,
            max_sections: 50,
            max_table_rows: 20,
            max_table_cells_per_row: 12,
            max_field_chars: 256,
            max_json_ld_scripts: 8,
            max_json_ld_bytes: 16 * 1024,
            max_json_ld_depth: 8,
            max_total_chars: 64 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredFacts {
    pub links: Vec<LinkFact>,
    pub images: Vec<ImageFact>,
    pub metadata: Vec<MetadataFact>,
    pub tables: Vec<TableFact>,
    #[serde(default)]
    pub media: Vec<MediaFact>,
    #[serde(default)]
    pub sections: Vec<SectionFact>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomRegion {
    Main,
    Article,
    Header,
    Footer,
    Nav,
    Aside,
    Form,
    Head,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkClass {
    Download,
    Attachment,
    Pdf,
    Contact,
    Email,
    Phone,
    Pricing,
    Documentation,
    Api,
    SourceRepository,
    License,
    Social,
    Navigation,
    Breadcrumb,
    Pagination,
    Legal,
    Share,
    Login,
    Anchor,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryParamFact {
    pub key: String,
    pub value: String,
    pub decoded_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkFact {
    pub text: String,
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<QueryParamFact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest_heading: Option<String>,
    #[serde(default)]
    pub dom_region: DomRegion,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<LinkClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageFact {
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest_heading: Option<String>,
    #[serde(default)]
    pub dom_region: DomRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataKind {
    ProductName,
    ProductPrice,
    ProductCurrency,
    ProductSku,
    ProductDescription,
    License,
    CanonicalUrl,
    ContactEmail,
    ArticleAuthor,
    PublishedDate,
    ModifiedDate,
    ImageUrl,
    PageTitle,
    Description,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataFact {
    pub source: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MetadataKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableFact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest_heading: Option<String>,
    #[serde(default)]
    pub dom_region: DomRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaSourceFact {
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaTrackFact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srclang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaFact {
    pub kind: MediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<MediaSourceFact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<MediaTrackFact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest_heading: Option<String>,
    #[serde(default)]
    pub dom_region: DomRegion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionFact {
    pub heading: String,
    pub level: u8,
    #[serde(default)]
    pub dom_region: DomRegion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredFactsRenderOptions {
    pub max_links: usize,
    pub max_metadata: usize,
    pub max_media: usize,
    pub max_tables: usize,
    pub max_chars: usize,
}

impl Default for StructuredFactsRenderOptions {
    fn default() -> Self {
        Self {
            max_links: 25,
            max_metadata: 30,
            max_media: 10,
            max_tables: 5,
            max_chars: 8 * 1024,
        }
    }
}

#[must_use]
pub(crate) fn extract_structured_facts(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
) -> StructuredFacts {
    let mut facts = StructuredFacts::default();
    let mut budget = CharBudget::new(options.max_total_chars);
    let headings = collect_headings(doc, options.max_sections, options.max_field_chars);

    facts.sections = headings.iter().map(|h| h.fact.clone()).collect();
    let section_chars = facts.sections.iter().map(section_fact_chars).sum();
    let _ = budget.try_consume(section_chars);

    extract_links(
        doc,
        base_url,
        options,
        &mut budget,
        &headings,
        &mut facts.links,
    );
    extract_images(
        doc,
        base_url,
        options,
        &mut budget,
        &headings,
        &mut facts.images,
    );
    extract_media(
        doc,
        base_url,
        options,
        &mut budget,
        &headings,
        &mut facts.media,
    );
    extract_metadata(doc, base_url, options, &mut budget, &mut facts.metadata);
    extract_tables(doc, options, &mut budget, &headings, &mut facts.tables);

    facts
}

#[must_use]
pub fn render_structured_facts_for_extraction(
    facts: &StructuredFacts,
    options: &StructuredFactsRenderOptions,
) -> String {
    let mut out = String::new();
    out.push_str("## Extracted Page Evidence\n");

    let high_value_links: Vec<&LinkFact> = facts
        .links
        .iter()
        .filter(|l| is_high_value_link(l))
        .collect();
    if !high_value_links.is_empty() {
        out.push_str("\nLinks:\n");
        for link in high_value_links.iter().take(options.max_links) {
            append_capped(&mut out, options.max_chars, "- ");
            let label = if link.text.is_empty() {
                "link"
            } else {
                &link.text
            };
            append_capped(&mut out, options.max_chars, label);
            append_capped(&mut out, options.max_chars, " → ");
            append_capped(&mut out, options.max_chars, &link.href);
            if let Some(email) = &link.email {
                append_capped(&mut out, options.max_chars, " email=");
                append_capped(&mut out, options.max_chars, email);
            }
            if let Some(phone) = &link.phone {
                append_capped(&mut out, options.max_chars, " phone=");
                append_capped(&mut out, options.max_chars, phone);
            }
            for param in &link.query {
                append_capped(&mut out, options.max_chars, " ");
                append_capped(&mut out, options.max_chars, &param.key);
                append_capped(&mut out, options.max_chars, "=\"");
                append_capped(&mut out, options.max_chars, &param.decoded_value);
                append_capped(&mut out, options.max_chars, "\"");
            }
            if let Some(heading) = &link.nearest_heading {
                append_capped(&mut out, options.max_chars, " section=\"");
                append_capped(&mut out, options.max_chars, heading);
                append_capped(&mut out, options.max_chars, "\"");
            }
            append_capped(&mut out, options.max_chars, "\n");
        }
    }

    if !facts.metadata.is_empty() {
        out.push_str("\nMetadata and schema facts:\n");
        for meta in facts.metadata.iter().take(options.max_metadata) {
            append_capped(&mut out, options.max_chars, "- ");
            let label = meta.label.as_deref().unwrap_or(&meta.path);
            append_capped(&mut out, options.max_chars, label);
            append_capped(&mut out, options.max_chars, ": ");
            append_capped(&mut out, options.max_chars, &meta.value);
            append_capped(&mut out, options.max_chars, " (");
            append_capped(&mut out, options.max_chars, &meta.source);
            append_capped(&mut out, options.max_chars, ")\n");
        }
    }

    if !facts.media.is_empty() {
        out.push_str("\nMedia:\n");
        for media in facts.media.iter().take(options.max_media) {
            append_capped(&mut out, options.max_chars, "- ");
            append_capped(
                &mut out,
                options.max_chars,
                match media.kind {
                    MediaKind::Video => "video",
                    MediaKind::Audio => "audio",
                },
            );
            if let Some(caption) = &media.caption {
                append_capped(&mut out, options.max_chars, " caption=\"");
                append_capped(&mut out, options.max_chars, caption);
                append_capped(&mut out, options.max_chars, "\"");
            }
            if let Some(source) = media.sources.first() {
                append_capped(&mut out, options.max_chars, " src=");
                append_capped(&mut out, options.max_chars, &source.src);
            }
            append_capped(&mut out, options.max_chars, "\n");
        }
    }

    if !facts.tables.is_empty() {
        out.push_str("\nTables:\n");
        for table in facts.tables.iter().take(options.max_tables) {
            append_capped(&mut out, options.max_chars, "- ");
            append_capped(
                &mut out,
                options.max_chars,
                table.caption.as_deref().unwrap_or("table"),
            );
            if !table.headers.is_empty() {
                append_capped(&mut out, options.max_chars, " headers: ");
                append_capped(&mut out, options.max_chars, &table.headers.join(" | "));
            }
            append_capped(&mut out, options.max_chars, "\n");
        }
    }

    let rendered_links = high_value_links.len().min(options.max_links);
    let omitted_links = facts.links.len().saturating_sub(rendered_links);
    if omitted_links > 0 {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for link in &facts.links {
            for class in &link.classes {
                if matches!(
                    class,
                    LinkClass::Navigation
                        | LinkClass::Legal
                        | LinkClass::Social
                        | LinkClass::Share
                        | LinkClass::Login
                        | LinkClass::Pagination
                ) {
                    *counts.entry(link_class_name(*class)).or_insert(0) += 1;
                }
            }
        }
        append_capped(&mut out, options.max_chars, "\nomitted low-value links: ");
        append_capped(&mut out, options.max_chars, &omitted_links.to_string());
        if !counts.is_empty() {
            append_capped(&mut out, options.max_chars, " (");
            let parts: Vec<String> = counts
                .into_iter()
                .map(|(k, v)| format!("{v} {k}"))
                .collect();
            append_capped(&mut out, options.max_chars, &parts.join(", "));
            append_capped(&mut out, options.max_chars, ")");
        }
        append_capped(&mut out, options.max_chars, "\n");
    }

    if out.len() > options.max_chars {
        out.truncate(options.max_chars);
    }
    out
}

fn append_capped(out: &mut String, max_chars: usize, value: &str) {
    if out.len() >= max_chars {
        return;
    }
    let remaining = max_chars - out.len();
    if value.len() <= remaining {
        out.push_str(value);
    } else {
        let mut boundary = remaining;
        while !value.is_char_boundary(boundary) {
            boundary -= 1;
        }
        out.push_str(&value[..boundary]);
    }
}

fn extract_links(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    headings: &[HeadingRef],
    out: &mut Vec<LinkFact>,
) {
    let mut seen = HashSet::new();
    for node in doc.select("a[href]").iter() {
        if out.len() >= options.max_links || budget.exhausted() {
            break;
        }
        if is_hidden_context(&node) {
            continue;
        }
        let Some(raw_href) = node.attr("href") else {
            continue;
        };
        let raw_href = raw_href.trim();
        let Some(normalized_href) =
            normalize_url(raw_href, base_url, &["http", "https", "mailto", "tel"])
        else {
            continue;
        };
        let text = clean_field(&node.text(), options.max_field_chars);
        let title = clean_optional(node.attr("title").as_deref(), options.max_field_chars);
        let explicit_download =
            clean_optional(node.attr("download").as_deref(), options.max_field_chars);
        let mut parts = parse_url_parts(raw_href, &normalized_href, options.max_field_chars);
        let download = explicit_download
            .or_else(|| downloadable_filename(&normalized_href, options.max_field_chars));
        if download.is_some() {
            parts.filename.clone_from(&download);
        }
        let region = dom_region(&node);
        let classes = classify_link(
            &text,
            &normalized_href,
            title.as_deref(),
            download.as_deref(),
            region,
            &node,
        );
        if text.is_empty()
            && title.is_none()
            && download.is_none()
            && !href_is_intrinsically_useful(&normalized_href)
            && !classes.iter().any(|class| {
                matches!(
                    class,
                    LinkClass::Email | LinkClass::Phone | LinkClass::Anchor
                )
            })
        {
            continue;
        }
        let href = clean_field(&normalized_href, options.max_field_chars);
        let raw_href_clean = clean_field(raw_href, options.max_field_chars);
        let key = format!("{href}\u{0}{text}\u{0}{title:?}\u{0}{download:?}\u{0}{region:?}");
        if !seen.insert(key) {
            continue;
        }
        let fact = LinkFact {
            text,
            href,
            raw_href: Some(raw_href_clean).filter(|raw| raw != &normalized_href),
            title,
            download,
            scheme: parts.scheme,
            host: parts.host,
            path: parts.path,
            filename: parts.filename,
            query: parts.query,
            fragment: parts.fragment,
            email: parts.email,
            phone: parts.phone,
            nearest_heading: nearest_heading(&node, headings),
            dom_region: region,
            classes,
        };
        if budget.try_consume(link_fact_chars(&fact)) {
            out.push(fact);
        }
    }
}

fn extract_images(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    headings: &[HeadingRef],
    out: &mut Vec<ImageFact>,
) {
    let mut seen = HashSet::new();
    for node in doc.select("img[src], img[data-src]").iter() {
        if out.len() >= options.max_images || budget.exhausted() {
            break;
        }
        if is_hidden_context(&node) {
            continue;
        }
        let raw_src = node.attr("src").or_else(|| node.attr("data-src"));
        let Some(raw_src) = raw_src else {
            continue;
        };
        let Some(normalized_src) = normalize_url(raw_src.trim(), base_url, &["http", "https"])
        else {
            continue;
        };
        let fact = ImageFact {
            src: clean_field(&normalized_src, options.max_field_chars),
            alt: clean_optional(node.attr("alt").as_deref(), options.max_field_chars),
            title: clean_optional(node.attr("title").as_deref(), options.max_field_chars),
            caption: find_caption(&node, options.max_field_chars),
            nearest_heading: nearest_heading(&node, headings),
            dom_region: dom_region(&node),
        };
        if fact.alt.is_none() && fact.title.is_none() && fact.caption.is_none() {
            continue;
        }
        let key = format!(
            "{}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
            fact.src, fact.alt, fact.title, fact.caption
        );
        if !seen.insert(key) {
            continue;
        }
        if budget.try_consume(image_fact_chars(&fact)) {
            out.push(fact);
        }
    }
}

fn extract_media(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    headings: &[HeadingRef],
    out: &mut Vec<MediaFact>,
) {
    let mut seen = HashSet::new();
    for node in doc.select("video, audio").iter() {
        if out.len() >= options.max_media || budget.exhausted() || is_hidden_context(&node) {
            continue;
        }
        let tag = tag_name(&node);
        let kind = if tag == "audio" {
            MediaKind::Audio
        } else {
            MediaKind::Video
        };
        let mut sources = Vec::new();
        if let Some(src) = node.attr("src") {
            if let Some(normalized) = normalize_url(src.trim(), base_url, &["http", "https"]) {
                sources.push(MediaSourceFact {
                    src: clean_field(&normalized, options.max_field_chars),
                    mime_type: clean_optional(
                        node.attr("type").as_deref(),
                        options.max_field_chars,
                    ),
                });
            }
        }
        for source in node.select("source[src]").iter() {
            if !belongs_to_media(&source, &node) || is_hidden_context(&source) {
                continue;
            }
            if let Some(src) = source.attr("src") {
                if let Some(normalized) = normalize_url(src.trim(), base_url, &["http", "https"]) {
                    sources.push(MediaSourceFact {
                        src: clean_field(&normalized, options.max_field_chars),
                        mime_type: clean_optional(
                            source.attr("type").as_deref(),
                            options.max_field_chars,
                        ),
                    });
                }
            }
        }
        let mut tracks = Vec::new();
        for track in node.select("track").iter() {
            if !belongs_to_media(&track, &node) || is_hidden_context(&track) {
                continue;
            }
            let src = track.attr("src").and_then(|src| {
                normalize_url(src.trim(), base_url, &["http", "https"])
                    .map(|url| clean_field(&url, options.max_field_chars))
            });
            tracks.push(MediaTrackFact {
                src,
                kind: clean_optional(track.attr("kind").as_deref(), options.max_field_chars),
                srclang: clean_optional(track.attr("srclang").as_deref(), options.max_field_chars),
                label: clean_optional(track.attr("label").as_deref(), options.max_field_chars),
            });
        }
        let poster = node.attr("poster").and_then(|poster| {
            normalize_url(poster.trim(), base_url, &["http", "https"])
                .map(|url| clean_field(&url, options.max_field_chars))
        });
        if sources.is_empty() && tracks.is_empty() && poster.is_none() {
            continue;
        }
        let fact = MediaFact {
            kind,
            sources,
            tracks,
            poster,
            caption: find_caption(&node, options.max_field_chars),
            nearest_heading: nearest_heading(&node, headings),
            dom_region: dom_region(&node),
        };
        let key = format!(
            "{:?}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
            fact.kind, fact.sources, fact.poster, fact.caption
        );
        if !seen.insert(key) {
            continue;
        }
        if budget.try_consume(media_fact_chars(&fact)) {
            out.push(fact);
        }
    }
}

fn extract_tables(
    doc: &Document,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    headings: &[HeadingRef],
    out: &mut Vec<TableFact>,
) {
    for table in doc.select("table").iter() {
        if out.len() >= options.max_tables || budget.exhausted() {
            break;
        }
        if is_hidden_context(&table) {
            continue;
        }
        let caption = clean_optional(
            table
                .select("caption")
                .iter()
                .find(|caption| belongs_to_table(caption, &table) && !is_hidden_context(caption))
                .map(|caption| caption.text())
                .unwrap_or_default()
                .trim()
                .into(),
            options.max_field_chars,
        );
        let headers = extract_table_headers(&table, options);
        let rows = extract_table_rows(&table, options, !headers.is_empty());
        if headers.is_empty() && rows.is_empty() && caption.is_none() {
            continue;
        }
        let fact = TableFact {
            caption,
            headers,
            rows,
            nearest_heading: nearest_heading(&table, headings),
            dom_region: dom_region(&table),
        };
        if budget.try_consume(table_fact_chars(&fact)) {
            out.push(fact);
        }
    }
}

fn extract_metadata(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
) {
    extract_link_tag_metadata(doc, base_url, options, budget, out);
    extract_json_ld_metadata(doc, base_url, options, budget, out);
    extract_meta_tag_metadata(doc, base_url, options, budget, out);
}

fn extract_link_tag_metadata(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
) {
    for node in doc.select("link[rel][href]").iter() {
        if out.len() >= options.max_metadata_facts || budget.exhausted() {
            break;
        }
        let Some(rel) = node.attr("rel") else {
            continue;
        };
        let Some(raw_href) = node.attr("href") else {
            continue;
        };
        let rel_lower = rel.to_ascii_lowercase();
        let path = if rel_lower.split_whitespace().any(|v| v == "canonical") {
            "canonical"
        } else if rel_lower.split_whitespace().any(|v| v == "license") {
            "license"
        } else {
            continue;
        };
        let value = normalize_metadata_value(path, &raw_href, base_url, options.max_field_chars);
        if value.is_empty() {
            continue;
        }
        let fact = MetadataFact {
            source: "link".to_string(),
            path: path.to_string(),
            item_type: None,
            value,
            kind: metadata_kind(None, path),
            label: metadata_label(metadata_kind(None, path), path),
        };
        if budget.try_consume(metadata_fact_chars(&fact)) {
            out.push(fact);
        }
    }
}

fn extract_json_ld_metadata(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
) {
    let mut seen = HashSet::new();
    let mut scripts_seen = 0;
    for script in doc.select("script").iter() {
        if out.len() >= options.max_metadata_facts || budget.exhausted() {
            break;
        }
        if !is_json_ld_script(&script) {
            continue;
        }
        scripts_seen += 1;
        if scripts_seen > options.max_json_ld_scripts {
            break;
        }
        let raw = script.text();
        if raw.len() > options.max_json_ld_bytes {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        collect_json_ld_values(
            &value, None, None, 0, base_url, options, budget, out, &mut seen,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_json_ld_values(
    value: &Value,
    path: Option<&str>,
    item_type: Option<&str>,
    depth: usize,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
    seen: &mut HashSet<String>,
) {
    if depth > options.max_json_ld_depth
        || out.len() >= options.max_metadata_facts
        || budget.exhausted()
    {
        return;
    }
    match value {
        Value::Object(map) => {
            let current_type = map.get("@type").and_then(Value::as_str).or(item_type);
            for (key, child) in map {
                if key == "@context" || key == "@type" {
                    continue;
                }
                let child_path = join_json_ld_path(path, key);
                collect_json_ld_values(
                    child,
                    Some(&child_path),
                    current_type,
                    depth + 1,
                    base_url,
                    options,
                    budget,
                    out,
                    seen,
                );
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_json_ld_values(
                    child,
                    path,
                    item_type,
                    depth + 1,
                    base_url,
                    options,
                    budget,
                    out,
                    seen,
                );
            }
        }
        _ => {
            let Some(path) = path else {
                return;
            };
            if !allowed_json_ld_path(path) {
                return;
            }
            let Some(raw_value) = json_scalar_to_string(value) else {
                return;
            };
            let normalized_value =
                normalize_metadata_value(path, &raw_value, base_url, options.max_field_chars);
            if normalized_value.is_empty() {
                return;
            }
            let prefixed = prefixed_path(item_type, path);
            let key = format!("json_ld\u{0}{prefixed}\u{0}{normalized_value}");
            if !seen.insert(key) {
                return;
            }
            let kind = metadata_kind(item_type, path);
            let fact = MetadataFact {
                source: "json_ld".to_string(),
                path: prefixed,
                item_type: item_type.map(ToString::to_string),
                value: normalized_value,
                kind,
                label: metadata_label(kind, path),
            };
            if budget.try_consume(metadata_fact_chars(&fact)) {
                out.push(fact);
            }
        }
    }
}

fn extract_meta_tag_metadata(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
) {
    let mut seen = HashSet::new();
    for node in doc.select("meta").iter() {
        if out.len() >= options.max_metadata_facts || budget.exhausted() {
            break;
        }
        let name = node
            .attr("property")
            .or_else(|| node.attr("name"))
            .map(|value| value.trim().to_ascii_lowercase());
        let Some(name) = name else { continue };
        if !allowed_meta_name(&name) {
            continue;
        }
        let Some(content) = node.attr("content") else {
            continue;
        };
        let normalized =
            normalize_metadata_value(&name, &content, base_url, options.max_field_chars);
        if normalized.is_empty() {
            continue;
        }
        let key = format!("meta\u{0}{name}\u{0}{normalized}");
        if !seen.insert(key) {
            continue;
        }
        let kind = metadata_kind(None, &name);
        let fact = MetadataFact {
            source: "meta".to_string(),
            path: name.clone(),
            item_type: None,
            value: normalized,
            kind,
            label: metadata_label(kind, &name),
        };
        if budget.try_consume(metadata_fact_chars(&fact)) {
            out.push(fact);
        }
    }
}

fn extract_table_headers(table: &Selection, options: &StructuredFactsOptions) -> Vec<String> {
    table
        .select("th")
        .iter()
        .filter(|cell| belongs_to_table(cell, table) && !is_hidden_context(cell))
        .filter_map(|cell| {
            let text = extract_table_cell_text(&cell, table, options.max_field_chars);
            (!text.is_empty()).then_some(text)
        })
        .take(options.max_table_cells_per_row)
        .collect()
}

fn extract_table_rows(
    table: &Selection,
    options: &StructuredFactsOptions,
    skip_header_rows: bool,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for row in table.select("tr").iter() {
        if rows.len() >= options.max_table_rows {
            break;
        }
        if !belongs_to_table(&row, table) || is_hidden_context(&row) {
            continue;
        }
        if skip_header_rows
            && row
                .select("th")
                .iter()
                .any(|cell| belongs_to_table(&cell, table) && !is_hidden_context(&cell))
        {
            continue;
        }
        let cells: Vec<String> = row
            .select("td, th")
            .iter()
            .filter(|cell| belongs_to_table(cell, table) && !is_hidden_context(cell))
            .filter_map(|cell| {
                let text = extract_table_cell_text(&cell, table, options.max_field_chars);
                (!text.is_empty()).then_some(text)
            })
            .take(options.max_table_cells_per_row)
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    rows
}

fn extract_table_cell_text(cell: &Selection, table: &Selection, max_chars: usize) -> String {
    let Some(table_key) = selection_key(table) else {
        return clean_field(&cell.text(), max_chars);
    };
    let Some(cell_key) = selection_key(cell) else {
        return clean_field(&cell.text(), max_chars);
    };
    let mut parts = Vec::new();
    for node in cell.select("*").iter() {
        let Some(key) = selection_key(&node) else {
            continue;
        };
        if key == cell_key {
            continue;
        }
        let mut current = node.nodes().first().copied();
        let mut skip = false;
        while let Some(ancestor) = current {
            if ancestor.is_element()
                && ancestor
                    .node_name()
                    .is_some_and(|tag| tag.eq_ignore_ascii_case("table"))
            {
                if key != table_key {
                    skip = true;
                }
                break;
            }
            if key == cell_key {
                break;
            }
            current = ancestor.parent();
        }
        if !skip {
            let text = node.text();
            if !text.trim().is_empty() {
                parts.push(text.to_string());
            }
        }
    }

    let mut joined = cell.text().to_string();
    for nested_table in cell.select("table").iter() {
        if !belongs_to_table(&nested_table, table) {
            let nested_text = nested_table.text().to_string();
            if !nested_text.trim().is_empty() {
                joined = joined.replacen(&nested_text, " ", 1);
            }
        }
    }
    clean_field(&joined, max_chars)
}

#[derive(Clone)]
struct HeadingRef {
    key: (dom_query::NodeId, usize),
    fact: SectionFact,
}

fn collect_headings(doc: &Document, max_sections: usize, max_chars: usize) -> Vec<HeadingRef> {
    let mut out = Vec::new();
    for heading in doc.select("h1, h2, h3, h4, h5, h6").iter() {
        if out.len() >= max_sections || is_hidden_context(&heading) {
            continue;
        }
        let text = clean_field(&heading.text(), max_chars);
        if text.is_empty() {
            continue;
        }
        let Some(key) = selection_key(&heading) else {
            continue;
        };
        let level = tag_name(&heading)
            .strip_prefix('h')
            .and_then(|n| n.parse::<u8>().ok())
            .unwrap_or(0);
        out.push(HeadingRef {
            key,
            fact: SectionFact {
                heading: text,
                level,
                dom_region: dom_region(&heading),
            },
        });
    }
    out
}

fn nearest_heading(sel: &Selection, headings: &[HeadingRef]) -> Option<String> {
    let key = selection_key(sel)?;
    headings
        .iter()
        .rfind(|heading| heading.key.1 == key.1 && heading.key.0 <= key.0)
        .map(|heading| heading.fact.heading.clone())
}

fn find_caption(sel: &Selection, max_chars: usize) -> Option<String> {
    let parent = sel.parent();
    if parent.length() > 0 && tag_name(&parent) == "figure" {
        let caption = parent.select("figcaption").first().text();
        let cleaned = clean_field(&caption, max_chars);
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }
    None
}

fn is_json_ld_script(script: &Selection) -> bool {
    script.attr("type").is_some_and(|script_type| {
        script_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .eq_ignore_ascii_case("application/ld+json")
    })
}

fn join_json_ld_path(parent: Option<&str>, key: &str) -> String {
    if matches!(key, "@graph" | "graph" | "itemListElement") {
        return parent.unwrap_or_default().to_string();
    }
    parent
        .filter(|parent| !parent.is_empty())
        .map_or_else(|| key.to_string(), |parent| format!("{parent}.{key}"))
}

fn downloadable_filename(href: &str, max_chars: usize) -> Option<String> {
    if !href_is_intrinsically_useful(href)
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
    {
        return None;
    }
    Url::parse(href)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(Iterator::last)
                .map(std::string::ToString::to_string)
        })
        .or_else(|| href.rsplit('/').next().map(str::to_string))
        .map(|name| clean_field(&name, max_chars))
        .filter(|name| !name.is_empty())
}

fn href_is_intrinsically_useful(href: &str) -> bool {
    if href.starts_with("mailto:") || href.starts_with("tel:") {
        return true;
    }
    let path = Url::parse(href).ok().map_or_else(
        || href.to_ascii_lowercase(),
        |url| url.path().to_ascii_lowercase(),
    );
    [
        ".csv", ".doc", ".docx", ".json", ".pdf", ".ppt", ".pptx", ".txt", ".xls", ".xlsx", ".xml",
        ".zip", ".tar", ".gz", ".tgz", ".whl",
    ]
    .iter()
    .any(|suffix| path.ends_with(suffix))
}

fn belongs_to_table(sel: &Selection, table: &Selection) -> bool {
    let Some(table_key) = selection_key(table) else {
        return false;
    };
    let mut current = sel.clone();
    while current.length() > 0 {
        let tag = tag_name(&current);
        if tag == "table" {
            return selection_key(&current).is_some_and(|key| key == table_key);
        }
        let parent = current.parent();
        if parent.length() == 0 {
            break;
        }
        current = parent;
    }
    false
}

fn belongs_to_media(sel: &Selection, media: &Selection) -> bool {
    let Some(media_key) = selection_key(media) else {
        return false;
    };
    let mut current = sel.parent();
    while current.length() > 0 {
        let tag = tag_name(&current);
        if matches!(tag.as_str(), "video" | "audio") {
            return selection_key(&current).is_some_and(|key| key == media_key);
        }
        current = current.parent();
    }
    false
}

fn selection_key(sel: &Selection) -> Option<(dom_query::NodeId, usize)> {
    sel.nodes()
        .first()
        .map(|node| (node.id, std::ptr::from_ref(node.tree) as usize))
}

fn is_hidden_context(sel: &Selection) -> bool {
    let mut current = sel.clone();
    while current.length() > 0 {
        if is_hidden_node(&current) {
            return true;
        }
        let parent = current.parent();
        if parent.length() == 0 {
            break;
        }
        current = parent;
    }
    false
}

fn is_hidden_node(sel: &Selection) -> bool {
    let tag = tag_name(sel);
    if matches!(tag.as_str(), "script" | "style" | "template" | "noscript") {
        return true;
    }
    if sel.has_attr("hidden") {
        return true;
    }
    if sel
        .attr("aria-hidden")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    {
        return true;
    }
    if let Some(style) = sel.attr("style") {
        let compact = style.to_ascii_lowercase().replace(char::is_whitespace, "");
        if compact.contains("display:none") || compact.contains("visibility:hidden") {
            return true;
        }
    }
    false
}

fn normalize_url(raw: &str, base_url: Option<&str>, allowed_schemes: &[&str]) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    if let Ok(url) = Url::parse(raw) {
        return allowed_schemes
            .contains(&url.scheme())
            .then(|| url.to_string());
    }
    let base = base_url.and_then(|base| Url::parse(base).ok())?;
    let joined = base.join(raw).ok()?;
    allowed_schemes
        .contains(&joined.scheme())
        .then(|| joined.to_string())
}

struct UrlParts {
    scheme: Option<String>,
    host: Option<String>,
    path: Option<String>,
    filename: Option<String>,
    query: Vec<QueryParamFact>,
    fragment: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

fn parse_url_parts(raw_href: &str, normalized_href: &str, max_chars: usize) -> UrlParts {
    let mut parts = UrlParts {
        scheme: None,
        host: None,
        path: None,
        filename: None,
        query: Vec::new(),
        fragment: None,
        email: None,
        phone: None,
    };
    if let Ok(url) = Url::parse(normalized_href) {
        parts.scheme = Some(url.scheme().to_string());
        parts.host = url.host_str().map(ToString::to_string);
        let path = url.path();
        if !path.is_empty() {
            parts.path = Some(clean_field(path, max_chars));
        }
        parts.filename = url
            .path_segments()
            .and_then(Iterator::last)
            .filter(|segment| !segment.is_empty())
            .map(|segment| clean_field(segment, max_chars));
        parts.fragment = url.fragment().map(|value| clean_field(value, max_chars));
        parts.query = query_params(url.query(), max_chars);
        if url.scheme() == "mailto" {
            parts.email =
                Some(clean_field(url.path(), max_chars)).filter(|value| !value.is_empty());
        } else if url.scheme() == "tel" {
            parts.phone =
                Some(clean_field(url.path(), max_chars)).filter(|value| !value.is_empty());
        }
    } else if let Some(rest) = raw_href.strip_prefix("mailto:") {
        let mut split = rest.splitn(2, '?');
        parts.scheme = Some("mailto".to_string());
        parts.email = split
            .next()
            .map(|value| clean_field(value, max_chars))
            .filter(|value| !value.is_empty());
        parts.query = query_params(split.next(), max_chars);
    } else if let Some(rest) = raw_href.strip_prefix("tel:") {
        parts.scheme = Some("tel".to_string());
        parts.phone = Some(clean_field(rest, max_chars)).filter(|value| !value.is_empty());
    }
    parts
}

fn query_params(query: Option<&str>, max_chars: usize) -> Vec<QueryParamFact> {
    query.map_or_else(Vec::new, |query| {
        query
            .split('&')
            .filter(|part| !part.is_empty())
            .filter_map(|part| {
                let mut split = part.splitn(2, '=');
                let key = split.next().unwrap_or_default();
                let value = split.next().unwrap_or_default();
                if key.is_empty() {
                    return None;
                }
                let decoded_key = percent_decode(key);
                let decoded_value = percent_decode(value);
                Some(QueryParamFact {
                    key: clean_field(&decoded_key, max_chars),
                    value: clean_field(value, max_chars),
                    decoded_value: clean_field(&decoded_value, max_chars),
                })
            })
            .collect()
    })
}

fn percent_decode(value: &str) -> String {
    let replaced = value.replace('+', " ");
    let bytes = replaced.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&replaced[i + 1..i + 3], 16) {
                out.push(hex);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn normalize_metadata_value(
    path: &str,
    value: &str,
    base_url: Option<&str>,
    max_chars: usize,
) -> String {
    let cleaned = clean_field(value, max_chars);
    if path.ends_with("url")
        || path.ends_with("image")
        || path.ends_with("license")
        || path == "og:image"
        || path == "canonical"
    {
        clean_field(
            &normalize_url(&cleaned, base_url, &["http", "https"]).unwrap_or_default(),
            max_chars,
        )
    } else if path.ends_with("email") {
        cleaned.trim_start_matches("mailto:").to_string()
    } else {
        cleaned
    }
}

fn clean_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(|v| clean_field(v, max_chars))
        .filter(|v| !v.is_empty())
}

fn clean_field(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.chars().take(max_chars).collect()
}

fn json_scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn allowed_json_ld_path(path: &str) -> bool {
    matches!(
        path,
        "name"
            | "url"
            | "email"
            | "price"
            | "sku"
            | "license"
            | "availability"
            | "description"
            | "brand.name"
            | "offers.price"
            | "offers.priceCurrency"
            | "telephone"
            | "author.name"
            | "publisher.name"
            | "datePublished"
            | "dateModified"
    )
}

fn allowed_meta_name(name: &str) -> bool {
    matches!(
        name,
        "description"
            | "og:title"
            | "og:description"
            | "og:image"
            | "twitter:title"
            | "twitter:description"
    )
}

fn metadata_kind(item_type: Option<&str>, path: &str) -> Option<MetadataKind> {
    let item_type = item_type.unwrap_or_default();
    match (item_type, path) {
        ("Product", "name") => Some(MetadataKind::ProductName),
        ("Product", "description") => Some(MetadataKind::ProductDescription),
        ("Product", "sku") => Some(MetadataKind::ProductSku),
        (_, "offers.price" | "price") => Some(MetadataKind::ProductPrice),
        (_, "offers.priceCurrency") => Some(MetadataKind::ProductCurrency),
        (_, "license") => Some(MetadataKind::License),
        (_, "canonical") => Some(MetadataKind::CanonicalUrl),
        (_, "email") => Some(MetadataKind::ContactEmail),
        (_, "author.name") => Some(MetadataKind::ArticleAuthor),
        (_, "datePublished") => Some(MetadataKind::PublishedDate),
        (_, "dateModified") => Some(MetadataKind::ModifiedDate),
        (_, "image" | "og:image") => Some(MetadataKind::ImageUrl),
        (_, "og:title" | "twitter:title") => Some(MetadataKind::PageTitle),
        (_, "description" | "og:description" | "twitter:description") => {
            Some(MetadataKind::Description)
        }
        _ => None,
    }
}

fn metadata_label(kind: Option<MetadataKind>, path: &str) -> Option<String> {
    kind.map(|kind| {
        match kind {
            MetadataKind::ProductName => "Product name",
            MetadataKind::ProductPrice => "Product price",
            MetadataKind::ProductCurrency => "Product currency",
            MetadataKind::ProductSku => "Product SKU",
            MetadataKind::ProductDescription => "Product description",
            MetadataKind::License => "License",
            MetadataKind::CanonicalUrl => "Canonical URL",
            MetadataKind::ContactEmail => "Contact email",
            MetadataKind::ArticleAuthor => "Article author",
            MetadataKind::PublishedDate => "Published date",
            MetadataKind::ModifiedDate => "Modified date",
            MetadataKind::ImageUrl => "Image URL",
            MetadataKind::PageTitle => "Page title",
            MetadataKind::Description => "Description",
        }
        .to_string()
    })
    .or_else(|| Some(path.to_string()).filter(|_| false))
}

fn prefixed_path(item_type: Option<&str>, path: &str) -> String {
    item_type
        .filter(|t| !t.is_empty())
        .map_or_else(|| path.to_string(), |t| format!("{t}.{path}"))
}

fn tag_name(sel: &Selection) -> String {
    sel.nodes()
        .first()
        .and_then(dom_query::NodeRef::node_name)
        .map(|t| t.to_string().to_ascii_lowercase())
        .unwrap_or_default()
}

fn dom_region(sel: &Selection) -> DomRegion {
    let mut current = sel.clone();
    while current.length() > 0 {
        match tag_name(&current).as_str() {
            "main" => return DomRegion::Main,
            "article" => return DomRegion::Article,
            "header" => return DomRegion::Header,
            "footer" => return DomRegion::Footer,
            "nav" => return DomRegion::Nav,
            "aside" => return DomRegion::Aside,
            "form" => return DomRegion::Form,
            "head" => return DomRegion::Head,
            _ => {}
        }
        let parent = current.parent();
        if parent.length() == 0 {
            break;
        }
        current = parent;
    }
    DomRegion::Unknown
}

fn classify_link(
    text: &str,
    href: &str,
    title: Option<&str>,
    download: Option<&str>,
    region: DomRegion,
    node: &Selection,
) -> Vec<LinkClass> {
    let mut classes = Vec::new();
    let lower = format!(
        "{} {} {} {} {}",
        text.to_ascii_lowercase(),
        href.to_ascii_lowercase(),
        title.unwrap_or_default().to_ascii_lowercase(),
        node.attr("rel").unwrap_or_default().to_ascii_lowercase(),
        node.attr("class").unwrap_or_default().to_ascii_lowercase()
    );
    if href.starts_with("mailto:") {
        push_class(&mut classes, LinkClass::Email);
        push_class(&mut classes, LinkClass::Contact);
    }
    if href.starts_with("tel:") {
        push_class(&mut classes, LinkClass::Phone);
        push_class(&mut classes, LinkClass::Contact);
    }
    if href.contains('#') {
        push_class(&mut classes, LinkClass::Anchor);
    }
    if download.is_some() || lower.contains("download") {
        push_class(&mut classes, LinkClass::Download);
    }
    if href_is_intrinsically_useful(href) {
        push_class(&mut classes, LinkClass::Attachment);
    }
    if lower.contains(".pdf") {
        push_class(&mut classes, LinkClass::Pdf);
    }
    if lower.contains("pricing") || lower.contains("price") {
        push_class(&mut classes, LinkClass::Pricing);
    }
    if lower.contains("docs") || lower.contains("documentation") || lower.contains("reference") {
        push_class(&mut classes, LinkClass::Documentation);
    }
    if lower.contains("api") {
        push_class(&mut classes, LinkClass::Api);
    }
    if lower.contains("github.com")
        || lower.contains("gitlab.com")
        || lower.contains("source")
        || lower.contains("repo")
    {
        push_class(&mut classes, LinkClass::SourceRepository);
    }
    if lower.contains("license") || lower.contains("copyright") {
        push_class(&mut classes, LinkClass::License);
    }
    if lower.contains("twitter")
        || lower.contains("linkedin")
        || lower.contains("facebook")
        || lower.contains("mastodon")
    {
        push_class(&mut classes, LinkClass::Social);
    }
    if lower.contains("share") {
        push_class(&mut classes, LinkClass::Share);
    }
    if lower.contains("login") || lower.contains("sign in") {
        push_class(&mut classes, LinkClass::Login);
    }
    if lower.contains("privacy") || lower.contains("terms") || lower.contains("legal") {
        push_class(&mut classes, LinkClass::Legal);
    }
    if lower.contains("page=") || lower.contains("next") || lower.contains("previous") {
        push_class(&mut classes, LinkClass::Pagination);
    }
    if matches!(region, DomRegion::Nav | DomRegion::Header) {
        push_class(&mut classes, LinkClass::Navigation);
    }
    if matches!(region, DomRegion::Footer) && classes.is_empty() {
        push_class(&mut classes, LinkClass::Legal);
    }
    if classes.is_empty() {
        classes.push(LinkClass::Unknown);
    }
    classes
}

fn push_class(classes: &mut Vec<LinkClass>, class: LinkClass) {
    if !classes.contains(&class) {
        classes.push(class);
    }
}

fn is_high_value_link(link: &LinkFact) -> bool {
    link.classes.iter().any(|class| {
        matches!(
            class,
            LinkClass::Download
                | LinkClass::Attachment
                | LinkClass::Pdf
                | LinkClass::Contact
                | LinkClass::Email
                | LinkClass::Phone
                | LinkClass::Pricing
                | LinkClass::Documentation
                | LinkClass::Api
                | LinkClass::SourceRepository
                | LinkClass::License
        )
    })
}

fn link_class_name(class: LinkClass) -> &'static str {
    match class {
        LinkClass::Download => "download",
        LinkClass::Attachment => "attachment",
        LinkClass::Pdf => "pdf",
        LinkClass::Contact => "contact",
        LinkClass::Email => "email",
        LinkClass::Phone => "phone",
        LinkClass::Pricing => "pricing",
        LinkClass::Documentation => "documentation",
        LinkClass::Api => "api",
        LinkClass::SourceRepository => "source",
        LinkClass::License => "license",
        LinkClass::Social => "social",
        LinkClass::Navigation => "navigation",
        LinkClass::Breadcrumb => "breadcrumb",
        LinkClass::Pagination => "pagination",
        LinkClass::Legal => "legal",
        LinkClass::Share => "share",
        LinkClass::Login => "login",
        LinkClass::Anchor => "anchor",
        LinkClass::Unknown => "unknown",
    }
}

fn link_fact_chars(fact: &LinkFact) -> usize {
    fact.text.len()
        + fact.href.len()
        + fact.raw_href.as_deref().map_or(0, str::len)
        + fact.title.as_deref().map_or(0, str::len)
        + fact.download.as_deref().map_or(0, str::len)
        + fact.scheme.as_deref().map_or(0, str::len)
        + fact.host.as_deref().map_or(0, str::len)
        + fact.path.as_deref().map_or(0, str::len)
        + fact.filename.as_deref().map_or(0, str::len)
        + fact
            .query
            .iter()
            .map(|p| p.key.len() + p.value.len() + p.decoded_value.len())
            .sum::<usize>()
        + fact.fragment.as_deref().map_or(0, str::len)
        + fact.email.as_deref().map_or(0, str::len)
        + fact.phone.as_deref().map_or(0, str::len)
        + fact.nearest_heading.as_deref().map_or(0, str::len)
}

fn image_fact_chars(fact: &ImageFact) -> usize {
    fact.src.len()
        + fact.alt.as_deref().map_or(0, str::len)
        + fact.title.as_deref().map_or(0, str::len)
        + fact.caption.as_deref().map_or(0, str::len)
        + fact.nearest_heading.as_deref().map_or(0, str::len)
}

fn metadata_fact_chars(fact: &MetadataFact) -> usize {
    fact.source.len()
        + fact.path.len()
        + fact.item_type.as_deref().map_or(0, str::len)
        + fact.value.len()
        + fact.label.as_deref().map_or(0, str::len)
}

fn table_fact_chars(fact: &TableFact) -> usize {
    fact.caption.as_deref().map_or(0, str::len)
        + fact.nearest_heading.as_deref().map_or(0, str::len)
        + fact.headers.iter().map(String::len).sum::<usize>()
        + fact
            .rows
            .iter()
            .flat_map(|row| row.iter())
            .map(String::len)
            .sum::<usize>()
}

fn media_fact_chars(fact: &MediaFact) -> usize {
    fact.sources
        .iter()
        .map(|s| s.src.len() + s.mime_type.as_deref().map_or(0, str::len))
        .sum::<usize>()
        + fact
            .tracks
            .iter()
            .map(|t| {
                t.src.as_deref().map_or(0, str::len)
                    + t.kind.as_deref().map_or(0, str::len)
                    + t.srclang.as_deref().map_or(0, str::len)
                    + t.label.as_deref().map_or(0, str::len)
            })
            .sum::<usize>()
        + fact.poster.as_deref().map_or(0, str::len)
        + fact.caption.as_deref().map_or(0, str::len)
        + fact.nearest_heading.as_deref().map_or(0, str::len)
}

fn section_fact_chars(fact: &SectionFact) -> usize {
    fact.heading.len() + 1
}

struct CharBudget {
    remaining: usize,
}

impl CharBudget {
    fn new(max_chars: usize) -> Self {
        Self {
            remaining: max_chars,
        }
    }

    fn exhausted(&self) -> bool {
        self.remaining == 0
    }

    fn try_consume(&mut self, chars: usize) -> bool {
        if chars > self.remaining {
            self.remaining = 0;
            false
        } else {
            self.remaining -= chars;
            true
        }
    }
}
