#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    deprecated
)]

use std::io::Write;
use std::process::{Command, Stdio};

use rs_trafilatura::{extract_with_options, Options, StructuredFactsOptions};

fn options_with_structured_facts() -> Options {
    Options {
        url: Some("https://example.com/base/page.html".to_string()),
        structured_facts: Some(StructuredFactsOptions::default()),
        ..Options::default()
    }
}

#[test]
fn default_extraction_does_not_generate_structured_facts() {
    let html = r#"
        <html><body><article><p>Main article text with enough words to extract cleanly.</p></article>
        <a href="mailto:help@example.com" title="Support">Support</a></body></html>
    "#;

    let baseline = extract_with_options(html, &Options::default()).unwrap();
    assert!(baseline.structured_facts.is_none());

    let with_facts = extract_with_options(html, &options_with_structured_facts()).unwrap();
    assert!(with_facts.structured_facts.is_some());
    assert_eq!(baseline.content_text, with_facts.content_text);
    assert_eq!(baseline.content_html, with_facts.content_html);
    assert_eq!(baseline.content_markdown, with_facts.content_markdown);
}

#[test]
fn extracts_visible_links_images_tables_and_metadata_from_precise_dom() {
    let html = r#"
        <html>
          <head>
            <meta name="description" content="Short public description">
            <script type="application/ld+json">
            {"@type":"Product","name":"Roadrunner Trap","sku":"SKU-42","offers":{"price":"19.95"},"email":"sales@example.com"}
            </script>
          </head>
          <body>
            <article>
              <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
              <a href="/license" title="License details" download="license.pdf">License</a>
              <a href="mailto:help@example.com">Email support</a>
              <figure>
                <img src="images/hero.png" alt="Hero alt" title="Hero title">
                <figcaption>Hero caption</figcaption>
              </figure>
              <table>
                <caption>Rates</caption>
                <thead><tr><th>Tier</th><th>Price</th></tr></thead>
                <tbody><tr><td>Starter</td><td>$9</td></tr></tbody>
              </table>
            </article>
          </body>
        </html>
    "#;

    let result = extract_with_options(html, &options_with_structured_facts()).unwrap();
    let facts = result
        .structured_facts
        .expect("structured facts should be present");

    assert_eq!(facts.links.len(), 2);
    assert_eq!(facts.links[0].text, "License");
    assert_eq!(facts.links[0].href, "https://example.com/license");
    assert_eq!(facts.links[0].title.as_deref(), Some("License details"));
    assert_eq!(facts.links[0].download.as_deref(), Some("license.pdf"));
    assert_eq!(facts.links[1].href, "mailto:help@example.com");

    assert_eq!(facts.images.len(), 1);
    assert_eq!(
        facts.images[0].src,
        "https://example.com/base/images/hero.png"
    );
    assert_eq!(facts.images[0].alt.as_deref(), Some("Hero alt"));
    assert_eq!(facts.images[0].title.as_deref(), Some("Hero title"));
    assert_eq!(facts.images[0].caption.as_deref(), Some("Hero caption"));

    assert!(facts.metadata.iter().any(|m| m.source == "json_ld"
        && m.path == "Product.name"
        && m.value == "Roadrunner Trap"));
    assert!(facts.metadata.iter().any(|m| m.source == "json_ld"
        && m.path == "Product.email"
        && m.value == "sales@example.com"));
    assert!(facts.metadata.iter().any(|m| m.source == "meta"
        && m.path == "description"
        && m.value == "Short public description"));

    assert_eq!(facts.tables.len(), 1);
    assert_eq!(facts.tables[0].caption.as_deref(), Some("Rates"));
    assert_eq!(facts.tables[0].headers, vec!["Tier", "Price"]);
    assert_eq!(facts.tables[0].rows, vec![vec!["Starter", "$9"]]);
}

