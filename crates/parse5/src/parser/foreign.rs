use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn should_process_start_tag_token_in_foreign_content(
        &self,
        token: &TagToken,
    ) -> bool {
        if !self.current_not_in_html() {
            return false;
        }

        let (current, current_tag_id) = self.current_node_and_tag_id_for_context();

        if token.tag_id == TagId::Svg
            && element_tag_name(&current).as_deref() == Some("annotation-xml")
            && is_in_namespace(&current, NS_MATHML)
        {
            return false;
        }

        let tokenizer_in_foreign_node = !self.is_integration_point(current_tag_id, &current, None);

        tokenizer_in_foreign_node
            || (matches!(token.tag_id, TagId::Mglyph | TagId::Malignmark)
                && !self.is_integration_point(current_tag_id, &current, Some(NS_HTML)))
    }

    pub(in crate::parser) fn start_tag_in_foreign_content(&mut self, mut token: TagToken) {
        if foreign_content::causes_exit(&token) {
            self.pop_until_html_or_integration_point();
            self.start_tag_outside_foreign_content(token);
            return;
        }

        let current = self.get_adjusted_current_element();
        let current_ns = namespace_uri(&current).unwrap_or_else(|| NS_HTML.to_string());

        if current_ns == NS_MATHML {
            foreign_content::adjust_token_mathml_attrs(&mut token);
        } else if current_ns == NS_SVG {
            foreign_content::adjust_token_svg_tag_name(&mut token);
            foreign_content::adjust_token_svg_attrs(&mut token);
        }

        foreign_content::adjust_token_xml_attrs(&mut token);

        if token.self_closing {
            self.append_element(token, &current_ns);
        } else {
            self.insert_element(token, &current_ns);
        }
    }

    pub(in crate::parser) fn end_tag_in_foreign_content(&mut self, mut token: TagToken) {
        if matches!(token.tag_id, TagId::P | TagId::Br) {
            self.pop_until_html_or_integration_point();
            self.end_tag_outside_foreign_content(token);
            return;
        }

        for idx in (1..=self.open_elements.stack_top.max(0) as usize).rev() {
            let element = self.open_elements.items[idx].clone();

            if is_in_namespace(&element, NS_HTML) {
                self.end_tag_outside_foreign_content(token);
                break;
            }

            let tag_name = element_tag_name(&element).unwrap_or_default();

            if tag_name.to_ascii_lowercase() == token.tag_name {
                token.tag_name = tag_name;
                self.current_token = Some(Token::Tag(token));
                self.shorten_open_elements_to_length(idx as isize);
                break;
            }
        }
    }

    pub(in crate::parser) fn pop_until_html_or_integration_point(&mut self) {
        while let Some(current) = self.open_elements.current.clone() {
            let current_tag_id = self.open_elements.current_tag_id.unwrap_or(TagId::Unknown);

            if is_in_namespace(&current, NS_HTML)
                || self.is_integration_point(current_tag_id, &current, None)
            {
                break;
            }

            self.pop_open_element();
        }
    }

    pub(in crate::parser) fn is_integration_point(
        &self,
        tag_id: TagId,
        element: &NodeRef,
        foreign_namespace: Option<&str>,
    ) -> bool {
        let element = element.borrow();

        foreign_content::is_integration_point(
            tag_id,
            element.namespace_uri().unwrap_or(NS_HTML),
            element.attrs().unwrap_or_default(),
            foreign_namespace,
        )
    }

    pub(in crate::parser) fn current_not_in_html(&self) -> bool {
        let current = self.current_node_and_tag_id_for_context().0;
        !std::rc::Rc::ptr_eq(&current, &self.document) && !is_in_namespace(&current, NS_HTML)
    }
}
