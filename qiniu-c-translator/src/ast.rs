#![allow(dead_code)]

use clang::{
    source::SourceLocation as ClangSourceLocation, Entity as ClangEntity, EntityKind as ClangEntityKind,
    Type as ClangType, TypeKind as ClangTypeKind,
};
use getset::{CopyGetters, Getters};
use std::fmt::Debug;
use tap::TapOps;

#[derive(Debug, Clone)]
pub enum Declaration {
    SourceFile(SourceFile),
    EnumConstant(EnumConstantDeclaration),
    Field(FieldDeclaration),
    Type(TypeDeclaration),
    Function(FunctionDeclaration),
    Parameter(ParameterDeclaration),
}

impl Declaration {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::SourceFile(source_file) => Some(source_file.path().as_str()),
            Self::EnumConstant(enum_constant) => Some(enum_constant.name().as_str()),
            Self::Field(field) => field.name.as_ref().map(|n| n.as_ref()),
            Self::Function(function) => Some(function.name().as_str()),
            Self::Parameter(parameter) => Some(parameter.name().as_str()),
            Self::Type(t) => t.name(),
        }
    }
    pub fn set_name(&mut self, new_name: String) {
        match self {
            Self::SourceFile(source_file) => source_file.path = new_name,
            Self::EnumConstant(enum_constant) => enum_constant.name = new_name,
            Self::Field(field) => field.name = Some(new_name),
            Self::Function(function) => function.name = new_name,
            Self::Parameter(parameter) => parameter.name = new_name,
            Self::Type(t) => t.set_name(new_name),
        }
    }
    pub fn location(&self) -> &SourceLocation {
        match self {
            Self::SourceFile(source_file) => source_file.location(),
            Self::EnumConstant(enum_constant) => enum_constant.location(),
            Self::Field(field) => field.location(),
            Self::Function(function) => function.location(),
            Self::Parameter(parameter) => parameter.location(),
            Self::Type(t) => t.location(),
        }
    }
    pub fn sub_declarations(&self) -> Vec<Declaration> {
        match self {
            Self::SourceFile(source_file) => source_file
                .type_declarations()
                .iter()
                .map(|t| Self::Type(t.to_owned()))
                .chain(
                    source_file
                        .function_declarations()
                        .iter()
                        .map(|f| Self::Function(f.to_owned())),
                )
                .collect(),
            Self::Function(function) => function
                .parameters()
                .iter()
                .map(|param| Self::Parameter(param.to_owned()))
                .collect(),
            Self::Type(t) => t.sub_declarations(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeDeclaration {
    Enum(EnumDeclaration),
    Struct(StructDeclaration),
}

impl TypeDeclaration {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Enum(declaration) => declaration.enum_name().as_ref().map(|s| s.as_ref()),
            Self::Struct(declaration) => declaration.struct_name().as_ref().map(|s| s.as_ref()),
        }
    }

    pub fn set_name(&mut self, new_name: String) {
        match self {
            Self::Enum(declaration) => declaration.enum_name = Some(new_name),
            Self::Struct(declaration) => declaration.struct_name = Some(new_name),
        }
    }

    pub fn location(&self) -> &SourceLocation {
        match self {
            Self::Enum(declaration) => declaration.location(),
            Self::Struct(declaration) => declaration.location(),
        }
    }

    pub fn sub_declarations(&self) -> Vec<Declaration> {
        match self {
            Self::Enum(declaration) => declaration
                .constants()
                .iter()
                .map(|constant| Declaration::EnumConstant(constant.to_owned()))
                .collect(),
            Self::Struct(declaration) => declaration
                .fields()
                .iter()
                .map(|field| Declaration::Field(field.to_owned()))
                .collect(),
        }
    }

    pub fn typedef_name(&self) -> Option<&str> {
        match self {
            Self::Enum(declaration) => declaration.typedef_name().as_ref().map(|n| n.as_ref()),
            Self::Struct(declaration) => declaration.typedef_name().as_ref().map(|n| n.as_ref()),
        }
    }
    pub fn set_typedef_name(&mut self, new_typedef_name: String) {
        match self {
            Self::Enum(declaration) => declaration.typedef_name = Some(new_typedef_name),
            Self::Struct(declaration) => declaration.typedef_name = Some(new_typedef_name),
        }
    }
}

#[derive(Debug, Clone, Getters)]
#[get = "pub"]
pub struct FunctionType {
    return_type: Type,
    parameter_types: Vec<Type>,
}

impl FunctionType {
    fn new(return_type: &ClangType, parameter_types: &[ClangType]) -> Self {
        Self {
            return_type: Type::new(return_type),
            parameter_types: parameter_types
                .iter()
                .map(|parameter_type| Type::new(parameter_type))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SubType {
    PointeeType(Type),
    FunctionType(FunctionType),
    OriginalType(Type),
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct Type {
    #[get_copy = "pub"]
    type_kind: ClangTypeKind,

    #[get = "pub"]
    display_name: String,

    #[get = "pub"]
    subtype: Option<Box<SubType>>,

    #[get_copy = "pub"]
    is_const: bool,
}

impl Type {
    fn new(clang_type: &ClangType) -> Self {
        Self {
            type_kind: clang_type.get_kind(),
            display_name: clang_type.get_display_name(),
            subtype: match clang_type.get_kind() {
                ClangTypeKind::Pointer => Some(Box::new(SubType::PointeeType(Self::new(
                    clang_type.get_pointee_type().as_ref().unwrap(),
                )))),
                ClangTypeKind::Typedef => Some(Box::new(SubType::OriginalType(Self::new(
                    &clang_type.get_canonical_type(),
                )))),
                ClangTypeKind::FunctionPrototype => Some(Box::new(SubType::FunctionType(FunctionType::new(
                    clang_type.get_result_type().as_ref().unwrap(),
                    clang_type.get_argument_types().unwrap_or_default().as_ref(),
                )))),
                _ => None,
            },
            is_const: clang_type.is_const_qualified(),
        }
    }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct SourceLocation {
    #[get = "pub"]
    path: String,

    #[get_copy = "pub"]
    line_number: u32,

    #[get_copy = "pub"]
    column_number: u32,
}

impl SourceLocation {
    fn new(loc: &ClangSourceLocation) -> Self {
        let loc = loc.get_presumed_location();
        Self {
            path: loc.0,
            line_number: loc.1,
            column_number: loc.2,
        }
    }

    fn from_path(path: String) -> Self {
        Self {
            path,
            line_number: 0,
            column_number: 0,
        }
    }
}

#[derive(Debug, Clone, Getters)]
#[get = "pub"]
pub struct SourceFile {
    path: String,
    location: SourceLocation,
    type_declarations: Vec<TypeDeclaration>,
    function_declarations: Vec<FunctionDeclaration>,
}

impl SourceFile {
    fn new(entity: &ClangEntity) -> Self {
        assert_eq!(entity.get_kind(), ClangEntityKind::TranslationUnit);
        Self {
            path: entity.get_name().unwrap(),
            type_declarations: Vec::new(),
            function_declarations: Vec::new(),
            location: SourceLocation::from_path(entity.get_name().unwrap()),
        }
        .tap(|source_file| {
            entity
                .get_children()
                .iter()
                .filter(|entity| entity.is_in_main_file())
                .for_each(|entity| match entity.get_kind() {
                    ClangEntityKind::EnumDecl => {
                        if let Some(name) = entity.get_name() {
                            source_file
                                .type_declarations
                                .push(TypeDeclaration::Enum(EnumDeclaration::new(Some(name), None, &entity)));
                        }
                    }
                    ClangEntityKind::StructDecl | ClangEntityKind::UnionDecl => {
                        if let Some(name) = entity.get_name() {
                            source_file
                                .type_declarations
                                .push(TypeDeclaration::Struct(StructDeclaration::new(
                                    Some(name),
                                    None,
                                    &entity,
                                )));
                        }
                    }
                    ClangEntityKind::TypedefDecl => {
                        if let Some(declaration_entity) =
                            entity.get_typedef_underlying_type().and_then(|t| t.get_declaration())
                        {
                            let typedef_name = entity.get_name().unwrap();
                            let declaration_entity_name = declaration_entity.get_name();
                            if let Some(type_declaration) =
                                source_file.type_declarations.iter_mut().find(|type_declaration| {
                                    match (type_declaration.name(), &declaration_entity_name) {
                                        (Some(type_declaration_name), Some(declaration_entity_name)) => {
                                            type_declaration_name == declaration_entity_name
                                        }
                                        _ => false,
                                    }
                                })
                            {
                                type_declaration.set_typedef_name(typedef_name);
                            } else {
                                source_file.type_declarations.push(match declaration_entity.get_kind() {
                                    ClangEntityKind::EnumDecl => TypeDeclaration::Enum(EnumDeclaration::new(
                                        None,
                                        Some(typedef_name),
                                        &declaration_entity,
                                    )),
                                    ClangEntityKind::StructDecl | ClangEntityKind::UnionDecl => {
                                        TypeDeclaration::Struct(StructDeclaration::new(
                                            None,
                                            Some(typedef_name),
                                            &declaration_entity,
                                        ))
                                    }
                                    _ => panic!("Unexpected typedef declaration entity: {:?}", declaration_entity),
                                });
                            }
                        }
                    }
                    ClangEntityKind::FunctionDecl => {
                        source_file
                            .function_declarations
                            .push(FunctionDeclaration::new(&entity));
                    }
                    _ => panic!("Unexpected entity: {:?}", entity),
                });
        })
    }

    pub fn parse(entity: &ClangEntity) -> Self {
        Self::new(entity)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum EnumConstantValue {
    Signed(i64),
    Unsigned(u64),
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct EnumConstantDeclaration {
    #[get = "pub"]
    name: String,

    #[get = "pub"]
    location: SourceLocation,

    #[get_copy = "pub"]
    constant_value: EnumConstantValue,
}

impl EnumConstantDeclaration {
    fn new(entity: &ClangEntity) -> Self {
        assert_eq!(entity.get_kind(), ClangEntityKind::EnumConstantDecl);
        let values = entity.get_enum_constant_value().unwrap();
        Self {
            name: entity.get_name().unwrap(),
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
            constant_value: if entity
                .get_semantic_parent()
                .unwrap()
                .get_enum_underlying_type()
                .unwrap()
                .is_signed_integer()
            {
                EnumConstantValue::Signed(values.0)
            } else {
                EnumConstantValue::Unsigned(values.1)
            },
        }
    }
}

#[derive(Debug, Clone, Getters)]
#[get = "pub"]
pub struct EnumDeclaration {
    enum_name: Option<String>,
    typedef_name: Option<String>,
    location: SourceLocation,
    constants: Vec<EnumConstantDeclaration>,
    enum_type: Type,
}

impl EnumDeclaration {
    fn new(enum_name: Option<String>, typedef_name: Option<String>, entity: &ClangEntity) -> Self {
        assert_eq!(entity.get_kind(), ClangEntityKind::EnumDecl);
        Self {
            enum_name,
            typedef_name,
            constants: entity
                .get_children()
                .iter()
                .map(|entity| EnumConstantDeclaration::new(entity))
                .collect(),
            enum_type: Type::new(entity.get_enum_underlying_type().as_ref().unwrap()),
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldType {
    NamedType(Type),
    AnonymousType(StructDeclaration),
}

#[derive(Debug, Clone, Getters)]
#[get = "pub"]
pub struct FieldDeclaration {
    name: Option<String>,
    location: SourceLocation,
    field_type: FieldType,
}

impl FieldDeclaration {
    fn new_with_named_type(entity: &ClangEntity) -> Self {
        assert_eq!(entity.get_kind(), ClangEntityKind::FieldDecl);
        Self {
            name: entity.get_name(),
            field_type: FieldType::NamedType(Type::new(entity.get_type().as_ref().unwrap())),
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
        }
    }

    fn new_with_anonymous_type(entity: &ClangEntity) -> Self {
        assert!([ClangEntityKind::StructDecl, ClangEntityKind::UnionDecl].contains(&entity.get_kind()));
        Self {
            name: entity.get_name(),
            field_type: FieldType::AnonymousType(StructDeclaration::new(None, None, entity)),
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
        }
    }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct StructDeclaration {
    #[get = "pub"]
    struct_name: Option<String>,

    #[get = "pub"]
    typedef_name: Option<String>,

    #[get = "pub"]
    location: SourceLocation,

    #[get = "pub"]
    fields: Vec<FieldDeclaration>,

    #[get_copy = "pub"]
    is_union: bool,
}

impl StructDeclaration {
    fn new(struct_name: Option<String>, typedef_name: Option<String>, entity: &ClangEntity) -> Self {
        assert!([ClangEntityKind::StructDecl, ClangEntityKind::UnionDecl].contains(&entity.get_kind()));
        Self {
            struct_name,
            typedef_name,
            fields: entity
                .get_children()
                .iter()
                .map(|entity| match entity.get_kind() {
                    ClangEntityKind::FieldDecl => FieldDeclaration::new_with_named_type(entity),
                    ClangEntityKind::StructDecl | ClangEntityKind::UnionDecl => {
                        FieldDeclaration::new_with_anonymous_type(entity)
                    }
                    _ => panic!("Unexpected entity in struct: {:?}", entity),
                })
                .collect(),
            is_union: entity.get_kind() == ClangEntityKind::UnionDecl,
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
        }
    }
}

#[derive(Debug, Clone, Getters)]
#[get = "pub"]
pub struct FunctionDeclaration {
    name: String,
    location: SourceLocation,
    return_type: Type,
    parameters: Vec<ParameterDeclaration>,
}

impl FunctionDeclaration {
    fn new(entity: &ClangEntity) -> Self {
        assert_eq!(entity.get_kind(), ClangEntityKind::FunctionDecl);
        Self {
            parameters: entity
                .get_arguments()
                .unwrap_or_default()
                .iter()
                .map(|param_entity| ParameterDeclaration::new(&param_entity))
                .collect(),
            name: entity.get_name().unwrap(),
            return_type: Type::new(entity.get_result_type().as_ref().unwrap()),
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
        }
    }
}

#[derive(Debug, Clone, Getters)]
#[get = "pub"]
pub struct ParameterDeclaration {
    name: String,
    location: SourceLocation,
    parameter_type: Type,
}

impl ParameterDeclaration {
    fn new(entity: &ClangEntity) -> Self {
        assert_eq!(entity.get_kind(), ClangEntityKind::ParmDecl);
        Self {
            name: entity.get_name().unwrap(),
            parameter_type: Type::new(entity.get_type().as_ref().unwrap()),
            location: SourceLocation::new(entity.get_location().as_ref().unwrap()),
        }
    }
}

pub fn dump_ast(entity: &ClangEntity, pretty_print: bool) {
    let source_file = SourceFile::parse(&entity);
    if pretty_print {
        println!("{:#?}", source_file);
    } else {
        println!("{:?}", source_file);
    }
}