#[test]
fn excludes_hidden_ancestor_and_unsafe_scheme_facts() {
    let html = r#"
        <html><body><article>
          <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
          <div hidden><a href="https://example.com/hidden">Hidden</a></div>
          <div aria-hidden="true"><img src="https://example.com/hidden.png" alt="Hidden"></div>
          <div style="display:none"><table><tr><td>Hidden cell</td></tr></table></div>
          <a href="javascript:alert(1)">Bad JS</a>
          <a href="data:text/plain,hello">Bad data</a>
          <a href="https://example.com/visible">Visible</a>
        </article></body></html>
    "#;

    let result = extract_with_options(html, &options_with_structured_facts()).unwrap();
    let facts = result.structured_facts.unwrap();

    assert_eq!(facts.links.len(), 1);
    assert_eq!(facts.links[0].href, "https://example.com/visible");
    assert!(facts.images.is_empty());
    assert!(facts.tables.is_empty());
}

#[test]
fn drops_unsafe_metadata_empty_links_hidden_table_cells_and_nested_table_leakage() {
    let html = r#"
        <html><head>
          <meta property="og:image" content="javascript:alert(1)">
          <script type="APPLICATION/LD+JSON; charset=utf-8">
          {"@graph":[{"@type":"Article","name":"Graph Article","url":"/article","image":"data:text/plain,bad"}]}
          </script>
        </head><body><article>
          <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
          <a href="https://example.com/empty"></a>
          <a href="/download.pdf"></a>
          <table>
            <caption>Outer Rates</caption>
            <tr><th>Name</th><th>Value</th></tr>
            <tr style="display:none"><td>Hidden</td><td>Hidden value</td></tr>
            <tr><td>Visible</td><td><table><caption>Inner</caption><tr><td>Inner cell</td></tr></table>Outer value</td></tr>
          </table>
        </article></body></html>
    "#;

    let result = extract_with_options(html, &options_with_structured_facts()).unwrap();
    let facts = result.structured_facts.unwrap();

    assert_eq!(facts.links.len(), 1);
    assert_eq!(facts.links[0].href, "https://example.com/download.pdf");
    assert_eq!(facts.links[0].download.as_deref(), Some("download.pdf"));

    assert!(facts
        .metadata
        .iter()
        .any(|m| m.source == "json_ld" && m.path == "Article.name" && m.value == "Graph Article"));
    assert!(facts.metadata.iter().any(|m| m.source == "json_ld"
        && m.path == "Article.url"
        && m.value == "https://example.com/article"));
    assert!(!facts
        .metadata
        .iter()
        .any(|m| m.value.contains("javascript") || m.value.contains("data:")));

    let outer = facts
        .tables
        .iter()
        .find(|table| table.caption.as_deref() == Some("Outer Rates"))
        .expect("outer table should be captured");
    assert_eq!(outer.headers, vec!["Name", "Value"]);
    assert_eq!(outer.rows, vec![vec!["Visible", "Outer value"]]);
    assert!(facts
        .tables
        .iter()
        .any(|table| table.caption.as_deref() == Some("Inner")));
}

#[test]
fn dedup_keys_preserve_distinct_descriptive_fields() {
    let html = r#"
        <html><body><article>
          <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
          <a href="/same" title="First">Same</a>
          <a href="/same" title="Second">Same</a>
          <figure><img src="/same.png" alt="First alt"><figcaption>First caption</figcaption></figure>
          <figure><img src="/same.png" alt="Second alt"><figcaption>Second caption</figcaption></figure>
        </article></body></html>
    "#;

    let result = extract_with_options(html, &options_with_structured_facts()).unwrap();
    let facts = result.structured_facts.unwrap();

    assert_eq!(facts.links.len(), 2);
    assert!(facts
        .links
        .iter()
        .any(|link| link.title.as_deref() == Some("First")));
    assert!(facts
        .links
        .iter()
        .any(|link| link.title.as_deref() == Some("Second")));
    assert_eq!(facts.images.len(), 2);
    assert!(facts
        .images
        .iter()
        .any(|image| image.alt.as_deref() == Some("First alt")));
    assert!(facts
        .images
        .iter()
        .any(|image| image.alt.as_deref() == Some("Second alt")));
}

