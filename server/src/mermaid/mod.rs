//! Mermaid sequence diagram parser and layout engine.
//!
//! Parses Mermaid `sequenceDiagram` syntax into an AST and lays out the diagram
//! as a collection of board object descriptors (rectangles, arrows, text, frames)
//! ready for creation via the AI tool system.

pub mod ast;
pub mod layout;
pub mod parse;

pub use layout::render_to_objects;
pub use parse::parse;

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
