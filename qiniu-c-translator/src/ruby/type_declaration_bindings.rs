use super::{
    ast::{Enum, EnumValue, Struct, StructField},
    dependency_resolver::DependenciesResolver,
    types::{BaseType, StructFieldType},
    utils::{
        filter_dependencies, get_random_constant_identifier, get_random_field_identifier, insert_type_constants,
        normalize_constant, ConstantType,
    },
};
use crate::ast::{EnumConstantValue, EnumDeclaration, FieldType, SourceFile, StructDeclaration, TypeDeclaration};
use tap::TapOps;

pub(super) fn insert_type_declaration_bindings(
    source_file: &SourceFile,
    dependency_resolver: &mut DependenciesResolver,
) {
    insert_predefined_struct_nodes(dependency_resolver);

    source_file.type_declarations().iter().for_each(|type_declaration| {
        match type_declaration {
            TypeDeclaration::Enum(e) => insert_enum_node(e, dependency_resolver),
            TypeDeclaration::Struct(s) => insert_struct_node(s, dependency_resolver),
        };
    });
}

fn insert_predefined_struct_nodes(dependency_resolver: &mut DependenciesResolver) {
    dependency_resolver.insert(
        "In6Addr".into(),
        Box::new(Struct::new("In6Addr", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![
                StructField::new("in6_addr_0", StructFieldType::BaseType(BaseType::U64)),
                StructField::new("in6_addr_1", StructFieldType::BaseType(BaseType::U64)),
            ]
        })),
        vec![],
    );

    dependency_resolver.insert(
        "I8".into(),
        Box::new(Struct::new("I8", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I8))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "U8".into(),
        Box::new(Struct::new("U8", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U8))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "I16".into(),
        Box::new(Struct::new("I16", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I16))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "U16".into(),
        Box::new(Struct::new("U16", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U16))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "I32".into(),
        Box::new(Struct::new("I32", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I32))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "U32".into(),
        Box::new(Struct::new("U32", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U32))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "I64".into(),
        Box::new(Struct::new("I64", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I64))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "U64".into(),
        Box::new(Struct::new("U64", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U64))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "Size".into(),
        Box::new(Struct::new("Size", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::Size))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "Ssize".into(),
        Box::new(Struct::new("Ssize", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::Ssize))]
        })),
        vec![],
    );
    dependency_resolver.insert(
        "Pointer".into(),
        Box::new(Struct::new("Pointer", false).tap(|struct_node| {
            *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::Pointer))]
        })),
        vec![],
    );
}

fn insert_enum_node(enum_declaration: &EnumDeclaration, dependency_resolver: &mut DependenciesResolver) -> String {
    let enum_inner = Enum::new(
        enum_declaration
            .typedef_name()
            .as_ref()
            .or_else(|| enum_declaration.enum_name().as_ref())
            .map(normalize_constant)
            .unwrap_or_else(get_random_constant_identifier)
            .tap(|constant| {
                insert_type_constants(constant.as_str(), ConstantType::Enum);
            }),
    );
    let enum_struct_wrapper =
        Box::new(Struct::new(format!("{}Wrapper", enum_inner.name()), false)).tap(|struct_node| {
            struct_node.fields_mut().push(StructField::new(
                "inner",
                StructFieldType::Plain(enum_inner.name().to_owned()),
            ));
        });

    let enum_node = Box::new(enum_inner).tap(|enum_node| {
        *enum_node.constants_mut() = enum_declaration
            .constants()
            .iter()
            .map(|constant| {
                let value = match constant.constant_value() {
                    EnumConstantValue::Signed(num) => num.to_string(),
                    EnumConstantValue::Unsigned(num) => num.to_string(),
                };
                EnumValue::new(constant.name(), value)
            })
            .collect();
    });
    let enum_name = enum_node.name().to_owned();
    dependency_resolver.insert(enum_name.to_owned(), enum_node, vec![]);
    dependency_resolver.insert(
        enum_struct_wrapper.name().to_owned(),
        enum_struct_wrapper,
        vec![enum_name.to_owned()],
    );
    enum_name
}

fn insert_struct_node(
    struct_declaration: &StructDeclaration,
    dependency_resolver: &mut DependenciesResolver,
) -> String {
    let struct_node = Box::new(
        Struct::new(
            struct_declaration
                .typedef_name()
                .as_ref()
                .or_else(|| struct_declaration.struct_name().as_ref())
                .map(normalize_constant)
                .unwrap_or_else(get_random_constant_identifier)
                .tap(|constant| {
                    insert_type_constants(constant.as_str(), ConstantType::Struct);
                }),
            struct_declaration.is_union(),
        )
        .tap(|struct_node| {
            *struct_node.fields_mut() = struct_declaration
                .fields()
                .iter()
                .map(|field| {
                    let field_name = field
                        .name()
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(get_random_field_identifier);
                    match field.field_type() {
                        FieldType::NamedType(t) => {
                            let struct_field_type = StructFieldType::from(t.to_owned(), true);
                            StructField::new(field_name, struct_field_type)
                        }
                        FieldType::AnonymousType(anon_struct_declaration) => {
                            let anon_struct_name = insert_struct_node(anon_struct_declaration, dependency_resolver);
                            StructField::new(field_name, StructFieldType::new_type_by_val(anon_struct_name))
                        }
                    }
                })
                .collect();
        }),
    );
    let dependencies = struct_node
        .fields()
        .iter()
        .filter_map(|field| filter_dependencies(field.field_type()))
        .collect::<Vec<_>>();
    let struct_name = struct_node.name().to_owned();
    dependency_resolver.insert(struct_name.to_owned(), struct_node, dependencies);
    struct_name
}
