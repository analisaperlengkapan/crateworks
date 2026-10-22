//! Namespaces: the identity of an element, rather than the prefix it was written with.
//!
//! The prefix is arbitrary. `<soap:Body xmlns:soap="urn:a">` and
//! `<env:Body xmlns:env="urn:a">` are the same element in the same namespace, and
//! a reader that matches on the name as written will find one and miss the other.
//! The namespace URI is the identity, the prefix is only how this document
//! happened to spell it, and that is what this module resolves.
//!
//! Bindings are scoped: `xmlns:p` declared on an element applies to it and to its
//! descendants, and a descendant may rebind `p` without affecting anything else.
//! A prefix can only be resolved with that chain in hand, so nothing here works on
//! an [`Element`] alone; every function walks from a root and carries the scope
//! down. That is the piece `soap-client` documented as missing.

use std::collections::BTreeMap;

use crate::Element;

/// The namespace every `xml:` prefix is bound to, without a declaration.
///
/// The `xml` prefix is bound by the specification itself, so a document that uses
/// `xml:lang` never declares it.
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The prefix reserved for namespace declarations.
pub const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// Splits a name into its prefix and its local name.
///
/// A name with no colon has no prefix. The split is on the **first** colon, which
/// is what the specification requires and what makes `a:b:c` the prefix `a` and
/// the local name `b:c`.
#[must_use]
pub fn split_name(name: &str) -> (Option<&str>, &str) {
    match name.split_once(':') {
        Some((prefix, local)) => (Some(prefix), local),
        None => (None, name),
    }
}

/// The namespace bindings in scope at one point in a document.
///
/// Built by starting from [`Namespaces::new`] and descending with
/// [`Namespaces::child`], which adds each element's `xmlns` declarations. Cheap to
/// clone, and cloning is how a walk keeps a parent's scope while entering a child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespaces {
    /// Prefix to URI. The empty string is the default namespace.
    bindings: BTreeMap<String, String>,
}

impl Default for Namespaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespaces {
    /// The outermost scope, with only the `xml` prefix bound.
    #[must_use]
    pub fn new() -> Self {
        let mut bindings = BTreeMap::new();
        bindings.insert("xml".to_string(), XML_NAMESPACE.to_string());
        Self { bindings }
    }

    /// The scope inside `element`: this one with the element's declarations added.
    ///
    /// A declaration here shadows an outer one, and `xmlns=""` removes the default
    /// namespace rather than binding it to the empty URI, which is a distinction
    /// the specification makes and a reader that ignores it gets wrong.
    #[must_use]
    pub fn child(&self, element: &Element) -> Self {
        let mut bindings = self.bindings.clone();
        for (name, value) in &element.attributes {
            if name == "xmlns" {
                if value.is_empty() {
                    bindings.remove("");
                } else {
                    bindings.insert(String::new(), value.clone());
                }
            } else if let Some(prefix) = name.strip_prefix("xmlns:") {
                if value.is_empty() {
                    bindings.remove(prefix);
                } else {
                    bindings.insert(prefix.to_string(), value.clone());
                }
            }
        }
        Self { bindings }
    }

    /// The URI a **prefix** is bound to, or `None` when it is unbound.
    ///
    /// The empty prefix is the default namespace. This takes a prefix, not a name:
    /// `resolve_prefix("xml")` gives the XML namespace, while `resolve("xml")`
    /// looks up a name with no prefix and finds the default namespace instead.
    /// Mixing the two up is easy and silent, so they are separate functions.
    #[must_use]
    pub fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        self.bindings.get(prefix).map(String::as_str)
    }

    /// The URI a **name** resolves to, or `None` when it is in no namespace.
    ///
    /// An unprefixed name resolves to the default namespace, or to nothing when
    /// there is none. A prefixed name resolves to its binding, or to nothing when
    /// the prefix is unbound - an unbound prefix is not an error here, it simply
    /// names nothing.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<&str> {
        let (prefix, _) = split_name(name);
        match prefix {
            Some(prefix) => self.resolve_prefix(prefix),
            None => self.resolve_prefix(""),
        }
    }

    /// The default namespace, which applies to unprefixed elements and not to
    /// unprefixed attributes.
    #[must_use]
    pub fn default_uri(&self) -> Option<&str> {
        self.resolve_prefix("")
    }

    /// Every binding in scope, including the implicit `xml` one.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.bindings
            .iter()
            .map(|(prefix, uri)| (prefix.as_str(), uri.as_str()))
    }
}

