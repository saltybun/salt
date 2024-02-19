use std::process::Command as ProcessCommand;
use std::{collections::HashMap, path::PathBuf};

use super::{log, Command, ProjectDefinition, ProjectOpts};
use markdown::Block;

impl From<Vec<markdown::Block>> for ProjectDefinition {
    fn from(value: Vec<markdown::Block>) -> Self {
        let mut def = ProjectDefinition {
            version: crate::app::VERSION.to_owned(),
            processed: false,
            docs: indexmap::IndexMap::new(),
            options: ProjectOpts {
                typ: "project".into(),
                name: String::new(),
            },
            commands: HashMap::new(),
            about: String::new(),
            help: String::from("this is a salt package"),
            is_pinned: false,
            project_path: PathBuf::new(),
            exec_path: PathBuf::new(),
        };

        // mode 0 = processing about
        // mode 1 = processing the body of commands
        // mode 2 = processing docs
        // mode 3 = processing options
        // mode 4 = processing package help
        let mut mode = 0;
        let mut doc_section = String::new();
        // println!("Values: {:?}", value);
        for block in value {
            match block {
                Block::OrderedList(li, li_type) => {
                    if !doc_section.is_empty() && mode == 2 {
                        def.docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::OrderedList(li, li_type)));
                        continue;
                    }
                }
                Block::CodeBlock(copt, code) => {
                    if !doc_section.is_empty() && mode == 2 {
                        def.docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::CodeBlock(copt, code)));
                        continue;
                    }
                }
                Block::Paragraph(pspans) => {
                    if !doc_section.is_empty() && mode == 2 {
                        def.docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::Paragraph(pspans.clone())));
                        continue;
                    }
                    if mode == 0 {
                        let mut cmd_info = String::new();
                        for span in pspans.clone() {
                            match span {
                                markdown::Span::Text(t) => cmd_info.push_str(&t),
                                markdown::Span::Code(c) => cmd_info.push_str(&c),
                                _ => continue,
                            }
                        }
                        def.about = cmd_info;
                    }
                    if mode == 4 {
                        let mut cmd_info = String::new();
                        for span in pspans {
                            match span {
                                markdown::Span::Text(t) => {
                                    if !t.starts_with("<!--") {
                                        cmd_info.push_str(&t)
                                    }
                                }
                                markdown::Span::Code(c) => cmd_info.push_str(&c),
                                _ => continue,
                            }
                        }
                        if !cmd_info.is_empty() {
                            def.help = cmd_info;
                        }
                    }
                    continue;
                }
                Block::Header(h, hsize) => {
                    // println!("header : {h:?}");

                    if hsize == 1_usize {
                        continue;
                    }
                    if hsize == 2_usize && h.len() != 1 {
                        return def;
                    }
                    if !doc_section.is_empty() && hsize == 4_usize {
                        def.docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::Header(h.clone(), 4_usize)));
                        continue;
                    }
                    if hsize == 3_usize {
                        mode = 2;
                        match h.first().unwrap() {
                            markdown::Span::Text(t) => {
                                doc_section = t.to_owned();
                                def.docs.entry(t.to_owned()).or_insert(Vec::new());
                            }
                            _ => {
                                return def;
                            }
                        }
                        continue;
                    }
                    if hsize == 2_usize {
                        doc_section = String::new();
                        match h.first().unwrap() {
                            // markdown::Span::Break => todo!(),
                            markdown::Span::Text(t) => match t.to_lowercase().as_str() {
                                "about" => {
                                    mode = 0;
                                    continue;
                                }
                                "help" => {
                                    mode = 4;
                                    continue;
                                }
                                "commands" | "command" => {
                                    mode = 1;
                                    continue;
                                }
                                "options" | "option" => {
                                    mode = 3;
                                    continue;
                                }
                                _ => {
                                    return def;
                                }
                            },
                            _ => {
                                return def;
                            }
                        }
                    }
                }
                Block::Blockquote(bq) => {
                    if !doc_section.is_empty() && mode == 2 {
                        def.docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::Blockquote(bq)));
                        continue;
                    }
                }
                Block::UnorderedList(items) => {
                    if !doc_section.is_empty() && mode == 2 {
                        def.docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::UnorderedList(items)));
                        continue;
                    }
                    if mode == 1 {
                        for item in items {
                            match item {
                                markdown::ListItem::Simple(span_vec) => {
                                    log!("command: {span_vec:?}");
                                    let mut text_info = String::new();
                                    let mut cmd_info = String::new();
                                    for span in span_vec {
                                        match span {
                                            markdown::Span::Text(t) => text_info.push_str(&t),
                                            markdown::Span::Code(c) => cmd_info.push_str(&c),
                                            _ => return def,
                                        };
                                    }
                                    log!("text info: {}", &text_info);
                                    log!("text info: {}", &cmd_info);
                                    // println!("joint: {cmd_info}");
                                    let splitted = text_info
                                        .split('-')
                                        .map(|e| e.trim())
                                        .collect::<Vec<&str>>();
                                    if splitted.len() == 1 {
                                        continue;
                                    }
                                    let cmd = Command {
                                        args: vec![],
                                        about: splitted
                                            .get(2)
                                            .unwrap_or(&cmd_info.as_str())
                                            .to_owned()
                                            .into(),
                                        command: cmd_info,
                                    };
                                    def.commands
                                        .insert(splitted.first().unwrap().to_owned().into(), cmd);
                                }
                                markdown::ListItem::Paragraph(_) => return def,
                            }
                        }
                        continue;
                    }
                    if mode == 3 {
                        for item in items {
                            match item {
                                markdown::ListItem::Simple(span_vec) => {
                                    let mut cmd_info = String::new();
                                    for span in span_vec {
                                        match span {
                                            markdown::Span::Text(t) => cmd_info.push_str(&t),
                                            markdown::Span::Code(c) => cmd_info.push_str(&c),
                                            _ => return def,
                                        };
                                    }

                                    let splitted = cmd_info
                                        .split('-')
                                        .map(|e| e.trim())
                                        .collect::<Vec<&str>>();
                                    if splitted.len() == 1 {
                                        continue;
                                    }

                                    // match the directives
                                    match splitted.first().unwrap().to_owned() {
                                        "type" => {
                                            def.options.typ =
                                                splitted.get(1).unwrap().to_owned().into();
                                        }
                                        "name" => {
                                            // this handles cases when the name of the project has
                                            // hyphen(-) in it
                                            def.options.name =
                                                splitted.get(1..).unwrap().to_owned().join("-");
                                        }
                                        _ => {
                                            continue;
                                        }
                                    }
                                }
                                markdown::ListItem::Paragraph(_) => return def,
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        def.processed = true;
        def
    }
}

pub(crate) fn parse_project_command(cmd: &str) -> Result<ProcessCommand, std::io::Error> {
    if cmd.starts_with("[") {
        let envs;
        let start_from_index;
        (envs, start_from_index) = parse_envs(cmd);
        if start_from_index >= cmd.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "environment block not terminated. missing ']' ?",
            ));
        }
        let mut command = parse_command(cmd, start_from_index)?;
        command.envs(&envs);
        return Ok(command);
    }

    parse_command(cmd, 0)
}

