use super::{
    ast::AttachFunction,
    dependency_resolver::DependenciesResolver,
    types::StructFieldType,
    utils::{filter_dependencies, insert_function_pointer_type_callback_name_map, try_to_extract_function_type},
};
use crate::ast::{FieldType, FunctionDeclaration, FunctionType, SourceFile, StructDeclaration, TypeDeclaration};
use tap::TapOps;

pub(super) fn insert_callback_declaration_bindings(
    source_file: &SourceFile,
    dependency_resolver: &mut DependenciesResolver,
) {
    source_file.type_declarations().iter().for_each(|type_declaration| {
        if let TypeDeclaration::Struct(s) = type_declaration {
            for_struct_node(s, dependency_resolver);
        }
    });
    source_file
        .function_declarations()
        .iter()
        .for_each(|function_declaration| {
            for_function_node(function_declaration, dependency_resolver);
        });
}
fn for_struct_node(struct_declaration: &StructDeclaration, dependency_resolver: &mut DependenciesResolver) {
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
            insert_callback_node(&callback_name, ft, dependency_resolver);
            insert_function_pointer_type_callback_name_map(ft, callback_name);
        });
}

fn for_function_node(function_declaration: &FunctionDeclaration, dependency_resolver: &mut DependenciesResolver) {
    function_declaration
        .parameters()
        .iter()
        .filter_map(|parameter| {
            try_to_extract_function_type(parameter.parameter_type())
                .map(|function_type| (function_declaration.name(), function_type))
        })
        .for_each(|(s, ft)| {
            let callback_name = format!("{}_{}_callback", function_declaration.name(), s);
            insert_callback_node(&callback_name, ft, dependency_resolver);
            insert_function_pointer_type_callback_name_map(ft, callback_name);
        });
}

fn insert_callback_node(
    name: impl Into<String>,
    function_type: &FunctionType,
    dependency_resolver: &mut DependenciesResolver,
) {
    let name = name.into();
    let new_callback_node = Box::new(AttachFunction::new(
        name.to_owned(),
        StructFieldType::from(function_type.return_type().to_owned(), false),
        true,
    ))
    .tap(|new_callback_node| {
        *new_callback_node.parameters_mut() = function_type
            .parameter_types()
            .iter()
            .map(|parameter_type| StructFieldType::from(parameter_type.to_owned(), false))
            .collect();
    });
    let mut dependencies = new_callback_node
        .parameters()
        .iter()
        .filter_map(filter_dependencies)
        .collect::<Vec<_>>();
    if let Some(return_type_dependency) = filter_dependencies(new_callback_node.return_value()) {
        dependencies.push(return_type_dependency);
    }

    dependency_resolver.insert(name, new_callback_node, dependencies);
}