#[test]
fn caps_url_fields_and_skips_blank_decorative_images() {
    let html = r#"
        <html><body><article>
          <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
          <a href="/very/long/path/that/exceeds/the/field/cap">Long href</a>
          <img src="/blank-tracker.png">
          <img src="/descriptive.png" alt="Descriptive image">
        </article></body></html>
    "#;

    let opts = Options {
        url: Some("https://example.com/root/".to_string()),
        structured_facts: Some(StructuredFactsOptions {
            max_field_chars: 32,
            ..StructuredFactsOptions::default()
        }),
        ..Options::default()
    };

    let result = extract_with_options(html, &opts).unwrap();
    let facts = result.structured_facts.unwrap();

    assert!(facts.links[0].href.chars().count() <= 32);
    assert_eq!(facts.images.len(), 1);
    assert_eq!(facts.images[0].alt.as_deref(), Some("Descriptive image"));
    assert!(facts.images[0].src.chars().count() <= 32);
}

#[test]
fn enforces_caps_dedup_and_malformed_json_ld_safety() {
    let html = r#"
        <html><head>
          <script type="application/ld+json">not json</script>
          <script type="application/ld+json">{"@type":"Product","name":"First"}</script>
          <script type="application/ld+json">{"@type":"Product","name":"Second"}</script>
        </head><body><article>
          <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
          <a href="/one">One</a>
          <a href="/one">One</a>
          <a href="/two">Two</a>
          <a href="/three">Three</a>
        </article></body></html>
    "#;

    let opts = Options {
        url: Some("https://example.com/root/".to_string()),
        structured_facts: Some(StructuredFactsOptions {
            max_links: 2,
            max_metadata_facts: 2,
            max_json_ld_scripts: 2,
            ..StructuredFactsOptions::default()
        }),
        ..Options::default()
    };

    let result = extract_with_options(html, &opts).unwrap();
    let facts = result.structured_facts.unwrap();

    assert_eq!(facts.links.len(), 2);
    assert_eq!(facts.links[0].href, "https://example.com/one");
    assert_eq!(facts.links[1].href, "https://example.com/two");
    assert!(facts.metadata.iter().any(|m| m.value == "First"));
    assert!(!facts.metadata.iter().any(|m| m.value == "Second"));
}

#[test]
fn deserializes_existing_structured_facts_json_shape_with_defaults() {
    let json = r#"{
        "links":[{"text":"Docs","href":"https://example.test/docs","title":"Docs title"}],
        "images":[{"src":"https://example.test/hero.png","alt":"Hero"}],
        "metadata":[{"source":"json_ld","path":"Product.name","item_type":"Product","value":"Widget"}],
        "tables":[{"caption":"Pricing","headers":["Plan"],"rows":[["Pro"]]}]
    }"#;

    let facts: rs_trafilatura::StructuredFacts = serde_json::from_str(json).unwrap();
    assert_eq!(facts.links[0].href, "https://example.test/docs");
    assert!(facts.links[0].classes.is_empty());
    assert_eq!(
        facts.links[0].dom_region,
        rs_trafilatura::DomRegion::Unknown
    );
    assert!(facts.media.is_empty());
    assert!(facts.sections.is_empty());
    assert_eq!(facts.metadata[0].kind, None);
}

