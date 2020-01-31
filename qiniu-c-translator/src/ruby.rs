use crate::{
    ast::{
        EnumConstantValue, EnumDeclaration, FieldType, FunctionDeclaration, SourceFile, StructDeclaration, SubType,
        Type, TypeDeclaration,
    },
    utils::{CodeGenerator, CodeWriter, RandomIdentifier, Writer},
};
use clang::{Entity as ClangEntity, TypeKind as ClangTypeKind};
use heck::CamelCase;
use lazy_static::lazy_static;
use std::{
    collections::HashSet,
    fmt,
    io::{Result, Write},
    process::{exit, Command, Stdio},
    sync::Mutex,
};
use tap::TapOps;

lazy_static! {
    static ref TYPE_CONSTANTS: Mutex<HashSet<String>> = Default::default();
    static ref IDENTIFIER_GENERATOR: RandomIdentifier = Default::default();
}

#[derive(Default)]
struct TopLevelNode {
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl CodeGenerator for TopLevelNode {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        for node in self.sub_nodes.iter() {
            output = node.generate_code(output)?;
        }
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        self.sub_nodes.iter().map(|node| node.as_ref()).collect::<Vec<_>>()
    }
}

#[derive(Default)]
struct RawCode {
    code: String,
}

impl RawCode {
    fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

impl CodeGenerator for RawCode {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&self.code)?;
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        Vec::new()
    }
}

#[derive(Default)]
struct Module {
    module_name: String,
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl Module {
    fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            ..Default::default()
        }
    }
}

impl CodeGenerator for Module {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!("module {}", self.module_name))?;
        output = output.try_with_next_level(|mut output| {
            for node in self.sub_nodes.iter() {
                output = node.generate_code(output)?;
            }
            Ok(output)
        })?;
        output.write("end")?;
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        self.sub_nodes.iter().map(|node| node.as_ref()).collect::<Vec<_>>()
    }
}

#[derive(Default)]
struct EnumValue {
    name: String,
    value: String,
}

#[derive(Default)]
struct Enum {
    enum_name: String,
    enum_constants: Vec<EnumValue>,
}

impl Enum {
    fn new(name: impl Into<String>) -> Self {
        Self {
            enum_name: name.into(),
            ..Default::default()
        }
    }
}

impl CodeGenerator for Enum {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!("{} = enum(", self.enum_name))?;
        output = output.try_with_next_level(|mut output| {
            for enum_value in self.enum_constants.iter() {
                output.write(&format!(":{}, {},", enum_value.name, enum_value.value))?;
            }
            Ok(output)
        })?;
        output.write(")")?;
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        Vec::new()
    }
}

#[derive(Debug, Copy, Clone)]
enum BaseType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Size,
    Usize,
    F32,
    F64,
    Ldouble,
    Pointer,
    Bool,
    Void,
    String,
    Char,
    Uchar,
    InAddr,
}

impl BaseType {
    fn from(c_type: Type) -> Self {
        match c_type.type_kind() {
            ClangTypeKind::Void => Self::Void,
            ClangTypeKind::Bool => Self::Bool,
            ClangTypeKind::CharS | ClangTypeKind::SChar => Self::Char,
            ClangTypeKind::CharU | ClangTypeKind::UChar => Self::Uchar,
            ClangTypeKind::UInt => Self::U32,
            ClangTypeKind::Int => Self::I32,
            ClangTypeKind::Long => Self::Size,
            ClangTypeKind::LongLong => Self::I64,
            ClangTypeKind::ULong => Self::Usize,
            ClangTypeKind::ULongLong => Self::U64,
            ClangTypeKind::Pointer => match c_type.display_name().as_str() {
                "const char *" | "char *" => Self::String,
                _ => Self::Pointer,
            },
            ClangTypeKind::Elaborated => match c_type.display_name().as_str() {
                "struct in_addr" => Self::InAddr,
                _ => panic!("Unrecognized elaborated type: {:?}", c_type),
            },
            _ => panic!("Unrecognized type: {:?}", c_type),
        }
    }

    fn to_symbol(self) -> &'static str {
        match self {
            Self::I8 => ":int8",
            Self::U8 => ":uint8",
            Self::I16 => ":int16",
            Self::U16 => ":uint16",
            Self::I32 => ":int32",
            Self::U32 => ":uint32",
            Self::I64 => ":int64",
            Self::U64 => ":uint64",
            Self::Size => ":long",
            Self::Usize => ":ulong",
            Self::F32 => ":float",
            Self::F64 => ":double",
            Self::Ldouble => ":long_double",
            Self::Pointer => ":pointer",
            Self::Bool => ":bool",
            Self::Void => ":void",
            Self::String => ":string",
            Self::Char => ":char",
            Self::Uchar => ":uchar",
            Self::InAddr => ":in_addr_t",
        }
    }
}

