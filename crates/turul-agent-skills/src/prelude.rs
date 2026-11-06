//! The types and functions most programs need, for one glob import.
//!
//! It re-exports [`Scope`], [`ScopeKind`], [`Skill`], [`SkillBuilder`],
//! [`SkillName`], [`activation`], [`discover`], [`load`], [`to_prompt`] and
//! [`validate`]. Error and diagnostic types are named through the crate root.
//!
//! # Examples
//!
//! ```
//! use turul_agent_skills::prelude::*;
//!
//! let skill = Skill::builder("summarize", "Summarize a document.").build()?;
//! let catalog = to_prompt(std::slice::from_ref(&skill)).unwrap();
//! assert!(catalog.contains("<name>\nsummarize\n</name>"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub use crate::{
    Scope, ScopeKind, Skill, SkillBuilder, SkillName, activation, discover, load, to_prompt,
    validate,
};
