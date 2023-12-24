use std::collections::HashMap;
use std::io::{Result, Write};
use std::path::PathBuf;
use std::process::Stdio;

use crate::watcher::async_watch;

use super::MDBundle;
use super::{BundleMap, SaltConfig};

static HBS_FILE: &str = include_str!("../../templates/salt.hbs");
static INIT_HBS_FILE: &str = include_str!("../../templates/init.hbs");
static SALTMD_STR: &str = include_str!("../../SALT.md");

static SALT_HBS_NAME: &str = "salt.hbs";
static INIT_HBS_NAME: &str = "init.hbs";

/// INTRINSICS are commands which are internal to salt bundler
const INTRINSICS: [(&str, &str, &str); 12] = [
    ("init", "i", "Initialize new salt bundle in this directory"),
    ("add", "a", "Adds a salt bundle to your machine"),
    ("doc", "d", "Opens SALT package doc as a HTML page"),
    ("edit", "e", "Open the project/bundle in your editor"),
    ("workspace", "ws", "Load workspace into salt memory"),
    // ("clone", "c", "Clones a salt repo and pins it"),
    // ("update", "u", "Update a salt bundle"),
    ("pin", "p", "pin a folder as a salt bundle"),
    ("open", "o", "open a salt bundle in default file explorer"),
    ("unpin", "unp", "unpin a pinned salt bundle"),
    ("watch", "w", "Runs a watcher for the bundle command"),
    (
        "jump",
        "j",
        "jump to bundle directory with cd $(s j BUNDLE)",
    ),
    ("+", "", "runs the command in pinned directory  +"),
    ("-", "", "run the last salt command"),
];

pub struct Interface {
    cache_path: PathBuf,
    bundles: Vec<MDBundle>,
    /// this is to check if there is bundle conflict
    bundle_map: BundleMap,
    /// TDOO: dont keep config as optional
    config: Option<SaltConfig>,
    /// full_config is untouched config which is directly read from config file
    /// it does not get mutated across whole flow
    full_config: Option<SaltConfig>,
    pub env_vars: HashMap<String, String>,
}

#[cfg(not(target_os = "windows"))]
fn clear_screen() {
    std::process::Command::new("clear").status().unwrap();
}

#[cfg(target_os = "windows")]
fn clear_screen() {
    std::process::Command::new("cls").status().unwrap();
}

fn load_envs(state: &mut Interface) -> Result<()> {
    state
        .env_vars
        .insert("SALT_ARCH".into(), std::env::consts::ARCH.into());
    state
        .env_vars
        .insert("SALT_OS".into(), std::env::consts::OS.into());
    state.env_vars.insert(
        "SALT_CWD".into(),
        std::env::current_dir().unwrap().to_string_lossy().into(),
    );
    Ok(())
}

fn load_config(state: &mut Interface) -> Result<()> {
    if let Some(home) = home::home_dir() {
        state.cache_path = home.join(".salt");
        let salt_config_path = state.cache_path.join(".config");
        match salt_config_path.exists() {
            true => {
                if let Ok(config_str) = std::fs::read_to_string(salt_config_path) {
                    let c = serde_json::from_str::<SaltConfig>(&config_str)
                        .expect("error while reading salt config");
                    state.config = Some(c.clone());
                    state.full_config = Some(c);
                }
            }
            false => {
                let cfg = SaltConfig {
                    editor: Some("vi".into()),
                    pinned_paths: HashMap::new(),
                };
                write_config(&cfg)?;
                state.config = Some(cfg.clone());
                state.full_config = Some(cfg);
            }
        }
    }
    Ok(())
}

fn write_config(c: &SaltConfig) -> Result<()> {
    if let Some(home) = home::home_dir() {
        let cfg_path = home.join(".salt").join(".config");
        let mut cfg_file = std::fs::File::create(cfg_path)?;
        let c = serde_json::to_string_pretty(&c).unwrap();
        cfg_file.write_all(c.as_bytes())?;
    }
    Ok(())
}

fn load_bundles(state: &mut Interface) -> Result<()> {
    load_current_dir_bundle(state)?;
    load_pinned_bundles(state)?;

    Ok(())
}

fn parse_bundle_from_path(path: &PathBuf) -> Result<MDBundle> {
    let md_str = std::fs::read_to_string(path).unwrap().to_string();
    let tokens = markdown::tokenize(&md_str);
    // println!("tok: {tokens:?}");
    // TODO: return error if processed is false
    Ok(crate::app::MDBundle::from(tokens))
}

