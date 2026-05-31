//! Batch HTML-to-Markdown extractor.
//! Reads all .html files from an input directory, extracts main content,
//! and writes .md files to an output directory.
//!
//! Usage: batch_markdown <input_dir> <output_dir>

use rs_trafilatura::{extract_with_options, Options};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: batch_markdown <input_dir> <output_dir>");
        std::process::exit(1);
    }

    let input_dir = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);

    if !input_dir.is_dir() {
        eprintln!("Input directory does not exist: {}", input_dir.display());
        std::process::exit(1);
    }

    fs::create_dir_all(&output_dir)?;

    let options = Options {
        output_markdown: true,
        include_tables: true,
        include_links: true,
        include_formatting: true,
        ..Options::default()
    };

    let mut entries: Vec<_> = fs::read_dir(&input_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "html"))
        .collect();

    entries.sort_by_key(std::fs::DirEntry::file_name);

    let total = entries.len();
    let mut success = 0;
    let mut failed = 0;
    let mut empty = 0;

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let Some(stem) = path.file_stem().map(|stem| stem.to_string_lossy()) else {
            eprintln!(
                "[{}/{}] ERROR skipping path without file stem: {}",
                i + 1,
                total,
                path.display()
            );
            failed += 1;
            continue;
        };
        let out_path = output_dir.join(format!("{stem}.md"));

        let html = match fs::read_to_string(&path) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[{}/{}] ERROR reading {}: {}", i + 1, total, stem, e);
                failed += 1;
                continue;
            }
        };

        match extract_with_options(&html, &options) {
            Ok(result) => {
                // Prefer markdown, fall back to plain text
                let content = result.content_markdown.unwrap_or(result.content_text);

                if content.trim().is_empty() {
                    eprintln!(
                        "[{}/{}] EMPTY: {} (confidence: {:.2})",
                        i + 1,
                        total,
                        stem,
                        result.extraction_quality
                    );
                    empty += 1;
                    continue;
                }

                // Build markdown with frontmatter
                let mut md = String::new();
                md.push_str("---\n");
                if let Some(ref title) = result.metadata.title {
                    writeln!(md, "title: \"{}\"", title.replace('"', "\\\""))?;
                }
                if let Some(ref author) = result.metadata.author {
                    writeln!(md, "author: \"{}\"", author.replace('"', "\\\""))?;
                }
                if let Some(ref date) = result.metadata.date {
                    writeln!(md, "date: \"{}\"", date.to_rfc3339())?;
                }
                let source_file = path.file_name().map_or_else(
                    || path.as_os_str().to_string_lossy(),
                    |name| name.to_string_lossy(),
                );
                writeln!(md, "source_file: \"{source_file}\"")?;
                writeln!(md, "confidence: {:.2}", result.extraction_quality)?;
                if let Some(ref pt) = result.metadata.page_type {
                    writeln!(md, "page_type: \"{pt}\"")?;
                }
                md.push_str("---\n\n");
                md.push_str(&content);

                fs::write(&out_path, &md)?;
                println!(
                    "[{}/{}] OK: {}.md ({} chars, confidence: {:.2})",
                    i + 1,
                    total,
                    stem,
                    content.len(),
                    result.extraction_quality
                );
                success += 1;
            }
            Err(e) => {
                eprintln!("[{}/{}] EXTRACT ERROR {}: {}", i + 1, total, stem, e);
                failed += 1;
            }
        }
    }

    println!("\nDone: {success} success, {empty} empty, {failed} failed (of {total} total)");

    Ok(())
}
