use std::collections::HashMap;
use std::io::{Result, Write};
use std::path::PathBuf;
use std::process::Stdio;

use crate::watcher::async_watch;

use super::{BundleMap, SaltBundle, SaltConfig};

/// INTRINSICS are commands which are internal to salt bundler
const INTRINSICS: [(&str, &str, &str); 8] = [
    ("init", "i", "Initialize new salt bundle in this directory"),
    ("add", "a", "Adds a salt bundle to your machine"),
    ("update", "u", "Update a salt bundle"),
    ("pin", "p", "pin a folder as a salt bundle"),
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
    bundles: Vec<SaltBundle>,
    // this is to check if there is bundle conflict
    bundle_map: BundleMap,
    // TDOO: dont keep config as optional
    config: Option<SaltConfig>,
}

#[cfg(not(target_os = "windows"))]
fn clear_screen() {
    std::process::Command::new("clear").status().unwrap();
}

#[cfg(target_os = "windows")]
fn clear_screen() {
    std::process::Command::new("cls").status().unwrap();
}

fn load_config(state: &mut Interface) -> Result<()> {
    if let Some(home) = home::home_dir() {
        let salt_config_path = home.join(".salt").join(".config");
        if salt_config_path.exists() {
            if let Ok(config_str) = std::fs::read_to_string(salt_config_path) {
                let c = serde_json::from_str::<SaltConfig>(&config_str)
                    .expect("error while reading salt config");
                state.config = Some(c);
            }
        } else {
            let cfg = SaltConfig {
                pinned_paths: HashMap::new(),
            };
            write_config(&cfg)?;
            state.config = Some(cfg);
        }
    }
    Ok(())
}

fn write_config(c: &SaltConfig) -> Result<()> {
    if let Some(home) = home::home_dir() {
        let cfg_path = home.join(".salt").join(".config");
        let mut cfg_file = std::fs::File::create(cfg_path)?;
        let c = serde_json::to_string(&c).unwrap();
        cfg_file.write_all(c.as_bytes())?;
    }
    Ok(())
}

fn load_bundles(state: &mut Interface) -> Result<()> {
    load_current_dir_bundle(state)?;
    load_added_bundles(state)?;
    load_pinned_bundles(state)?;

    Ok(())
}

fn is_intrinsic_bundle(bundle_name: &str) -> bool {
    for ibundle in INTRINSICS {
        if ibundle.0 == bundle_name {
            return true;
        }
    }
    false
}

fn load_current_dir_bundle(state: &mut Interface) -> Result<()> {
    let cwd = std::env::current_dir().unwrap();
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
    if let Ok(curr_bundle_str) = std::fs::read_to_string("./salt.json") {
        let mut curr_bundle = match serde_json::from_str::<SaltBundle>(&curr_bundle_str) {
            Ok(b) => b,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
        };
        if is_intrinsic_bundle(&curr_bundle.name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("cannot use {}", curr_bundle.name),
            ));
        }

        if state.bundle_map.contains_key(&curr_bundle.name) {
            println!(
                "there is a name conflict for bundle: {} at path: ./salt.json",
                curr_bundle.name
            );
            return Ok(());
        }
        curr_bundle.exec_path = cwd.clone();
        curr_bundle.bundle_path = cwd;
        state.bundles.push(curr_bundle.clone());
        state
            .bundle_map
            .insert(curr_bundle.name.clone(), curr_bundle);
    }

    Ok(())
}

fn load_added_bundles(state: &mut Interface) -> Result<()> {
    if let Some(home_dir) = home::home_dir() {
        let salt_cache_dir = std::path::Path::new(home_dir.to_str().unwrap()).join(".salt");
        if !salt_cache_dir.exists() {
            std::fs::create_dir(&salt_cache_dir)?;
        }
        let paths = std::fs::read_dir(&salt_cache_dir).unwrap();
        for path in paths {
            let path_dir = path.unwrap().path();
            if std::fs::metadata(&path_dir).unwrap().is_dir() {
                let bundle_json =
                    std::path::Path::new(path_dir.to_owned().to_str().unwrap()).join("salt.json");
                if !bundle_json.exists() {
                    continue;
                }
                let bundle_str = std::fs::read_to_string(bundle_json.to_str().unwrap())?;
                let mut bundle = serde_json::from_str::<SaltBundle>(&bundle_str).unwrap();
                if is_intrinsic_bundle(bundle.name.as_str()) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        format!(
                            "cannot use {} as a bundle name, it is an intrinsic command",
                            bundle.name
                        ),
                    ));
                }
                if state.bundle_map.contains_key(&bundle.name) {
                    println!(
                        "there is a name conflict for bundle: {} at path: {}",
                        bundle.name,
                        bundle_json.to_str().unwrap()
                    );
                    continue;
                }

                bundle.bundle_path = path_dir;
                bundle.exec_path = std::env::current_dir().unwrap();
                state.bundles.push(bundle.clone());
                state.bundle_map.insert(bundle.name.clone(), bundle);
            }
        }
    }
    Ok(())
}

