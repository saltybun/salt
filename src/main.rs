use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// Notes:
// pass commonly used settings to devs: SALT_ENV , SALT_ARCH , SALT_OS , SALT_ARGS , SALT_PWD
// salt prefixes "watch:" can run the command and watch for changes in SALT_PWD
// salt prefixes "i:{brew}" will install brew in your system provided you are on mac
// .salt will be the cache directory

#[derive(Serialize, Deserialize)]
struct Command {
    pub name: String,
    pub about: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct SaltBundle {
    pub name: String,
    pub version: String,
    pub description: String,
    pub commands: Vec<Command>,
}

fn main() -> std::io::Result<()> {
    let bundles = load_bundles()?;
    clear_screen();
    display_salt(bundles);

    // let matches = clap_app!(salt =>
    //     (version: "🧂 gives you superpowers")
    //     (author: "Author: <github.com/codekidx>")
    //     (@subcommand add =>
    //         (about: "Adds the given BUNDLE of scripts. A BUNDLE must be a Git repository.")
    //         (@arg BUNDLE: +takes_value +required "A git repo where you have your salt bundle")
    //     )
    // )
    // .setting(clap::AppSettings::SubcommandRequiredElseHelp)
    // .subcommand(SubCommand::with_name("run").about("runs a file"))
    // .get_matches();

    // if matches.is_present("add") {
    //     let bundle_link = matches
    //         .subcommand_matches("add")
    //         .unwrap()
    //         .value_of("BUNDLE")
    //         .unwrap();
    //     if !bundle_link.ends_with(".git") {}
    // } else {
    //     let a = matches.value_of("debug").unwrap();
    //     println!("a: {}", a);
    // }

    return Ok(());
}

#[cfg(not(target_os = "windows"))]
fn clear_screen() {
    std::process::Command::new("clear").status().unwrap();
}

#[cfg(target_os = "windows")]
fn clear_screen() {
    std::process::Command::new("cls").status().unwrap();
}

fn load_bundles() -> std::io::Result<Vec<SaltBundle>> {
    let mut bundles: Vec<SaltBundle> = vec![];
    load_current_dir_bundle(&mut bundles)?;
    load_added_bundles(&mut bundles)?;

    Ok(bundles)
}

fn load_current_dir_bundle(bundles: &mut Vec<SaltBundle>) -> std::io::Result<()> {
    let curr_bundle_str = std::fs::read_to_string("./salt.json")?;
    let curr_bundle = serde_json::from_str::<SaltBundle>(&curr_bundle_str).unwrap();
    bundles.push(curr_bundle);
    Ok(())
}

fn load_added_bundles(bundles: &mut Vec<SaltBundle>) -> std::io::Result<()> {
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
                bundles.push(bundle);
            }
        }
    }
    Ok(())
}

fn display_salt(bundles: Vec<SaltBundle>) {
    let mut help: String = r#" ==[🧂]== gives you superpowers
version: 0.1.0

Salt commands:
add          - Adds a salt bundle to your machine

Bundle commands:
"#
    .into();
    for bundle in bundles {
        if bundle.name == "add" {
            panic!("cannot use bundle name as 'add' as it is a salt intrinsic")
        }
        help.push_str(format!("{}       - {}\n", bundle.name, bundle.description).as_str());
    }

    println!("{}", String::from(help))
}
