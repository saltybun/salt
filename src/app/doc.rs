use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use markdown::{Block, Span};
use serde::{Deserialize, Serialize};

use super::ProjectDefinition;

#[derive(Serialize, Deserialize)]
pub struct Doc {
    version: String,
    project: String,
    script_content: String,
    // TODO: consider converting this to struct for clearer understanding
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
                if !t.starts_with("<!--") {
                    html.push_str(t);
                }
            }
            markdown::Span::Code(c) => {
                html.push_str(&format!("<code>{c}</code>"));
            }
            markdown::Span::Link(text, link, _) => {
                html.push_str(&format!(r#"<a href="{link}" target="_blank">{text}</a>"#));
            }
            markdown::Span::Image(_, _, _) => todo!(),
            markdown::Span::Emphasis(ispans) => {
                let emph_html = spans_to_html(ispans);
                html.push_str(&format!("<i>{}</i>", &emph_html));
            }
            markdown::Span::Strong(spans) => {
                let strong_html = spans_to_html(spans);
                html.push_str(&format!("<b>{}</b>", &strong_html));
            }
        }
    }

    html
}

fn blocks_to_html(html: &mut String, script_content: &mut String, blocks: &Vec<Block>, uid: usize) {
    for (i, block) in blocks.iter().enumerate() {
        match block {
            Block::Blockquote(bq) => {
                html.push_str(r#"<div class="bq">"#);
                blocks_to_html(html, script_content, bq, uid);
                html.push_str("</div>");
            }
            Block::Header(h, _) => {
                html.push_str("<h4>");
                html.push_str(&spans_to_html(h));
                html.push_str("</h4>");
            }
            Block::Paragraph(spans) => {
                html.push_str(&spans_to_html(spans));
                // peeking if next block is also a paragraph
                if blocks.get(i + 1).is_some()
                    && matches!(blocks.get(i + 1).unwrap(), Block::Paragraph(_))
                {
                    html.push_str("<br /><br />")
                }
            }
            Block::CodeBlock(meta, cblock) => {
                if meta.is_some() && meta.as_ref().unwrap().eq("dot") {
                    // uid_cblock is the combination of dot graph value
                    // and appended by -- and followed by uid
                    // which is the position of this section
                    let mut uid_cblock = cblock.to_owned();
                    uid_cblock.push_str(&format!("--{}--", uid));

                    // create a unique id from the uid_cblock for injecting
                    // graphviz graph on page load
                    let viz_element = format!("viz-{}", get_hashed_id(uid_cblock));

                    // create a div with viz element id
                    html.push_str(&format!("<div id='{}'></div>", viz_element));

                    // add function call to load dot graph into the viz element on window load
                    append_dot_script_block(&viz_element, script_content, cblock);
                } else {
                    html.push_str(&format!("<pre>{cblock}</pre>"));
                }
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
                            blocks_to_html(&mut list_para, script_content, p, uid);
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
                            blocks_to_html(&mut list_para, script_content, p, uid);
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

fn append_dot_script_block(viz_element: &String, script_content: &mut String, cblock: &str) {
    let dot_block = format!(
        r#"draw_into_element(`{}`, '{}');

    "#,
        cblock, viz_element
    );
    script_content.push_str(&dot_block);
}

fn get_html(blocks: &Vec<Block>, script_content: &mut String, uid: usize) -> String {
    let mut html = String::new();
    blocks_to_html(&mut html, script_content, blocks, uid);
    html
}

impl From<ProjectDefinition> for Doc {
    fn from(value: ProjectDefinition) -> Self {
        let mut doc = Doc {
            version: value.version.clone(),
            project: value.options.name,
            script_content: String::new(),
            contents: vec![],
            titles: vec![],
            about: value.about,
            commands: vec![],
        };

        // here we create map of all sections/titles
        // (which is shown in the left side) of the documentation
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

        // here we push contents of each section
        for (i, content) in value.docs.values().enumerate() {
            let mut script_chunk = String::new();
            let html = get_html(content, &mut script_chunk, i);
            doc.script_content.push_str(&script_chunk);

            // with corresponding:
            // - html string
            // - title of each section
            // - and if we want to show that section active (default active: 0)
            let data = (
                html,
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