fn parse_command(cmd: &str, start_from_index: usize) -> Result<ProcessCommand, std::io::Error> {
    let our_cmd = &cmd[start_from_index..];
    let our_cmd = our_cmd.trim();
    let splitted_cmd = our_cmd.split(' ').collect::<Vec<&str>>();
    let mut pcmd = std::process::Command::new(splitted_cmd.first().unwrap());
    pcmd.args(&splitted_cmd[1..]);
    Ok(pcmd)
}

fn parse_envs(cmd: &str) -> (HashMap<String, String>, usize) {
    let mut envs = HashMap::new();
    let mut index = 0;
    while index < cmd.len() {
        let mut c = cmd.chars().nth(index).unwrap();
        if c == '[' || c == ']' {
            index += 1;
            continue;
        }

        let mut vars = String::new();
        while c != ']' && index < cmd.len() {
            vars.push_str(&c.to_string());
            index += 1;
            c = cmd.chars().nth(index).unwrap();
        }

        for kv in vars.split(" ") {
            if !kv.contains("=") {
                continue;
            }
            let env_kv: Vec<&str> = kv.split("=").into_iter().collect();
            let key = env_kv[0].to_owned();
            let value = env_kv[1].to_owned();
            envs.insert(key, value);
        }

        return (envs, index + 1);
    }

    (envs, index)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use crate::app::parser::parse_project_command;

    use super::{parse_command, parse_envs};

    #[test]
    fn test_basic_parse_command() {
        let c = "go run main.go";
        let cmd_res = parse_command(c, 0);

        assert_eq!(cmd_res.is_ok(), true);
        assert_eq!(cmd_res.unwrap().get_program(), "go");
    }

    #[test]
    fn test_basic_parse_envs() {
        let c = "[a=1 b=2]";
        let tuple = parse_envs(c);
        assert_eq!(tuple.0.get("a"), Some(&"1".to_owned()));
        assert_eq!(tuple.0.get("b"), Some(&"2".to_owned()));

        assert_eq!(tuple.1, 9);
    }

    #[test]
    fn test_basic_command_with_envs() {
        let c = "[a=1 b=2] go run main.go";
        let parse_res = parse_project_command(c);

        assert_eq!(parse_res.is_ok(), true);

        assert_eq!(parse_res.as_ref().unwrap().get_program(), "go");

        let first_env = parse_res.as_ref().unwrap().get_envs().nth(0).unwrap();
        let second_env = parse_res.as_ref().unwrap().get_envs().nth(1).unwrap();
        assert_eq!(first_env.0, "a");
        assert_eq!(first_env.1.as_ref().unwrap(), &OsStr::new("1"));
        assert_eq!(second_env.0, "b");
        assert_eq!(second_env.1.as_ref().unwrap(), &OsStr::new("2"));
    }
}
