//! Finanzas core: data model, envelope-budget rules and SQLite storage shared by
//! the desktop app and the MCP server.

pub mod domain;
pub mod models;
pub mod store;
pub mod service;

pub use models::*;
