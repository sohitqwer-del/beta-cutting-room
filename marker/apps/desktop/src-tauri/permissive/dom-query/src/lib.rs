//! Original Apache-2.0 compatibility adapter for Tauri's HTML operations.
//! This is not the upstream dom_query implementation. Only Tauri's four
//! selectors and basic tag selectors are supported. HTML parsing/escaping uses
//! the permissively licensed html5ever and RcDom implementations.
use html5ever::{LocalName, QualName, ns, parse_document, serialize, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

/// Parsed document with an HTML5 tree.
pub struct Document {
    dom: RcDom,
    pub tree: Tree,
}
/// Element factory attached to a document.
pub struct Tree;
/// Mutable shared node in the parsed tree.
#[derive(Clone)]
pub struct NodeRef<'a> {
    handle: Handle,
    lifetime: PhantomData<&'a ()>,
}
/// Selected elements in document order.
pub struct Selection<'a> {
    nodes: Vec<NodeRef<'a>>,
}
impl From<String> for Document {
    fn from(source: String) -> Self {
        Self {
            dom: parse_document(RcDom::default(), Default::default()).one(source),
            tree: Tree,
        }
    }
}
impl Document {
    /// Serialize using the HTML5 serializer, including proper attribute escaping.
    pub fn html(&self) -> String {
        let mut out = Vec::new();
        let serializable = SerializableHandle::from(Rc::clone(&self.dom.document));
        // Vec writes cannot fail. The serializer emits UTF-8.
        if serialize(&mut out, &serializable, Default::default()).is_err() {
            return String::new();
        }
        String::from_utf8_lossy(&out).into_owned()
    }
    /// Return the HTML head inserted by the HTML5 parser.
    pub fn head(&self) -> Option<NodeRef<'_>> {
        self.select("head").nodes.into_iter().next()
    }
    /// Return the HTML root inserted by the HTML5 parser.
    pub fn html_root(&self) -> NodeRef<'_> {
        self.select("html")
            .nodes
            .into_iter()
            .next()
            .unwrap_or_else(|| NodeRef::new(Rc::clone(&self.dom.document)))
    }
    /// Select the tag/attribute patterns used by the pinned Tauri code generator.
    pub fn select(&self, selector: &str) -> Selection<'_> {
        fn visit<'a>(node: &Handle, selector: &str, out: &mut Vec<NodeRef<'a>>) {
            let reference = NodeRef::new(Rc::clone(node));
            if let NodeData::Element { name, .. } = &node.data {
                let tag = name.local.as_ref();
                let matches = match selector {
                    "script[src^='http']" => {
                        tag == "script"
                            && reference.attr("src").is_some_and(|s| s.starts_with("http"))
                    }
                    "script[src]" => tag == "script" && reference.attr("src").is_some(),
                    "script:not(:empty)" => tag == "script" && !node.children.borrow().is_empty(),
                    tag_selector => tag == tag_selector,
                };
                if matches {
                    out.push(reference);
                }
            }
            for child in node.children.borrow().iter() {
                visit(child, selector, out);
            }
        }
        let mut nodes = Vec::new();
        visit(&self.dom.document, selector, &mut nodes);
        Selection { nodes }
    }
}
impl Tree {
    /// Create a detached HTML element.
    pub fn new_element(&self, tag: &str) -> NodeRef<'_> {
        NodeRef::new(Node::new(NodeData::Element {
            name: QualName::new(None, ns!(html), LocalName::from(tag)),
            attrs: RefCell::new(Vec::new()),
            template_contents: RefCell::new(None),
            mathml_annotation_xml_integration_point: false,
        }))
    }
}
impl<'a> Selection<'a> {
    /// Iterate over selected nodes.
    pub fn iter(&self) -> std::slice::Iter<'_, NodeRef<'a>> {
        self.nodes.iter()
    }

