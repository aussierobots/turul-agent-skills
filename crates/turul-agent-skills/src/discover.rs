use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ErrorCode, ValidationError, display_path};
use crate::load::{Loaded, load};
use crate::name::SkillName;
use crate::skill::Skill;
use crate::validate::find_skill_md;

/// Deepest directory level scanned below a root; directories deeper are not
/// entered. [`discover`] and [`activation`](crate::activation) both honor it.
///
/// ```
/// assert_eq!(turul_agent_skills::MAX_SCAN_DEPTH, 6);
/// ```
pub const MAX_SCAN_DEPTH: usize = 6;

/// Most directories scanned in one scope before it is cut short, which
/// [`discover`] reports as [`ErrorCode::ScanLimitReached`].
///
/// ```
/// assert_eq!(turul_agent_skills::MAX_SCAN_DIRS, 2000);
/// ```
pub const MAX_SCAN_DIRS: usize = 2000;

/// Directory names no scan enters.
pub(crate) const SKIPPED_DIRS: [&str; 2] = [".git", "node_modules"];

/// Where a scope's skills come from; a project skill beats a user skill of the same name.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::ScopeKind;
///
/// assert!(ScopeKind::Project < ScopeKind::User);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ScopeKind {
    /// Skills that belong to the project being worked on.
    Project,
    /// Skills that belong to the user, available in every project.
    User,
}

/// A directory tree to scan for skills.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use turul_agent_skills::{Scope, ScopeKind};
///
/// let scope = Scope { kind: ScopeKind::User, root: PathBuf::from("/home/me/.agents/skills") };
/// assert_eq!(scope.kind, ScopeKind::User);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    /// Whether the skills are project or user skills.
    pub kind: ScopeKind,
    /// The directory whose subdirectories are skills.
    pub root: PathBuf,
}

impl Scope {
    /// The two conventional scopes under `base`, both of `kind`:
    /// `<base>/.agents/skills` then `<base>/.claude/skills`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use turul_agent_skills::{Scope, ScopeKind};
    ///
    /// let scopes = Scope::conventional(ScopeKind::Project, Path::new("/work/app"));
    /// assert_eq!(scopes[0].root, Path::new("/work/app/.agents/skills"));
    /// assert_eq!(scopes[1].root, Path::new("/work/app/.claude/skills"));
    /// ```
    pub fn conventional(kind: ScopeKind, base: &Path) -> Vec<Scope> {
        [".agents/skills", ".claude/skills"]
            .into_iter()
            .map(|relative| Scope {
                kind,
                root: base.join(relative),
            })
            .collect()
    }
}

/// A problem found while discovering, with the path it concerns.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, Scope, ScopeKind, discover};
///
/// let tmp = tempfile::tempdir()?;
/// let skills = tmp.path().join(".agents/skills");
/// std::fs::create_dir_all(skills.join("broken"))?;
/// std::fs::write(skills.join("broken/SKILL.md"), "no frontmatter")?;
///
/// let found = discover(&Scope::conventional(ScopeKind::Project, tmp.path()));
/// let diagnostic = &found.diagnostics[0];
/// assert!(diagnostic.path.ends_with("broken"));
/// assert_eq!(diagnostic.error.code(), ErrorCode::FrontmatterMissing);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// The skill directory or scan path the problem concerns.
    pub path: PathBuf,
    /// What is wrong.
    pub error: ValidationError,
}

/// The outcome of [`discover`]: the winning skills and every problem met on the way.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, Scope, ScopeKind, Skill, discover};
///
/// let project = tempfile::tempdir()?;
/// let user = tempfile::tempdir()?;
/// Skill::builder("pdf-tools", "Project copy.").build()?.write_to(project.path())?;
/// Skill::builder("pdf-tools", "User copy.").build()?.write_to(user.path())?;
///
/// let found = discover(&[
///     Scope { kind: ScopeKind::User, root: user.path().to_path_buf() },
///     Scope { kind: ScopeKind::Project, root: project.path().to_path_buf() },
/// ]);
/// assert_eq!(found.skills.len(), 1);
/// assert_eq!(found.skills[0].properties.description, "Project copy.");
/// assert_eq!(found.diagnostics[0].error.code(), ErrorCode::SkillShadowed);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Discovered {
    /// The winning skills, sorted by name.
    pub skills: Vec<Skill>,
    /// Problems in scan order, then one per shadowed skill.
    pub diagnostics: Vec<Diagnostic>,
}

struct Candidate {
    kind: ScopeKind,
    path: PathBuf,
    skill: Skill,
}

