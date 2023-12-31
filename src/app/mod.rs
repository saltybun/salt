use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) mod doc;
pub(crate) mod interface;
pub(crate) mod parser;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Command {
    pub about: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct MDOptions {
    pub(crate) typ: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub struct MDBundle {
    pub(crate) version: String,
    pub(crate) processed: bool,
    pub(crate) docs: indexmap::IndexMap<String, Vec<markdown::Block>>,
    pub(crate) options: MDOptions,
    pub(crate) commands: HashMap<String, Command>,
    pub(crate) about: String,
    pub(crate) help: String,

    pub is_pinned: bool,
    pub bundle_path: PathBuf,
    pub exec_path: PathBuf,
    pub watcher: Watcher,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Watcher {
    pub debounce_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaltConfig {
    pub editor: Option<String>,
    pub pinned_paths: HashMap<String, String>,
}

pub type BundleMap = HashMap<String, MDBundle>;

#[cfg(debug_assertions)]
macro_rules! log {
    ($( $args:expr ),*) => {
        print!("[DEBUG]  ");
        println!( $( $args ),* );
    }
}

pub(crate) use log;
