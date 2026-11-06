//! The project README shows code from the library README, where every snippet compiles and
//! runs as a doctest. Each Rust block in the project README must equal one library block with
//! its hidden (`# `) setup lines removed, so the project README cannot drift from the API.

use std::path::Path;

fn rust_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in markdown.lines() {
        match (&mut current, line.trim_end()) {
            (None, "```rust") => current = Some(Vec::new()),
            (Some(lines), "```") => {
                blocks.push(lines.join("\n"));
                current = None;
            }
            (Some(lines), _) => lines.push(line),
            (None, _) => {}
        }
    }
    blocks
}

fn without_hidden_lines(block: &str) -> String {
    block
        .lines()
        .filter(|line| !(line.starts_with("# ") || *line == "#"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn project_readme_snippets_are_doctested_library_snippets() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let library: Vec<String> = rust_blocks(&read(&crate_dir.join("README.md")))
        .iter()
        .map(|b| without_hidden_lines(b))
        .collect();
    let project = rust_blocks(&read(&crate_dir.join("../../README.md")));
    assert!(!project.is_empty(), "project README has no Rust snippets");
    for block in &project {
        assert!(
            library.contains(block),
            "project README snippet is not a library README doctest:\n{block}"
        );
    }
}

#[test]
fn hidden_lines_are_stripped() {
    let block = "# let dir = x;\nlet a = 1;\n#\n# Ok::<(), ()>(())";
    assert_eq!(without_hidden_lines(block), "let a = 1;");
}
