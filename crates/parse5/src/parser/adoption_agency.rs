use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn call_adoption_agency(&mut self, token: TagToken) {
        for _ in 0..AA_OUTER_LOOP_ITER {
            let Some(formatting_element_entry) =
                self.aa_obtain_formatting_element_entry(token.clone())
            else {
                break;
            };

            let Some(furthest_block) = self.aa_obtain_furthest_block(&formatting_element_entry)
            else {
                break;
            };

            self.active_formatting_elements
                .set_bookmark(&formatting_element_entry);

            let Some(formatting_element) = formatting_element_entry.element_ref() else {
                break;
            };
            let last_element = self.aa_inner_loop(&furthest_block, &formatting_element);
            let common_ancestor = self.open_elements.get_common_ancestor(&formatting_element);

            detach_node(&last_element);
            if let Some(common_ancestor) = common_ancestor {
                self.aa_insert_last_node_in_common_ancestor(&common_ancestor, last_element);
            }

            self.aa_replace_formatting_element(&furthest_block, &formatting_element_entry);
        }
    }

    pub(in crate::parser) fn aa_obtain_formatting_element_entry(
        &mut self,
        token: TagToken,
    ) -> Option<Entry> {
        let Some(formatting_element_entry) = self
            .active_formatting_elements
            .get_element_entry_in_scope_with_tag_name(&token.tag_name)
        else {
            self.generic_end_tag_in_body(token);
            return None;
        };

        if let Some(element) = formatting_element_entry.element_ref() {
            if !self.open_elements.contains(&element) {
                self.active_formatting_elements
                    .remove_entry(&formatting_element_entry);
                return None;
            } else if !self.open_elements.has_in_scope(token.tag_id) {
                return None;
            }
        }

        Some(formatting_element_entry)
    }

    pub(in crate::parser) fn aa_obtain_furthest_block(
        &mut self,
        formatting_element_entry: &Entry,
    ) -> Option<NodeRef> {
        let formatting_element = formatting_element_entry.element_ref()?;
        let mut furthest_block = None;
        let mut formatting_element_idx = 0isize;

        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let element = self.open_elements.items[idx].clone();

            if std::rc::Rc::ptr_eq(&element, &formatting_element) {
                formatting_element_idx = idx as isize;
                break;
            }

            if self.is_special_element(&element, self.open_elements.tag_ids[idx]) {
                furthest_block = Some(element);
            }
        }

        if furthest_block.is_none() {
            self.shorten_open_elements_to_length(formatting_element_idx.max(0));
            self.active_formatting_elements
                .remove_entry(formatting_element_entry);
        }

        furthest_block
    }

    pub(in crate::parser) fn aa_inner_loop(
        &mut self,
        furthest_block: &NodeRef,
        formatting_element: &NodeRef,
    ) -> NodeRef {
        let mut last_element = furthest_block.clone();
        let Some(mut element) = self.open_elements.get_common_ancestor(furthest_block) else {
            return last_element;
        };

        for idx in 0.. {
            if std::rc::Rc::ptr_eq(&element, formatting_element) {
                break;
            }

            let next_element = self.open_elements.get_common_ancestor(&element);
            let element_entry = self.active_formatting_elements.get_element_entry(&element);
            let counter_overflow = element_entry.is_some() && idx >= AA_INNER_LOOP_ITER;

            if let Some(element_entry) = element_entry {
                if counter_overflow {
                    self.active_formatting_elements.remove_entry(&element_entry);
                    self.remove_open_element(&element);
                } else {
                    let new_element = self.aa_recreate_element_from_entry(&element_entry);

                    if std::rc::Rc::ptr_eq(&last_element, furthest_block) {
                        self.active_formatting_elements.set_bookmark(&element_entry);
                    }

                    detach_node(&last_element);
                    append_child(&new_element, last_element);
                    last_element = new_element;
                }
            } else {
                self.remove_open_element(&element);
            }

            let Some(next_element) = next_element else {
                break;
            };
            element = next_element;
        }

        last_element
    }

    pub(in crate::parser) fn aa_recreate_element_from_entry(
        &mut self,
        element_entry: &Entry,
    ) -> NodeRef {
        let element = element_entry
            .element_ref()
            .expect("formatting element entry must have an element");
        let token = element_entry
            .token
            .clone()
            .expect("formatting element entry must have a token");
        let namespace_uri = namespace_uri(&element).unwrap_or_else(|| NS_HTML.to_string());
        let new_element = create_element(token.tag_name.clone(), namespace_uri, token.attrs);

        self.open_elements.replace(&element, new_element.clone());
        self.active_formatting_elements
            .replace_entry_element(element_entry, new_element.clone());

        new_element
    }

    pub(in crate::parser) fn aa_insert_last_node_in_common_ancestor(
        &mut self,
        common_ancestor: &NodeRef,
        last_element: NodeRef,
    ) {
        let tag_id = element_tag_name(common_ancestor)
            .as_deref()
            .map(get_tag_id)
            .unwrap_or(TagId::Unknown);

        if is_table_structure_tag(tag_id) {
            self.foster_parent_element(last_element);
            return;
        }

        let target = if element_tag_name(common_ancestor).as_deref() == Some(TAG_NAME_TEMPLATE)
            && is_in_namespace(common_ancestor, NS_HTML)
        {
            crate::tree_adapters::default::get_template_content(common_ancestor)
                .unwrap_or_else(|| common_ancestor.clone())
        } else {
            common_ancestor.clone()
        };

        append_child(&target, last_element);
    }

    pub(in crate::parser) fn aa_replace_formatting_element(
        &mut self,
        furthest_block: &NodeRef,
        formatting_element_entry: &Entry,
    ) {
        let Some(formatting_element) = formatting_element_entry.element_ref() else {
            return;
        };
        let Some(token) = formatting_element_entry.token.clone() else {
            return;
        };
        let namespace_uri =
            namespace_uri(&formatting_element).unwrap_or_else(|| NS_HTML.to_string());
        let new_element =
            create_element(token.tag_name.clone(), namespace_uri, token.attrs.clone());

        self.adopt_nodes(furthest_block, &new_element);
        append_child(furthest_block, new_element.clone());

        self.active_formatting_elements
            .insert_element_after_bookmark(new_element.clone(), token.clone());
        self.active_formatting_elements
            .remove_entry(formatting_element_entry);

        self.remove_open_element(&formatting_element);
        self.open_elements
            .insert_after(furthest_block, new_element, token.tag_id);
    }
}
