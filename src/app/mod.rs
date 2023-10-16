use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) mod interface;

#[derive(Serialize, Deserialize, Clone)]
pub struct Command {
    pub about: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaltBundle {
    pub name: String,
    pub requires: Option<Vec<String>>,
    // TODO: we can exclude some paths from getting notification event
    // for restart
    // pub exclude: Vec<String>,
    pub version: String,
    pub description: String,
    pub commands: HashMap<String, Command>,

    pub watcher: Watcher,

    #[serde(skip_serializing, skip_deserializing)]
    pub is_pinned: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub bundle_path: PathBuf,
    #[serde(skip_serializing, skip_deserializing)]
    pub exec_path: PathBuf,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Watcher {
    pub debounce_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaltConfig {
    pub pinned_paths: HashMap<String, String>,
}

pub type BundleMap = HashMap<String, SaltBundle>;
