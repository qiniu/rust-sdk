mod ast;
mod attach_function_declaration_bindings;
mod callback_declaration_bindings;
mod dependency_resolver;
mod ffi_bindings;
mod ffi_wrapper_classes;
mod find_types;
mod type_declaration_bindings;
mod types;
mod utils;

use ast::{Module, RawCode, TopLevelNode};
use attach_function_declaration_bindings::insert_attach_function_declaration_bindings;
use callback_declaration_bindings::insert_callback_declaration_bindings;
use dependency_resolver::DependenciesResolver;
use ffi_bindings::insert_ffi_bindings;
use ffi_wrapper_classes::insert_ffi_wrapper_classes;
use find_types::find_all_type_constants;
use type_declaration_bindings::insert_type_declaration_bindings;

use crate::{
    ast::SourceFile,
    classifier::Classifier,
    utils::{CodeGenerator, CodeWriter, Writer},
};
use std::{
    io::{Result, Write},
    process::{exit, Command, Stdio},
};
use tap::TapOps;

const CORE_FFI_MODULE_NAME: &str = "CoreFFI";

#[derive(Default)]
pub(super) struct GenerateBindings {
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

    /// 开始构建 FFI 声明文件
    ///
    /// 分成以下几个步骤:
    /// 1. 找出即将声明的结构体或枚举类型
    /// 2. 开始构建 FFI 模块，并初始化 FFI 模块
    /// 3. 在 FFI 模块内部构建 CoreFFI 模块
    ///    1. 创建依赖管理器
    ///    2. 将所有回调节点，类型声明节点，函数声明节点以此放入依赖管理器
    ///    3. 依赖管理器进行拓扑排序，得到的结果输出到 CoreFFI 模块内
    /// 4. 在 CoreFFI 模块外部，构建 FFI 封装类型
    fn build_without_syntax_check(&self, source_file: &SourceFile, output: Writer) -> Result<Writer> {
        find_all_type_constants(source_file);

        let mut top_level_node = TopLevelNode::default();
        top_level_node
            .sub_nodes_mut()
            .push(Box::new(RawCode::new("require 'ffi'")));

        if let Some(top_level_module) = self.module_names.iter().rev().fold(None, |module, module_name| {
            Some(Box::new(Module::new(module_name, false).tap(|m| {
                if let Some(module) = module {
                    m.sub_nodes_mut().push(module);
                } else {
                    let mut core_ffi_module = Module::new(CORE_FFI_MODULE_NAME, false);
                    insert_ffi_bindings(&self.version_constant, &mut core_ffi_module);
                    let mut dependency_resolver = DependenciesResolver::new();
                    insert_callback_declaration_bindings(source_file, &mut dependency_resolver);
                    insert_type_declaration_bindings(source_file, &mut dependency_resolver);
                    insert_attach_function_declaration_bindings(source_file, &mut dependency_resolver);
                    core_ffi_module.sub_nodes_mut().extend(dependency_resolver.resolve());
                    *m.sub_nodes_mut() = vec![Box::new(core_ffi_module)];
                    insert_ffi_wrapper_classes(&self.classifier, m);
                }
            })) as Box<dyn CodeGenerator>)
        }) {
            top_level_node.sub_nodes_mut().push(top_level_module);
        }

        Ok(top_level_node
            .generate_code(CodeWriter::new(output, 2, 0))?
            .into_inner())
    }
}