enum StructFieldType {
    BaseType(BaseType),
    ByVal(String),
    ByPtr(String),
}

impl StructFieldType {
    fn from(c_type: Type) -> Self {
        match c_type.type_kind() {
            ClangTypeKind::Typedef => Self::new_type_by_val(normalize_constant(c_type.display_name())),
            ClangTypeKind::Pointer => {
                if let Some(subtype) = c_type.subtype().as_ref() {
                    if let SubType::PointeeType(pointer_type) = subtype.as_ref() {
                        if pointer_type.type_kind() == ClangTypeKind::Typedef {
                            return Self::new_type_by_ptr(normalize_constant(pointer_type.display_name()));
                        }
                    }
                }
                Self::new_base_type(c_type)
            }
            _ => Self::new_base_type(c_type),
        }
    }

    fn new_base_type(c_type: Type) -> Self {
        if c_type.type_kind() == ClangTypeKind::Elaborated {
            match c_type.display_name().as_str() {
                "struct in_addr" => {
                    return Self::BaseType(BaseType::InAddr);
                }
                "struct in6_addr" => {
                    return Self::new_type_by_val("In6Addr".to_owned());
                }
                _ => {}
            }
        }
        Self::BaseType(BaseType::from(c_type))
    }

    fn new_type_by_val(t: String) -> Self {
        if TYPE_CONSTANTS.lock().unwrap().get(&t).is_none() {
            return match t.as_str() {
                "Int8T" => Self::BaseType(BaseType::I8),
                "Int16T" => Self::BaseType(BaseType::I16),
                "Int32T" => Self::BaseType(BaseType::I32),
                "Int64T" => Self::BaseType(BaseType::I64),
                "Uint8T" => Self::BaseType(BaseType::U8),
                "Uint16T" => Self::BaseType(BaseType::U16),
                "Uint32T" => Self::BaseType(BaseType::U32),
                "Uint64T" => Self::BaseType(BaseType::U64),
                "SizeT" => Self::BaseType(BaseType::Size),
                "UsizeT" => Self::BaseType(BaseType::Usize),
                "CurLcode" => Self::BaseType(BaseType::Usize),
                _ => panic!("Unrecognized base type: {}", t),
            };
        }
        Self::ByVal(t)
    }

    #[inline]
    fn new_type_by_ptr(t: String) -> Self {
        Self::ByPtr(t)
    }
}

impl fmt::Display for StructFieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StructFieldType::BaseType(type_name) => type_name.to_symbol().fmt(f),
            StructFieldType::ByVal(type_name) => type_name.fmt(f),
            StructFieldType::ByPtr(_) => Self::BaseType(BaseType::Pointer).fmt(f),
        }
    }
}

impl Default for StructFieldType {
    #[inline]
    fn default() -> Self {
        Self::BaseType(BaseType::Void)
    }
}

struct StructField {
    name: String,
    field_type: StructFieldType,
}

#[derive(Default)]
struct Struct {
    struct_name: String,
    fields: Vec<StructField>,
    is_union: bool,
}