/// Whether an element matches a namespace URI and local name.
///
/// `uri` of `None` matches an element in no namespace, which is what an
/// unprefixed element with no default namespace is.
fn matches(element: &Element, scope: &Namespaces, uri: Option<&str>, local: &str) -> bool {
    let (prefix, element_local) = split_name(&element.name);
    if element_local != local {
        return false;
    }
    let resolved = match prefix {
        // An unbound prefix is not "no namespace": the name resolves to nothing,
        // so it matches no query. Treating it as no namespace would let `<evil:Body>`
        // with `evil` unbound pass for an un-namespaced `Body`, which is exactly the
        // confusion this module exists to prevent.
        Some(prefix) => match scope.resolve_prefix(prefix) {
            Some(bound) => Some(bound),
            None => return false,
        },
        None => scope.default_uri(),
    };
    resolved == uri
}

/// Calls `visit` for every element from `root` down, in document order, with the
/// scope that applies to it.
///
/// `visit` returns `true` to stop the walk. The traversal keeps its own stack
/// rather than recursing, so a tree that was built by hand past the parser's
/// depth cap is walked without touching the call stack.
fn walk<'a>(
    root: &'a Element,
    visit: &mut impl FnMut(&'a Element, &Namespaces) -> bool,
) -> Option<(&'a Element, Namespaces)> {
    let mut stack = vec![(root, Namespaces::new())];
    // Children are pushed in reverse so they pop in document order.
    while let Some((element, scope)) = stack.pop() {
        let inner = scope.child(element);
        if visit(element, &inner) {
            return Some((element, inner));
        }
        for child in element.children.iter().rev() {
            if let crate::Node::Element(child) = child {
                stack.push((child, inner.clone()));
            }
        }
    }
    None
}

/// Finds the first element with this namespace URI and local name, at any depth.
///
/// The prefix does not matter: any prefix bound to `uri` matches, and so does an
/// unprefixed element when `uri` is the default namespace. Pass `None` for an
/// element in no namespace.
///
/// # Example
///
/// ```
/// use xmlkit::{XML_NAMESPACE, find_namespaced, parse};
///
/// // The prefix in the document is irrelevant; the URI is the identity.
/// let doc = parse(r#"<env:Envelope xmlns:env="urn:soap"><env:Body/></env:Envelope>"#).unwrap();
/// assert!(find_namespaced(&doc, Some("urn:soap"), "Body").is_some());
/// assert!(find_namespaced(&doc, Some("urn:other"), "Body").is_none());
/// ```
#[must_use]
pub fn find_namespaced<'a>(
    root: &'a Element,
    uri: Option<&str>,
    local: &str,
) -> Option<&'a Element> {
    walk(root, &mut |element, scope| {
        matches(element, scope, uri, local)
    })
    .map(|(element, _)| element)
}

/// Every element with this namespace URI and local name, in document order.
#[must_use]
pub fn all_namespaced<'a>(root: &'a Element, uri: Option<&str>, local: &str) -> Vec<&'a Element> {
    let mut found = Vec::new();
    walk(root, &mut |element, scope| {
        if matches(element, scope, uri, local) {
            found.push(element);
        }
        false
    });
    found
}

