use std::{collections::HashMap, io::Write, path::PathBuf, process::Stdio, time::Duration};

use notify::Watcher;
use notify_debouncer_full::new_debouncer;
use serde::{Deserialize, Serialize};

const INTRINSICS: [(&str, &str); 6] = [
    ("init", "Initialize new salt bundle in this directory"),
    ("add", "Adds a salt bundle to your machine"),
    ("update", "Update a salt bundle"),
    ("pin", "pin a folder as a salt bundle"),
    ("watch", "Runs a watcher for the bundle command"),
    ("i", "Install a dependency"),
];

type BundleMap = HashMap<String, SaltBundle>;

// Notes:
// pass commonly used settings to devs: SALT_ENV , SALT_ARCH , SALT_OS , SALT_ARGS , SALT_PWD
// .salt will be the cache directory

#[derive(Serialize, Deserialize, Clone)]
struct Command {
    pub about: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SaltBundle {
    pub name: String,
    pub requires: Option<Vec<String>>,
    // TODO: we can exclude some paths from getting notification event
    // for restart
    // pub exclude: Vec<String>,
    pub version: String,
    pub description: String,
    pub commands: HashMap<String, Command>,

    #[serde(skip_serializing, skip_deserializing)]
    pub is_pinned: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub path: PathBuf,
}

struct InterfaceState {
    bundles: Vec<SaltBundle>,
    // this is to check if there is bundle conflict
    bundle_map: BundleMap,
    // TDOO: dont keep config as optional
    config: Option<SaltConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SaltConfig {
    pub pinned_paths: HashMap<String, String>,
}

impl InterfaceState {
    pub fn new() -> Self {
        Self {
            bundle_map: HashMap::new(),
            bundles: vec![],
            config: None,
        }
    }
}

fn async_debouncer() -> notify::Result<(
    notify_debouncer_full::Debouncer<notify::FsEventWatcher, notify_debouncer_full::FileIdMap>,
    std::sync::mpsc::Receiver<
        Result<Vec<notify_debouncer_full::DebouncedEvent>, Vec<notify::Error>>,
    >,
)> {
    let (tx, rx) = std::sync::mpsc::channel();
    // TODO: this debouncer duration can be taken from bundle config as well
    // in key watcher:{ duration: Number(1) }
    let debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;
    Ok((debouncer, rx))
}

async fn async_watch<P: AsRef<std::path::Path>>(command: &Command, path: P) -> notify::Result<()> {
    println!("Starting to watch: {}", path.as_ref().to_string_lossy());
    let (mut debouncer, rx) = async_debouncer()?;
    let mut child;
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer
        .watcher()
        .watch(path.as_ref(), notify::RecursiveMode::Recursive)?;
    // watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive)?;

    let mut cmd_proc = std::process::Command::new(&command.command);
    cmd_proc.args(&command.args);
    child = cmd_proc.spawn().unwrap();
    println!("starting first: {}", child.id());
    while let Ok(res) = rx.recv() {
        match res {
            Ok(event) => {
                println!("changed: {:?}", event);
                println!("killing: {}", child.id());
                child.kill()?;
                let mut cmd_proc = std::process::Command::new(&command.command);
                cmd_proc.args(&command.args);
                child = cmd_proc.spawn().unwrap();
                println!("started: {}", child.id());
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut state = InterfaceState::new();
    clear_screen();
    load_config(&mut state)?;
    load_bundles(&mut state)?;

    let mut args: Vec<String> = std::env::args().collect();
    if let Some(bundle) = args.get(1) {
        match bundle.as_str() {
            "init" => init_bundle()?,
            "add" => add_bundle(args.get(2))?,
            "watch" => {
                args.rotate_left(1);
                start_watcher(&state, args)?
            }
            "pin" => pin_bundle(state.config)?,
            // "i" => install_program()?,
            _ => run_bundle_cmd(&state, bundle.to_owned(), args)?,
        }
    } else {
        display_salt_help(&state.bundles);
    }

    return Ok(());
}

fn load_config(state: &mut InterfaceState) -> std::io::Result<()> {
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

fn pin_bundle(cfg: Option<SaltConfig>) -> std::io::Result<()> {
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
    let mut c = cfg.unwrap();
    c.pinned_paths
        .insert(bundle.name, cwd.to_str().unwrap().into());

    write_config(&c)?;

    println!("pinned :: {}", cwd.to_string_lossy());
    Ok(())
}

fn write_config(c: &SaltConfig) -> std::io::Result<()> {
    if let Some(home) = home::home_dir() {
        let cfg_path = home.join(".salt").join(".config");
        let mut cfg_file = std::fs::File::create(cfg_path)?;
        let c = serde_json::to_string(&c).unwrap();
        cfg_file.write_all(c.as_bytes())?;
    }
    Ok(())
}

fn init_bundle() -> std::io::Result<()> {
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
        Command {
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
        path: PathBuf::new(),
    };
    let new_bundle_string = serde_json::to_string_pretty::<SaltBundle>(&new_bundle).unwrap();
    let mut file = std::fs::File::create(bundle_file_path)?;
    file.write_all(new_bundle_string.as_bytes())?;

    Ok(())
}

fn run_bundle_cmd(
    state: &InterfaceState,
    bundle_name: String,
    args: Vec<String>,
) -> std::io::Result<()> {
    if !state.bundle_map.contains_key(&bundle_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("command: {} not found", bundle_name),
        ));
    }
    if let Some(command) = args.get(2) {
        if let Some(b) = state.bundle_map.get(&bundle_name) {
            if let Some(c) = b.commands.get(command.as_str()) {
                if b.is_pinned {
                    let mbundle_path = state
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
    display_bundle_command_help(
        bundle_name.as_str(),
        state.bundle_map.get(&bundle_name).unwrap(),
    );
    Ok(())
}

fn run_watch_bundle_cmd(
    state: &InterfaceState,
    bundle_name: &String,
    args: &Vec<String>,
) -> std::io::Result<()> {
    if let Some(bundle) = state.bundle_map.get(bundle_name) {
        if let Some(command) = bundle.commands.get(args.get(2).unwrap()) {
            futures::executor::block_on(async {
                if let Err(e) =
                    async_watch(command, std::path::PathBuf::from(bundle.path.clone())).await
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

#[cfg(not(target_os = "windows"))]
fn clear_screen() {
    std::process::Command::new("clear").status().unwrap();
}

#[cfg(target_os = "windows")]
fn clear_screen() {
    std::process::Command::new("cls").status().unwrap();
}

fn add_bundle(bundle_link: Option<&String>) -> std::io::Result<()> {
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
    check_or_install_git()?;
    // get a proper bundle name from the git repository link
    let bundle_name = get_bundle_name(bundle_link.unwrap())?;
    // clone the git repository provided
    clone_bundle(&bundle_link.unwrap(), &bundle_name)?;
    Ok(())
}

fn get_bundle_name(bundle_link: &String) -> std::io::Result<String> {
    let mut check_remote_cmd = std::process::Command::new("git");
    check_remote_cmd.args(["ls-remote", bundle_link]);
    check_remote_cmd.stdout(Stdio::null());
    check_remote_cmd.stderr(Stdio::null());
    let check_remote_cmd_status = check_remote_cmd.status()?.code();
    match check_remote_cmd_status {
        Some(128) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the repository does not exists or has incorrect access rights",
            ));
        }
        _ => {}
    };

    let link_cmp: Vec<&str> = bundle_link.split("/").collect();
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
        .replace("salt_", ""))
}

fn clone_bundle(link: &str, name: &str) -> std::io::Result<()> {
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

fn check_or_install_git() -> std::io::Result<()> {
    Ok(())
}

fn start_watcher(state: &InterfaceState, args: Vec<String>) -> std::io::Result<()> {
    let args_len = args.len();
    let err = Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "usage: watch {BUNDLE} {COMMAND}",
    ));
    if args_len < 3 {
        return err;
    }
    if let Some(bundle) = args.get(1) {
        run_watch_bundle_cmd(state, bundle, &args)?;
    }

    return err;
}

fn _install_program() -> std::io::Result<()> {
    Ok(())
}

fn load_bundles(state: &mut InterfaceState) -> std::io::Result<()> {
    load_current_dir_bundle(state)?;
    load_added_bundles(state)?;
    load_marked_bundles(state)?;

    Ok(())
}

fn is_intrinsic_bundle(bundle_name: &str) -> bool {
    for ibundle in INTRINSICS {
        if ibundle.0 == bundle_name {
            return true;
        }
    }
    return false;
}

fn load_current_dir_bundle(state: &mut InterfaceState) -> std::io::Result<()> {
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
                "there is a name conflict for bundle: {} at path: {}",
                curr_bundle.name, "./salt.json"
            );
            return Ok(());
        }
        curr_bundle.path = cwd;
        state.bundles.push(curr_bundle.clone());
        state
            .bundle_map
            .insert(curr_bundle.name.clone(), curr_bundle);
    }

    Ok(())
}

fn load_added_bundles(state: &mut InterfaceState) -> std::io::Result<()> {
    if let Some(home_dir) = home::home_dir() {
        let salt_cache_dir = std::path::Path::new(home_dir.to_str().unwrap()).join(".salt");
        if !salt_cache_dir.exists() {
            std::fs::create_dir(salt_cache_dir.to_owned())?;
        }
        let paths = std::fs::read_dir(salt_cache_dir.to_owned()).unwrap();
        for path in paths {
            let path_dir = path.unwrap().path();
            if std::fs::metadata(path_dir.to_owned()).unwrap().is_dir() {
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

                bundle.path = path_dir;
                state.bundles.push(bundle.clone());
                state.bundle_map.insert(bundle.name.clone(), bundle);
            }
        }
    }
    Ok(())
}

fn load_marked_bundles(state: &mut InterfaceState) -> std::io::Result<()> {
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
        bundle.path = mpath.clone();
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

fn display_salt_help(bundles: &Vec<SaltBundle>) {
    let mut help: String = r#" [🧂] gives you superpowers
version: 0.1.0

Salt commands:
"#
    .into();
    for ibundle in INTRINSICS {
        help.push_str(format!("{}       - {}\n", ibundle.0, ibundle.1).as_str());
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

fn display_bundle_command_help(name: &str, bundle: &SaltBundle) {
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
