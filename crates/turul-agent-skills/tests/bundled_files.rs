//! A skill's bundled files: one complete, uncapped, deterministic walk that
//! `activation` also lists from.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use turul_agent_skills::{MAX_RESOURCE_FILES, Skill, activation, bundled_files, load, load_str};

/// A loaded skill `my-skill` under a fresh temporary root, with `files` written beside it.
fn skill_with(files: &[&str]) -> (TempDir, PathBuf, Skill) {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().canonicalize().unwrap().join("my-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: d\n---\nBody\n",
    )
    .unwrap();
    for file in files {
        write(&dir, file);
    }
    let skill = load(&dir).unwrap().skill;
    (tmp, dir, skill)
}

fn write(dir: &Path, relative: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "x").unwrap();
}

#[test]
fn lists_every_file_except_skill_md_in_byte_order() {
    let (_tmp, _dir, skill) = skill_with(&[
        "scripts/run.sh",
        "Z.txt",
        "a.txt",
        "references/guide.md",
        "assets/b/c.json",
    ]);
    assert_eq!(
        bundled_files(&skill).unwrap(),
        [
            "Z.txt",
            "a.txt",
            "assets/b/c.json",
            "references/guide.md",
            "scripts/run.sh"
        ]
    );
}

#[test]
fn same_directory_gives_same_list() {
    let (_tmp, _dir, skill) = skill_with(&["b", "a", "c/d", "c/a"]);
    let first = bundled_files(&skill).unwrap();
    for _ in 0..5 {
        assert_eq!(bundled_files(&skill).unwrap(), first);
    }
}

#[test]
fn has_no_cap() {
    let names: Vec<String> = (0..MAX_RESOURCE_FILES * 3)
        .map(|i| format!("f{i:04}.txt"))
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let (_tmp, _dir, skill) = skill_with(&refs);
    assert_eq!(bundled_files(&skill).unwrap(), names);
}

#[test]
fn includes_hidden_files_and_node_modules_and_skips_git() {
    let (_tmp, _dir, skill) = skill_with(&[
        ".env.example",
        ".config/settings.json",
        ".git/HEAD",
        "deep/.git/config",
        "node_modules/x/index.js",
    ]);
    assert_eq!(
        bundled_files(&skill).unwrap(),
        [
            ".config/settings.json",
            ".env.example",
            "node_modules/x/index.js"
        ]
    );
}

#[test]
fn lists_every_depth() {
    let deep = format!("{}leaf.txt", "d/".repeat(20));
    let (_tmp, _dir, skill) = skill_with(&[&deep]);
    assert_eq!(bundled_files(&skill).unwrap(), [deep]);
}

#[cfg(unix)]
#[test]
fn lists_symlinks_without_following_them() {
    let (tmp, dir, skill) = skill_with(&["real.txt"]);
    let outside = tmp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "x").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("linked-dir")).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.txt"), dir.join("linked-file")).unwrap();
    std::os::unix::fs::symlink(dir.join("real.txt"), dir.join("inner-link")).unwrap();
    assert_eq!(
        bundled_files(&skill).unwrap(),
        ["inner-link", "linked-dir", "linked-file", "real.txt"]
    );
}

#[test]
fn skill_without_location_has_none() {
    let built = Skill::builder("my-skill", "d").build().unwrap();
    assert!(bundled_files(&built).unwrap().is_empty());
    let embedded = load_str("my-skill", "---\nname: my-skill\ndescription: d\n---\n")
        .unwrap()
        .skill;
    assert!(bundled_files(&embedded).unwrap().is_empty());
}

#[test]
fn lowercase_skill_md_is_the_skill_file() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().canonicalize().unwrap().join("my-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("skill.md"),
        "---\nname: my-skill\ndescription: d\n---\n",
    )
    .unwrap();
    write(&dir, "notes.md");
    let skill = load(&dir).unwrap().skill;
    assert_eq!(bundled_files(&skill).unwrap(), ["notes.md"]);
}

#[cfg(unix)]
#[test]
fn non_utf8_name_is_an_error_not_a_partial_list() {
    use std::os::unix::ffi::OsStrExt;
    let (_tmp, dir, skill) = skill_with(&["ok.txt"]);
    let bad = std::ffi::OsStr::from_bytes(b"bad-\xff.txt");
    if fs::write(dir.join(bad), "x").is_err() {
        return; // the filesystem refuses non-UTF-8 names
    }
    let error = bundled_files(&skill).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn activation_lists_the_first_files_of_the_same_walk() {
    let names: Vec<String> = (0..MAX_RESOURCE_FILES + 5)
        .map(|i| format!("f{i:04}.txt"))
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let (_tmp, _dir, skill) = skill_with(&refs);
    let listed: Vec<String> = activation(&skill)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<file>"))
        .map(|line| line.trim_end_matches("</file>").to_string())
        .collect();
    let all = bundled_files(&skill).unwrap();
    assert_eq!(listed, all[..MAX_RESOURCE_FILES]);
}