fn load_pinned_bundles(state: &mut Interface) -> Result<()> {
    for (_, mpath_str) in state.config.as_ref().unwrap().pinned_paths.iter() {
        let mpath = std::path::PathBuf::from(mpath_str);
        if !mpath.join("salt.json").exists() {
            continue;
        }
        let bundle_str = std::fs::read_to_string(mpath.join("salt.json"))
            .expect("cannot read salt bundle file in marked loader");
        let mut bundle = serde_json::from_str::<SaltBundle>(&bundle_str)
            .expect("unable to parse salt budle in marked loader");
        bundle.is_pinned = true;
        bundle.bundle_path = mpath.clone();
        bundle.exec_path = mpath.clone();
        // TODO: can be extracted as a function .. the same code is used to
        // load added bundle and
        if is_intrinsic_bundle(bundle.name.as_str()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!(
                    "cannot use {} as a bundle name, it is an intrinsic command",
                    bundle.name
                ),
            ));
        }
        if state.bundle_map.contains_key(&bundle.name) {
            println!(
                "there is a name conflict for bundle: {} at path: {}",
                bundle.name,
                mpath.to_str().unwrap()
            );
            continue;
        }
        state.bundles.push(bundle.clone());
        state.bundle_map.insert(bundle.name.clone(), bundle);
    }
    Ok(())
}

impl Interface {
    pub fn init() -> Result<Self> {
        let mut app = Self {
            bundle_map: HashMap::new(),
            bundles: vec![],
            config: None,
        };
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

    pub fn run(&self, args: &[String]) -> Result<()> {
        if let Some(bundle) = args.get(1) {
            match bundle.as_str() {
                "init" | "i" => self.init_bundle()?,
                "add" | "a" => self.add_bundle(args.get(2))?,
                "watch" | "w" => {
                    let mut a = args.to_owned();
                    a.rotate_left(1);
                    self.start_watcher(&a)?
                }
                "update" | "u" => self.update_bundles()?,
                "pin" | "p" => self.pin_bundle()?,
                "jump" | "j" => self.jump_to_bundle(args)?,
                // "install" | "-in" => self.install_deps()?,
                "+" => self.run_wildcard(args)?,
                "-" => self.run_last_cmd()?,
                _ => self.run_bundle_cmd(bundle.to_owned(), args)?,
            }
        } else {
            self.display_salt_help(&self.bundles);
        }

        Ok(())
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

    fn run_last_cmd(&self) -> Result<()> {
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

    fn update_bundles(&self) -> Result<()> {
        if let Some(home) = home::home_dir() {
            let salt_cache_dir = std::path::Path::new(home.to_str().unwrap()).join(".salt");
            if !salt_cache_dir.exists() {
                std::fs::create_dir(&salt_cache_dir)?;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no added salt bundles",
                ));
            }
            let paths = std::fs::read_dir(&salt_cache_dir).unwrap();
            for path in paths {
                let path_dir = path.unwrap();
                if path_dir.path().is_dir() {
                    std::env::set_current_dir(path_dir.path())?;
                    let mut pull_cmd = std::process::Command::new("git");
                    pull_cmd.args(["pull", "origin", "main"]);
                    match pull_cmd.status()?.code() {
                        Some(0) => {
                            println!(
                                "{} :: updated",
                                path_dir.path().file_name().unwrap().to_string_lossy()
                            );
                        }
                        _ => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Interrupted,
                                "failed to run pull command",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn pin_bundle(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        if !cwd.join("salt.json").exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not a salt bundle",
            ));
        }
        let bundle = serde_json::from_str::<SaltBundle>(
            &std::fs::read_to_string(cwd.join("salt.json")).unwrap(),
        )
        .unwrap();
        let mut c = self.config.clone().unwrap();
        c.pinned_paths
            .insert(bundle.name, cwd.to_str().unwrap().into());
        write_config(&c)?;

        println!("pinned :: {}", cwd.to_string_lossy());
        Ok(())
    }

    fn init_bundle(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        let bundle_file_path = cwd.join("salt.json");
        if bundle_file_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "already a salt bundle!",
            ));
        }

        let mut sample_commands = HashMap::new();
        sample_commands.insert(
            String::from("fp"),
            crate::app::Command {
                about: "prints all file paths in this directory".into(),
                command: "find".into(),
                args: vec![".".into()],
            },
        );
        let new_bundle = SaltBundle {
            name: cwd.file_name().unwrap().to_str().unwrap().to_owned(),
            requires: Some(vec![]),
            version: "0.1.0".into(),
            description: "this is a fresh salt bundle".into(),
            commands: sample_commands,
            is_pinned: false,
            bundle_path: PathBuf::new(),
            exec_path: PathBuf::new(),
            watcher: super::Watcher { debounce_secs: 1 },
        };
        let new_bundle_string = serde_json::to_string_pretty::<SaltBundle>(&new_bundle).unwrap();
        let mut file = std::fs::File::create(bundle_file_path)?;
        file.write_all(new_bundle_string.as_bytes())?;

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
                            .get(&b.name)
                            .unwrap();
                        std::env::set_current_dir(mbundle_path)?;
                    }
                    let mut cmd = std::process::Command::new(c.command.clone());
                    cmd.args(c.args.as_slice());
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

    fn display_salt_help(&self, bundles: &Vec<SaltBundle>) {
        clear_screen();
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
                    bundle.name,
                    bundle.version,
                    if bundle.is_pinned { "📌" } else { "" },
                    bundle.description
                )
                .as_str(),
            );
        }

        println!("{}", help)
    }

    fn display_bundle_command_help(&self, name: &str, bundle: &SaltBundle) {
        clear_screen();
        let mut help: String = format!(
            r#"[🧂 Bundle :: {}]
    
Commands:
"#,
            name
        );
        for (cmd_name, cmd_info) in bundle.commands.iter() {
            help.push_str(format!("{}            - {}\n", cmd_name, cmd_info.about).as_str());
        }

        println!("{}", help)
    }
}