/// The namespace URI of `target`, resolving through its ancestors.
///
/// Returns `None` when `target` is not in the tree rooted at `root`, when its name
/// has no namespace, or when its prefix is unbound. `Some("")` is never returned;
/// an element in no namespace gives `None`.
#[must_use]
pub fn element_namespace(root: &Element, target: &Element) -> Option<String> {
    walk(root, &mut |element, _| std::ptr::eq(element, target))
        .and_then(|(element, scope)| scope.resolve(&element.name).map(str::to_string))
}

/// The in-scope namespace bindings at `target`, or `None` when it is not in the
/// tree rooted at `root`.
///
/// This is the whole chain, not just the element's own declarations, which is what
/// a caller needs to resolve a name written by hand.
#[must_use]
pub fn namespaces_at(root: &Element, target: &Element) -> Option<Namespaces> {
    walk(root, &mut |element, _| std::ptr::eq(element, target)).map(|(_, scope)| scope)
}

/// A namespaced attribute value.
///
/// Attributes are the one place the default namespace does **not** apply: an
/// unprefixed attribute is in no namespace even inside an element that declares
/// one. So `uri` of `""` looks for an unprefixed attribute, and any other value
/// looks for one whose prefix is bound to that URI - `xml:lang` is the usual case,
/// and it resolves through [`XML_NAMESPACE`] without a declaration.
#[must_use]
pub fn attribute_namespaced<'a>(
    root: &'a Element,
    target: &'a Element,
    uri: &str,
    local: &str,
) -> Option<&'a str> {
    let scope = namespaces_at(root, target)?;
    target.attributes.iter().find_map(|(name, value)| {
        let (prefix, attribute_local) = split_name(name);
        if attribute_local != local {
            return None;
        }
        let attribute_uri = match prefix {
            // An unprefixed attribute is in no namespace, so the default
            // namespace must not be consulted here.
            None => "",
            Some(prefix) => scope.resolve_prefix(prefix)?,
        };
        (attribute_uri == uri).then_some(value.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn the_prefix_is_not_the_identity() {
        // Two documents that spell the same element differently. Both lookups
        // must find it, which matching on the name as written cannot do.
        let a = parse(r#"<soap:Body xmlns:soap="urn:soap"/>"#).unwrap();
        let b = parse(r#"<env:Body xmlns:env="urn:soap"/>"#).unwrap();
        assert!(find_namespaced(&a, Some("urn:soap"), "Body").is_some());
        assert!(find_namespaced(&b, Some("urn:soap"), "Body").is_some());
        assert_eq!(element_namespace(&a, &a).as_deref(), Some("urn:soap"));
        assert_eq!(element_namespace(&b, &b).as_deref(), Some("urn:soap"));
    }

    #[test]
    fn a_rebound_prefix_shadows_the_outer_one() {
        let doc = parse(
            r#"<r xmlns:p="urn:outer"><p:item id="1"/><c xmlns:p="urn:inner"><p:item id="2"/></c></r>"#,
        )
        .unwrap();
        let outer = all_namespaced(&doc, Some("urn:outer"), "item");
        let inner = all_namespaced(&doc, Some("urn:inner"), "item");
        assert_eq!(outer.len(), 1);
        assert_eq!(inner.len(), 1);
        assert_eq!(outer[0].attr("id"), Some("1"));
        assert_eq!(inner[0].attr("id"), Some("2"));
    }

    #[test]
    fn the_default_namespace_applies_to_elements_at_any_depth() {
        let doc = parse(r#"<a xmlns="urn:d"><b><c/></b></a>"#).unwrap();
        assert_eq!(element_namespace(&doc, &doc).as_deref(), Some("urn:d"));
        assert!(find_namespaced(&doc, Some("urn:d"), "c").is_some());
        assert!(find_namespaced(&doc, None, "c").is_none());
    }

    #[test]
    fn an_empty_default_namespace_declaration_removes_it() {
        // `xmlns=""` is not a binding to the empty URI, it is the absence of one,
        // and an element under it is in no namespace.
        let doc = parse(r#"<a xmlns="urn:d"><b xmlns=""><c/></b></a>"#).unwrap();
        assert!(find_namespaced(&doc, None, "c").is_some());
        assert!(find_namespaced(&doc, Some("urn:d"), "c").is_none());
    }

    #[test]
    fn the_xml_prefix_is_bound_without_a_declaration() {
        let doc = parse(r#"<a xml:lang="en"/>"#).unwrap();
        assert_eq!(
            attribute_namespaced(&doc, &doc, XML_NAMESPACE, "lang"),
            Some("en")
        );
    }

    #[test]
    fn an_unprefixed_attribute_is_in_no_namespace() {
        // The rule that catches people out: a default namespace does not reach
        // attributes, so `id` here is not in `urn:d`.
        let doc = parse(r#"<a xmlns="urn:d" id="7"/>"#).unwrap();
        assert_eq!(attribute_namespaced(&doc, &doc, "", "id"), Some("7"));
        assert_eq!(attribute_namespaced(&doc, &doc, "urn:d", "id"), None);
    }

    #[test]
    fn an_unbound_prefix_names_nothing_rather_than_failing() {
        let doc = parse(r#"<p:a><p:b/></p:a>"#).unwrap();
        // Not "no namespace": an unbound prefix matches nothing at all, so a
        // document cannot smuggle an element into a no-namespace query by leaving
        // its prefix undeclared.
        assert!(find_namespaced(&doc, None, "b").is_none());
        assert!(find_namespaced(&doc, Some("urn:x"), "b").is_none());
        assert_eq!(element_namespace(&doc, &doc), None);
        // The local name is still available for a caller that does not care
        // about namespaces, which is what `local_name` strips the prefix for.
        assert_eq!(crate::local_name(&doc.name), "a");
    }

    #[test]
    fn namespaces_at_reports_the_whole_chain() {
        let doc = parse(r#"<a xmlns:x="urn:x"><b xmlns:y="urn:y"><c/></b></a>"#).unwrap();
        let c = find_namespaced(&doc, None, "c").unwrap();
        let scope = namespaces_at(&doc, c).unwrap();
        assert_eq!(scope.resolve("x:a"), Some("urn:x"));
        assert_eq!(scope.resolve("y:b"), Some("urn:y"));
        // A prefix declared on a descendant is not visible to an ancestor.
        let a_scope = namespaces_at(&doc, &doc).unwrap();
        assert_eq!(a_scope.resolve("y:b"), None);
    }

    #[test]
    fn a_target_outside_the_tree_has_no_scope() {
        let doc = parse("<a/>").unwrap();
        let stranger = Element::new("b");
        assert!(namespaces_at(&doc, &stranger).is_none());
        assert_eq!(element_namespace(&doc, &stranger), None);
    }

    #[test]
    fn split_name_splits_on_the_first_colon() {
        assert_eq!(split_name("a"), (None, "a"));
        assert_eq!(split_name("a:b"), (Some("a"), "b"));
        assert_eq!(split_name("a:b:c"), (Some("a"), "b:c"));
        assert_eq!(split_name(":a"), (Some(""), "a"));
    }

    #[test]
    fn a_hand_built_tree_deeper_than_the_parse_cap_still_walks() {
        // The parser caps depth, but a tree can be built by hand without that
        // cap, so the walk must not recurse on the call stack.
        let mut root = Element::new("root");
        let mut cursor = &mut root;
        for _ in 0..10_000 {
            cursor.push(Element::new("x"));
            cursor = match cursor.children.last_mut() {
                Some(crate::Node::Element(child)) => child,
                _ => unreachable!(),
            };
        }
        assert_eq!(all_namespaced(&root, None, "x").len(), 10_000);
        assert!(find_namespaced(&root, None, "x").is_some());
    }
}