/// Scans `scopes` in the order given and returns the skills found.
///
/// A project skill beats a user skill of the same name; otherwise the earlier
/// scope, then the earlier path in a byte-sorted depth-first scan, wins. Losers
/// and every load problem are reported as diagnostics; the scan always
/// continues. Symbolic links that resolve outside their scope root are not
/// followed. A scope whose root does not exist is skipped silently, as is a
/// root already scanned by an earlier scope.
///
/// Scanning stops descending at [`MAX_SCAN_DEPTH`] levels and stops a scope at
/// [`MAX_SCAN_DIRS`] directories. Directories named `.git` and `node_modules`
/// are never entered.
///
/// This function does not return a `Result`; problems appear in
/// [`Discovered::diagnostics`] with codes such as
/// [`SkillShadowed`](ErrorCode::SkillShadowed),
/// [`ScanLimitReached`](ErrorCode::ScanLimitReached),
/// [`SymlinkEscape`](ErrorCode::SymlinkEscape),
/// [`ScanIo`](ErrorCode::ScanIo) and every code [`load`](crate::load) can
/// report.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{Scope, ScopeKind, Skill, discover};
///
/// let tmp = tempfile::tempdir()?;
/// let skills = tmp.path().join(".agents/skills");
/// Skill::builder("pdf-tools", "Work with PDFs.").build()?.write_to(&skills)?;
/// Skill::builder("changelog", "Write changelogs.").build()?.write_to(&skills)?;
///
/// let found = discover(&Scope::conventional(ScopeKind::Project, tmp.path()));
/// let names: Vec<&str> = found.skills.iter().map(|s| s.properties.name.as_str()).collect();
/// assert_eq!(names, ["changelog", "pdf-tools"]);
/// assert!(found.diagnostics.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn discover(scopes: &[Scope]) -> Discovered {
    let mut diagnostics = Vec::new();
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for scope in scopes {
        scan_scope(scope, &mut seen, &mut candidates, &mut diagnostics);
    }

    candidates.sort_by_key(|candidate| candidate.kind);
    let mut winners: Vec<(SkillName, PathBuf)> = Vec::new();
    let mut skills = Vec::new();
    let mut shadows = Vec::new();
    for candidate in candidates {
        let name = candidate.skill.properties.name.clone();
        match winners.iter().find(|(won, _)| *won == name) {
            Some((_, winner)) => shadows.push((name, candidate.path, winner.clone())),
            None => {
                winners.push((name, candidate.path));
                skills.push(candidate.skill);
            }
        }
    }
    skills.sort_by(|a, b| a.properties.name.cmp(&b.properties.name));
    shadows.sort_by(|a, b| a.0.cmp(&b.0));
    diagnostics.extend(shadows.into_iter().map(|(name, path, winner)| Diagnostic {
        error: ValidationError::new(
            ErrorCode::SkillShadowed,
            format!(
                "Skill '{}' at {} is shadowed by {}",
                name.as_str(),
                display_path(&path),
                display_path(&winner)
            ),
        ),
        path,
    }));
    Discovered {
        skills,
        diagnostics,
    }
}

struct Walk<'a> {
    scope: &'a Scope,
    canonical_root: PathBuf,
    scanned: usize,
    stopped: bool,
    seen: &'a mut HashSet<PathBuf>,
    candidates: &'a mut Vec<Candidate>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

fn scan_scope(
    scope: &Scope,
    seen: &mut HashSet<PathBuf>,
    candidates: &mut Vec<Candidate>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Ok(canonical_root) = scope.root.canonicalize() else {
        return;
    };
    if !seen.insert(canonical_root.clone()) {
        return;
    }
    let mut walk = Walk {
        scope,
        canonical_root,
        scanned: 0,
        stopped: false,
        seen,
        candidates,
        diagnostics,
    };
    walk.directory(&scope.root, 0);
}

impl Walk<'_> {
    fn report(&mut self, path: &Path, code: ErrorCode, message: String) {
        self.diagnostics.push(Diagnostic {
            path: path.to_path_buf(),
            error: ValidationError::new(code, message),
        });
    }

    fn escape(&mut self, path: &Path) {
        let message = format!(
            "Not following {}: it resolves outside {}",
            display_path(path),
            display_path(&self.scope.root)
        );
        self.report(path, ErrorCode::SymlinkEscape, message);
    }

    fn inside_root(&self, path: &Path) -> bool {
        path.canonicalize()
            .is_ok_and(|canonical| canonical.starts_with(&self.canonical_root))
    }

    fn directory(&mut self, dir: &Path, depth: usize) {
        if self.stopped {
            return;
        }
        if self.scanned >= MAX_SCAN_DIRS {
            self.stopped = true;
            let message = format!(
                "Stopped scanning {} after {MAX_SCAN_DIRS} directories",
                display_path(&self.scope.root)
            );
            self.report(
                &self.scope.root.clone(),
                ErrorCode::ScanLimitReached,
                message,
            );
            return;
        }
        self.scanned += 1;
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                let message = format!("Cannot read {}: {error}", display_path(dir));
                self.report(dir, ErrorCode::ScanIo, message);
                return;
            }
        };
        let mut children: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
        children.sort_by(|a, b| {
            a.file_name()
                .map(|n| n.as_encoded_bytes())
                .cmp(&b.file_name().map(|n| n.as_encoded_bytes()))
        });
        for child in children {
            if self.stopped {
                return;
            }
            self.child(&child, depth + 1);
        }
    }

    fn child(&mut self, path: &Path, depth: usize) {
        let name = path.file_name().map(|n| n.to_string_lossy());
        if depth > MAX_SCAN_DEPTH
            || name.as_deref().is_some_and(|n| SKIPPED_DIRS.contains(&n))
            || !path.is_dir()
        {
            return;
        }
        if !self.inside_root(path) {
            self.escape(path);
            return;
        }
        let Ok(canonical) = path.canonicalize() else {
            return;
        };
        if !self.seen.insert(canonical) {
            return;
        }
        match find_skill_md(path) {
            Some(file) if !self.inside_root(&file) => self.escape(&file),
            Some(_) => self.load_candidate(path),
            None => self.directory(path, depth),
        }
    }

    fn load_candidate(&mut self, dir: &Path) {
        let (skill, problems) = match load(dir) {
            Ok(Loaded { skill, diagnostics }) => (Some(skill), diagnostics),
            Err(skip) => (None, skip.errors),
        };
        for error in problems {
            self.diagnostics.push(Diagnostic {
                path: dir.to_path_buf(),
                error,
            });
        }
        if let Some(skill) = skill {
            self.candidates.push(Candidate {
                kind: self.scope.kind,
                path: dir.to_path_buf(),
                skill,
            });
        }
    }
}
