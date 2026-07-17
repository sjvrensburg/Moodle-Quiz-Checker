//! Minimal DOM-like XML tree, built on top of quick-xml's pull parser.
//!
//! Moodle's Moodle-XML export format is small/medium sized and deeply nested with
//! many optional elements, so a tiny in-memory tree is much easier (and safer) to
//! walk than a hand-rolled streaming state machine.

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<Node>,
    pub text: String,
}

impl Node {
    /// First direct child with the given tag name.
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    /// All direct children with the given tag name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Convenience: text of the first `<text>` grandchild, e.g. `<name><text>Foo</text></name>`.
    pub fn text_of(&self, name: &str) -> Option<String> {
        self.child(name).and_then(|n| n.child("text")).map(|t| t.text.clone())
    }

    /// Text of a direct child that itself holds the text (no `<text>` wrapper), trimmed.
    pub fn direct_text_of(&self, name: &str) -> Option<String> {
        self.child(name).map(|n| n.text.trim().to_string())
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(|s| s.as_str())
    }

    /// All descendants (at any depth) with the given tag name.
    pub fn find_all<'a>(&'a self, name: &str) -> Vec<&'a Node> {
        let mut out = Vec::new();
        for child in &self.children {
            if child.name == name {
                out.push(child);
            }
            out.extend(child.find_all(name));
        }
        out
    }

    pub fn own_text(&self) -> String {
        // For a node like <text><![CDATA[...]]></text> the CDATA content lands directly
        // in `text`; for a node with a `<text>` child wrapper, prefer that.
        if let Some(t) = self.child("text") {
            t.text.clone()
        } else {
            self.text.clone()
        }
    }
}

pub fn parse(xml: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<Node> = vec![Node {
        name: "#root".into(),
        ..Default::default()
    }];

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut node = Node {
                    name,
                    ..Default::default()
                };
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                    let val = a.unescape_value().unwrap_or_default().to_string();
                    node.attrs.insert(key, val);
                }
                stack.push(node);
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut node = Node {
                    name,
                    ..Default::default()
                };
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                    let val = a.unescape_value().unwrap_or_default().to_string();
                    node.attrs.insert(key, val);
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            Ok(Event::End(_)) => {
                if let Some(finished) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(finished);
                    } else {
                        stack.push(finished);
                        break;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let txt = e.unescape().unwrap_or_default().to_string();
                if let Some(cur) = stack.last_mut() {
                    cur.text.push_str(&txt);
                }
            }
            Ok(Event::CData(e)) => {
                let txt = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some(cur) = stack.last_mut() {
                    cur.text.push_str(&txt);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(format!("XML parse error at position {}: {e}", reader.buffer_position())),
        }
    }

    stack.pop().ok_or_else(|| "empty document".to_string())
}
