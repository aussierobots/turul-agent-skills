//! Discovery over temporary directory trees.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use turul_agent_skills::{
    Discovered, ErrorCode, MAX_SCAN_DEPTH, MAX_SCAN_DIRS, Scope, ScopeKind, discover,
};

fn write_skill(dir: &Path, name: &str) -> PathBuf {
    let skill = dir.join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Skill {name}.\n---\nBody of {name}.\n"),
    )
    .unwrap();
    skill
}

fn scope(kind: ScopeKind, root: &Path) -> Scope {
    Scope {
        kind,
        root: root.to_path_buf(),
    }
}

fn names(found: &Discovered) -> Vec<&str> {
    found
        .skills
        .iter()
        .map(|s| s.properties.name.as_str())
        .collect()
}

fn codes(found: &Discovered) -> Vec<ErrorCode> {
    found.diagnostics.iter().map(|d| d.error.code()).collect()
}

#[test]
fn finds_nested_skills_and_stops_at_skill_dir() {
    let tmp = TempDir::new().unwrap();
    write_skill(tmp.path(), "top");
    write_skill(&tmp.path().join("group/deeper"), "nested");
    write_skill(&tmp.path().join("top/scripts"), "inside-skill");
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["nested", "top"]);
    assert!(found.diagnostics.is_empty());
}

#[test]
fn skips_git_and_node_modules() {
    let tmp = TempDir::new().unwrap();
    write_skill(&tmp.path().join(".git"), "in-git");
    write_skill(&tmp.path().join("node_modules"), "in-modules");
    write_skill(tmp.path(), "kept");
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["kept"]);
}

#[test]
fn depth_cap_enforced() {
    let tmp = TempDir::new().unwrap();
    let mut within = tmp.path().to_path_buf();
    for level in 1..MAX_SCAN_DEPTH {
        within = within.join(format!("d{level}"));
    }
    write_skill(&within, "at-cap");
    write_skill(&within.join("deeper"), "over-cap");
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["at-cap"]);
}

#[test]
fn dir_cap_enforced_with_diagnostic() {
    let tmp = TempDir::new().unwrap();
    for i in 0..MAX_SCAN_DIRS + 100 {
        fs::create_dir(tmp.path().join(format!("d{i:05}"))).unwrap();
    }
    write_skill(tmp.path(), "zzz-after-cap");
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert!(found.skills.is_empty());
    assert_eq!(codes(&found), [ErrorCode::ScanLimitReached]);
}

#[test]
fn project_beats_user_with_shadow_diagnostic() {
    let tmp = TempDir::new().unwrap();
    let user = tmp.path().join("user");
    let project = tmp.path().join("project");
    let user_skill = write_skill(&user, "same");
    let project_skill = write_skill(&project, "same");
    let found = discover(&[
        scope(ScopeKind::User, &user),
        scope(ScopeKind::Project, &project),
    ]);
    assert_eq!(found.skills.len(), 1);
    let winner = found.skills[0].location().unwrap();
    assert_eq!(
        winner,
        project_skill.canonicalize().unwrap().join("SKILL.md")
    );
    assert_eq!(found.diagnostics.len(), 1);
    let shadow = &found.diagnostics[0];
    assert_eq!(shadow.error.code(), ErrorCode::SkillShadowed);
    assert_eq!(shadow.path, user_skill);
}

#[test]
fn within_kind_first_scope_then_sorted_path_wins() {
    let tmp = TempDir::new().unwrap();
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    write_skill(&second, "dup");
    write_skill(&first.join("b"), "dup");
    write_skill(&first.join("a"), "dup");
    let found = discover(&[
        scope(ScopeKind::Project, &first),
        scope(ScopeKind::Project, &second),
    ]);
    assert_eq!(found.skills.len(), 1);
    let winner = found.skills[0].location().unwrap();
    assert!(winner.starts_with(first.canonicalize().unwrap().join("a")));
    assert_eq!(codes(&found), [ErrorCode::SkillShadowed; 2]);
    assert_eq!(found.diagnostics[0].path, first.join("b/dup"));
    assert_eq!(found.diagnostics[1].path, second.join("dup"));
}

#[test]
fn output_sorted_by_name() {
    let tmp = TempDir::new().unwrap();
    for name in ["delta", "alpha", "charlie", "bravo"] {
        write_skill(tmp.path(), name);
    }
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["alpha", "bravo", "charlie", "delta"]);
}

#[test]
fn discover_is_deterministic() {
    let tmp = TempDir::new().unwrap();
    for name in ["one", "two", "three"] {
        write_skill(&tmp.path().join("a"), name);
        write_skill(&tmp.path().join("b"), name);
    }
    fs::create_dir_all(tmp.path().join("bad")).unwrap();
    fs::write(tmp.path().join("bad/SKILL.md"), "no frontmatter").unwrap();
    let scopes = [scope(ScopeKind::Project, tmp.path())];
    assert_eq!(discover(&scopes), discover(&scopes));
}

