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
pub(crate) struct ProjectOpts {
    pub(crate) typ: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub struct ProjectDefinition {
    pub(crate) version: String,
    pub(crate) processed: bool,
    pub(crate) docs: indexmap::IndexMap<String, Vec<markdown::Block>>,
    pub(crate) options: ProjectOpts,
    pub(crate) commands: HashMap<String, Command>,
    pub(crate) about: String,
    pub(crate) help: String,

    pub is_pinned: bool,
    pub project_path: PathBuf,
    pub exec_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaltConfig {
    pub editor: Option<String>,
    pub pinned_paths: HashMap<String, String>,
}

pub type ProjectMap = HashMap<String, ProjectDefinition>;

#[cfg(debug_assertions)]
macro_rules! log {
    ($( $args:expr ),*) => {
        print!("[DEBUG]  ");
        println!( $( $args ),* );
    }
}

pub(crate) use log;
