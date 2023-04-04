use crate::parser::Extractor;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::PathBuf;
use tracing::event;

pub mod candidate;
pub mod glob;
pub mod location;
pub mod modifier;
pub mod parser;
pub mod utility;
pub mod variant;

#[derive(Debug, Clone)]
pub struct ChangedContent {
    pub file: Option<PathBuf>,
    pub content: Option<String>,
    pub extension: String,
}

pub fn parse_candidate_strings_from_files(changed_content: Vec<ChangedContent>) -> Vec<String> {
    // if matches!(std::env::var("DEBUG"), Ok(value) if value.eq("*") || value.eq("1") || value.eq("true") || value.contains("tailwind"))
    // {
    //     tracing_subscriber::fmt()
    //         .with_max_level(tracing::Level::INFO)
    //         .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
    //         .compact()
    //         .init();
    // }

    parse_all_blobs(read_all_files(changed_content))
}

#[derive(Debug, Clone)]
pub struct ContentPathInfo {
    pub base: String,
}

pub fn resolve_content_paths(args: ContentPathInfo) -> Vec<String> {
    let ignored_extensions = include_str!("fixtures/ignored-extensions.txt")
        .trim()
        .lines()
        .collect::<Vec<_>>();
    let ignored_files = include_str!("fixtures/ignored-files.txt")
        .trim()
        .lines()
        .collect::<Vec<_>>();
    let binary_extensions = include_str!("fixtures/binary-extensions.txt")
        .trim()
        .lines()
        .collect::<Vec<_>>();

    WalkBuilder::new(args.base)
        .hidden(false)
        .filter_entry(move |entry| {
            // Skip known ignored folders
            if entry.file_type().unwrap().is_dir() {
                return entry
                    .file_name()
                    .to_str()
                    .map(|s| s != ".git")
                    .unwrap_or(false);
            }

            // Skip known ignored files
            if entry
                .file_name()
                .to_str()
                .map(|s| ignored_files.contains(&s))
                .unwrap_or(false)
            {
                return false;
            }

            // Skip known ignored extensions
            return entry
                .path()
                .extension()
                .map(|s| s.to_str().unwrap_or_default())
                .map(|ext| !ignored_extensions.contains(&ext) && !binary_extensions.contains(&ext))
                .unwrap_or(false);
        })
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .filter_map(|s| {
            // Convert s to a `String`
            s.path()
                .to_path_buf()
                .as_os_str()
                .to_str()
                .map(|s| s.to_string())
        })
        .collect()
}

#[tracing::instrument(skip(changed_content))]
fn read_all_files(changed_content: Vec<ChangedContent>) -> Vec<Vec<u8>> {
    event!(
        tracing::Level::INFO,
        "Reading {:?} file(s)",
        changed_content.len()
    );

    changed_content
        .into_par_iter()
        .map(|c| match (c.file, c.content) {
            (Some(file), None) => std::fs::read(file).unwrap(),
            (None, Some(content)) => content.into_bytes(),
            _ => Default::default(),
        })
        .collect()
}

#[tracing::instrument(skip(blobs))]
fn parse_all_blobs(blobs: Vec<Vec<u8>>) -> Vec<String> {
    let input: Vec<_> = blobs.iter().map(|blob| &blob[..]).collect();
    let input = &input[..];

    let mut result: Vec<String> = input
        .par_iter()
        .map(|input| Extractor::unique(input, Default::default()))
        .reduce(Default::default, |mut a, b| {
            a.extend(b);
            a
        })
        .into_iter()
        .map(|s| {
            // SAFETY: When we parsed the candidates, we already guaranteed that the byte slices
            // are valid, therefore we don't have to re-check here when we want to convert it back
            // to a string.
            unsafe { String::from_utf8_unchecked(s.to_vec()) }
        })
        .collect();
    result.sort();
    result
}
