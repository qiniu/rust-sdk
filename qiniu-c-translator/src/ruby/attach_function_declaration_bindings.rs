use super::{
    ast::{AttachFunction, Module},
    types::{BaseType, StructFieldType},
};
use crate::{
    ast::{FunctionDeclaration, SourceFile},
    utils::CodeGenerator,
};
use tap::TapOps;

pub(super) fn insert_attach_function_declaration_bindings(source_file: &SourceFile, module: &mut Module) {
    insert_predefined_attach_function_nodes(module.sub_nodes_mut());

    source_file
        .function_declarations()
        .iter()
        .for_each(|function_declaration| {
            insert_attach_function_node(function_declaration, module.sub_nodes_mut());
        });
}

fn insert_predefined_attach_function_nodes(nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    nodes.push(
        Box::new(AttachFunction::new(
            "fdopen",
            StructFieldType::BaseType(BaseType::Pointer),
            false,
        ))
        .tap(|attach_function| {
            *attach_function.parameters_mut() = vec![
                StructFieldType::BaseType(BaseType::I32),
                StructFieldType::BaseType(BaseType::String),
            ]
        }),
    )
}

fn insert_attach_function_node(
    function_declaration: &FunctionDeclaration,
    nodes: &mut Vec<Box<dyn CodeGenerator>>,
) -> String {
    let function_node = Box::new(AttachFunction::new(
        function_declaration.name(),
        StructFieldType::from(function_declaration.return_type().to_owned(), false),
        false,
    ))
    .tap(|attach_function| {
        *attach_function.parameters_mut() = function_declaration
            .parameters()
            .iter()
            .map(|parameter| StructFieldType::from(parameter.parameter_type().to_owned(), false))
            .collect();
    });
    function_node.name().to_owned().tap(|_| {
        nodes.push(function_node);
    })
}