    /// Selected nodes in document order.
    pub fn nodes(&self) -> &[NodeRef<'a>] {
        &self.nodes
    }
}
impl NodeRef<'_> {
    fn new(handle: Handle) -> Self {
        Self {
            handle,
            lifetime: PhantomData,
        }
    }
    /// Read an attribute as owned text.
    pub fn attr(&self, key: &str) -> Option<String> {
        if let NodeData::Element { attrs, .. } = &self.handle.data {
            attrs
                .borrow()
                .iter()
                .find(|a| a.name.local.as_ref() == key)
                .map(|a| a.value.to_string())
        } else {
            None
        }
    }
    /// Set or replace an HTML attribute.
    pub fn set_attr(&self, key: &str, value: &str) {
        if let NodeData::Element { attrs, .. } = &self.handle.data {
            let mut attrs = attrs.borrow_mut();
            if let Some(attr) = attrs.iter_mut().find(|a| a.name.local.as_ref() == key) {
                attr.value = value.into();
            } else {
                attrs.push(html5ever::Attribute {
                    name: QualName::new(None, ns!(), LocalName::from(key)),
                    value: value.into(),
                });
            }
        }
    }
    /// Remove an HTML attribute.
    pub fn remove_attr(&self, key: &str) {
        if let NodeData::Element { attrs, .. } = &self.handle.data {
            attrs.borrow_mut().retain(|a| a.name.local.as_ref() != key);
        }
    }
    /// Read descendant text. Script text stays literal for CSP hashing.
    pub fn text(&self) -> String {
        fn text(node: &Handle, out: &mut String) {
            if let NodeData::Text { contents } = &node.data {
                out.push_str(&contents.borrow());
            }
            for child in node.children.borrow().iter() {
                text(child, out);
            }
        }
        let mut out = String::new();
        text(&self.handle, &mut out);
        out
    }
    /// Replace children with a text node.
    pub fn set_text(&self, text: impl AsRef<str>) {
        self.handle.children.borrow_mut().clear();
        self.append_child(&Self::new(Node::new(NodeData::Text {
            contents: RefCell::new(text.as_ref().into()),
        })));
    }
    /// Append a newly created node.
    pub fn append_child(&self, child: &Self) {
        child.handle.parent.set(Some(Rc::downgrade(&self.handle)));
        self.handle
            .children
            .borrow_mut()
            .push(Rc::clone(&child.handle));
    }
    /// Prepend a newly created node.
    pub fn prepend_child(&self, child: &Self) {
        child.handle.parent.set(Some(Rc::downgrade(&self.handle)));
        self.handle
            .children
            .borrow_mut()
            .insert(0, Rc::clone(&child.handle));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csp_and_script_injection_preserve_html_and_escape_attributes() {
        let d = Document::from(
            "<!DOCTYPE html><html><head></head><body><h1>Hello</h1></body></html>".to_string(),
        );
        let head = d.head().unwrap();
        let meta = d.tree.new_element("meta");
        meta.set_attr("content", r#"a"<&"#);
        head.append_child(&meta);
        let script = d.tree.new_element("script");
        script.set_text("if (a < b) alert('ok');");
        head.prepend_child(&script);
        let html = d.html();
        let reopened = Document::from(html.clone());
        let metas = reopened.select("meta");
        assert_eq!(metas.nodes().len(), 1);
        assert_eq!(metas.nodes()[0].attr("content").as_deref(), Some(r#"a"<&"#));
        assert!(html.contains("if (a < b)"));
        assert!(html.contains("<h1>Hello</h1>"));
        assert_eq!(d.select("script:not(:empty)").nodes().len(), 1);
    }
    #[test]
    fn nonce_selectors_skip_local_and_find_remote_scripts_and_styles() {
        let d=Document::from("<script src='http://example.test/a.js'></script><script src='/local.js'></script><style>body{}</style>".to_string());
        assert_eq!(d.select("script[src^='http']").nodes().len(), 1);
        assert_eq!(d.select("script[src]").nodes().len(), 2);
        let styles = d.select("style");
        styles.nodes()[0].set_attr("nonce", "token");
        assert!(d.html().contains("nonce=\"token\""));
    }
}
