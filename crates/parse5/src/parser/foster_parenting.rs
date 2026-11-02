use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn should_foster_parent_on_insertion(&self) -> bool {
        self.foster_parenting_enabled
            && self
                .open_elements
                .current_tag_id
                .is_some_and(is_table_structure_tag)
    }

    pub(in crate::parser) fn find_foster_parenting_location(&self) -> (NodeRef, Option<NodeRef>) {
        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let open_element = self.open_elements.items[idx].clone();

            match self.open_elements.tag_ids[idx] {
                TagId::Template if is_in_namespace(&open_element, NS_HTML) => {
                    if let Some(content) =
                        crate::tree_adapters::default::get_template_content(&open_element)
                    {
                        return (content, None);
                    }
                }
                TagId::Table => {
                    let parent = open_element.borrow().parent();

                    if let Some(parent) = parent {
                        return (parent, Some(open_element));
                    }

                    let parent = idx
                        .checked_sub(1)
                        .and_then(|parent_idx| self.open_elements.items.get(parent_idx))
                        .cloned()
                        .unwrap_or_else(|| self.document.clone());
                    return (parent, None);
                }
                _ => {}
            }
        }

        (
            self.open_elements
                .items
                .first()
                .cloned()
                .unwrap_or_else(|| self.document.clone()),
            None,
        )
    }

    pub(in crate::parser) fn foster_parent_element(&mut self, element: NodeRef) {
        let (parent, before_element) = self.find_foster_parenting_location();

        if let Some(before_element) = before_element {
            crate::tree_adapters::default::insert_before(&parent, element, &before_element);
        } else {
            append_child(&parent, element);
        }
    }
}