fn is_intrinsic_bundle(bundle_name: &str) -> bool {
    for ibundle in INTRINSICS {
        if ibundle.0 == bundle_name || ibundle.1 == bundle_name {
            return true;
        }
    }
    false
}

fn load_current_dir_bundle(state: &mut Interface) -> Result<()> {
    let cwd = std::env::current_dir().unwrap();
    let saltmd = cwd.join("SALT.md");
    if !saltmd.exists() {
        println!("not a salt project or bundle");
        return Ok(());
    }
    let md_str = std::fs::read_to_string(saltmd).unwrap().to_string();
    let tokens = markdown::tokenize(&md_str);
    // println!("tokens: {tokens:?}");

    let mut marked_key = String::new();
    for (k, v) in state.config.as_mut().unwrap().pinned_paths.iter() {
        if v == cwd.to_str().unwrap() {
            marked_key = k.clone();
        }
    }
    if !marked_key.is_empty() {
        // TODO: find a way to do debug logging
        // println!("current directory is also marked, loading only once");
        state
            .config
            .as_mut()
            .unwrap()
            .pinned_paths
            .remove_entry(&marked_key);
    }
    let mut curr_bundle = crate::app::MDBundle::from(tokens);
    // println!("curr bundle: {curr_bundle:?}");
    if curr_bundle.options.name.is_empty() {
        println!(
            "current salt {package} doesn't have a name!",
            package = curr_bundle.options.typ
        );
        return Ok(());
    }
    if is_intrinsic_bundle(&curr_bundle.options.name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!(
                "cannot use name {}, as it is an intrinsic command",
                curr_bundle.options.name
            ),
        ));
    }

    if state.bundle_map.contains_key(&curr_bundle.options.name) {
        println!(
            "there is a name conflict for bundle: {} at path: ./SALT.md",
            curr_bundle.options.name
        );
        return Ok(());
    }
    curr_bundle.exec_path = cwd.clone();
    curr_bundle.bundle_path = cwd;
    state.bundles.push(curr_bundle.clone());
    state
        .bundle_map
        .insert(curr_bundle.options.name.clone(), curr_bundle);

    Ok(())
}

fn load_pinned_bundles(state: &mut Interface) -> Result<()> {
    for (_, mpath_str) in state.config.as_ref().unwrap().pinned_paths.iter() {
        let mpath = std::path::PathBuf::from(mpath_str);
        if !mpath.join("SALT.md").exists() {
            continue;
        }

        let mut bundle = parse_bundle_from_path(&mpath.join("SALT.md"))?;
        bundle.is_pinned = true;
        bundle.bundle_path = mpath.clone();
        bundle.exec_path = mpath.clone();
        // TODO: can be extracted as a function .. the same code is used to
        // load added bundle and
        if is_intrinsic_bundle(bundle.options.name.as_str()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!(
                    "cannot use {} as a bundle name, it is an intrinsic command",
                    bundle.options.name
                ),
            ));
        }
        if state.bundle_map.contains_key(&bundle.options.name) {
            println!(
                "there is a name conflict for bundle: {} at path: {}",
                bundle.options.name,
                mpath.to_str().unwrap()
            );
            continue;
        }
        state.bundles.push(bundle.clone());
        state.bundle_map.insert(bundle.options.name.clone(), bundle);
    }
    Ok(())
}

fn open_explorer(path: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut open_cmd = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut open_cmd = std::process::Command::new("start");
    #[cfg(target_os = "linux")]
    let mut open_cmd = std::process::Command::new("xdg-open");
    if !path.is_empty() {
        open_cmd.arg(path);
        return open_cmd.status().map(|_| ());
    }
    Ok(())
}

fn is_cwd_salt_bundle() -> Result<bool> {
    Ok(std::env::current_dir()?.join("SALT.md").exists())
}

impl Interface {
    pub fn init() -> Result<Self> {
        let mut app = Self {
            cache_path: PathBuf::new(),
            bundle_map: HashMap::new(),
            bundles: vec![],
            config: None,
            full_config: None,
            env_vars: HashMap::new(),
        };
        load_envs(&mut app)?;
        load_config(&mut app)?;
        load_bundles(&mut app)?;

        Ok(app)
    }

