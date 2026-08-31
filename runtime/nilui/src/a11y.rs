// runtime/nilui/src/a11y.rs — Accessibility Tree Collector
use crate::Element;

pub fn collect_text_nodes(elem: &Element, out: &mut Vec<String>) {
    match elem {
        Element::Text { content } => out.push(content.clone()),
        Element::Button { label, .. } => out.push(label.clone()),
        Element::Column { children } | Element::Row { children } | Element::Stack { children } => {
            for c in children {
                collect_text_nodes(c, out);
            }
        }
        Element::Input { text, .. } => out.push(text.clone()),
    }
}
