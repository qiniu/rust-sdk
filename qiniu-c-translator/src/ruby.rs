use crate::{
    ast::{
        EnumConstantValue, EnumDeclaration, FieldType, FunctionDeclaration, ParameterDeclaration, SourceFile,
        StructDeclaration, SubType, Type, TypeDeclaration,
    },
    classifier::{Class, Classifier},
    utils::{CodeGenerator, CodeWriter, RandomIdentifier, Writer},
};
use clang::{Entity as ClangEntity, TypeKind as ClangTypeKind};
use heck::CamelCase;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fmt,
    io::{Result, Write},
    process::{exit, Command, Stdio},
    sync::Mutex,
};
use tap::TapOps;

lazy_static! {
    static ref TYPE_CONSTANTS: Mutex<HashMap<String, ConstantType>> = Default::default();
    static ref IDENTIFIER_GENERATOR: RandomIdentifier = Default::default();
}

const CORE_FFI_MODULE_NAME: &str = "CoreFFI";
const CLASS_CONTEXT_INSTANCE_VARIABLE_NAME: &str = "@instance";

enum ConstantType {
    Enum,
    Struct,
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
    name: String,
    is_class: bool,
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl Module {
    fn new(name: impl Into<String>, is_class: bool) -> Self {
        Self {
            name: name.into(),
            is_class,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Module {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        if self.is_class {
            output.write(&format!("class {}", self.name))?;
        } else {
            output.write(&format!("module {}", self.name))?;
        }
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
    name: String,
    constants: Vec<EnumValue>,
}

impl Enum {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

impl CodeGenerator for Enum {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!("{} = enum(", self.name))?;
        output = output.try_with_next_level(|mut output| {
            for enum_value in self.constants.iter() {
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
    Ssize,
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
            ClangTypeKind::Long => Self::Ssize,
            ClangTypeKind::LongLong => Self::I64,
            ClangTypeKind::ULong => Self::Size,
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
            Self::Size => ":ulong",
            Self::Ssize => ":long",
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
                "CurLcode" => Self::BaseType(BaseType::Size),
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
            StructFieldType::ByVal(type_name) => match TYPE_CONSTANTS.lock().unwrap().get(type_name) {
                Some(ConstantType::Struct) => write!(f, "{}.by_value", type_name),
                _ => type_name.fmt(f),
            },
            StructFieldType::ByPtr(type_name) => match TYPE_CONSTANTS.lock().unwrap().get(type_name) {
                Some(ConstantType::Struct) => write!(f, "{}.ptr", type_name),
                _ => Self::BaseType(BaseType::Pointer).fmt(f),
            },
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
    name: String,
    fields: Vec<StructField>,
    is_union: bool,
}

impl Struct {
    fn new(name: impl Into<String>, is_union: bool) -> Self {
        Self {
            name: name.into(),
            is_union,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Struct {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!(
            "class {} < FFI::{}",
            self.name,
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
struct AttachFunction {
    name: String,
    parameters: Vec<StructFieldType>,
    return_value: StructFieldType,
}

impl AttachFunction {
    fn new(name: impl Into<String>, return_value: StructFieldType) -> Self {
        Self {
            name: name.into(),
            return_value,
            ..Default::default()
        }
    }
}

impl CodeGenerator for AttachFunction {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!(
            "attach_function :{}, [{}], {}",
            self.name,
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
struct Method {
    name: String,
    is_class_method: bool,
    parameter_names: Vec<String>,
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl Method {
    fn new(name: impl Into<String>, is_class_method: bool) -> Self {
        Self {
            name: name.into(),
            is_class_method,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Method {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        let parameters = if self.parameter_names.is_empty() {
            String::new()
        } else {
            format!("({})", self.parameter_names.join(", "))
        };
        if self.is_class_method {
            output.write(&format!("def self.{}{}", self.name, parameters))?;
        } else {
            output.write(&format!("def {}{}", self.name, parameters))?;
        }
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

enum Context {
    Modules(Vec<String>),
    Instance(String),
}

#[derive(Default)]
struct MethodCall {
    receiver_names: Vec<String>,
    context: Option<Context>,
    method_name: String,
    parameter_names: Vec<String>,
}

impl MethodCall {
    fn new(context: Option<Context>, method_name: String) -> Self {
        Self {
            context,
            method_name,
            ..Default::default()
        }
    }
}

impl CodeGenerator for MethodCall {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        let receivers = if self.receiver_names.is_empty() {
            String::new()
        } else {
            format!("{} = ", self.receiver_names.join(", "))
        };
        let full_method_name = match &self.context {
            Some(Context::Modules(modules)) => format!("{}::{}", modules.join("::"), self.method_name),
            Some(Context::Instance(instance)) => format!("{}.{}", instance, self.method_name),
            None => self.method_name.to_owned(),
        };
        let parameters = if self.parameter_names.is_empty() {
            String::new()
        } else {
            format!("({})", self.parameter_names.join(", "))
        };
        output.write(&format!("{}{}{}", receivers, full_method_name, parameters))?;
        Ok(output)
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        Vec::new()
    }
}

#[derive(Default)]
struct Proc {
    is_lambda: bool,
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl Proc {
    fn new(is_lambda: bool) -> Self {
        Self {
            is_lambda,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Proc {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(if self.is_lambda { "lambda do" } else { "proc do" })?;
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
pub struct GenerateBindings {
    module_names: Vec<String>,
    version_constant: String,
    classifier: Classifier,
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

    pub fn build(mut self, entity: &ClangEntity, classifier: Classifier, output: &mut dyn Write) -> Result<()> {
        self.classifier = classifier;
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
            Some(Box::new(Module::new(module_name, false).tap(|m| {
                if let Some(module) = module {
                    m.sub_nodes = vec![module];
                } else {
                    let mut core_ffi_module = Module::new(CORE_FFI_MODULE_NAME, false);
                    self.insert_ffi_bindings(&mut core_ffi_module);
                    self.insert_type_declaration_bindings(&source_file, &mut core_ffi_module);
                    self.insert_attach_function_declaration_bindings(&source_file, &mut core_ffi_module);
                    m.sub_nodes = vec![
                        Box::new(core_ffi_module),
                        Box::new(RawCode::new(format!("private_constant :{}", CORE_FFI_MODULE_NAME))),
                    ];
                    self.insert_ffi_wrapper_classes(m);
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
            TYPE_CONSTANTS
                .lock()
                .unwrap()
                .insert("In6Addr".to_owned(), ConstantType::Struct);
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
                            TYPE_CONSTANTS
                                .lock()
                                .unwrap()
                                .insert(constant.to_owned(), ConstantType::Enum);
                        }),
                )
                .tap(|enum_node| {
                    enum_node.constants = enum_declaration
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
            enum_node.name.to_owned().tap(|_| {
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
                            TYPE_CONSTANTS
                                .lock()
                                .unwrap()
                                .insert(constant.to_owned(), ConstantType::Struct);
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
            struct_node.name.to_owned().tap(|_| {
                nodes.push(struct_node);
            })
        }
    }

    fn insert_attach_function_declaration_bindings(&self, source_file: &SourceFile, module: &mut Module) {
        source_file.function_declarations().iter().for_each(|type_declaration| {
            insert_attach_function_node(type_declaration, &mut module.sub_nodes);
        });

        fn insert_attach_function_node(
            function_declaration: &FunctionDeclaration,
            nodes: &mut Vec<Box<dyn CodeGenerator>>,
        ) -> String {
            let mut function_node = Box::new(AttachFunction::new(
                function_declaration.name().to_owned(),
                StructFieldType::from(function_declaration.return_type().to_owned()),
            ));
            for parameter in function_declaration.parameters().iter() {
                function_node
                    .parameters
                    .push(StructFieldType::from(parameter.parameter_type().to_owned()));
            }
            function_node.name.to_owned().tap(|_| {
                nodes.push(function_node);
            })
        }
    }

    fn insert_ffi_wrapper_classes(&self, module: &mut Module) {
        for class in self.classifier.classes().iter() {
            module
                .sub_nodes
                .push(Box::new(Module::new(class.name().to_owned(), true)).tap(|new_class| {
                    insert_constructor_node(class, &mut new_class.sub_nodes);
                    insert_destructor_node(class, &mut new_class.sub_nodes);
                    insert_methods_nodes(class, &mut new_class.sub_nodes);
                }));
        }

        fn insert_constructor_node(class: &Class, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
            nodes.push(Box::new(Method::new("initialize", false)).tap(|initializer| {
                if let Some(constructor) = class.constructor().as_ref() {
                    normalize_parameters_for_method_and_method_call(
                        initializer,
                        constructor.parameters(),
                        constructor.return_type(),
                        MethodCall::new(
                            Some(Context::Modules(vec![CORE_FFI_MODULE_NAME.to_owned()])),
                            constructor.name().to_owned(),
                        )
                        .tap(|method_call| {
                            method_call.receiver_names = vec![CLASS_CONTEXT_INSTANCE_VARIABLE_NAME.to_owned()];
                        }),
                    );
                }
                if class.destructor().is_some() {
                    initializer.sub_nodes.push(Box::new(RawCode::new(format!(
                        "ObjectSpace.define_finalizer(self, self.class.finalize({}))",
                        CLASS_CONTEXT_INSTANCE_VARIABLE_NAME
                    ))));
                }
            }));
        }

        fn insert_destructor_node(class: &Class, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
            if let Some(destructor) = class.destructor().as_ref() {
                nodes.push(Box::new(Method::new("finalize", true)).tap(|finalizer| {
                    finalizer.parameter_names = vec!["instance".to_owned()];
                    finalizer.sub_nodes.push(Box::new(Proc::new(false)).tap(|proc| {
                        proc.sub_nodes.push(
                            Box::new(MethodCall::new(
                                Some(Context::Modules(vec![CORE_FFI_MODULE_NAME.to_owned()])),
                                destructor.name().to_owned(),
                            ))
                            .tap(|finalizer_method_call| {
                                finalizer_method_call.parameter_names = finalizer.parameter_names.clone();
                            }),
                        )
                    }))
                }));
            }
        }

        fn insert_methods_nodes(class: &Class, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
            for method in class.methods().iter() {
                let new_method_name = if method.declaration().return_type().type_kind() == ClangTypeKind::Bool {
                    format!("{}?", method.name())
                } else {
                    method.name().to_owned()
                };
                nodes.push(Box::new(Method::new(new_method_name, false)).tap(|new_method| {
                    if let Some(first_parameter) = method.declaration().parameters().first() {
                        if class.ffi_class_name()
                            == normalize_constant(first_parameter.parameter_type().display_name()).as_str()
                        {
                            normalize_parameters_for_method_and_method_call(
                                new_method,
                                method.declaration().parameters().get(1..).unwrap_or(&[]),
                                method.declaration().return_type(),
                                MethodCall::new(
                                    Some(Context::Modules(vec![CORE_FFI_MODULE_NAME.to_owned()])),
                                    method.declaration().name().to_owned(),
                                )
                                .tap(|method_call| {
                                    method_call.parameter_names = vec![CLASS_CONTEXT_INSTANCE_VARIABLE_NAME.to_owned()];
                                }),
                            );
                        } else {
                            panic!("Cannot support method call without context: {}", method.name());
                        }
                    }
                }))
            }
        }

        fn normalize_parameters_for_method_and_method_call(
            method: &mut Method,
            parameters: &[ParameterDeclaration],
            return_type: &Type,
            mut method_call: MethodCall,
        ) {
            let identifier_generator = RandomIdentifier::new();
            let mut skip = 0usize;
            for (i, parameter) in parameters.iter().enumerate() {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let cur_param_type = parameter.parameter_type();
                let next_param_type = parameters.get(i + 1).map(|next| next.parameter_type());
                if is_str(cur_param_type) {
                    method.parameter_names.push(parameter.name().to_owned());
                    method_call
                        .parameter_names
                        .push(format!("{}.encode('UTF-8')", parameter.name()));
                } else if is_str_list(cur_param_type) && next_param_type.map(is_size) == Some(true) {
                    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
                    method.sub_nodes.push(
                        Box::new(MethodCall::new(
                            Some(Context::Modules(vec!["FFI".to_owned(), "MemoryPointer".to_owned()])),
                            "new".to_owned(),
                        ))
                        .tap(|ffi_memory_new| {
                            ffi_memory_new.parameter_names =
                                vec![":string".to_owned(), format!("{}.size", parameter.name())];
                            ffi_memory_new.receiver_names = vec![temp_pointer_variable_name.to_owned()];
                        }),
                    );
                    method.sub_nodes.push(Box::new(RawCode::new(format!(
                        "{}.write_array_of_pointer({}.map {{|s| FFI::MemoryPointer.from_string(s) }})",
                        temp_pointer_variable_name,
                        parameter.name(),
                    ))));
                    method.parameter_names.push(parameter.name().to_owned());
                    method_call.parameter_names.push(temp_pointer_variable_name.to_owned());
                    method_call.parameter_names.push(format!("{}.size", parameter.name()));
                    skip = 1;
                } else {
                    method.parameter_names.push(parameter.name().to_owned());
                    method_call.parameter_names.push(parameter.name().to_owned());
                }
            }
            if method_call.receiver_names.is_empty() && is_str(return_type) {
                let temp_pointer_variable_name = identifier_generator.lower_camel_case();
                method_call.receiver_names = vec![temp_pointer_variable_name.to_owned()];
                method.sub_nodes.push(Box::new(method_call));
                method.sub_nodes.push(
                    Box::new(MethodCall::new(
                        Some(Context::Instance(temp_pointer_variable_name)),
                        "force_encoding".to_owned(),
                    ))
                    .tap(|encode_method_call| {
                        encode_method_call.parameter_names = vec!["'UTF-8'".to_owned()];
                    }),
                )
            } else {
                method.sub_nodes.push(Box::new(method_call));
            }

            fn is_str(t: &Type) -> bool {
                if t.type_kind() == ClangTypeKind::Pointer {
                    if let Some(subtype1) = t.subtype().as_ref() {
                        if let SubType::PointeeType(sub_pointee_type1) = subtype1.as_ref() {
                            if (sub_pointee_type1.type_kind() == ClangTypeKind::CharS
                                || sub_pointee_type1.type_kind() == ClangTypeKind::SChar)
                                && sub_pointee_type1.is_const()
                            {
                                return true;
                            }
                        }
                    }
                }
                false
            }

            fn is_str_list(t: &Type) -> bool {
                if t.type_kind() == ClangTypeKind::Pointer {
                    if let Some(subtype1) = t.subtype().as_ref() {
                        if let SubType::PointeeType(sub_pointee_type1) = subtype1.as_ref() {
                            return sub_pointee_type1.is_const() && is_str(sub_pointee_type1);
                        }
                    }
                }
                false
            }

            fn is_size(t: &Type) -> bool {
                t.type_kind() == ClangTypeKind::Typedef && t.display_name().as_str() == "size_t"
            }
        }
    }
}

fn normalize_constant(name: impl AsRef<str>) -> String {
    name.as_ref().split(' ').last().unwrap().to_camel_case()
}