#[test]
fn collects_all_safe_visible_links_with_url_parts_classes_and_context() {
    let html = r##"
        <html><body>
          <header><nav><a href="/home">Home</a></nav></header>
          <article>
            <h2>Downloads</h2>
            <a href="/files/report.pdf?ref=Parser%20Eval#top" rel="license noopener" target="_blank" download="report-final.pdf">Download report</a>
            <a href="mailto:support@example.test?subject=Parser%20Eval&subject=Again">Email support</a>
            <a href="tel:+15551234567">Call us</a>
            <a href="#details">Details</a>
          </article>
          <footer><a href="/privacy">Privacy</a></footer>
        </body></html>
    "##;

    let result = extract_with_options(html, &options_with_structured_facts()).unwrap();
    let facts = result.structured_facts.unwrap();

    assert_eq!(facts.links.len(), 6);
    let download = facts
        .links
        .iter()
        .find(|l| l.text == "Download report")
        .unwrap();
    assert_eq!(
        download.raw_href.as_deref(),
        Some("/files/report.pdf?ref=Parser%20Eval#top")
    );
    assert_eq!(download.scheme.as_deref(), Some("https"));
    assert_eq!(download.host.as_deref(), Some("example.com"));
    assert_eq!(download.path.as_deref(), Some("/files/report.pdf"));
    assert_eq!(download.filename.as_deref(), Some("report-final.pdf"));
    assert_eq!(download.fragment.as_deref(), Some("top"));
    assert_eq!(download.query[0].key, "ref");
    assert_eq!(download.query[0].decoded_value, "Parser Eval");
    assert!(download
        .classes
        .contains(&rs_trafilatura::LinkClass::Download));
    assert!(download.classes.contains(&rs_trafilatura::LinkClass::Pdf));
    assert_eq!(download.dom_region, rs_trafilatura::DomRegion::Article);
    assert_eq!(download.nearest_heading.as_deref(), Some("Downloads"));

    let mail = facts
        .links
        .iter()
        .find(|l| l.email.as_deref() == Some("support@example.test"))
        .unwrap();
    assert_eq!(mail.scheme.as_deref(), Some("mailto"));
    assert_eq!(mail.query.len(), 2);
    assert_eq!(mail.query[0].decoded_value, "Parser Eval");
    assert!(mail.classes.contains(&rs_trafilatura::LinkClass::Email));
    assert!(mail.classes.contains(&rs_trafilatura::LinkClass::Contact));

    let tel = facts
        .links
        .iter()
        .find(|l| l.scheme.as_deref() == Some("tel"))
        .unwrap();
    assert_eq!(tel.phone.as_deref(), Some("+15551234567"));
    assert!(tel.classes.contains(&rs_trafilatura::LinkClass::Phone));

    let footer = facts.links.iter().find(|l| l.text == "Privacy").unwrap();
    assert_eq!(footer.dom_region, rs_trafilatura::DomRegion::Footer);
    assert!(footer.classes.contains(&rs_trafilatura::LinkClass::Legal));
}

#[test]
fn captures_sections_media_and_semantic_metadata_aliases_without_changing_content() {
    let html = r#"
        <html>
          <head>
            <link rel="canonical" href="/canonical-product">
            <link rel="license" href="/license">
            <script type="application/ld+json">
            {"@type":"Product","name":"Widget Pro","sku":"W-1","offers":[{"price":"19.99","priceCurrency":"USD"}],"license":"/license"}
            </script>
          </head>
          <body><article>
            <h1>Widget Pro</h1>
            <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
            <figure>
              <video poster="/poster.jpg"><source src="/demo.webm" type="video/webm"><track src="/captions.vtt" kind="captions" srclang="en" label="English"></video>
              <figcaption>Demo video</figcaption>
            </figure>
            <figure>
              <audio><source src="/intro.mp3" type="audio/mpeg"></audio>
              <figcaption>Intro audio</figcaption>
            </figure>
          </article></body>
        </html>
    "#;

    let baseline = extract_with_options(html, &Options::default()).unwrap();
    let result = extract_with_options(html, &options_with_structured_facts()).unwrap();
    assert_eq!(baseline.content_text, result.content_text);
    assert_eq!(baseline.content_html, result.content_html);
    assert_eq!(baseline.content_markdown, result.content_markdown);

    let facts = result.structured_facts.unwrap();
    assert!(facts
        .sections
        .iter()
        .any(|s| s.heading == "Widget Pro" && s.level == 1));
    assert_eq!(facts.media.len(), 2);
    let video = facts
        .media
        .iter()
        .find(|m| m.kind == rs_trafilatura::MediaKind::Video)
        .unwrap();
    assert_eq!(
        video.poster.as_deref(),
        Some("https://example.com/poster.jpg")
    );
    assert_eq!(video.sources[0].src, "https://example.com/demo.webm");
    assert_eq!(
        video.tracks[0].src.as_deref(),
        Some("https://example.com/captions.vtt")
    );
    assert_eq!(video.caption.as_deref(), Some("Demo video"));

    assert!(facts.metadata.iter().any(|m| m.kind
        == Some(rs_trafilatura::MetadataKind::ProductName)
        && m.value == "Widget Pro"));
    assert!(facts
        .metadata
        .iter()
        .any(|m| m.kind == Some(rs_trafilatura::MetadataKind::ProductPrice) && m.value == "19.99"));
    assert!(facts.metadata.iter().any(|m| m.kind
        == Some(rs_trafilatura::MetadataKind::CanonicalUrl)
        && m.value == "https://example.com/canonical-product"));
}