    pub fn save_to_history(&self, args: &Vec<String>) -> Result<()> {
        if args.len() <= 2 {
            return Ok(());
        }
        if let Some(home) = home::home_dir() {
            let history_file_path = home.join(".salt").join(".history");
            let mut hfile = std::fs::File::create(history_file_path)?;
            hfile.write_all(args.join(" ").as_bytes())?;
        }
        Ok(())
    }

    pub fn run(&mut self, args: &[String]) -> Result<()> {
        self.env_vars.insert("SALT_ARGS".into(), args.join(" "));
        if let Some(command) = args.get(1) {
            match command.as_str() {
                "init" | "i" => self.init_bundle()?,
                "add" | "a" => self.add_bundle(args.get(2))?,
                "watch" | "w" => {
                    let mut a = args.to_owned();
                    a.rotate_left(1);
                    self.start_watcher(&a)?
                }
                "edit" | "e" => self.open_editor(args)?,
                // "update" | "u" => self.update_bundles()?,
                "workspace" | "ws" => self.load_workspace(args)?,
                "open" | "o" => self.open_bundle(args)?,
                "doc" | "d" => self.open_doc(args)?,
                "pin" | "p" => self.pin_bundle()?,
                "unpin" | "unp" => self.unpin_bundle(args)?,
                "jump" | "j" => self.jump_to_bundle(args)?,
                // "clone" | "c" => self.clone_salt_repo(args)?,
                // "install" | "-in" => self.install_deps()?,
                "+" => self.run_wildcard(args)?,
                "-" => self.run_last_cmd()?,
                _ => self.run_bundle_cmd(command.to_owned(), args)?,
            }
        } else {
            self.display_salt_help(&self.bundles);
        }

        Ok(())
    }

    fn load_workspace(&self, _args: &[String]) -> Result<()> {
        Ok(())
    }

