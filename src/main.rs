use std::{collections::HashMap, io::Write, process::Stdio};

use serde::{Deserialize, Serialize};

static INTRINSICS: [(&str, &str); 6] = [
    ("init", "Initialize new salt bundle in this directory"),
    ("add", "Adds a salt bundle to your machine"),
    ("update", "Update a salt bundle"),
    ("mark", "marks a folder as a salt bundle"),
    ("watch", "Runs a watcher for the bundle command"),
    ("i", "Install a dependency"),
];

// Notes:
// pass commonly used settings to devs: SALT_ENV , SALT_ARCH , SALT_OS , SALT_ARGS , SALT_PWD
// salt prefixes "watch:" can run the command and watch for changes in SALT_PWD
// salt prefixes "i:{brew}" will install brew in your system provided you are on mac
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
    pub version: String,
    pub description: String,
    pub commands: HashMap<String, Command>,
}

struct InterfaceState {
    bundles: Vec<SaltBundle>,
    // this is to check if there is bundle conflict
    bundle_map: HashMap<String, HashMap<String, Command>>,
}

impl InterfaceState {
    pub fn new() -> Self {
        Self {
            bundle_map: HashMap::new(),
            bundles: vec![],
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = InterfaceState::new();
    clear_screen();
    load_bundles(&mut state)?;

    let args: Vec<String> = std::env::args().collect();
    if let Some(bundle) = args.get(1) {
        match bundle.as_str() {
            "init" => init_bundle()?,
            "add" => add_bundle(args.get(2))?,
            "watch" => start_watcher()?,
            // "i" => install_program()?,
            _ => run_bundle_cmd(&state.bundle_map, bundle.to_owned(), args)?,
        }
    } else {
        display_salt_help(&state.bundles);
    }

    return Ok(());
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
    };
    let new_bundle_string = serde_json::to_string_pretty::<SaltBundle>(&new_bundle).unwrap();
    let mut file = std::fs::File::create(bundle_file_path)?;
    file.write_all(new_bundle_string.as_bytes())?;

    Ok(())
}

fn run_bundle_cmd(
    bundle_map: &HashMap<String, HashMap<String, Command>>,
    bundle_name: String,
    args: Vec<String>,
) -> std::io::Result<()> {
    if !bundle_map.contains_key(&bundle_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("command: {} not found", bundle_name),
        ));
    }
    if let Some(command) = args.get(2) {
        if let Some(b) = bundle_map.get(&bundle_name) {
            if let Some(c) = b.get(command.as_str()) {
                let mut cmd = std::process::Command::new(c.command.clone());
                cmd.args(c.args.as_slice());
                cmd.status()?;
            }
        }
        println!("Cannot find command in this bundle");
    } else {
        // TODO: display bundle help
        // display_bundle_help(&state.bundles);
    }
    Ok(())
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

fn start_watcher() -> std::io::Result<()> {
    Ok(())
}

fn _install_program() -> std::io::Result<()> {
    Ok(())
}

fn load_bundles(state: &mut InterfaceState) -> std::io::Result<()> {
    load_current_dir_bundle(state)?;
    load_added_bundles(state)?;

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
    if let Ok(curr_bundle_str) = std::fs::read_to_string("./salt.json") {
        let curr_bundle = match serde_json::from_str::<SaltBundle>(&curr_bundle_str) {
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

        state.bundles.push(curr_bundle.clone());
        state
            .bundle_map
            .insert(curr_bundle.name, curr_bundle.commands);
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
            let it = path.unwrap().path();
            if std::fs::metadata(it.to_owned()).unwrap().is_dir() {
                let bundle_json =
                    std::path::Path::new(it.to_owned().to_str().unwrap()).join("salt.json");
                if !bundle_json.exists() {
                    continue;
                }
                let bundle_str = std::fs::read_to_string(bundle_json.to_str().unwrap())?;
                let bundle = serde_json::from_str::<SaltBundle>(&bundle_str).unwrap();
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
                state.bundles.push(bundle.clone());
                state.bundle_map.insert(bundle.name, bundle.commands);
            }
        }
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
                "{} [{}]            - {}\n",
                bundle.name, bundle.version, bundle.description
            )
            .as_str(),
        );
    }

    println!("{}", String::from(help))
}
