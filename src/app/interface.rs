use std::collections::HashMap;
use std::io::{Result, Write};
use std::path::PathBuf;
use std::process::Stdio;

use crate::app::log;
use crate::app::parser::parse_project_command;

use super::ProjectDefinition;
use super::{ProjectMap, SaltConfig};

static HBS_FILE: &str = include_str!("../../templates/salt.hbs");
static INIT_HBS_FILE: &str = include_str!("../../templates/init.hbs");

static SALT_HBS_NAME: &str = "salt.hbs";
static INIT_HBS_NAME: &str = "init.hbs";

/// INTRINSICS are commands which are internal to salt projectr
const INTRINSICS: [(&str, &str, &str); 10] = [
    ("init", "i", "Initialize new salt project in this directory"),
    ("add", "a", "Adds a salt bundle to your machine"),
    ("doc", "d", "Opens SALT package doc as a HTML page"),
    ("edit", "e", "Open the project/project in your editor"),
    // ("clone", "c", "Clones a salt repo and pins it"),
    // ("update", "u", "Update a salt project"),
    ("pin", "p", "pin a folder as a salt project"),
    ("open", "o", "open a salt project in default file explorer"),
    ("unpin", "unp", "unpin a pinned salt project"),
    (
        "jump",
        "j",
        "jump to project directory with cd $(s j {{PROJECT}})",
    ),
    ("+", "", "runs the command in pinned directory  +"),
    ("-", "", "run the last salt command"),
];

pub struct Interface {
    cache_path: PathBuf,
    projects_path: PathBuf,
    projects: Vec<ProjectDefinition>,
    /// this is to check if there is project conflict
    project_map: ProjectMap,
    // TODO: dont keep config as optional
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

    log!("loaded envs");
    Ok(())
}

