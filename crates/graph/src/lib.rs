//! Document graph: topic/concept hierarchy and semantic link management.
//!
//! This crate builds a two-level topic → concept hierarchy from markdown
//! headings and assigns text chunks to the appropriate level based on their
//! character offsets. It also exposes the underlying graph types for semantic
//! links between chunks.
//!
//! # Modules
//!
//! - [`heading_parser`] — Markdown heading extraction.
//! - [`hierarchy`] — Topic/concept tree builder.
//! - [`types`] — Core graph domain types.

pub mod heading_parser;
pub mod hierarchy;
pub mod types;

pub use hierarchy::{ConceptWithChunks, HierarchyResult, TopicWithConcepts};
pub use types::{Concept, HeadingSpan, SemanticLink, Topic};

/// Top-level graph container for document/section/chunk relationships.
///
/// Currently acts as a namespace; future iterations may hold an in-memory
/// graph index or adjacency structures.
///
/// # Examples
///
/// ```
/// use graph::Graph;
/// let _g = Graph;
/// ```
pub struct Graph;
