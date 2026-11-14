use std::fs;
use std::io;
use std::path::Path;

use crate::skill::Skill;

/// Version-control metadata: never part of a skill's content. Distinct from the
/// discovery skip list, which also avoids `node_modules` only because it holds no skills.
const VCS_DIR: &str = ".git";

/// The files bundled with a skill: every file under its directory except the
/// `SKILL.md` it was loaded from, as relative paths.
///
/// This is the one definition of a skill's bundled files; [`activation`](crate::activation)
/// lists the same files. The walk is complete and uncapped, with this policy:
///
/// - Paths are relative to the skill directory, joined with `/`, and sorted by
///   byte order, so the same directory always gives the same list.
/// - A symbolic link is listed under its own name and never followed, whatever
///   it points to. A caller that opens one resolves it, and decides whether a
///   target outside the skill directory is acceptable.
/// - Directories named `.git` (version-control metadata) are not entered. Every
///   other directory, `node_modules` included, is skill content: the spec lets a
///   skill directory hold any files and directories.
/// - Hidden files and directories (names starting with `.`) are included.
/// - Anything that is neither a file, a directory nor a symbolic link (a
///   socket, a FIFO, a device) is skipped.
///
/// A skill without a [`Skill::location`] (built, or loaded with
/// [`load_str`](crate::load_str)) has no directory and so no bundled files.
///
/// # Errors
///
/// Returns the I/O error of the first directory or entry that cannot be read, and
/// [`io::ErrorKind::InvalidData`] for a file or directory name that is not UTF-8.
/// A partial list is never returned.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{Skill, bundled_files, load};
///
/// let tmp = tempfile::tempdir()?;
/// let file = Skill::builder("pdf-tools", "Work with PDFs.").build()?.write_to(tmp.path())?;
/// let dir = file.parent().unwrap();
/// std::fs::create_dir(dir.join("scripts"))?;
/// std::fs::write(dir.join("scripts/extract.py"), "print()")?;
/// std::fs::write(dir.join("README.md"), "notes")?;
///
/// let skill = load(dir)?.skill;
/// assert_eq!(bundled_files(&skill)?, ["README.md", "scripts/extract.py"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn bundled_files(skill: &Skill) -> io::Result<Vec<String>> {
    let Some(skill_file) = skill.location() else {
        return Ok(Vec::new());
    };
    let dir = skill_file.parent().unwrap_or(skill_file);
    let mut files = Vec::new();
    collect(dir, "", skill_file, &mut files)?;
    files.sort_unstable();
    Ok(files)
}

fn collect(dir: &Path, prefix: &str, skill_file: &Path, files: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name().into_string().map_err(|name| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("file name is not UTF-8: {}", Path::new(&name).display()),
            )
        })?;
        let relative = format!("{prefix}{name}");
        if kind.is_dir() {
            if name != VCS_DIR {
                collect(&entry.path(), &format!("{relative}/"), skill_file, files)?;
            }
        } else if (kind.is_file() || kind.is_symlink()) && entry.path() != skill_file {
            files.push(relative);
        }
    }
    Ok(())
}
