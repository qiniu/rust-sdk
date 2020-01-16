use crate::ast::SourceFile;
use crate::utils::generate_prefix_spaces;
use clang::Entity as ClangEntity;
use std::{
    io::{Result, Write},
    process::{exit, Command, Stdio},
};
use tap::TapOps;

trait CodeGenerator {
    fn generate_code(&self, output: &mut dyn Write, level: usize, spaces: usize) -> Result<()>;
    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator>;
}

#[derive(Default)]
struct TopLevelNode {
    sub_nodes: Vec<Box<dyn CodeGenerator>>,
}

impl CodeGenerator for TopLevelNode {
    fn generate_code(&self, output: &mut dyn Write, _: usize, spaces: usize) -> Result<()> {
        for node in self.sub_nodes.iter() {
            node.generate_code(output, 0, spaces)?;
        }
        Ok(())
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
    fn generate_code(&self, output: &mut dyn Write, level: usize, spaces: usize) -> Result<()> {
        let prefix_spaces = generate_prefix_spaces(level, spaces);
        self.code
            .lines()
            .try_for_each(|line| output.write_fmt(format_args!("{}{}\n", prefix_spaces, line)))?;
        Ok(())
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
    fn generate_code(&self, output: &mut dyn Write, level: usize, spaces: usize) -> Result<()> {
        let prefix_spaces = generate_prefix_spaces(level, spaces);
        output.write_fmt(format_args!("{}module {}\n", prefix_spaces, self.module_name))?;
        self.sub_nodes
            .iter()
            .try_for_each(|node| node.generate_code(output, level + 1, spaces))?;
        output.write_fmt(format_args!("{}end\n", prefix_spaces))?;
        Ok(())
    }

    fn sub_nodes(&self) -> Vec<&dyn CodeGenerator> {
        self.sub_nodes.iter().map(|node| node.as_ref()).collect::<Vec<_>>()
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
        let mut output_buf = Vec::new();
        self.build_without_syntax_check(entity, &mut output_buf)?;
        output.write_all(&output_buf)?;
        self.check_syntax(&output_buf)?;
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

    fn build_without_syntax_check(&self, entity: &ClangEntity, output: &mut dyn Write) -> Result<()> {
        let source_file = SourceFile::parse(&entity);
        let mut top_level_node = TopLevelNode::default();

        if let Some(top_level_module) = self.module_names.iter().rev().fold(None, |module, module_name| {
            Some(Box::new(Module::new(module_name).tap(|m| {
                if let Some(module) = module {
                    m.sub_nodes = vec![module];
                } else {
                    let mut core_ffi_module = Module::new("CoreFFI");
                    self.insert_ffi_bindings(&source_file, &mut core_ffi_module);
                    m.sub_nodes = vec![
                        Box::new(core_ffi_module),
                        Box::new(RawCode::new("private_constant :CoreFFI")),
                    ];
                }
            })) as Box<dyn CodeGenerator>)
        }) {
            top_level_node.sub_nodes.push(top_level_module);
        }

        top_level_node.generate_code(output, 0, 2)?;
        Ok(())
    }

    fn insert_ffi_bindings(&self, source_file: &SourceFile, module: &mut Module) {
        module.sub_nodes.push(Box::new(RawCode::new("extend FFI::Library")));
        module.sub_nodes.push(Box::new(RawCode::new(
            "DEFAULT_RELEASE_TARGET_DIR = ".to_owned()
                + "File.expand_path(File.join('..', '..', '..', '..', '..', 'target', 'release'))",
        )));
        module
            .sub_nodes
            .push(Box::new(RawCode::new("private_constant :DEFAULT_RELEASE_TARGET_DIR")));
        module.sub_nodes.push(Box::new(RawCode::new(
            "ffi_lib [".to_owned()
                + &format!("\"qiniu_ng_c-#{{{}}}\", ", self.version_constant)
                + "'qiniu_ng_c', "
                + &format!("File.expand_path(File.join(DEFAULT_RELEASE_TARGET_DIR, \"#{{FFI::Platform::LIBPREFIX}}qiniu_ng_c-#{{{}}}.#{{FFI::Platform::LIBSUFFIX}}\"), __dir__), ", self.version_constant)
                + "File.expand_path(File.join(DEFAULT_RELEASE_TARGET_DIR, \"#{FFI::Platform::LIBPREFIX}qiniu_ng_c.#{FFI::Platform::LIBSUFFIX}\"), __dir__)"
                + "]"
        )));
    }
}