impl Struct {
    fn new(name: impl Into<String>, is_union: bool) -> Self {
        Self {
            struct_name: name.into(),
            is_union,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Struct {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!(
            "class {} < FFI::{}",
            self.struct_name,
            if self.is_union { "Union" } else { "Struct" }
        ))?;
        output = output.try_with_next_level(|mut output| {
            for (idx, field) in self.fields.iter().enumerate() {
                output.write(&format!(
                    "{}:{}, {}{}",
                    if idx == 0 { "layout " } else { "       " },
                    field.name,
                    field.field_type,
                    if idx == self.fields.len() - 1 { "" } else { "," },
                ))?;
            }
            Ok(output)
        })?;
        output.write("end")?;
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        Vec::new()
    }
}

#[derive(Default)]
struct Function {
    function_name: String,
    parameters: Vec<StructFieldType>,
    return_value: StructFieldType,
}

impl Function {
    fn new(name: impl Into<String>, return_value: StructFieldType) -> Self {
        Self {
            function_name: name.into(),
            return_value,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Function {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!(
            "attach_function :{}, [{}], {}",
            self.function_name,
            self.parameters
                .iter()
                .map(|parameter| parameter.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.return_value
        ))?;
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        Vec::new()
    }
}

#[derive(Default)]
pub struct GenerateBindings {
    module_names: Vec<String>,
    version_constant: String,
}

impl GenerateBindings {
    pub fn module_names(mut self, module_names: impl AsRef<[String]>) -> Self {
        self.module_names = module_names.as_ref().to_vec();
        self
    }

    pub fn version_constant(mut self, version_constant: impl Into<String>) -> Self {
        self.version_constant = version_constant.into();
        self
    }

    pub fn build(self, entity: &ClangEntity, output: &mut dyn Write) -> Result<()> {
        let mut output_buf = self.build_without_syntax_check(entity, Writer::Memory(Vec::new()))?;
        match &mut output_buf {
            Writer::Memory(output_buf) => {
                output.write_all(output_buf)?;
                self.check_syntax(output_buf)?;
            }
            _ => panic!("Unexpected output_buf: {:?}", output_buf),
        }
        Ok(())
    }

    fn check_syntax(&self, input: &[u8]) -> Result<()> {
        let mut process = Command::new("ruby")
            .arg("-wc")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;
        process.stdin.as_mut().unwrap().write_all(input)?;
        let status_code = process.wait()?;
        if !status_code.success() {
            exit(status_code.code().unwrap_or(i32::max_value()));
        }
        Ok(())
    }

    fn build_without_syntax_check(&self, entity: &ClangEntity, output: Writer) -> Result<Writer> {
        let source_file = SourceFile::parse(&entity);
        let mut top_level_node = TopLevelNode::default();
        top_level_node.sub_nodes.push(Box::new(RawCode::new("require 'ffi'")));

        if let Some(top_level_module) = self.module_names.iter().rev().fold(None, |module, module_name| {
            Some(Box::new(Module::new(module_name).tap(|m| {
                if let Some(module) = module {
                    m.sub_nodes = vec![module];
                } else {
                    let mut core_ffi_module = Module::new("CoreFFI");
                    self.insert_ffi_bindings(&mut core_ffi_module);
                    self.insert_type_declaration_bindings(&source_file, &mut core_ffi_module);
                    self.insert_function_declaration_bindings(&source_file, &mut core_ffi_module);
                    m.sub_nodes = vec![
                        Box::new(core_ffi_module),
                        Box::new(RawCode::new("private_constant :CoreFFI")),
                    ];
                }
            })) as Box<dyn CodeGenerator>)
        }) {
            top_level_node.sub_nodes.push(top_level_module);
        }

        Ok(top_level_node
            .generate_code(CodeWriter::new(output, 2, 0))?
            .into_inner())
    }

    fn insert_ffi_bindings(&self, module: &mut Module) {
        module.sub_nodes.push(Box::new(RawCode::new("extend FFI::Library")));
        module.sub_nodes.push(Box::new(RawCode::new(
            "DEFAULT_TARGET_DIR = ".to_owned()
                + "File.expand_path(File.join('..', '..', '..', '..', 'target'), __dir__)",
        )));
        module
            .sub_nodes
            .push(Box::new(RawCode::new("private_constant :DEFAULT_TARGET_DIR")));
        module.sub_nodes.push(Box::new(RawCode::new(
            "ffi_lib [".to_owned()
                + &format!("\"qiniu_ng_c-#{{{}}}\", ", self.version_constant)
                + "'qiniu_ng_c', "
                + &format!("File.expand_path(File.join(DEFAULT_TARGET_DIR, 'release', \"#{{FFI::Platform::LIBPREFIX}}qiniu_ng_c-#{{{}}}.#{{FFI::Platform::LIBSUFFIX}}\"), __dir__), ", self.version_constant)
                + "File.expand_path(File.join(DEFAULT_TARGET_DIR, 'release', \"#{FFI::Platform::LIBPREFIX}qiniu_ng_c.#{FFI::Platform::LIBSUFFIX}\"), __dir__),"
                + &format!("File.expand_path(File.join(DEFAULT_TARGET_DIR, 'debug', \"#{{FFI::Platform::LIBPREFIX}}qiniu_ng_c-#{{{}}}.#{{FFI::Platform::LIBSUFFIX}}\"), __dir__), ", self.version_constant)
                + "File.expand_path(File.join(DEFAULT_TARGET_DIR, 'debug', \"#{FFI::Platform::LIBPREFIX}qiniu_ng_c.#{FFI::Platform::LIBSUFFIX}\"), __dir__)"
                + "]"
        )));
    }

    fn insert_type_declaration_bindings(&self, source_file: &SourceFile, module: &mut Module) {
        insert_predefined_struct_nodes(&mut module.sub_nodes);

        source_file
            .type_declarations()
            .iter()
            .for_each(|type_declaration| match type_declaration {
                TypeDeclaration::Enum(e) => {
                    insert_enum_node(e, &mut module.sub_nodes);
                }
                TypeDeclaration::Struct(s) => {
                    insert_struct_node(s, &mut module.sub_nodes);
                }
            });

        fn insert_predefined_struct_nodes(nodes: &mut Vec<Box<dyn CodeGenerator>>) {
            nodes.push(Box::new(Struct::new("In6Addr", false).tap(|struct_node| {
                struct_node.fields = vec![
                    StructField {
                        name: "in6_addr_0".into(),
                        field_type: StructFieldType::BaseType(BaseType::U64),
                    },
                    StructField {
                        name: "in6_addr_1".into(),
                        field_type: StructFieldType::BaseType(BaseType::U64),
                    },
                ]
            })));
            TYPE_CONSTANTS.lock().unwrap().insert("In6Addr".to_owned());
        }

        fn insert_enum_node(enum_declaration: &EnumDeclaration, nodes: &mut Vec<Box<dyn CodeGenerator>>) -> String {
            let enum_node = Box::new(
                Enum::new(
                    enum_declaration
                        .typedef_name()
                        .as_ref()
                        .or_else(|| enum_declaration.enum_name().as_ref())
                        .map(normalize_constant)
                        .unwrap_or_else(|| IDENTIFIER_GENERATOR.upper_camel_case())
                        .tap(|constant| {
                            TYPE_CONSTANTS.lock().unwrap().insert(constant.to_owned());
                        }),
                )
                .tap(|enum_node| {
                    enum_node.enum_constants = enum_declaration
                        .constants()
                        .iter()
                        .map(|constant| {
                            let value: String = match constant.constant_value() {
                                EnumConstantValue::Signed(num) => num.to_string(),
                                EnumConstantValue::Unsigned(num) => num.to_string(),
                            };
                            EnumValue {
                                name: constant.name().to_owned(),
                                value,
                            }
                        })
                        .collect();
                }),
            );
            enum_node.enum_name.to_owned().tap(|_| {
                nodes.push(enum_node);
            })
        }

        fn insert_struct_node(
            struct_declaration: &StructDeclaration,
            nodes: &mut Vec<Box<dyn CodeGenerator>>,
        ) -> String {
            let struct_node = Box::new(
                Struct::new(
                    struct_declaration
                        .typedef_name()
                        .as_ref()
                        .or_else(|| struct_declaration.struct_name().as_ref())
                        .map(normalize_constant)
                        .unwrap_or_else(|| IDENTIFIER_GENERATOR.upper_camel_case())
                        .tap(|constant| {
                            TYPE_CONSTANTS.lock().unwrap().insert(constant.to_owned());
                        }),
                    struct_declaration.is_union(),
                )
                .tap(|struct_node| {
                    struct_node.fields = struct_declaration
                        .fields()
                        .iter()
                        .map(|field| match field.field_type() {
                            FieldType::NamedType(t) => StructField {
                                name: field
                                    .name()
                                    .as_ref()
                                    .map(|name| name.to_owned())
                                    .unwrap_or_else(|| IDENTIFIER_GENERATOR.snack_case()),
                                field_type: StructFieldType::from(t.to_owned()),
                            },
                            FieldType::AnonymousType(anon_struct_declaration) => {
                                let anon_struct_name = insert_struct_node(anon_struct_declaration, nodes);
                                StructField {
                                    name: field
                                        .name()
                                        .as_ref()
                                        .map(|name| name.to_owned())
                                        .unwrap_or_else(|| IDENTIFIER_GENERATOR.snack_case()),
                                    field_type: StructFieldType::new_type_by_val(anon_struct_name),
                                }
                            }
                        })
                        .collect();
                }),
            );
            struct_node.struct_name.to_owned().tap(|_| {
                nodes.push(struct_node);
            })
        }
    }

    fn insert_function_declaration_bindings(&self, source_file: &SourceFile, module: &mut Module) {
        source_file.function_declarations().iter().for_each(|type_declaration| {
            insert_function_node(type_declaration, &mut module.sub_nodes);
        });

        fn insert_function_node(
            function_declaration: &FunctionDeclaration,
            nodes: &mut Vec<Box<dyn CodeGenerator>>,
        ) -> String {
            let mut function_node = Box::new(Function::new(
                function_declaration.name().to_owned(),
                StructFieldType::from(function_declaration.return_type().to_owned()),
            ));
            for parameter in function_declaration.parameters().iter() {
                function_node
                    .parameters
                    .push(StructFieldType::from(parameter.parameter_type().to_owned()));
            }
            function_node.function_name.to_owned().tap(|_| {
                nodes.push(function_node);
            })
        }
    }
}

fn normalize_constant(name: impl AsRef<str>) -> String {
    name.as_ref().split(' ').last().unwrap().to_camel_case()
}
