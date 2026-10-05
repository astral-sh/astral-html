//! Shared fixtures and correctness checks for the comparison benchmarks.

pub mod adapters;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const NATIVE_DOCUMENTS: &[&str] = &["astral-document", "astral-tl", "scraper"];

#[derive(Debug, Deserialize)]
pub struct FixtureInfo {
    pub id: String,
    pub title: String,
    pub file: String,
    pub category: String,
    pub bytes: usize,
    pub sha256: String,
}

pub struct Fixture {
    pub info: FixtureInfo,
    pub source: String,
}

/// Read and verify the pinned corpus before any timed work starts.
pub fn fixtures() -> Result<Vec<Fixture>, String> {
    #[derive(Deserialize)]
    struct Manifest {
        version: u32,
        fixtures: Vec<FixtureInfo>,
    }

    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let manifest: Manifest = serde_json::from_slice(
        &fs::read(directory.join("manifest.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if manifest.version != 1 || manifest.fixtures.is_empty() {
        return Err("expected a nonempty version 1 fixture manifest".into());
    }
    let mut ids = HashSet::new();
    manifest
        .fixtures
        .into_iter()
        .map(|info| {
            if info.id.contains('/') || !ids.insert(info.id.clone()) {
                return Err(format!("invalid or duplicate fixture ID: {}", info.id));
            }
            let source = fs::read_to_string(directory.join(&info.file))
                .map_err(|error| format!("{}: {error}", info.file))?;
            let digest = format!("{:x}", Sha256::digest(source.as_bytes()));
            if source.len() != info.bytes || digest != info.sha256 {
                return Err(format!("{}: fixture size or SHA-256 mismatch", info.id));
            }
            Ok(Fixture { info, source })
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct Eligibility {
    pub workload: &'static str,
    pub fixture: String,
    pub parser: &'static str,
    pub eligible: bool,
    pub links: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Compare complete owned extraction results with the browser-tree reference.
/// Native-document timings use the same eligibility gate, but retain each
/// library's own representation; they do not imply equivalent tree semantics.
pub fn validate(fixtures: &[Fixture]) -> Result<Vec<Eligibility>, String> {
    adapters::prepare();
    let mut rows = Vec::new();
    for fixture in fixtures {
        let expected = adapters::extract("scraper", &fixture.source)
            .map_err(|error| format!("{}: reference failed: {error}", fixture.info.id))?;
        for &parser in adapters::IMPLEMENTATIONS {
            let actual = adapters::extract(parser, &fixture.source);
            let links = actual.as_ref().ok().map(Vec::len);
            let error = match actual {
                Ok(actual) if actual == expected => None,
                Ok(actual) => {
                    let index = actual
                        .iter()
                        .zip(&expected)
                        .position(|(actual, expected)| actual != expected)
                        .unwrap_or(actual.len().min(expected.len()));
                    Some(format!(
                        "expected {} links, got {}; first difference at {index}: expected {:?}, got {:?}",
                        expected.len(),
                        actual.len(),
                        expected.get(index),
                        actual.get(index)
                    ))
                }
                Err(error) => Some(error),
            };
            for workload in ["extract-links", "parse-document"] {
                if workload == "parse-document" && !NATIVE_DOCUMENTS.contains(&parser) {
                    continue;
                }
                rows.push(Eligibility {
                    workload,
                    fixture: fixture.info.id.clone(),
                    parser,
                    eligible: error.is_none(),
                    links,
                    error: error.clone(),
                });
            }
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    #[test]
    fn corpus_validation_covers_every_supported_pair() {
        let fixtures = super::fixtures().unwrap();
        let rows = super::validate(&fixtures).unwrap();
        assert_eq!(
            rows.len(),
            fixtures.len()
                * (super::adapters::IMPLEMENTATIONS.len() + super::NATIVE_DOCUMENTS.len())
        );
        for fixture in fixtures {
            for parser in ["astral-reader", "astral-document", "scraper"] {
                let row = rows
                    .iter()
                    .find(|row| {
                        row.workload == "extract-links"
                            && row.fixture == fixture.info.id
                            && row.parser == parser
                    })
                    .unwrap();
                assert!(row.eligible, "{row:?}");
                assert!(row.links.is_some_and(|count| count > 0), "{row:?}");
            }
        }
    }
}
