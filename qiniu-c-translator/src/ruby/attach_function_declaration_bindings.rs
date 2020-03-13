use super::{
    ast::AttachFunction, dependency_resolver::DependenciesResolver, types::StructFieldType, utils::filter_dependencies,
};
use crate::ast::{FunctionDeclaration, SourceFile};
use tap::TapOps;

pub(super) fn insert_attach_function_declaration_bindings(
    source_file: &SourceFile,
    dependency_resolver: &mut DependenciesResolver,
) {
    source_file
        .function_declarations()
        .iter()
        .for_each(|function_declaration| {
            insert_attach_function_node(function_declaration, dependency_resolver);
        });
}

fn insert_attach_function_node(
    function_declaration: &FunctionDeclaration,
    dependency_resolver: &mut DependenciesResolver,
) -> String {
    let function_node = Box::new(AttachFunction::new(
        function_declaration.name(),
        StructFieldType::from(function_declaration.return_type().to_owned(), false),
        false,
    ))
    .tap(|function_node| {
        *function_node.parameters_mut() = function_declaration
            .parameters()
            .iter()
            .map(|parameter| StructFieldType::from(parameter.parameter_type().to_owned(), false))
            .collect();
    });
    let function_name = function_node.name().to_owned();
    let mut dependencies = function_node
        .parameters()
        .iter()
        .filter_map(filter_dependencies)
        .collect::<Vec<_>>();
    if let Some(return_type_dependency) = filter_dependencies(function_node.return_value()) {
        dependencies.push(return_type_dependency);
    }
    dependency_resolver.insert(function_name.to_owned(), function_node, dependencies);
    function_name
}
