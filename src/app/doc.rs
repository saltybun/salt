use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use markdown::{Block, Span};
use serde::{Deserialize, Serialize};

use super::MDBundle;

#[derive(Serialize, Deserialize)]
pub struct Doc {
    project: String,
    titles: Vec<(String, String, String)>,
    contents: Vec<(String, String, String)>,
    about: String,
    commands: Vec<(String, String)>,
}

fn get_hashed_id<T: Hash>(obj: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    obj.hash(&mut hasher);
    hasher.finish()
}

fn spans_to_html(spans: &Vec<Span>) -> String {
    let mut html = String::new();
    for span in spans {
        match span {
            markdown::Span::Break => {
                html.push_str("<br />");
            }
            markdown::Span::Text(t) => {
                html.push_str(&t);
            }
            markdown::Span::Code(c) => {
                html.push_str(&format!("<code>{c}</code>"));
            }
            markdown::Span::Link(_, _, _) => todo!(),
            markdown::Span::Image(_, _, _) => todo!(),
            markdown::Span::Emphasis(_) => todo!(),
            markdown::Span::Strong(_) => todo!(),
        }
    }

    html
}

fn blocks_to_html(html: &mut String, blocks: &Vec<Block>) {
    for block in blocks {
        match block {
            Block::Header(h, _) => {
                html.push_str("<h4>");
                html.push_str(&spans_to_html(h));
                html.push_str("</h4>");
            }
            Block::Paragraph(spans) => {
                html.push_str(&spans_to_html(spans));
            }
            // Block::Blockquote(quote) => {

            // },
            Block::CodeBlock(_lang, cblock) => {
                html.push_str(&format!("<pre>{cblock}</pre>"));
            }
            Block::OrderedList(items, _) => {
                html.push_str("<ol>");
                // TODO: extract this for loop into a function
                for item in items {
                    match item {
                        markdown::ListItem::Simple(t) => {
                            html.push_str(&format!("<li>{}</li>", spans_to_html(t)));
                        }
                        markdown::ListItem::Paragraph(p) => {
                            let mut list_para = String::from("<li>");
                            blocks_to_html(&mut list_para, p);
                            html.push_str(&list_para);
                            html.push_str("</li>");
                        }
                    }
                }
                html.push_str("</ol>");
            }
            Block::UnorderedList(items) => {
                html.push_str("<ul>");
                for item in items {
                    match item {
                        markdown::ListItem::Simple(t) => {
                            html.push_str(&format!("<li>{}</li>", spans_to_html(t)));
                        }
                        markdown::ListItem::Paragraph(p) => {
                            let mut list_para = String::from("<li>");
                            blocks_to_html(&mut list_para, p);
                            html.push_str(&list_para);
                            html.push_str("</li>");
                        }
                    }
                }
                html.push_str("</ul>");
            }
            _ => continue,
        }
    }
}

fn get_html(blocks: &Vec<Block>) -> String {
    let mut html = String::new();
    blocks_to_html(&mut html, blocks);
    html
}

impl From<MDBundle> for Doc {
    fn from(value: MDBundle) -> Self {
        let mut doc = Doc {
            project: value.options.name,
            contents: vec![],
            titles: vec![],
            about: value.about,
            commands: vec![],
        };
        let mut index_idhash_map: HashMap<usize, String> = HashMap::new();
        for (i, title) in value.docs.keys().enumerate() {
            let idhash = get_hashed_id(title.clone()).to_string();
            index_idhash_map.insert(i, idhash.clone());
            let data = (
                title.to_owned(),
                idhash,
                if i == 0 { "active".into() } else { "".into() },
            );
            doc.titles.push(data);
        }
        for (i, content) in value.docs.values().enumerate() {
            let data = (
                get_html(content),
                index_idhash_map.get(&i).unwrap().to_owned(),
                if i == 0 {
                    "show active".into()
                } else {
                    "".into()
                },
            );
            doc.contents.push(data);
        }
        for (k, v) in value.commands {
            let data = (k.clone(), v.about.clone());
            doc.commands.push(data);
        }
        doc
    }
}
