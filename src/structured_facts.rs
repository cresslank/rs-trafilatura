//! Opt-in extraction of bounded structured facts from the pre-cleaning DOM.
//!
//! These facts are intentionally separate from the main text/HTML/Markdown
//! extraction path. They preserve useful attributes for downstream structured
//! extraction without mutating the default parser output.

use crate::dom::{Document, Selection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredFactsOptions {
    pub max_links: usize,
    pub max_images: usize,
    pub max_metadata_facts: usize,
    pub max_tables: usize,
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
            max_links: 50,
            max_images: 25,
            max_metadata_facts: 40,
            max_tables: 10,
            max_table_rows: 20,
            max_table_cells_per_row: 12,
            max_field_chars: 256,
            max_json_ld_scripts: 8,
            max_json_ld_bytes: 16 * 1024,
            max_json_ld_depth: 8,
            max_total_chars: 20 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredFacts {
    pub links: Vec<LinkFact>,
    pub images: Vec<ImageFact>,
    pub metadata: Vec<MetadataFact>,
    pub tables: Vec<TableFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkFact {
    pub text: String,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataFact {
    pub source: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableFact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[must_use]
pub(crate) fn extract_structured_facts(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
) -> StructuredFacts {
    let mut facts = StructuredFacts::default();
    let mut budget = CharBudget::new(options.max_total_chars);

    extract_links(doc, base_url, options, &mut budget, &mut facts.links);
    extract_images(doc, base_url, options, &mut budget, &mut facts.images);
    extract_metadata(doc, base_url, options, &mut budget, &mut facts.metadata);
    extract_tables(doc, options, &mut budget, &mut facts.tables);

    facts
}

fn extract_links(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
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
        let Some(normalized_href) =
            normalize_url(raw_href.trim(), base_url, &["http", "https", "mailto"])
        else {
            continue;
        };
        let text = clean_field(&node.text(), options.max_field_chars);
        let title = clean_optional(node.attr("title").as_deref(), options.max_field_chars);
        let download = clean_optional(node.attr("download").as_deref(), options.max_field_chars)
            .or_else(|| downloadable_filename(&normalized_href, options.max_field_chars));
        if text.is_empty()
            && title.is_none()
            && download.is_none()
            && !href_is_intrinsically_useful(&normalized_href)
        {
            continue;
        }
        let href = clean_field(&normalized_href, options.max_field_chars);
        let key = format!("{href}\u{0}{text}\u{0}{title:?}\u{0}{download:?}");
        if !seen.insert(key) {
            continue;
        }
        let fact = LinkFact {
            text,
            href,
            title,
            download,
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
            caption: find_image_caption(&node, options.max_field_chars),
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

fn extract_tables(
    doc: &Document,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
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
    extract_json_ld_metadata(doc, base_url, options, budget, out);
    extract_meta_tag_metadata(doc, base_url, options, budget, out);
}

fn extract_json_ld_metadata(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
) {
    for script in doc
        .select("script[type]")
        .iter()
        .filter(is_json_ld_script)
        .take(options.max_json_ld_scripts)
    {
        if out.len() >= options.max_metadata_facts || budget.exhausted() {
            break;
        }
        let text = script.text();
        let raw = text.trim();
        if raw.is_empty() || raw.len() > options.max_json_ld_bytes {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            visit_json_ld(&value, None, None, base_url, options, budget, out, 0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_json_ld(
    value: &Value,
    item_type: Option<&str>,
    path: Option<&str>,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
    depth: usize,
) {
    if depth > options.max_json_ld_depth
        || out.len() >= options.max_metadata_facts
        || budget.exhausted()
    {
        return;
    }

    match value {
        Value::Array(values) => {
            for child in values {
                visit_json_ld(
                    child,
                    item_type,
                    path,
                    base_url,
                    options,
                    budget,
                    out,
                    depth + 1,
                );
                if out.len() >= options.max_metadata_facts || budget.exhausted() {
                    break;
                }
            }
        }
        Value::Object(map) => {
            let mut current_type = item_type.map(str::to_owned);
            if let Some(t) = map.get("@type").and_then(json_scalar_to_string) {
                current_type = Some(clean_field(&t, options.max_field_chars));
                push_metadata(
                    out,
                    budget,
                    MetadataFact {
                        source: "json_ld".to_string(),
                        path: prefixed_path(current_type.as_deref(), "@type"),
                        item_type: current_type.clone(),
                        value: clean_field(&t, options.max_field_chars),
                    },
                );
            }

            for (key, child) in map {
                if key == "@type" {
                    continue;
                }
                let child_path = join_json_ld_path(path, key);
                if let Some(value) = json_scalar_to_string(child) {
                    if allowed_json_ld_path(&child_path) {
                        let value = normalize_metadata_value(
                            &child_path,
                            &value,
                            base_url,
                            options.max_field_chars,
                        );
                        if !value.is_empty() {
                            push_metadata(
                                out,
                                budget,
                                MetadataFact {
                                    source: "json_ld".to_string(),
                                    path: prefixed_path(current_type.as_deref(), &child_path),
                                    item_type: current_type.clone(),
                                    value,
                                },
                            );
                        }
                    }
                } else {
                    visit_json_ld(
                        child,
                        current_type.as_deref(),
                        Some(&child_path),
                        base_url,
                        options,
                        budget,
                        out,
                        depth + 1,
                    );
                }
                if out.len() >= options.max_metadata_facts || budget.exhausted() {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn extract_meta_tag_metadata(
    doc: &Document,
    base_url: Option<&str>,
    options: &StructuredFactsOptions,
    budget: &mut CharBudget,
    out: &mut Vec<MetadataFact>,
) {
    for meta in doc.select("meta").iter() {
        if out.len() >= options.max_metadata_facts || budget.exhausted() {
            break;
        }
        let name = meta
            .attr("name")
            .or_else(|| meta.attr("property"))
            .map(|s| s.to_string());
        let Some(name) = name else {
            continue;
        };
        let normalized = name.trim().to_ascii_lowercase();
        if !allowed_meta_name(&normalized) {
            continue;
        }
        let Some(content) = meta.attr("content") else {
            continue;
        };
        let value = normalize_metadata_value(
            &normalized,
            content.as_ref(),
            base_url,
            options.max_field_chars,
        );
        if value.is_empty() {
            continue;
        }
        push_metadata(
            out,
            budget,
            MetadataFact {
                source: "meta".to_string(),
                path: normalized,
                item_type: None,
                value,
            },
        );
    }
}

fn push_metadata(out: &mut Vec<MetadataFact>, budget: &mut CharBudget, fact: MetadataFact) {
    if budget.try_consume(metadata_fact_chars(&fact)) {
        out.push(fact);
    }
}

fn extract_table_headers(table: &Selection, options: &StructuredFactsOptions) -> Vec<String> {
    let mut headers = Vec::new();
    let header_row = if table.select("thead tr").length() > 0 {
        table.select("thead tr").first()
    } else {
        table.select("tr").first()
    };

    if header_row.length() == 0
        || is_hidden_context(&header_row)
        || !belongs_to_table(&header_row, table)
    {
        return headers;
    }

    for cell in header_row
        .select("th")
        .iter()
        .filter(|cell| belongs_to_table(cell, table) && !is_hidden_context(cell))
        .take(options.max_table_cells_per_row)
    {
        let text = clean_field(&cell.text(), options.max_field_chars);
        if !text.is_empty() {
            headers.push(clean_table_cell_text(&cell, table, options.max_field_chars));
        }
    }
    headers
}

fn extract_table_rows(
    table: &Selection,
    options: &StructuredFactsOptions,
    skip_first_header_row: bool,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for (idx, tr) in table.select("tbody tr, table > tr").iter().enumerate() {
        if rows.len() >= options.max_table_rows {
            break;
        }
        if !belongs_to_table(&tr, table) || is_hidden_context(&tr) {
            continue;
        }
        if skip_first_header_row && idx == 0 && tr.select("th").length() > 0 {
            continue;
        }
        let mut row = Vec::new();
        for cell in tr
            .select("td, th")
            .iter()
            .filter(|cell| belongs_to_table(cell, table) && !is_hidden_context(cell))
            .take(options.max_table_cells_per_row)
        {
            row.push(clean_table_cell_text(&cell, table, options.max_field_chars));
        }
        if row.iter().any(|cell| !cell.is_empty()) {
            rows.push(row);
        }
    }
    rows
}

fn clean_table_cell_text(cell: &Selection, table: &Selection, max_chars: usize) -> String {
    let Some(table_key) = selection_key(table) else {
        return String::new();
    };
    let Some(cell_key) = selection_key(cell) else {
        return String::new();
    };
    let Some(cell_node) = cell.nodes().first().copied() else {
        return String::new();
    };

    let mut parts = Vec::new();
    for node in cell_node.descendants() {
        if !node.is_text() {
            continue;
        }
        let mut current = node.parent();
        let mut skip = false;
        while let Some(ancestor) = current {
            let key = (ancestor.id, std::ptr::from_ref(ancestor.tree) as usize);
            let ancestor_selection = Selection::from(ancestor);
            if is_hidden_node(&ancestor_selection) {
                skip = true;
                break;
            }
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

    clean_field(&parts.join(" "), max_chars)
}

fn find_image_caption(img: &Selection, max_chars: usize) -> Option<String> {
    let parent = img.parent();
    if parent.length() > 0 {
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
    if !href_is_intrinsically_useful(href) || href.starts_with("mailto:") {
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
    if href.starts_with("mailto:") {
        return true;
    }
    let path = Url::parse(href).ok().map_or_else(
        || href.to_ascii_lowercase(),
        |url| url.path().to_ascii_lowercase(),
    );
    [
        ".csv", ".doc", ".docx", ".json", ".pdf", ".ppt", ".pptx", ".txt", ".xls", ".xlsx", ".xml",
        ".zip",
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
        let tag = current
            .nodes()
            .first()
            .and_then(dom_query::NodeRef::node_name)
            .map(|t| t.to_string().to_ascii_lowercase())
            .unwrap_or_default();
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
    let tag = sel
        .nodes()
        .first()
        .and_then(dom_query::NodeRef::node_name)
        .map(|t| t.to_string().to_ascii_lowercase())
        .unwrap_or_default();
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
    if raw.is_empty() || raw.starts_with('#') {
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

fn prefixed_path(item_type: Option<&str>, path: &str) -> String {
    item_type
        .filter(|t| !t.is_empty())
        .map_or_else(|| path.to_string(), |t| format!("{t}.{path}"))
}

fn link_fact_chars(fact: &LinkFact) -> usize {
    fact.text.len()
        + fact.href.len()
        + fact.title.as_deref().map_or(0, str::len)
        + fact.download.as_deref().map_or(0, str::len)
}

fn image_fact_chars(fact: &ImageFact) -> usize {
    fact.src.len()
        + fact.alt.as_deref().map_or(0, str::len)
        + fact.title.as_deref().map_or(0, str::len)
        + fact.caption.as_deref().map_or(0, str::len)
}

fn metadata_fact_chars(fact: &MetadataFact) -> usize {
    fact.source.len()
        + fact.path.len()
        + fact.item_type.as_deref().map_or(0, str::len)
        + fact.value.len()
}

fn table_fact_chars(fact: &TableFact) -> usize {
    fact.caption.as_deref().map_or(0, str::len)
        + fact.headers.iter().map(String::len).sum::<usize>()
        + fact
            .rows
            .iter()
            .flat_map(|row| row.iter())
            .map(String::len)
            .sum::<usize>()
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
