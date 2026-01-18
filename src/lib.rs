// Library crate entry point for integration tests
// This file re-exports the necessary modules for testing

pub mod db;
pub mod email;
pub mod epub_gen;
pub mod feed;
#[cfg(feature = "mem_opt")]
pub mod image;
#[cfg(not(feature = "mem_opt"))]
#[path = "image_inmem.rs"]
pub mod image;
pub mod models;
pub mod opds;
pub mod processor;
pub mod util;
pub mod handlers;
pub mod routes;
pub mod scheduler;
pub mod templates;
