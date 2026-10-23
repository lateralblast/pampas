//! pampas as a library: the providers, the config and key handling, and the work of a run, shared by
//! the command line (`src/main.rs`) and the GUI.

pub mod browser;
pub mod engine;
pub mod history;
pub mod keystore;
pub mod merge;
pub mod output;
pub mod paths;
pub mod provider;
pub mod scrape;
pub mod settings;
pub mod usage;
