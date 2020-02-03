use crate::{
    ast::{
        EnumConstantValue, EnumDeclaration, FieldType, FunctionDeclaration, FunctionType, ParameterDeclaration,
        SourceFile, StructDeclaration, Type, TypeDeclaration, TypeKind,
    },
    classifier::{Class, Classifier},
    utils::{CodeGenerator, CodeWriter, RandomIdentifier, Writer},
};
use clang::TypeKind as ClangTypeKind;
use heck::CamelCase;
use lazy_static::lazy_static;
use matches::matches;
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
    static ref FUNCTION_POINTER_DEFS: Mutex<HashMap<String, String>> = Default::default();
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
            TypeKind::Base(ClangTypeKind::Void) => Self::Void,
            TypeKind::Base(ClangTypeKind::Bool) => Self::Bool,
            TypeKind::Base(ClangTypeKind::CharS) | TypeKind::Base(ClangTypeKind::SChar) => Self::Char,
            TypeKind::Base(ClangTypeKind::CharU) | TypeKind::Base(ClangTypeKind::UChar) => Self::Uchar,
            TypeKind::Base(ClangTypeKind::UInt) => Self::U32,
            TypeKind::Base(ClangTypeKind::Int) => Self::I32,
            TypeKind::Base(ClangTypeKind::Long) => Self::Ssize,
            TypeKind::Base(ClangTypeKind::LongLong) => Self::I64,
            TypeKind::Base(ClangTypeKind::ULong) => Self::Size,
            TypeKind::Base(ClangTypeKind::ULongLong) => Self::U64,
            TypeKind::Pointer { subtype } => match subtype.type_kind() {
                TypeKind::Base(ClangTypeKind::CharS)
                | TypeKind::Base(ClangTypeKind::SChar)
                | TypeKind::Base(ClangTypeKind::CharU)
                | TypeKind::Base(ClangTypeKind::UChar)
                    if subtype.is_const() =>
                {
                    Self::String
                }
                _ => Self::Pointer,
            },
            TypeKind::Base(ClangTypeKind::Elaborated) => match c_type.display_name().as_str() {
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
    ByCallback(String),
}

impl StructFieldType {
    fn from(c_type: Type) -> Self {
        return match c_type.type_kind() {
            TypeKind::Typedef { .. } => Self::new_type_by_val(normalize_constant(c_type.display_name())),
            TypeKind::Pointer { subtype: pointer_type } => match pointer_type.type_kind() {
                TypeKind::Typedef { .. } => Self::new_type_by_ptr(normalize_constant(pointer_type.display_name())),
                TypeKind::Function { .. } => {
                    if let Some(callback_name) = FUNCTION_POINTER_DEFS.lock().unwrap().get(pointer_type.display_name())
                    {
                        Self::ByCallback(callback_name.to_owned())
                    } else {
                        Self::BaseType(BaseType::Pointer)
                    }
                }
                _ => new_base_type(c_type),
            },
            _ => new_base_type(c_type),
        };

        fn new_base_type(c_type: Type) -> StructFieldType {
            if matches!(c_type.type_kind(), TypeKind::Base(ClangTypeKind::Elaborated)) {
                match c_type.display_name().as_str() {
                    "struct in_addr" => {
                        return StructFieldType::BaseType(BaseType::InAddr);
                    }
                    "struct in6_addr" => {
                        return StructFieldType::new_type_by_val("In6Addr".to_owned());
                    }
                    _ => {}
                }
            }
            StructFieldType::BaseType(BaseType::from(c_type))
        }
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
            Self::BaseType(type_name) => type_name.to_symbol().fmt(f),
            Self::ByVal(type_name) => match TYPE_CONSTANTS.lock().unwrap().get(type_name) {
                Some(ConstantType::Struct) => write!(f, "{}.by_value", type_name),
                _ => type_name.fmt(f),
            },
            Self::ByPtr(type_name) => match TYPE_CONSTANTS.lock().unwrap().get(type_name) {
                Some(ConstantType::Struct) => write!(f, "{}.ptr", type_name),
                _ => Self::BaseType(BaseType::Pointer).fmt(f),
            },
            Self::ByCallback(callback_name) => write!(f, ":{}", callback_name),
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
}

#[derive(Default)]
struct AttachFunction {
    name: String,
    parameters: Vec<StructFieldType>,
    return_value: StructFieldType,
    is_callback: bool,
}

impl AttachFunction {
    fn new(name: impl Into<String>, return_value: StructFieldType, is_callback: bool) -> Self {
        Self {
            name: name.into(),
            return_value,
            is_callback,
            ..Default::default()
        }
    }
}

impl CodeGenerator for AttachFunction {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&format!(
            "{} :{}, [{}], {}",
            if self.is_callback {
                "callback"
            } else {
                "attach_function"
            },
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
}

#[derive(Default)]
struct Method {
    name: String,
    is_class_method: bool,
    parameter_names: Vec<String>,
    return_node: Option<Box<dyn CodeGenerator>>,
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
        if let Some(return_node) = self.return_node.as_ref() {
            output = return_node.generate_code(output)?;
        }
        output.write("end")?;
        Ok(output)
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
}

#[derive(Default)]
struct Proc {
    is_lambda: bool,
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
    receiver_name: Option<String>,
    parameter_names: Vec<String>,
}

impl Proc {
    fn new(receiver_name: Option<String>, is_lambda: bool) -> Self {
        Self {
            receiver_name,
            is_lambda,
            ..Default::default()
        }
    }
}

impl CodeGenerator for Proc {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        let receiver = self
            .receiver_name
            .as_ref()
            .map(|name| format!("{} = ", name))
            .unwrap_or_else(String::new);
        let parameters = if self.parameter_names.is_empty() {
            String::new()
        } else {
            format!(" |{}|", self.parameter_names.join(", "))
        };
        if self.is_lambda {
            output.write(&format!("{}lambda do{}", receiver, parameters))?;
        } else {
            output.write(&format!("{}proc do{}", receiver, parameters))?;
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
}

enum InsertCallbackDeclarationBindingsFor {
    Structs,
    Functions,
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

