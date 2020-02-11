use super::types::StructFieldType;
use crate::utils::{CodeGenerator, CodeWriter};
use getset::{CopyGetters, Getters, MutGetters};
use std::io::Result;

#[derive(Default, Getters, MutGetters)]
#[get = "pub(super)"]
#[get_mut = "pub(super)"]
pub(super) struct TopLevelNode {
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

#[derive(Default, Getters, MutGetters)]
#[get = "pub(super)"]
#[get_mut = "pub(super)"]
pub(super) struct RawCode {
    code: String,
}

impl RawCode {
    pub(super) fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

impl CodeGenerator for RawCode {
    fn generate_code(&self, mut output: CodeWriter) -> Result<CodeWriter> {
        output.write(&self.code)?;
        Ok(output)
    }
}

#[derive(Default, Getters, CopyGetters, MutGetters)]
pub(super) struct Module {
    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    name: String,

    #[get_copy = "pub(super)"]
    #[get_mut = "pub(super)"]
    is_class: bool,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl Module {
    pub(super) fn new(name: impl Into<String>, is_class: bool) -> Self {
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

#[derive(Default, Getters, MutGetters)]
#[get = "pub(super)"]
#[get_mut = "pub(super)"]
pub(super) struct EnumValue {
    name: String,
    value: String,
}

impl EnumValue {
    pub(super) fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[derive(Default, Getters, MutGetters)]
#[get = "pub(super)"]
#[get_mut = "pub(super)"]
pub(super) struct Enum {
    name: String,
    constants: Vec<EnumValue>,
}

impl Enum {
    pub(super) fn new(name: impl Into<String>) -> Self {
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

#[derive(Getters, MutGetters)]
pub(super) struct StructField {
    name: String,
    field_type: StructFieldType,
}

impl StructField {
    pub(super) fn new(name: impl Into<String>, field_type: StructFieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
        }
    }
}

#[derive(Default, Getters, CopyGetters, MutGetters)]
pub(super) struct Struct {
    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    name: String,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    fields: Vec<StructField>,

    #[get_copy = "pub(super)"]
    #[get_mut = "pub(super)"]
    is_union: bool,
}

impl Struct {
    pub(super) fn new(name: impl Into<String>, is_union: bool) -> Self {
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

#[derive(Default, CopyGetters, Getters, MutGetters)]
pub(super) struct AttachFunction {
    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    name: String,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    parameters: Vec<StructFieldType>,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    return_value: StructFieldType,

    #[get_copy = "pub(super)"]
    #[get_mut = "pub(super)"]
    is_callback: bool,
}

impl AttachFunction {
    pub(super) fn new(name: impl Into<String>, return_value: StructFieldType, is_callback: bool) -> Self {
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
            "{} :{}, [{}], {}{}",
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
            self.return_value,
            if self.is_callback { "" } else { ", blocking: true" }
        ))?;
        Ok(output)
    }
}

#[derive(Default, CopyGetters, Getters, MutGetters)]
pub(super) struct Method {
    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    name: String,

    #[get_copy = "pub(super)"]
    #[get_mut = "pub(super)"]
    is_class_method: bool,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    parameter_names: Vec<String>,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    return_names: Vec<String>,

    asc_nodes: Vec<Box<dyn CodeGenerator>>,
    desc_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl Method {
    pub(super) fn new(name: impl Into<String>, is_class_method: bool) -> Self {
        Self {
            name: name.into(),
            is_class_method,
            ..Default::default()
        }
    }

    pub(super) fn insert_asc_node(&mut self, node: Box<dyn CodeGenerator>) {
        self.asc_nodes.push(node);
    }

    pub(super) fn insert_desc_node(&mut self, node: Box<dyn CodeGenerator>) {
        self.desc_nodes.push(node);
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
            for node in self.asc_nodes.iter() {
                output = node.generate_code(output)?;
            }
            for node in self.desc_nodes.iter().rev() {
                output = node.generate_code(output)?;
            }
            match self.return_names.len() {
                0 => {}
                1 => output.write(self.return_names.first().unwrap())?,
                _ => output.write(&format!("return {}", self.return_names.join(", ")))?,
            }
            Ok(output)
        })?;
        output.write("end")?;
        Ok(output)
    }
}

pub(super) enum Context {
    Modules(Vec<String>),
    Instance(String),
}

#[derive(Default, Getters, CopyGetters, MutGetters)]
pub(super) struct MethodCall {
    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    receiver_names: Vec<String>,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    context: Option<Context>,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    method_name: String,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    parameter_names: Vec<String>,

    #[get_copy = "pub(super)"]
    #[get_mut = "pub(super)"]
    nil_safe: bool,
}

impl MethodCall {
    pub(super) fn new(context: Option<Context>, method_name: impl Into<String>) -> Self {
        Self {
            context,
            method_name: method_name.into(),
            nil_safe: true,
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
            Some(Context::Instance(instance)) => {
                if self.nil_safe {
                    format!("{}&.{}", instance, self.method_name)
                } else {
                    format!("{}.{}", instance, self.method_name)
                }
            }
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

#[derive(Default, CopyGetters, Getters, MutGetters)]
pub(super) struct Proc {
    #[get_copy = "pub(super)"]
    #[get_mut = "pub(super)"]
    is_lambda: bool,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    sub_nodes: Vec<Box<dyn CodeGenerator>>,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    receiver_name: Option<String>,

    #[get = "pub(super)"]
    #[get_mut = "pub(super)"]
    parameter_names: Vec<String>,
}

impl Proc {
    pub(super) fn new(receiver_name: Option<String>, is_lambda: bool) -> Self {
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
