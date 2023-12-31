use std::{collections::HashMap, path::PathBuf};

use super::{Command, MDBundle, MDOptions};
use crate::app::Watcher;
use markdown::Block;

pub static VERSION: &str = "0.1.0";

impl From<Vec<markdown::Block>> for MDBundle {
    fn from(value: Vec<markdown::Block>) -> Self {
        let mut bundle = MDBundle {
            version: VERSION.to_owned(),
            processed: false,
            docs: indexmap::IndexMap::new(),
            options: MDOptions {
                typ: "bundle".into(),
                name: String::new(),
            },
            watcher: Watcher { debounce_secs: 2 },
            commands: HashMap::new(),
            about: String::new(),
            help: String::from("this is a salt package"),
            is_pinned: false,
            bundle_path: PathBuf::new(),
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
                        bundle
                            .docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::OrderedList(li, li_type)));
                        continue;
                    }
                }
                Block::CodeBlock(copt, code) => {
                    if !doc_section.is_empty() && mode == 2 {
                        bundle
                            .docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::CodeBlock(copt, code)));
                        continue;
                    }
                }
                Block::Paragraph(pspans) => {
                    if !doc_section.is_empty() && mode == 2 {
                        bundle
                            .docs
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
                        bundle.about = cmd_info;
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
                            bundle.help = cmd_info;
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
                        return bundle;
                    }
                    if !doc_section.is_empty() && hsize == 4_usize {
                        bundle
                            .docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::Header(h.clone(), 4_usize)));
                        continue;
                    }
                    if hsize == 3_usize {
                        mode = 2;
                        match h.first().unwrap() {
                            markdown::Span::Text(t) => {
                                doc_section = t.to_owned();
                                bundle.docs.entry(t.to_owned()).or_insert(Vec::new());
                            }
                            _ => {
                                return bundle;
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
                                    return bundle;
                                }
                            },
                            _ => {
                                return bundle;
                            }
                        }
                    }
                }
                Block::Blockquote(bq) => {
                    if !doc_section.is_empty() && mode == 2 {
                        bundle
                            .docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::Blockquote(bq)));
                        continue;
                    }
                }
                Block::UnorderedList(items) => {
                    if !doc_section.is_empty() && mode == 2 {
                        bundle
                            .docs
                            .entry(doc_section.clone())
                            .and_modify(|e| e.push(Block::UnorderedList(items)));
                        continue;
                    }
                    if mode == 1 {
                        for item in items {
                            match item {
                                markdown::ListItem::Simple(span_vec) => {
                                    // println!("command: {span_vec:?}");
                                    let mut cmd_info = String::new();
                                    for span in span_vec {
                                        match span {
                                            markdown::Span::Text(t) => cmd_info.push_str(&t),
                                            markdown::Span::Code(c) => cmd_info.push_str(&c),
                                            _ => return bundle,
                                        };
                                    }
                                    // println!("joint: {cmd_info}");
                                    let splitted = cmd_info
                                        .split('-')
                                        .map(|e| e.trim())
                                        .collect::<Vec<&str>>();
                                    if splitted.len() == 1 {
                                        continue;
                                    }
                                    let cmd = Command {
                                        args: vec![],
                                        about: splitted.get(2).unwrap().to_owned().into(),
                                        command: splitted.get(1).unwrap().to_owned().into(),
                                    };
                                    bundle
                                        .commands
                                        .insert(splitted.first().unwrap().to_owned().into(), cmd);
                                }
                                markdown::ListItem::Paragraph(_) => return bundle,
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
                                            _ => return bundle,
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
                                            bundle.options.typ =
                                                splitted.get(1).unwrap().to_owned().into();
                                        }
                                        "name" => {
                                            // this handles cases when the name of the project has
                                            // hyphen(-) in it
                                            bundle.options.name = splitted
                                                .get(1..splitted.len())
                                                .unwrap()
                                                .to_owned()
                                                .join("-");
                                        }
                                        _ => {
                                            continue;
                                        }
                                    }
                                }
                                markdown::ListItem::Paragraph(_) => return bundle,
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        bundle.processed = true;
        bundle
    }
}
