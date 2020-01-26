use crate::utils::{CodeWriter, Writer};
use clang::Entity;
use std::io::{stdout, Result};

pub fn dump_entity(entity: &Entity, pretty_print: bool) -> Result<()> {
    dump_entity_with_level(entity, pretty_print, CodeWriter::new(Writer::Stdout(stdout()), 4, 0))?;
    Ok(())
}

fn dump_entity_with_level(entity: &Entity, pretty_print: bool, mut output: CodeWriter) -> Result<CodeWriter> {
    if entity.is_in_main_file() {
        {
            let dumped = if pretty_print {
                format!("{:#?}", entity)
            } else {
                format!("{:?}", entity)
            };
            output.write(&dumped)?;
        }
        output = output.try_with_next_level(|mut output| {
            for child in entity.get_children() {
                output = dump_entity_with_level(&child, pretty_print, output)?;
            }
            Ok(output)
        })?;
    }
    Ok(output)
}