fn load_config(state: &mut Interface) -> Result<()> {
    if let Some(home) = home::home_dir() {
        state.cache_path = home.join(".salt");
        state.projects_path = home.join("salt_projects");

        // check if config folder exists inside cache path
        let salt_config_path = state.cache_path.join(".config");
        match salt_config_path.exists() {
            true => {
                if let Ok(config_str) = std::fs::read_to_string(salt_config_path) {
                    let c = serde_json::from_str::<SaltConfig>(&config_str)
                        .expect("error while reading salt config");
                    log!("salt config {:?}", &c);
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

        // if salt workspace folder is missing inside home directory
        // create it
        if !state.projects_path.exists() {
            std::fs::create_dir(&state.projects_path)?;
        }
    }
    Ok(())
}

fn write_config(c: &SaltConfig) -> Result<()> {
    if let Some(home) = home::home_dir() {
        // if cache directory is not there create one
        let cache_dir = home.join(".salt");
        if !cache_dir.exists() {
            log!("cache dir not found | creating one");
            std::fs::create_dir(&cache_dir)?;
        }

        // write json config
        log!("writing initial config");
        let cfg_path = cache_dir.join(".config");
        let mut cfg_file = std::fs::File::create(cfg_path)?;
        let c = serde_json::to_string_pretty(&c).unwrap();
        cfg_file.write_all(c.as_bytes())?;
    }
    Ok(())
}

fn load_projects(state: &mut Interface) -> Result<()> {
    load_current_dir_project(state)?;
    load_pinned_projects(state)?;

    Ok(())
}

fn load_workspaces(_state: &mut Interface) -> Result<()> {
    Ok(())
}

fn parse_project_from_path(path: &PathBuf) -> Result<ProjectDefinition> {
    let md_str = std::fs::read_to_string(path).unwrap().to_string();
    let tokens = markdown::tokenize(&md_str);
    // println!("tok: {tokens:?}");
    // TODO: return error if processed is false
    Ok(crate::app::ProjectDefinition::from(tokens))
}

fn is_project_a_intrinsic(project_name: &str) -> bool {
    for i in INTRINSICS {
        if i.0 == project_name || i.1 == project_name {
            return true;
        }
    }
    false
}

fn load_current_dir_project(state: &mut Interface) -> Result<()> {
    let cwd = std::env::current_dir().unwrap();
    let saltmd = cwd.join("SALT.md");
    if !saltmd.exists() {
        log!("not a salt project or project");
        return Ok(());
    }
    let md_str = std::fs::read_to_string(saltmd).unwrap().to_string();
    let tokens = markdown::tokenize(&md_str);
    log!("markdown tokens: {tokens:?}");

    let mut marked_key = String::new();
    for (k, v) in state.config.as_mut().unwrap().pinned_paths.iter() {
        if v == cwd.to_str().unwrap() {
            marked_key = k.clone();
        }
    }
    if !marked_key.is_empty() {
        log!("current directory is also marked, loading only once");
        state
            .config
            .as_mut()
            .unwrap()
            .pinned_paths
            .remove_entry(&marked_key);
    }
    let mut def = crate::app::ProjectDefinition::from(tokens);
    log!("this project: {def:?}");
    if def.options.name.is_empty() {
        println!(
            "current salt {package} doesn't have a name!",
            package = def.options.typ
        );
        return Ok(());
    }
    if is_project_a_intrinsic(&def.options.name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!(
                "cannot use name {}, as it is an intrinsic command",
                def.options.name
            ),
        ));
    }

    if state.project_map.contains_key(&def.options.name) {
        println!(
            "there is a name conflict for project: {} at path: ./SALT.md",
            def.options.name
        );
        return Ok(());
    }
    def.exec_path = cwd.clone();
    def.project_path = cwd;
    state.projects.push(def.clone());
    state.project_map.insert(def.options.name.clone(), def);

    Ok(())
}

fn load_pinned_projects(state: &mut Interface) -> Result<()> {
    for (_, mpath_str) in state.config.as_ref().unwrap().pinned_paths.iter() {
        let mpath = std::path::PathBuf::from(mpath_str);
        log!("pinned path: {mpath:?}");
        if !mpath.join("SALT.md").exists() {
            log!("pinned path: {mpath:?} does not contain SALT.md");
            continue;
        }

        let mut project = parse_project_from_path(&mpath.join("SALT.md"))?;
        project.is_pinned = true;
        project.project_path = mpath.clone();
        project.exec_path = mpath.clone();
        // TODO: can be extracted as a function .. the same code is used to
        // load added project and
        if is_project_a_intrinsic(project.options.name.as_str()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!(
                    "cannot use {} as a project name, it is an intrinsic command",
                    project.options.name
                ),
            ));
        }
        if state.project_map.contains_key(&project.options.name) {
            println!(
                "there is a name conflict for project: {} at path: {}",
                project.options.name,
                mpath.to_str().unwrap()
            );
            continue;
        }
        state.projects.push(project.clone());
        state
            .project_map
            .insert(project.options.name.clone(), project);
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

fn is_cwd_salt_project() -> Result<bool> {
    Ok(std::env::current_dir()?.join("SALT.md").exists())
}

impl Interface {
    pub fn init() -> Result<Self> {
        let mut app = Self {
            cache_path: PathBuf::new(),
            projects_path: PathBuf::new(),
            project_map: HashMap::new(),
            projects: vec![],
            config: None,
            full_config: None,
            env_vars: HashMap::new(),
        };
        load_envs(&mut app)?;
        load_config(&mut app)?;
        load_projects(&mut app)?;
        load_workspaces(&mut app)?;

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
                "init" | "i" => self.init_project()?,
                "add" | "a" => self.add_project(args.get(2))?,
                "edit" | "e" => self.open_editor(args)?,
                // "update" | "u" => self.update_projects()?,
                "workspace" | "w" => self.load_workspace(args)?,
                "open" | "o" => self.open_project(args)?,
                "doc" | "d" => self.open_doc(args)?,
                "pin" | "p" => self.pin_project()?,
                "unpin" | "unp" => self.unpin_project(args)?,
                "jump" | "j" => self.jump_to_project(args)?,
                // "clone" | "c" => self.clone_salt_repo(args)?,
                // "install" | "-in" => self.install_deps()?,
                "+" => self.run_wildcard(args)?,
                "-" => self.run_last_cmd()?,
                _ => self.run_project_cmd(command.to_owned(), args)?,
            }
        } else {
            self.display_salt_help(&self.projects);
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
        if let Some(project_name) = args.get(2) {
            if let Some(project) = self.project_map.get(project_name) {
                let mut editor_cmd = std::process::Command::new(config.editor.as_ref().unwrap());
                editor_cmd.arg(project.exec_path.to_str().unwrap());
                editor_cmd.status()?;
            }
        } else if is_cwd_salt_project()? {
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
            let project = crate::app::ProjectDefinition::from(tokens);
            let doc = crate::app::doc::Doc::from(project);
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
        webbrowser::open_browser(webbrowser::Browser::Default, "https://saltybun.github.io")?;
        Ok(())
    }

    fn open_doc(&self, args: &[String]) -> Result<()> {
        if let Some(project_name) = args.get(2) {
            if project_name.eq("help") {
                return self.open_salt_doc();
            }
            if project_name.starts_with("https") || project_name.starts_with("http") {
                return self.open_doc_from_web(project_name);
            }
            if let Some(project) = self.project_map.get(project_name) {
                let doc = crate::app::doc::Doc::from(project.to_owned());
                let doc_path = self.cache_path.join(format!("{}.html", project_name));
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
                    "no such project",
                ));
            }
        } else if std::env::current_dir()?.join("SALT.md").exists() {
            let project = parse_project_from_path(&std::env::current_dir()?.join("SALT.md"))?;
            let doc = crate::app::doc::Doc::from(project.to_owned());
            let doc_path = self
                .cache_path
                .join(format!("{}.html", project.options.name));
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

    fn open_project(&self, args: &[String]) -> Result<()> {
        if let Some(project_name) = args.get(2) {
            if let Some(project) = self.project_map.get(project_name) {
                log!("has project");
                return open_explorer(project.exec_path.to_str().unwrap());
            }
        }
        open_explorer("")
    }

    fn unpin_project(&self, args: &[String]) -> Result<()> {
        let not_found_err = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "project not found",
        ));
        if let Some(project_name) = args.get(2) {
            if self.project_map.get(project_name).is_some() {
                let mut c = self.full_config.clone().unwrap();
                c.pinned_paths.remove(project_name);
                log!("config: {:?}", &c);
                return write_config(&c);
            }
        }

        not_found_err
    }

    fn jump_to_project(&self, args: &[String]) -> Result<()> {
        let not_found_err = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "project not found",
        ));
        if let Some(project_name) = args.get(2) {
            if let Some(project) = self.project_map.get(project_name) {
                println!("{}", project.exec_path.to_str().unwrap());
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
        if let Some(project_name) = args.get(2) {
            // if we have a project with the name given as 2nd arg
            // run the command of the project
            if let Some(project) = self.project_map.get(project_name) {
                std::env::set_current_dir(&project.exec_path)?;
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
                        "usage: salt + {project} {commands...}",
                    ));
                }
            }
            // if the project is not found in our project map, lets allow user
            // to run this command in the current directory
            // I don't know why someone would use it this way but it is better than
            // wasting user's time
            let mut some_cmd = std::process::Command::new(project_name);
            some_cmd.envs(&self.env_vars);
            if let Some(cmd_args) = args.get(3..) {
                some_cmd.args(cmd_args);
            }
            return some_cmd.status().map(|_| ());
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "project not found",
        ))
    }

    fn add_project(&self, project_link: Option<&String>) -> Result<()> {
        // if there is no project link provided
        // shout at them!
        if project_link.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "add requires additional argument: a git repository link",
            ));
        }
        // check if git is installed
        // if not install the latest git version
        // check_or_install_git()?;
        // get a proper project name from the git repository link
        let project_name = self.get_project_name(project_link.unwrap())?;
        // clone the git repository provided
        self.clone_project(project_link.unwrap(), &project_name)?;
        Ok(())
    }

    fn pin_project(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        if !cwd.join("SALT.md").exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not a salt project",
            ));
        }
        let project = parse_project_from_path(&cwd.join("SALT.md"))?;
        let mut c = self.config.clone().unwrap();
        c.pinned_paths
            .insert(project.options.name, cwd.to_str().unwrap().into());
        write_config(&c)?;

        println!("pinned :: {}", cwd.to_string_lossy());
        Ok(())
    }

    fn init_project(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        let folder_name = cwd.file_name().unwrap().to_str().unwrap();
        let project_file_path = cwd.join("SALT.md");

        // check if this folder is already a salt project
        if project_file_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "already a salt project!",
            ));
        }

        // initialize init.hbs template
        let mut reg = handlebars::Handlebars::new();
        // TODO: handle unwrap
        reg.register_template_string(INIT_HBS_NAME, INIT_HBS_FILE)
            .unwrap();
        let html = reg.render(INIT_HBS_NAME, &folder_name).unwrap();
        std::fs::write(project_file_path, html)?;
        Ok(())
    }

    fn run_project_cmd(&self, project_name: String, args: &[String]) -> Result<()> {
        if !self.project_map.contains_key(&project_name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("command: {} not found", project_name),
            ));
        }
        if let Some(command) = args.get(2) {
            if let Some(b) = self.project_map.get(&project_name) {
                if let Some(c) = b.commands.get(command.as_str()) {
                    if b.is_pinned {
                        let mproject_path = self
                            .config
                            .as_ref()
                            .unwrap()
                            .pinned_paths
                            .get(&b.options.name)
                            .unwrap();

                        log!("setting current working dir: {mproject_path}");
                        std::env::set_current_dir(mproject_path)?;
                    } else {
                        let pwd = std::env::current_dir()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_owned();

                        log!("setting current working dir: {pwd}");
                        std::env::set_current_dir(pwd)?;
                    }

                    log!("running command: {}", &c.command);
                    let mut cmd = parse_project_command(&c.command)?;
                    cmd.envs(&self.env_vars);
                    cmd.status()?;
                    return Ok(());
                }
            }
        }

        println!("Cannot find command in this project, here's something to work with...");
        self.display_project_command_help(
            project_name.as_str(),
            self.project_map.get(&project_name).unwrap(),
        );
        Ok(())
    }

    fn get_project_name(&self, project_link: &String) -> Result<String> {
        let mut check_remote_cmd = std::process::Command::new("git");
        check_remote_cmd.args(["ls-remote", project_link]);
        check_remote_cmd.stdout(Stdio::null());
        check_remote_cmd.stderr(Stdio::null());
        let check_remote_cmd_status = check_remote_cmd.status()?.code();
        if let Some(128) = check_remote_cmd_status {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the repository does not exists or has incorrect access rights",
            ));
        };

        let link_cmp: Vec<&str> = project_link.split('/').collect();
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

    fn clone_project(&self, link: &str, name: &str) -> Result<()> {
        if let Some(home_dir) = home::home_dir() {
            let project_dir = std::path::Path::new(home_dir.to_str().unwrap())
                .join(".salt")
                .join(name);
            if project_dir.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "a project with this name already exists",
                ));
            }

            // TODO: think of a way to check if it is a salt project by just
            // checking if this remote has salt.json in it.
            // then only clone it
            let mut clone_cmd = std::process::Command::new("git");
            clone_cmd.args(["clone", link, project_dir.to_str().unwrap()]);
            match clone_cmd.status()?.code() {
                Some(code) => {
                    if code == 0 {
                        let project_salt_file = project_dir.join("salt.json");
                        if !project_salt_file.exists() {
                            // TODO: remove_dir fails if directory is not empty
                            // fix it!
                            std::fs::remove_dir(project_dir)?;
                            // this is not a valid salt project
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "not a valid salt project",
                            ));
                        }
                        // TODO: print some message for the developer
                        // TODO: we can update the cache here
                        return Ok(());
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "error while cloning project",
                    ));
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "error while cloning project",
                    ));
                }
            }
        }
        Ok(())
    }

    fn display_salt_help(&self, projects: &Vec<ProjectDefinition>) {
        // clear_screen();
        let mut help = format!(
            r#" [🧂] gives you superpowers
version: {}
    
Salt commands:
"#,
            crate::app::VERSION
        );
        for iproject in INTRINSICS {
            help.push_str(
                format!(
                    "{} {}      - {}\n",
                    iproject.0,
                    if !iproject.1.is_empty() {
                        format!("[{}]", iproject.1)
                    } else {
                        "".into()
                    },
                    iproject.2
                )
                .as_str(),
            );
        }
        help.push_str("\nproject commands:\n");
        for project in projects {
            help.push_str(
                format!(
                    "{} [{}] {}            - {}\n",
                    project.options.name,
                    "0.1",
                    if project.is_pinned { "📌" } else { "" },
                    project.help
                )
                .as_str(),
            );
        }

        println!("{}", help)
    }

    fn display_project_command_help(&self, name: &str, project: &ProjectDefinition) {
        clear_screen();
        let mut help: String = format!(
            r#"[🧂 {} :: {}]

{}
    
Commands:
"#,
            project.options.name.to_uppercase(),
            name,
            project.about
        );
        for (cmd_name, cmd_info) in project.commands.iter() {
            help.push_str(format!("{}            - {}\n", cmd_name, cmd_info.about).as_str());
        }

        println!("{help}")
    }
}
