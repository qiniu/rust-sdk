use crate::utils::generate_prefix_spaces;
use clang::Entity;

pub fn dump_entity(entity: &Entity, pretty_print: bool) {
    dump_entity_with_level(entity, pretty_print, 0);
}

fn dump_entity_with_level(entity: &Entity, pretty_print: bool, level: usize) {
    if entity.is_in_main_file() {
        let prefix_spaces = generate_prefix_spaces(level, 4);
        if pretty_print {
            format!("{:#?}", entity)
                .lines()
                .for_each(|line| println!("{}{}", prefix_spaces, line));
        } else {
            println!("{}{:?}", prefix_spaces, entity);
        }
        for child in entity.get_children() {
            dump_entity_with_level(&child, pretty_print, level + 1);
        }
    }
}
