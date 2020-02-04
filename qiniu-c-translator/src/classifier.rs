use crate::{
    ast::FunctionDeclaration,
    utils::{CodeWriter, Writer},
};
use getset::Getters;
use regex::Regex;
use std::{
    io::{stdout, Result},
    iter::Iterator,
};
use tap::TapOps;

#[derive(Debug, Default, Getters)]
pub struct Classifier {
    #[get = "pub"]
    classes: Vec<Class>,
}

impl Classifier {
    pub fn add_class(&mut self, class: Class) {
        self.classes.push(class);
    }
}

#[derive(Debug, Getters)]
#[get = "pub"]
pub struct Method {
    name: String,
    declaration: FunctionDeclaration,
    receive_pointers_parameter_names: Vec<String>,
}

#[derive(Debug, Getters)]
#[get = "pub"]
pub struct Class {
    name: String,
    ffi_class_name: String,
    destructor: Option<FunctionDeclaration>,
    methods: Vec<Method>,
}

impl Class {
    pub fn new<'a>(
        name: &str,
        ffi_class_name: &str,
        function_name_captures_regex: Regex,
        function_name_exclude_regex: Option<Regex>,
        functions_iter: impl Iterator<Item = &'a FunctionDeclaration>,
        destructor: Option<&str>,
        function_receive_pointers_parameter_names: Vec<(&str, &str)>,
    ) -> Self {
        Class {
            name: name.to_owned(),
            ffi_class_name: ffi_class_name.to_owned(),
            destructor: None,
            methods: Vec::new(),
        }
        .tap(|class| {
            for function_declaration in functions_iter {
                if class.destructor.is_none() {
                    if let Some(destructor_name) = destructor {
                        if function_declaration.name() == destructor_name {
                            class.destructor = Some(function_declaration.to_owned());
                            continue;
                        }
                    }
                }
                if !function_name_exclude_regex
                    .as_ref()
                    .map(|r| r.is_match(function_declaration.name()))
                    .unwrap_or(false)
                {
                    if let Some(captures) = function_name_captures_regex.captures(function_declaration.name()) {
                        if class.destructor.is_none()
                            && destructor.is_none()
                            && function_declaration.name().ends_with("_free")
                        {
                            class.destructor = Some(function_declaration.to_owned());
                        } else if Some(function_declaration.name()) != class.destructor.as_ref().map(|c| c.name()) {
                            class.methods.push(Method {
                                name: captures
                                    .get(1)
                                    .expect("Captures at lease 1 part as method name")
                                    .as_str()
                                    .to_owned(),
                                declaration: function_declaration.to_owned(),
                                receive_pointers_parameter_names: function_receive_pointers_parameter_names
                                    .iter()
                                    .filter_map(|&(func_name, param_name)| {
                                        if func_name == function_declaration.name() {
                                            Some(param_name.to_owned())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect(),
                            });
                        }
                    }
                }
            }
        })
    }
}

pub fn dump_classifier(classifier: &Classifier) -> Result<()> {
    dump(classifier, CodeWriter::new(Writer::Stdout(stdout()), 4, 0))?;
    return Ok(());

    fn dump(classifier: &Classifier, mut output: CodeWriter) -> Result<CodeWriter> {
        for class in classifier.classes().iter() {
            output.write(class.name())?;
            output = output.try_with_next_level(|mut output| {
                output.write(&format!(
                    "destructor: {:?}",
                    class.destructor().as_ref().map(|f| f.name())
                ))?;
                output.write("methods:")?;
                for method in class.methods().iter() {
                    output = output.try_with_next_level(|mut output| {
                        output.write(&format!("- {}: {}", method.name(), method.declaration().name()))?;
                        Ok(output)
                    })?;
                }
                Ok(output)
            })?;
        }
        Ok(output)
    }
}
