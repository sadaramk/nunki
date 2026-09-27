//! What a machine is asked to load, and how it tells two books apart.
//!
//! Two properties, from one measurement. A comment inserted at the top of one
//! file changed 83 lines of `llms-full.txt` across five pages — every one a
//! citation of the file that moved — while the system described was identical.
//! The corpus is mostly citations, and citations are mostly noise to a reader
//! trying to understand a system rather than audit one.
//!
//! So: the evidence ledger leaves the corpus but not the book, and a fingerprint
//! answers the question the churn raised — is this the same system, or did the
//! code just move?

use std::path::{Path, PathBuf};
use std::process::Command;

use nunki_book::{model_hash, plan, BookOptions};

fn fixtures() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures")).to_path_buf()
}

fn files_of(repo: &Path) -> std::collections::BTreeMap<String, String> {
    let out = tempfile::tempdir().unwrap();
    plan(repo, out.path(), &BookOptions::default()).unwrap().files
}

/// The ledger is 17% of what an agent loads and the least useful part of it per
/// byte. It stays on its page, where an auditor reads it; it leaves the file
/// whose whole purpose is to be poured into a context window.
#[test]
fn the_evidence_ledger_stays_on_its_page_and_leaves_the_corpus() {
    let files = files_of(&fixtures().join("polyglot-shop"));
    let page = &files["pages/09-evidence-and-unknowns.md"];
    let corpus = &files["llms-full.txt"];

    let rows = |text: &str| {
        text.split("## Citation index").nth(1).map(|t| t.lines().filter(|l| l.starts_with("| ")).count()).unwrap_or(0)
    };
    assert!(rows(page) > 50, "the page keeps the whole ledger: {} rows", rows(page));
    assert_eq!(rows(corpus), 0, "the corpus carries no ledger rows");

    // Left out, not lost: the corpus says what is missing and where it is.
    let note = corpus.split("## Citation index").nth(1).expect("the corpus says the ledger was left out");
    assert!(
        note.contains("Left out of this file"),
        "the omission is stated, not silent: {}",
        &note[..200.min(note.len())]
    );
    assert!(note.contains("pages/09-evidence-and-unknowns.md"), "and says where to read it instead");

    // Everything else survived. Dropping one section must not drop the page.
    for heading in ["## Observed versus declared", "## What remains unknown"] {
        assert!(corpus.contains(heading), "the corpus lost {heading} along with the ledger");
    }
}

/// The saving is the point, and it is stated as a property rather than a number
/// so it survives the fixture growing.
#[test]
fn dropping_the_ledger_is_worth_doing() {
    let files = files_of(&fixtures().join("polyglot-shop"));
    let page = files["pages/09-evidence-and-unknowns.md"].len();
    let corpus = files["llms-full.txt"].len();
    let pages: usize = files.iter().filter(|(p, _)| p.starts_with("pages/")).map(|(_, t)| t.len()).sum();

    assert!(
        corpus + page / 2 < pages + page,
        "the corpus ({corpus}) should be materially smaller than every page concatenated ({})",
        pages + page
    );
}

fn git(repo: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.email=t@e", "-c", "user.name=t"])
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "git {args:?}");
}

/// The property the fingerprint exists for: a commit that moves code without
/// changing what it does must not move it.
///
/// This is also the test that fails if a field carrying a location, a date or a
/// size is ever added to the typed models without being normalised — which is
/// how `size` was found, holding `"8 files · 190 lines"` and moving the
/// fingerprint because somebody wrote a comment.
#[test]
fn model_hash_ignores_where_the_code_sits() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("shop");
    let src = fixtures().join("polyglot-shop");
    let copy = Command::new("cp").arg("-R").arg(&src).arg(&repo).status().unwrap();
    assert!(copy.success());
    git(&repo, &["init", "-q", "."]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    let before = model_hash(&files_of(&repo));

    // A comment at the top of a cited file: every citation of it shifts by one.
    let handler = repo.join("api-gateway/src/routes/checkout.ts");
    let text = std::fs::read_to_string(&handler).unwrap();
    std::fs::write(&handler, format!("// a comment, nothing more\n{text}")).unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "comment only"]);
    let after = model_hash(&files_of(&repo));

    assert_eq!(before, after, "a comment moved the fingerprint, so it is reporting changes that did not happen");
}

/// And the property that stops it being useless: something the book actually
/// claims must move it.
#[test]
fn model_hash_moves_when_the_system_does() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("shop");
    let copy = Command::new("cp").arg("-R").arg(fixtures().join("polyglot-shop")).arg(&repo).status().unwrap();
    assert!(copy.success());
    git(&repo, &["init", "-q", "."]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    let before = model_hash(&files_of(&repo));

    let routes = repo.join("api-gateway/src/routes/catalog.ts");
    let text = std::fs::read_to_string(&routes).unwrap();
    std::fs::write(
        &routes,
        format!("{text}\ncatalogRouter.delete(\"/catalog/:id\", (req, res) => {{ res.status(204).end(); }});\n"),
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "a new operation"]);
    let after = model_hash(&files_of(&repo));

    assert_ne!(before, after, "a new operation did not move the fingerprint, so it cannot be trusted to detect one");
}

/// It is published where both kinds of reader look, or it may as well not exist.
#[test]
fn the_fingerprint_is_published_for_a_reader_to_compare() {
    let files = files_of(&fixtures().join("polyglot-shop"));
    let hash = model_hash(&files);
    assert!(files["llms.txt"].contains(&hash), "llms.txt does not carry the fingerprint an agent would compare");
}