#[cfg(unix)]
#[test]
fn symlink_escaping_root_not_followed() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    let outside = tmp.path().join("outside");
    write_skill(&outside, "escaped");
    let root = tmp.path().join("root");
    fs::create_dir(&root).unwrap();
    symlink(&outside, root.join("link")).unwrap();
    let found = discover(&[scope(ScopeKind::Project, &root)]);
    assert!(found.skills.is_empty());
    assert_eq!(codes(&found), [ErrorCode::SymlinkEscape]);
}

#[cfg(unix)]
#[test]
fn symlink_inside_root_followed_once() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    write_skill(&tmp.path().join("real"), "linked");
    symlink(tmp.path().join("real"), tmp.path().join("alias")).unwrap();
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["linked"]);
    assert!(found.diagnostics.is_empty());
}

#[test]
fn malformed_skill_is_diagnostic_scan_continues() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("a-broken")).unwrap();
    fs::write(tmp.path().join("a-broken/SKILL.md"), "no frontmatter").unwrap();
    write_skill(tmp.path(), "good");
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["good"]);
    assert_eq!(codes(&found), [ErrorCode::FrontmatterMissing]);
    assert_eq!(found.diagnostics[0].path, tmp.path().join("a-broken"));
}

#[test]
fn missing_root_ignored() {
    let tmp = TempDir::new().unwrap();
    let found = discover(&[scope(ScopeKind::User, &tmp.path().join("absent"))]);
    assert!(found.skills.is_empty());
    assert!(found.diagnostics.is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_root_gives_scan_io() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("locked");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&root).is_ok();
    let found = discover(&[scope(ScopeKind::Project, &root)]);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return; // privileged user: permissions do not apply
    }
    assert_eq!(codes(&found), [ErrorCode::ScanIo]);
    assert!(
        found.diagnostics[0]
            .error
            .message()
            .starts_with(&format!("Cannot read {}: ", root.display()))
    );
}

#[cfg(unix)]
#[test]
fn skill_file_symlink_outside_root_skipped() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    let outside = write_skill(&tmp.path().join("outside"), "stolen");
    let root = tmp.path().join("root");
    fs::create_dir_all(root.join("stolen")).unwrap();
    symlink(outside.join("SKILL.md"), root.join("stolen/SKILL.md")).unwrap();
    let found = discover(&[scope(ScopeKind::Project, &root)]);
    assert!(found.skills.is_empty());
    assert_eq!(codes(&found), [ErrorCode::SymlinkEscape]);
}

#[test]
fn overlapping_scopes_load_once() {
    let tmp = TempDir::new().unwrap();
    write_skill(&tmp.path().join("sub"), "once");
    let found = discover(&[
        scope(ScopeKind::Project, tmp.path()),
        scope(ScopeKind::Project, &tmp.path().join("sub")),
        scope(ScopeKind::User, tmp.path()),
    ]);
    assert_eq!(names(&found), ["once"]);
    assert!(found.diagnostics.is_empty());
}

#[test]
fn discovery_messages_match_text() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    let loser = write_skill(&b, "dup");
    let winner = write_skill(&a, "dup");
    let found = discover(&[scope(ScopeKind::Project, &a), scope(ScopeKind::Project, &b)]);
    assert_eq!(
        found.diagnostics[0].error.message(),
        format!(
            "Skill 'dup' at {} is shadowed by {}",
            loser.display(),
            winner.display()
        )
    );

    let capped = tmp.path().join("capped");
    for i in 0..MAX_SCAN_DIRS + 1 {
        fs::create_dir_all(capped.join(format!("d{i:05}"))).unwrap();
    }
    let found = discover(&[scope(ScopeKind::Project, &capped)]);
    assert_eq!(
        found.diagnostics[0].error.message(),
        format!(
            "Stopped scanning {} after {MAX_SCAN_DIRS} directories",
            capped.display()
        )
    );
    assert_eq!(
        found.diagnostics[0].error.code().as_str(),
        "scan-limit-reached"
    );

    #[cfg(unix)]
    {
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        std::os::unix::fs::symlink(&a, root.join("link")).unwrap();
        let found = discover(&[scope(ScopeKind::Project, &root)]);
        assert_eq!(
            found.diagnostics[0].error.message(),
            format!(
                "Not following {}: it resolves outside {}",
                root.join("link").display(),
                root.display()
            )
        );
        assert_eq!(found.diagnostics[0].error.code().as_str(), "symlink-escape");
    }
}

#[test]
fn lowercase_skill_md_discovered() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("lower");
    fs::create_dir(&dir).unwrap();
    fs::write(
        dir.join("skill.md"),
        "---\nname: lower\ndescription: Lowercase file.\n---\n",
    )
    .unwrap();
    let found = discover(&[scope(ScopeKind::Project, tmp.path())]);
    assert_eq!(names(&found), ["lower"]);
}

#[test]
fn conventional_scopes_paths() {
    let base = Path::new("/work/app");
    let scopes = Scope::conventional(ScopeKind::Project, base);
    assert_eq!(
        scopes,
        [
            scope(ScopeKind::Project, &base.join(".agents/skills")),
            scope(ScopeKind::Project, &base.join(".claude/skills")),
        ]
    );
}