    fn open_editor(&self, args: &[String]) -> Result<()> {
        let config = self.config.as_ref().unwrap();
        if config.editor.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "add your editor config to $HOME/.salt/.config",
            ));
        }
        if let Some(bundle_name) = args.get(2) {
            if let Some(bundle) = self.bundle_map.get(bundle_name) {
                let mut editor_cmd = std::process::Command::new(config.editor.as_ref().unwrap());
                editor_cmd.arg(bundle.exec_path.to_str().unwrap());
                editor_cmd.status()?;
            }
        } else if is_cwd_salt_bundle()? {
            let mut editor_cmd = std::process::Command::new(config.editor.as_ref().unwrap());
            editor_cmd.arg(std::env::current_dir().unwrap());
            editor_cmd.status()?;
        }
        Ok(())
    }

    fn open_doc_from_web(&self, link: &str) -> Result<()> {
        if link.contains("github.com") {
            let mut raw_gh_link = link.replace("github.com", "raw.githubusercontent.com");
            if !raw_gh_link.ends_with("SALT.md") && !raw_gh_link.ends_with('/') {
                raw_gh_link.push_str("/main/SALT.md");
            } else if !raw_gh_link.ends_with("SALT.md") {
                raw_gh_link.push_str("SALT.md");
            }
            println!("hitting: {}", raw_gh_link);
            let resp = reqwest::blocking::get(&raw_gh_link)
                .unwrap()
                .text()
                .unwrap();
            let tokens = markdown::tokenize(&resp);
            let bundle = crate::app::MDBundle::from(tokens);
            let doc = crate::app::doc::Doc::from(bundle);
            let doc_html_name = raw_gh_link
                .replace("https://", "")
                .replace("http://", "")
                .replace('/', "")
                .replace("raw.githubusercontent.com", "")
                .trim_start_matches('-')
                .to_owned();
            let doc_path = self.cache_path.join(format!("{}.html", doc_html_name));
            let mut reg = handlebars::Handlebars::new();
            // TODO: handle unwrap
            reg.register_template_string(SALT_HBS_NAME, HBS_FILE)
                .unwrap();
            let html = reg.render(SALT_HBS_NAME, &doc).unwrap();
            std::fs::write(doc_path.clone(), html)?;
            webbrowser::open_browser(webbrowser::Browser::Default, doc_path.to_str().unwrap())?;
        }
        Ok(())
    }

    fn open_salt_doc(&self) -> Result<()> {
        let salt_html_fname = format!("salt-help-{}.html", crate::app::parser::VERSION);
        let tokens = markdown::tokenize(SALTMD_STR);
        let bundle = crate::app::MDBundle::from(tokens);
        let doc = crate::app::doc::Doc::from(bundle.to_owned());
        let doc_path = self.cache_path.join(salt_html_fname);
        // short circuit if doc is already there for the current version of salt
        #[cfg(not(debug_assertions))]
        if doc_path.exists() {
            // TODO: handle unwrap
            webbrowser::open_browser(webbrowser::Browser::Default, doc_path.to_str().unwrap())?;
            return Ok(());
        }
        let mut reg = handlebars::Handlebars::new();
        // TODO: handle unwrap
        reg.register_template_string(SALT_HBS_NAME, HBS_FILE)
            .unwrap();
        let html = reg.render(SALT_HBS_NAME, &doc).unwrap();
        std::fs::write(doc_path.clone(), html)?;
        webbrowser::open_browser(webbrowser::Browser::Default, doc_path.to_str().unwrap())?;
        Ok(())
    }

    fn open_doc(&self, args: &[String]) -> Result<()> {
        if let Some(bundle_name) = args.get(2) {
            if bundle_name.eq("help") {
                return self.open_salt_doc();
            }
            if bundle_name.starts_with("https") || bundle_name.starts_with("http") {
                return self.open_doc_from_web(bundle_name);
            }
            if let Some(bundle) = self.bundle_map.get(bundle_name) {
                let doc = crate::app::doc::Doc::from(bundle.to_owned());
                let doc_path = self.cache_path.join(format!("{}.html", bundle_name));
                let mut reg = handlebars::Handlebars::new();
                // TODO: handle unwrap
                reg.register_template_string(SALT_HBS_NAME, HBS_FILE)
                    .unwrap();
                let html = reg.render(SALT_HBS_NAME, &doc).unwrap();
                std::fs::write(doc_path.clone(), html)?;
                webbrowser::open_browser(webbrowser::Browser::Default, doc_path.to_str().unwrap())?;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no such bundle",
                ));
            }
        } else if std::env::current_dir()?.join("SALT.md").exists() {
            let bundle = parse_bundle_from_path(&std::env::current_dir()?.join("SALT.md"))?;
            let doc = crate::app::doc::Doc::from(bundle.to_owned());
            let doc_path = self
                .cache_path
                .join(format!("{}.html", bundle.options.name));
            let mut reg = handlebars::Handlebars::new();
            // TODO: handle unwrap
            reg.register_template_string(SALT_HBS_NAME, HBS_FILE)
                .unwrap();
            let html = reg.render(SALT_HBS_NAME, &doc).unwrap();
            std::fs::write(doc_path.clone(), html)?;
            webbrowser::open_browser(webbrowser::Browser::Default, doc_path.to_str().unwrap())?;
        }

        Ok(())
    }

    fn open_bundle(&self, args: &[String]) -> Result<()> {
        if let Some(bundle_name) = args.get(2) {
            if let Some(bundle) = self.bundle_map.get(bundle_name) {
                dbg!("has bundle");
                return open_explorer(bundle.exec_path.to_str().unwrap());
            }
        }
        open_explorer("")
    }

    fn unpin_bundle(&self, args: &[String]) -> Result<()> {
        let not_found_err = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "bundle not found",
        ));
        if let Some(bundle_name) = args.get(2) {
            if self.bundle_map.get(bundle_name).is_some() {
                let mut c = self.full_config.clone().unwrap();
                c.pinned_paths.remove(bundle_name);
                dbg!(&c);
                return write_config(&c);
            }
        }

        not_found_err
    }

    fn jump_to_bundle(&self, args: &[String]) -> Result<()> {
        let not_found_err = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "bundle not found",
        ));
        if let Some(bundle_name) = args.get(2) {
            if let Some(bundle) = self.bundle_map.get(bundle_name) {
                println!("{}", bundle.exec_path.to_str().unwrap());
                return Ok(());
            }
            return not_found_err;
        }
        not_found_err
    }

    fn run_last_cmd(&mut self) -> Result<()> {
        if let Some(home) = home::home_dir() {
            let history_file_path = home.join(".salt").join(".history");
            let cmd_str = std::fs::read_to_string(history_file_path)?;
            self.run(
                &cmd_str
                    .split(' ')
                    .map(|s| s.to_owned())
                    .collect::<Vec<String>>(),
            )?;
        }
        Ok(())
    }

    fn run_wildcard(&self, args: &[String]) -> Result<()> {
        if let Some(bundle_name) = args.get(2) {
            // if we have a bundle with the name given as 2nd arg
            // run the command of the bundle
            if let Some(bundle) = self.bundle_map.get(bundle_name) {
                std::env::set_current_dir(&bundle.exec_path)?;
                if let Some(cmd) = args.get(3) {
                    let mut some_cmd = std::process::Command::new(cmd);
                    some_cmd.envs(&self.env_vars);
                    if let Some(cmd_args) = args.get(4..) {
                        some_cmd.args(cmd_args);
                    }
                    return some_cmd.status().map(|_| ());
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "usage: salt + {bundle} {commands...}",
                    ));
                }
            }
            // if the bundle is not found in our bundle map, lets allow user
            // to run this command in the current directory
            // I don't know why someone would use it this way but it is better than
            // wasting user's time
            let mut some_cmd = std::process::Command::new(bundle_name);
            some_cmd.envs(&self.env_vars);
            if let Some(cmd_args) = args.get(3..) {
                some_cmd.args(cmd_args);
            }
            return some_cmd.status().map(|_| ());
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "bundle not found",
        ))
    }

    fn add_bundle(&self, bundle_link: Option<&String>) -> Result<()> {
        // if there is no bundle link provided
        // shout at them!
        if bundle_link.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "add requires additional argument: a git repository link",
            ));
        }
        // check if git is installed
        // if not install the latest git versino
        // check_or_install_git()?;
        // get a proper bundle name from the git repository link
        let bundle_name = self.get_bundle_name(bundle_link.unwrap())?;
        // clone the git repository provided
        self.clone_bundle(bundle_link.unwrap(), &bundle_name)?;
        Ok(())
    }

    fn pin_bundle(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        if !cwd.join("SALT.md").exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not a salt bundle",
            ));
        }
        let bundle = parse_bundle_from_path(&cwd.join("SALT.md"))?;
        let mut c = self.config.clone().unwrap();
        c.pinned_paths
            .insert(bundle.options.name, cwd.to_str().unwrap().into());
        write_config(&c)?;

        println!("pinned :: {}", cwd.to_string_lossy());
        Ok(())
    }

    fn init_bundle(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        let folder_name = cwd.file_name().unwrap().to_str().unwrap();
        let bundle_file_path = cwd.join("SALT.md");
        if bundle_file_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "already a salt bundle!",
            ));
        }
        let mut reg = handlebars::Handlebars::new();
        // TODO: handle unwrap
        reg.register_template_string(INIT_HBS_NAME, INIT_HBS_FILE)
            .unwrap();
        let html = reg.render(INIT_HBS_NAME, &folder_name).unwrap();
        std::fs::write(bundle_file_path, html)?;
        Ok(())
    }

    fn run_bundle_cmd(&self, bundle_name: String, args: &[String]) -> Result<()> {
        if !self.bundle_map.contains_key(&bundle_name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("command: {} not found", bundle_name),
            ));
        }
        if let Some(command) = args.get(2) {
            if let Some(b) = self.bundle_map.get(&bundle_name) {
                if let Some(c) = b.commands.get(command.as_str()) {
                    if b.is_pinned {
                        let mbundle_path = self
                            .config
                            .as_ref()
                            .unwrap()
                            .pinned_paths
                            .get(&b.options.name)
                            .unwrap();
                        std::env::set_current_dir(mbundle_path)?;
                    }
                    let mut cmd = std::process::Command::new(
                        c.command.split(' ').collect::<Vec<&str>>().first().unwrap(),
                    );
                    cmd.envs(&self.env_vars);
                    cmd.args(&c.command.split(' ').collect::<Vec<&str>>()[1..]);
                    cmd.status()?;
                    return Ok(());
                }
            }
        }

        println!("Cannot find command in this bundle, here's something to work with...");
        self.display_bundle_command_help(
            bundle_name.as_str(),
            self.bundle_map.get(&bundle_name).unwrap(),
        );
        Ok(())
    }

    fn get_bundle_name(&self, bundle_link: &String) -> Result<String> {
        let mut check_remote_cmd = std::process::Command::new("git");
        check_remote_cmd.args(["ls-remote", bundle_link]);
        check_remote_cmd.stdout(Stdio::null());
        check_remote_cmd.stderr(Stdio::null());
        let check_remote_cmd_status = check_remote_cmd.status()?.code();
        if let Some(128) = check_remote_cmd_status {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the repository does not exists or has incorrect access rights",
            ));
        };

        let link_cmp: Vec<&str> = bundle_link.split('/').collect();
        let last_cmp = match link_cmp.last() {
            Some(l) => l,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "the repository URL cannot be broken down as a resource",
                ));
            }
        };
        Ok(last_cmp
            .replace(".git", "")
            .replace("salt-", "")
            .replace("salt_", "")
            .replace("salt", ""))
    }

    fn clone_bundle(&self, link: &str, name: &str) -> Result<()> {
        if let Some(home_dir) = home::home_dir() {
            let bundle_dir = std::path::Path::new(home_dir.to_str().unwrap())
                .join(".salt")
                .join(name);
            if bundle_dir.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "a bundle with this name already exists",
                ));
            }

            // TODO: think of a way to check if it is a salt bundle by just
            // checking if this remote has salt.json in it.
            // then only clone it
            let mut clone_cmd = std::process::Command::new("git");
            clone_cmd.args(["clone", link, bundle_dir.to_str().unwrap()]);
            match clone_cmd.status()?.code() {
                Some(code) => {
                    if code == 0 {
                        let bundle_salt_file = bundle_dir.join("salt.json");
                        if !bundle_salt_file.exists() {
                            // TODO: remove_dir fails if directory is not empty
                            // fix it!
                            std::fs::remove_dir(bundle_dir)?;
                            // this is not a valid salt bundle
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "not a valid salt bundle",
                            ));
                        }
                        // TODO: print some message for the developer
                        // TODO: we can update the cache here
                        return Ok(());
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "error while cloning bundle",
                    ));
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "error while cloning bundle",
                    ));
                }
            }
        }
        Ok(())
    }

    fn start_watcher(&self, args: &Vec<String>) -> Result<()> {
        let args_len = args.len();
        let err = Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "usage: watch {BUNDLE} {COMMAND}",
        ));
        if args_len < 3 {
            return err;
        }
        if let Some(bundle) = args.get(1) {
            self.run_watch_bundle_cmd(bundle, args)?;
        }

        err
    }

    fn run_watch_bundle_cmd(&self, bundle_name: &String, args: &[String]) -> Result<()> {
        if let Some(bundle) = self.bundle_map.get(bundle_name) {
            if let Some(command) = bundle.commands.get(args.get(2).unwrap()) {
                futures::executor::block_on(async {
                    if let Err(e) = async_watch(
                        self,
                        command,
                        bundle.exec_path.clone(),
                        bundle.watcher.debounce_secs,
                    )
                    .await
                    {
                        println!("error: {:?}", e)
                    }
                });
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "command invalid or not found",
        ))
    }

    fn display_salt_help(&self, bundles: &Vec<MDBundle>) {
        // clear_screen();
        let mut help: String = r#" [🧂] gives you superpowers
version: 0.1.0
    
Salt commands:
"#
        .into();
        for ibundle in INTRINSICS {
            help.push_str(
                format!(
                    "{} {}      - {}\n",
                    ibundle.0,
                    if !ibundle.1.is_empty() {
                        format!("[{}]", ibundle.1)
                    } else {
                        "".into()
                    },
                    ibundle.2
                )
                .as_str(),
            );
        }
        help.push_str("\nBundle commands:\n");
        for bundle in bundles {
            help.push_str(
                format!(
                    "{} [{}] {}            - {}\n",
                    bundle.options.name,
                    "0.1",
                    if bundle.is_pinned { "📌" } else { "" },
                    bundle.help
                )
                .as_str(),
            );
        }

        println!("{}", help)
    }

    fn display_bundle_command_help(&self, name: &str, bundle: &MDBundle) {
        clear_screen();
        let mut help: String = format!(
            r#"[🧂 {} :: {}]

{}
    
Commands:
"#,
            bundle.options.name.to_uppercase(),
            name,
            bundle.about
        );
        for (cmd_name, cmd_info) in bundle.commands.iter() {
            help.push_str(format!("{}            - {}\n", cmd_name, cmd_info.about).as_str());
        }

        println!("{help}")
    }
}