#[test]
fn renders_compact_high_value_fact_block_without_low_value_link_dump() {
    let html = r#"
        <html><body>
          <nav><a href="/home">Home</a><a href="/login">Login</a></nav>
          <article>
            <h2>Support</h2>
            <a href="mailto:support@example.test?subject=Parser%20Eval">Email support</a>
            <a href="/api/reference?access_token=s3cr3t&subject=Visible">API Reference</a>
          </article>
          <footer><a href="/privacy">Privacy</a></footer>
        </body></html>
    "#;
    let facts = extract_with_options(html, &options_with_structured_facts())
        .unwrap()
        .structured_facts
        .unwrap();
    let rendered = rs_trafilatura::render_structured_facts_for_extraction(
        &facts,
        &rs_trafilatura::StructuredFactsRenderOptions {
            max_chars: 1200,
            ..Default::default()
        },
    );

    assert!(rendered.contains("support@example.test"));
    assert!(rendered.contains("Parser Eval"));
    assert!(rendered.contains("API Reference"));
    assert!(rendered.contains("access_token=\"[redacted]\""));
    assert!(rendered.contains("access_token=[redacted]"));
    assert!(rendered.contains("Visible"));
    assert!(!rendered.contains("s3cr3t"));
    assert!(rendered.contains("omitted"));
    assert!(!rendered.contains("/privacy"));
    assert!(!rendered.contains("/login"));
}

#[test]
fn renders_attribute_media_table_and_visible_key_value_evidence_generically() {
    let html = r#"
        <html><body>
          <main>
            <h1>Product page</h1>
            <p>Status: Ships today</p>
            <p>Availability - In stock</p>
            <p>The visible prose intentionally omits several exact attribute values.</p>
            <a href="/license" title="MIT/X derivative license">open source</a>
            <figure>
              <img data-src="/assets/q4-revenue.png" alt="Quarterly revenue chart showing 18 percent Q4 growth" title="Q4 revenue rose 18 percent">
              <figcaption>Revenue chart</figcaption>
            </figure>
            <table>
              <thead><tr><th>Lane</th><th>Included pages</th><th>Support window</th></tr></thead>
              <tbody>
                <tr><th>Selector</th><td>100,000</td><td>next business day</td></tr>
                <tr><th>Sidecar</th><td>250,000</td><td>four hours</td></tr>
              </tbody>
            </table>
          </main>
        </body></html>
    "#;

    let facts = extract_with_options(html, &options_with_structured_facts())
        .unwrap()
        .structured_facts
        .unwrap();
    assert_eq!(
        facts.tables[0].headers,
        vec!["Lane", "Included pages", "Support window"]
    );
    assert_eq!(
        facts.tables[0].rows,
        vec![
            vec!["Selector", "100,000", "next business day"],
            vec!["Sidecar", "250,000", "four hours"],
        ]
    );

    let rendered = rs_trafilatura::render_structured_facts_for_extraction(
        &facts,
        &rs_trafilatura::StructuredFactsRenderOptions {
            max_chars: 2400,
            ..Default::default()
        },
    );

    assert!(rendered.contains("MIT/X derivative license"));
    assert!(rendered.contains("Quarterly revenue chart showing 18 percent Q4 growth"));
    assert!(rendered.contains("Q4 revenue rose 18 percent"));
    assert!(rendered.contains("Revenue chart"));
    assert!(rendered.contains("Status: Ships today"));
    assert!(rendered.contains("Availability: In stock"));
    assert!(rendered.contains("Selector | 100,000 | next business day"));
    assert!(rendered.contains("Included pages=100,000"));
    assert!(rendered.contains("normalized=100000"));
}

