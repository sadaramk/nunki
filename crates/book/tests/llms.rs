//! `llms.txt` is the index an agent reads first, and it has to describe the
//! directory it sits in.
//!
//! It did not. `behaviour.json` — every requirement and rule already parsed,
//! with the evidence behind each — was written beside it and named nowhere, so
//! an agent reading the index learned about sixteen Markdown pages and went off
//! to parse prose for facts that were sitting in JSON next to it. The cause was
//! ordering: the index was assembled halfway through rendering, before
//! `index.html` and `behaviour.json` existed. An index built before its subject
//! indexes whatever happened to exist first.
//!
//! So the test is not "does it mention behaviour.json". It is: **every file the
//! book writes is either named in the index or deliberately excluded**, with
//! nowhere for a new output to hide.

use std::path::{Path, PathBuf};

use nunki_book::{plan, BookOptions};

fn fixtures() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures")).to_path_buf()
}

fn book_of(fixture: &str) -> (std::collections::BTreeMap<String, String>, tempfile::TempDir) {
    let out = tempfile::tempdir().unwrap();
    let files = plan(&fixtures().join(fixture), out.path(), &BookOptions::default()).unwrap().files;
    (files, out)
}

/// The property that cannot rot: adding an output file forces a decision about
/// whether an agent should be pointed at it, because the alternative is a
/// failing test rather than an index that quietly omits it.
#[test]
fn every_file_the_book_writes_is_advertised_or_deliberately_not() {
    let (files, _keep) = book_of("polyglot-shop");
    let index = &files["llms.txt"];

    let mut missing: Vec<&String> = Vec::new();
    for path in files.keys() {
        if nunki_book::markdown::unadvertised(path) {
            continue;
        }
        if !index.contains(path.as_str()) {
            missing.push(path);
        }
    }
    assert!(
        missing.is_empty(),
        "written but not named in llms.txt, and not excluded by `unadvertised`: {missing:#?}\n\n\
         Either point an agent at it, or say in `unadvertised` why not."
    );
}

/// The exclusions are a decision, not a hole. Each one is here because pointing
/// an agent at it would waste the read, and this pins the reason: a `.svg` is a
/// picture of the `.ir.json` beside it, and `index.html` is a reader, not a
/// document.
#[test]
fn what_is_excluded_is_excluded_for_a_reason_that_still_holds() {
    let (files, _keep) = book_of("polyglot-shop");
    assert!(nunki_book::markdown::unadvertised("index.html"));
    assert!(nunki_book::markdown::unadvertised("llms.txt"));
    assert!(nunki_book::markdown::unadvertised("diagrams/containers.svg"));
    // Found by the exhaustiveness check above rather than by inspection: the
    // Markdown mirror's own index is this index, written for a person.
    assert!(nunki_book::markdown::unadvertised("README.md"));

    // Every excluded diagram rendering has the machine-readable form of the
    // same figure advertised in its place, so nothing is actually unreachable.
    for path in files.keys().filter(|p| p.ends_with(".svg")) {
        let ir = path.replace(".svg", ".ir.json");
        assert!(files.contains_key(&ir), "{path} is excluded but {ir} does not exist to stand in for it");
        assert!(files["llms.txt"].contains(&ir), "{ir} must be advertised, since {path} is not");
    }
}

/// The two files a machine would most want, and the one it cannot get from the
/// code at all. Named explicitly rather than left to the exhaustiveness check,
/// because these are the reason the issue existed.
#[test]
fn the_structured_model_and_its_provenance_are_named_first() {
    let (files, _keep) = book_of("polyglot-shop");
    let index = &files["llms.txt"];

    let structured = index.find("## Structured model").expect("the index has a structured section");
    let pages = index.find("## Pages").expect("the index has a pages section");
    assert!(structured < pages, "an agent reading one thing should reach the model before the prose");

    for (path, why) in [
        ("behaviour.json", "the requirements and rules as data"),
        ("manifest.json", "what this book was generated from"),
        ("authored.json", "what only a person can answer"),
    ] {
        assert!(index.contains(path), "llms.txt never mentions {path} ({why})");
    }

    // `behaviour.json` must be *sized*, not merely named. Its size can only be
    // known if the index was built after it was written, so this is what fails
    // if the index ever moves back ahead of the files it describes — the
    // original defect, which a name hardcoded in a template would survive.
    let line = index.lines().find(|l| l.contains("behaviour.json")).unwrap();
    assert!(
        line.contains(" · ") && line.contains("kB"),
        "behaviour.json is named without a size, so the index was built before it existed: {line}"
    );
}

/// An agent choosing what to read under a budget needs to know what a read
/// costs before it pays. The figures are exact — we are holding the bytes.
#[test]
fn every_page_says_what_it_costs() {
    let (files, _keep) = book_of("polyglot-shop");
    let index = &files["llms.txt"];

    for (path, text) in files.iter().filter(|(p, _)| p.starts_with("pages/")) {
        let line =
            index.lines().find(|l| l.contains(path.as_str())).unwrap_or_else(|| panic!("{path} is not in the index"));
        assert!(line.contains(" · "), "{path} has no size: {line}");

        // The number is the file's own, not a guess: a 2.2 kB page cannot be
        // announced as 6 kB, and a rounded figure must round to the truth.
        let stated = line.split(" · ").nth(1).and_then(|s| s.split(':').next()).unwrap_or_default().trim();
        let actual = if text.len() < 1024 {
            format!("{} B", text.len())
        } else {
            format!("{:.1} kB", text.len() as f64 / 1024.0)
        };
        assert_eq!(stated, actual, "{path} is advertised as {stated} but is {actual}");
    }
}

/// A command-line tool writes fewer pages and no data model; the index still
/// has to describe whatever it did write, rather than a shape assumed here.
#[test]
fn a_book_of_a_different_shape_is_described_just_as_completely() {
    let (files, _keep) = book_of("cli-tool");
    let index = &files["llms.txt"];
    let missing: Vec<&String> =
        files.keys().filter(|p| !nunki_book::markdown::unadvertised(p) && !index.contains(p.as_str())).collect();
    assert!(missing.is_empty(), "a CLI's book leaves these unadvertised: {missing:#?}");
}
