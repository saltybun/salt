use std::collections::HashMap;

use markdown::Block;

use super::Command;

#[derive(Debug)]
pub(crate) struct MDOptions {
    typ: String,
}

#[derive(Debug)]
pub(crate) struct MDBundle {
    processed: bool,
    docs: HashMap<String, Vec<Block>>,
    options: MDOptions,
    commands: HashMap<String, Command>,
    about: String,
}

impl From<Vec<markdown::Block>> for MDBundle {
    fn from(value: Vec<markdown::Block>) -> Self {
        let mut bundle = MDBundle {
            processed: false,
            docs: HashMap::new(),
            options: MDOptions {
                typ: "bundle".into(),
            },
            commands: HashMap::new(),
            about: String::new(),
        };

        // mode 0 = processing about
        // mode 1 = processing the body of commands
        // mode 2 = processing docs
        // mode 3 = processing options
        let mut mode = 0;
        let mut doc_section = String::new();
        println!("Values: {:?}", value);
        for block in value {
            match block {
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
                        for span in pspans {
                            match span {
                                markdown::Span::Text(t) => cmd_info.push_str(&t),
                                markdown::Span::Code(c) => cmd_info.push_str(&c),
                                _ => continue,
                            }
                        }
                        bundle.about = cmd_info;
                    }
                }
                Block::Header(h, hsize) => {
                    println!("header : {h:?}");

                    if hsize == 1 as usize {
                        continue;
                    }
                    if hsize == 2 as usize && h.len() != 1 {
                        return bundle;
                    }
                    if hsize == 3 as usize {
                        mode = 2;
                        match h.first().unwrap() {
                            markdown::Span::Text(t) => {
                                doc_section = t.to_owned();
                                bundle.docs.insert(t.to_owned(), Vec::new());
                            }
                            _ => {
                                return bundle;
                            }
                        }
                        continue;
                    }
                    if hsize == 2 as usize {
                        doc_section = String::new();
                        match h.first().unwrap() {
                            // markdown::Span::Break => todo!(),
                            markdown::Span::Text(t) => match t.to_lowercase().as_str() {
                                "about" => {
                                    mode = 0;
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
                Block::UnorderedList(items) => {
                    if mode == 1 {
                        for item in items {
                            match item {
                                markdown::ListItem::Simple(span_vec) => {
                                    println!("command: {span_vec:?}");
                                    let mut cmd_info = String::new();
                                    for span in span_vec {
                                        match span {
                                            markdown::Span::Text(t) => cmd_info.push_str(&t),
                                            markdown::Span::Code(c) => cmd_info.push_str(&c),
                                            _ => return bundle,
                                        };
                                    }
                                    println!("joint: {cmd_info}");
                                    let splitted = cmd_info
                                        .split("-")
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
                                        .insert(splitted.get(0).unwrap().to_owned().into(), cmd);
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
                                        .split("-")
                                        .map(|e| e.trim())
                                        .collect::<Vec<&str>>();
                                    if splitted.len() == 1 {
                                        continue;
                                    }
                                    match splitted.first().unwrap().to_owned() {
                                        "type" => {
                                            bundle.options.typ =
                                                splitted.get(1).unwrap().to_owned().into();
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