#[test]
fn structured_facts_sections_respect_total_character_budget() {
    let html = r#"
        <html><body><article>
          <h1>Very long heading that would exceed a tiny structured facts budget</h1>
          <p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p>
        </article></body></html>
    "#;
    let options = Options {
        url: Some("https://example.com/base/page.html".to_string()),
        structured_facts: Some(StructuredFactsOptions {
            max_total_chars: 1,
            ..StructuredFactsOptions::default()
        }),
        ..Options::default()
    };

    let facts = extract_with_options(html, &options)
        .unwrap()
        .structured_facts
        .unwrap();

    assert!(facts.sections.is_empty());
    assert!(facts.links.is_empty());
    assert!(facts.images.is_empty());
    assert!(facts.metadata.is_empty());
    assert!(facts.media.is_empty());
    assert!(facts.tables.is_empty());
}

#[test]
fn extract_stdin_structured_facts_flag_emits_empty_arrays_and_preserves_markdown_mode() {
    let html = r#"<html><body><article><p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p></article></body></html>"#;
    let bin = env!("CARGO_BIN_EXE_extract_stdin");

    let without = run_extract_stdin(
        bin,
        &["--url", "https://example.com/base/page.html", "--markdown"],
        html,
    );
    let with = run_extract_stdin(
        bin,
        &[
            "--url",
            "https://example.com/base/page.html",
            "--markdown",
            "--structured-facts",
        ],
        html,
    );

    assert!(without.get("structured_facts").is_none());
    let facts = with
        .get("structured_facts")
        .expect("structured_facts field should be present when requested");
    assert_eq!(facts["links"].as_array().unwrap().len(), 0);
    assert_eq!(facts["images"].as_array().unwrap().len(), 0);
    assert_eq!(facts["metadata"].as_array().unwrap().len(), 0);
    assert_eq!(facts["tables"].as_array().unwrap().len(), 0);
    assert_eq!(without["main_content"], with["main_content"]);
    assert_eq!(without["content_markdown"], with["content_markdown"]);
}

#[test]
fn extract_stdin_can_render_structured_facts_markdown() {
    let html = r#"<html><body><article><p>Main article text with enough words to extract cleanly and keep the extractor satisfied.</p><a href="mailto:support@example.test?subject=Parser%20Eval">Email support</a></article></body></html>"#;
    let bin = env!("CARGO_BIN_EXE_extract_stdin");

    let output = run_extract_stdin(
        bin,
        &[
            "--url",
            "https://example.com/base/page.html",
            "--render-structured-facts",
        ],
        html,
    );

    let rendered = output["structured_facts_markdown"].as_str().unwrap();
    assert!(rendered.contains("support@example.test"));
    assert!(rendered.contains("Parser Eval"));
    assert!(output.get("structured_facts").is_some());
}

fn run_extract_stdin(bin: &str, args: &[&str], html: &str) -> serde_json::Value {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn extract_stdin");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(html.as_bytes())
        .unwrap();
    let output = child.wait_with_output().expect("wait extract_stdin");
    assert!(
        output.status.success(),
        "extract_stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}
