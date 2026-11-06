#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

mod discover;
mod error;
mod load;
mod name;
mod parse;
mod prompt;
mod render;
mod skill;
mod validate;

pub use discover::{
    Diagnostic, Discovered, MAX_SCAN_DEPTH, MAX_SCAN_DIRS, Scope, ScopeKind, discover,
};
pub use error::{ErrorCode, ValidationError, display_path};
pub use load::{LoadSkip, Loaded, load};
pub use name::{NameError, NameViolation, SkillName};
pub use parse::{ParseError, RawSkill, RawValue, SkillProperties, parse_skill_md};
pub use prompt::{MAX_RESOURCE_FILES, activation, to_prompt};
pub use skill::Skill;
pub use validate::validate;
