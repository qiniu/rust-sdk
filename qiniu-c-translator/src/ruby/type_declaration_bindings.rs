use super::{
    ast::{Enum, EnumValue, Module, Struct, StructField},
    types::{BaseType, StructFieldType},
    utils::{
        get_random_constant_identifier, get_random_field_identifier, insert_type_constants, normalize_constant,
        ConstantType,
    },
};
use crate::{
    ast::{EnumConstantValue, EnumDeclaration, FieldType, SourceFile, StructDeclaration, TypeDeclaration},
    utils::CodeGenerator,
};
use tap::TapOps;

pub(super) fn insert_type_declaration_bindings(source_file: &SourceFile, module: &mut Module) {
    insert_predefined_struct_nodes(module.sub_nodes_mut());

    source_file
        .type_declarations()
        .iter()
        .for_each(|type_declaration| match type_declaration {
            TypeDeclaration::Enum(e) => {
                insert_enum_node(e, module.sub_nodes_mut());
            }
            TypeDeclaration::Struct(s) => {
                insert_struct_node(s, module.sub_nodes_mut());
            }
        });
}

fn insert_predefined_struct_nodes(nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    nodes.push(Box::new(Struct::new("In6Addr", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![
            StructField::new("in6_addr_0", StructFieldType::BaseType(BaseType::U64)),
            StructField::new("in6_addr_1", StructFieldType::BaseType(BaseType::U64)),
        ]
    })));
    insert_type_constants("In6Addr", ConstantType::Struct);

    nodes.push(Box::new(Struct::new("I8", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I8))]
    })));
    nodes.push(Box::new(Struct::new("U8", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U8))]
    })));
    nodes.push(Box::new(Struct::new("I16", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I16))]
    })));
    nodes.push(Box::new(Struct::new("U16", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U16))]
    })));
    nodes.push(Box::new(Struct::new("I32", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I32))]
    })));
    nodes.push(Box::new(Struct::new("U32", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U32))]
    })));
    nodes.push(Box::new(Struct::new("I64", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::I64))]
    })));
    nodes.push(Box::new(Struct::new("U64", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::U64))]
    })));
    nodes.push(Box::new(Struct::new("Size", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::Size))]
    })));
    nodes.push(Box::new(Struct::new("Ssize", false).tap(|struct_node| {
        *struct_node.fields_mut() = vec![StructField::new("value", StructFieldType::BaseType(BaseType::Ssize))]
    })));
}

fn insert_enum_node(enum_declaration: &EnumDeclaration, nodes: &mut Vec<Box<dyn CodeGenerator>>) -> String {
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
    enum_node.name().to_owned().tap(|_| {
        nodes.push(enum_node);
        nodes.push(enum_struct_wrapper);
    })
}

fn insert_struct_node(struct_declaration: &StructDeclaration, nodes: &mut Vec<Box<dyn CodeGenerator>>) -> String {
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
                            StructField::new(field_name, StructFieldType::from(t.to_owned(), true))
                        }
                        FieldType::AnonymousType(anon_struct_declaration) => {
                            let anon_struct_name = insert_struct_node(anon_struct_declaration, nodes);
                            StructField::new(field_name, StructFieldType::new_type_by_val(anon_struct_name))
                        }
                    }
                })
                .collect();
        }),
    );
    struct_node.name().to_owned().tap(|_| {
        nodes.push(struct_node);
    })
}