    pub fn build(mut self, source_file: &SourceFile, classifier: Classifier, output: &mut dyn Write) -> Result<()> {
        self.classifier = classifier;
        let mut output_buf = self.build_without_syntax_check(source_file, Writer::Memory(Vec::new()))?;
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

    fn build_without_syntax_check(&self, source_file: &SourceFile, output: Writer) -> Result<Writer> {
        let mut top_level_node = TopLevelNode::default();
        top_level_node.sub_nodes.push(Box::new(RawCode::new("require 'ffi'")));

        if let Some(top_level_module) = self.module_names.iter().rev().fold(None, |module, module_name| {
            Some(Box::new(Module::new(module_name, false).tap(|m| {
                if let Some(module) = module {
                    m.sub_nodes = vec![module];
                } else {
                    let mut core_ffi_module = Module::new(CORE_FFI_MODULE_NAME, false);
                    self.insert_ffi_bindings(&mut core_ffi_module);
                    self.insert_callback_declaration_bindings(
                        source_file,
                        &mut core_ffi_module,
                        InsertCallbackDeclarationBindingsFor::Structs,
                    );
                    self.insert_type_declaration_bindings(source_file, &mut core_ffi_module);
                    self.insert_callback_declaration_bindings(
                        source_file,
                        &mut core_ffi_module,
                        InsertCallbackDeclarationBindingsFor::Functions,
                    );
                    self.insert_attach_function_declaration_bindings(source_file, &mut core_ffi_module);
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
            "DEFAULT_TARGET_DIR = ".to_owned() + "File.expand_path(File.join('..', '..', '..', 'target'), __dir__)",
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

    fn insert_callback_declaration_bindings(
        &self,
        source_file: &SourceFile,
        module: &mut Module,
        insert_callback_declaration_bindings_for: InsertCallbackDeclarationBindingsFor,
    ) {
        match insert_callback_declaration_bindings_for {
            InsertCallbackDeclarationBindingsFor::Structs => {
                source_file.type_declarations().iter().for_each(|type_declaration| {
                    if let TypeDeclaration::Struct(s) = type_declaration {
                        insert_callback_declaration_bindings_for_struct_node(s, &mut module.sub_nodes);
                    }
                });
            }
            InsertCallbackDeclarationBindingsFor::Functions => {
                source_file
                    .function_declarations()
                    .iter()
                    .for_each(|function_declaration| {
                        insert_callback_declaration_bindings_for_function_node(
                            function_declaration,
                            &mut module.sub_nodes,
                        );
                    });
            }
        }

        fn insert_callback_declaration_bindings_for_struct_node(
            struct_declaration: &StructDeclaration,
            nodes: &mut Vec<Box<dyn CodeGenerator>>,
        ) {
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
                    insert_callback_node(callback_name.to_owned(), ft, nodes);
                    FUNCTION_POINTER_DEFS
                        .lock()
                        .unwrap()
                        .insert(ft.display_name().to_owned(), callback_name);
                });
        }

        fn insert_callback_declaration_bindings_for_function_node(
            function_declaration: &FunctionDeclaration,
            nodes: &mut Vec<Box<dyn CodeGenerator>>,
        ) {
            function_declaration
                .parameters()
                .iter()
                .filter_map(|parameter| {
                    try_to_extract_function_type(parameter.parameter_type())
                        .map(|function_type| (function_declaration.name(), function_type))
                })
                .for_each(|(s, ft)| {
                    let callback_name = format!("{}_{}_callback", function_declaration.name(), s);
                    insert_callback_node(callback_name.to_owned(), ft, nodes);
                    FUNCTION_POINTER_DEFS
                        .lock()
                        .unwrap()
                        .insert(ft.display_name().to_owned(), callback_name);
                });
        }

        fn insert_callback_node(name: String, function_type: &FunctionType, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
            nodes.push(
                Box::new(AttachFunction::new(
                    name,
                    StructFieldType::from(function_type.return_type().to_owned()),
                    true,
                ))
                .tap(|attach_function| {
                    attach_function.parameters = function_type
                        .parameter_types()
                        .iter()
                        .map(|parameter_type| StructFieldType::from(parameter_type.to_owned()))
                        .collect();
                }),
            );
        }
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
                        .map(|field| {
                            let field_name = field
                                .name()
                                .as_ref()
                                .map(|name| name.to_owned())
                                .unwrap_or_else(|| IDENTIFIER_GENERATOR.snack_case());
                            match field.field_type() {
                                FieldType::NamedType(t) => StructField {
                                    name: field_name,
                                    field_type: StructFieldType::from(t.to_owned()),
                                },
                                FieldType::AnonymousType(anon_struct_declaration) => {
                                    let anon_struct_name = insert_struct_node(anon_struct_declaration, nodes);
                                    StructField {
                                        name: field_name,
                                        field_type: StructFieldType::new_type_by_val(anon_struct_name),
                                    }
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
        source_file
            .function_declarations()
            .iter()
            .for_each(|function_declaration| {
                insert_attach_function_node(function_declaration, &mut module.sub_nodes);
            });

        fn insert_attach_function_node(
            function_declaration: &FunctionDeclaration,
            nodes: &mut Vec<Box<dyn CodeGenerator>>,
        ) -> String {
            let function_node = Box::new(AttachFunction::new(
                function_declaration.name().to_owned(),
                StructFieldType::from(function_declaration.return_type().to_owned()),
                false,
            ))
            .tap(|attach_function| {
                attach_function.parameters = function_declaration
                    .parameters()
                    .iter()
                    .map(|parameter| StructFieldType::from(parameter.parameter_type().to_owned()))
                    .collect();
            });
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
                    finalizer.sub_nodes.push(Box::new(Proc::new(None, false)).tap(|proc| {
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
                let new_method_name = if matches!(
                    method.declaration().return_type().type_kind(),
                    TypeKind::Base(ClangTypeKind::Bool)
                ) {
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
                            new_method.is_class_method = true;
                            normalize_parameters_for_method_and_method_call(
                                new_method,
                                method.declaration().parameters(),
                                method.declaration().return_type(),
                                MethodCall::new(
                                    Some(Context::Modules(vec![CORE_FFI_MODULE_NAME.to_owned()])),
                                    method.declaration().name().to_owned(),
                                )
                                .tap(|_method_call| {
                                    // TODO
                                }),
                            );
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
                method.parameter_names.push(parameter.name().to_owned());

                let cur_param_type = parameter.parameter_type();
                let next_param_type = parameters.get(i + 1).map(|next| next.parameter_type());
                if is_const_str_type(cur_param_type) {
                    method_call
                        .parameter_names
                        .push(format!("{}.encode(Encoding::UTF_8)", parameter.name()));
                } else if is_const_str_list_type(cur_param_type) && next_param_type.map(is_size_type) == Some(true) {
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
                    method_call.parameter_names.push(temp_pointer_variable_name.to_owned());
                    method_call.parameter_names.push(format!("{}.size", parameter.name()));
                    skip = 1;
                } else if is_const_binary_type(cur_param_type) && next_param_type.map(is_size_type) == Some(true) {
                    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
                    method.sub_nodes.push(
                        Box::new(MethodCall::new(
                            Some(Context::Modules(vec!["FFI".to_owned(), "MemoryPointer".to_owned()])),
                            "from_string".to_owned(),
                        ))
                        .tap(|ffi_memory_new| {
                            ffi_memory_new.parameter_names = vec![parameter.name().to_owned()];
                            ffi_memory_new.receiver_names = vec![temp_pointer_variable_name.to_owned()];
                        }),
                    );
                    method_call.parameter_names.push(temp_pointer_variable_name.to_owned());
                    method_call
                        .parameter_names
                        .push(format!("{}.bytesize", parameter.name()));
                    skip = 1;
                } else if let Some(function_type) = try_to_extract_function_type(cur_param_type) {
                    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
                    method.sub_nodes.push(
                        Box::new(Proc::new(Some(temp_pointer_variable_name.to_owned()), false)).tap(|in_proc| {
                            in_proc.parameter_names = function_type
                                .parameter_types()
                                .iter()
                                .enumerate()
                                .map(|(i, _)| format!("__{}", i))
                                .collect();
                            in_proc.sub_nodes.push(
                                Box::new(MethodCall::new(
                                    Some(Context::Instance(parameter.name().to_owned())),
                                    "call".to_owned(),
                                ))
                                .tap(|proc_call| {
                                    proc_call.parameter_names = function_type
                                        .parameter_types()
                                        .iter()
                                        .enumerate()
                                        .map(|(i, t)| {
                                            if is_const_str_type(t) {
                                                format!("__{}.force_encoding(Encoding::UTF_8)", i)
                                            } else {
                                                format!("__{}", i)
                                            }
                                        })
                                        .collect();
                                }),
                            )
                        }),
                    );
                    method_call.parameter_names.push(temp_pointer_variable_name.to_owned());
                } else {
                    method_call.parameter_names.push(parameter.name().to_owned());
                }
            }
            if method_call.receiver_names.is_empty() && is_const_str_type(return_type) {
                let temp_pointer_variable_name = identifier_generator.lower_camel_case();
                method_call.receiver_names = vec![temp_pointer_variable_name.to_owned()];
                method.sub_nodes.push(Box::new(method_call));
                method.sub_nodes.push(
                    Box::new(MethodCall::new(
                        Some(Context::Instance(temp_pointer_variable_name)),
                        "force_encoding".to_owned(),
                    ))
                    .tap(|encode_method_call| {
                        encode_method_call.parameter_names = vec!["Encoding::UTF_8".to_owned()];
                    }),
                )
            } else {
                method.sub_nodes.push(Box::new(method_call));
            }
        }

        fn is_const_str_type(t: &Type) -> bool {
            if let TypeKind::Pointer { subtype } = t.type_kind() {
                match subtype.type_kind() {
                    TypeKind::Base(ClangTypeKind::CharS)
                    | TypeKind::Base(ClangTypeKind::SChar)
                    | TypeKind::Base(ClangTypeKind::CharU)
                    | TypeKind::Base(ClangTypeKind::UChar)
                        if subtype.is_const() =>
                    {
                        true
                    }
                    _ => false,
                }
            } else {
                false
            }
        }

        fn is_const_binary_type(t: &Type) -> bool {
            if let TypeKind::Pointer { subtype } = t.type_kind() {
                match subtype.type_kind() {
                    TypeKind::Base(ClangTypeKind::Void) if subtype.is_const() => true,
                    _ => false,
                }
            } else {
                false
            }
        }

        fn is_const_str_list_type(t: &Type) -> bool {
            if let TypeKind::Pointer { subtype } = t.type_kind() {
                subtype.is_const() && is_const_str_type(&subtype)
            } else {
                false
            }
        }

        fn is_size_type(t: &Type) -> bool {
            if let TypeKind::Typedef { subtype } = t.type_kind() {
                matches!(subtype.type_kind(), TypeKind::Base(ClangTypeKind::ULong))
                    && t.display_name().as_str() == "size_t"
            } else {
                false
            }
        }
    }
}

fn normalize_constant(name: impl AsRef<str>) -> String {
    name.as_ref().split(' ').last().unwrap().to_camel_case()
}

fn try_to_extract_function_type(t: &Type) -> Option<&FunctionType> {
    if let TypeKind::Pointer { subtype } = t.type_kind() {
        if let TypeKind::Function { subtype } = subtype.type_kind() {
            return Some(&subtype);
        }
    }
    None
}
