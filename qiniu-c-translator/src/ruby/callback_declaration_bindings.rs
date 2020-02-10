use super::{
    ast::{AttachFunction, Module},
    types::StructFieldType,
    utils::{insert_function_pointer_type_callback_name_map, try_to_extract_function_type},
};
use crate::{
    ast::{FieldType, FunctionDeclaration, FunctionType, SourceFile, StructDeclaration, TypeDeclaration},
    utils::CodeGenerator,
};
use tap::TapOps;

pub(super) enum For {
    Structs,
    Functions,
}

pub(super) fn insert_callback_declaration_bindings(source_file: &SourceFile, module: &mut Module, callback_for: For) {
    match callback_for {
        For::Structs => {
            source_file.type_declarations().iter().for_each(|type_declaration| {
                if let TypeDeclaration::Struct(s) = type_declaration {
                    for_struct_node(s, module.sub_nodes_mut());
                }
            });
        }
        For::Functions => {
            source_file
                .function_declarations()
                .iter()
                .for_each(|function_declaration| {
                    for_function_node(function_declaration, module.sub_nodes_mut());
                });
        }
    }
}
fn for_struct_node(struct_declaration: &StructDeclaration, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    struct_declaration
        .fields()
        .iter()
        .filter_map(|field| {
            if let FieldType::NamedType(named_type) = field.field_type() {
                try_to_extract_function_type(&named_type).map(|function_type| (field.name(), function_type))
            } else {
                None
            }
        })
        .for_each(|(s, ft)| {
            let callback_name = format!(
                "{}_{}_callback",
                struct_declaration
                    .struct_name()
                    .as_ref()
                    .or_else(|| struct_declaration.typedef_name().as_ref())
                    .unwrap(),
                s.as_ref().unwrap(),
            );
            insert_callback_node(&callback_name, ft, nodes);
            insert_function_pointer_type_callback_name_map(ft, callback_name);
        });
}

fn for_function_node(function_declaration: &FunctionDeclaration, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    function_declaration
        .parameters()
        .iter()
        .filter_map(|parameter| {
            try_to_extract_function_type(parameter.parameter_type())
                .map(|function_type| (function_declaration.name(), function_type))
        })
        .for_each(|(s, ft)| {
            let callback_name = format!("{}_{}_callback", function_declaration.name(), s);
            insert_callback_node(&callback_name, ft, nodes);
            insert_function_pointer_type_callback_name_map(ft, callback_name);
        });
}

fn insert_callback_node(
    name: impl Into<String>,
    function_type: &FunctionType,
    nodes: &mut Vec<Box<dyn CodeGenerator>>,
) {
    nodes.push(
        Box::new(AttachFunction::new(
            name,
            StructFieldType::from(function_type.return_type().to_owned(), false),
            true,
        ))
        .tap(|attach_function| {
            *attach_function.parameters_mut() = function_type
                .parameter_types()
                .iter()
                .map(|parameter_type| StructFieldType::from(parameter_type.to_owned(), false))
                .collect();
        }),
    );
}
