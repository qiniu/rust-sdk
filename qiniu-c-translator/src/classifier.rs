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
}

#[derive(Debug, Getters)]
#[get = "pub"]
pub struct Class {
    name: String,
    ffi_class_name: String,
    constructor: Option<FunctionDeclaration>,
    destructor: Option<FunctionDeclaration>,
    methods: Vec<Method>,
}

impl Class {
    pub fn new<'a>(
        name: impl Into<String>,
        ffi_class_name: impl Into<String>,
        function_name_captures_regex: Regex,
        function_name_exclude_regex: Option<Regex>,
        functions_iter: impl Iterator<Item = &'a FunctionDeclaration>,
        constructor: Option<&str>,
        destructor: Option<&str>,
    ) -> Self {
        Class {
            name: name.into(),
            ffi_class_name: ffi_class_name.into(),
            constructor: None,
            destructor: None,
            methods: Vec::new(),
        }
        .tap(|class| {
            for function_declaration in functions_iter {
                if class.constructor.is_none() {
                    if let Some(constructor_name) = constructor {
                        if function_declaration.name() == constructor_name {
                            class.constructor = Some(function_declaration.to_owned());
                        }
                    }
                }
                if class.destructor.is_none() {
                    if let Some(destructor_name) = destructor {
                        if function_declaration.name() == destructor_name {
                            class.destructor = Some(function_declaration.to_owned());
                        }
                    }
                }
                if !function_name_exclude_regex
                    .as_ref()
                    .map(|r| r.is_match(function_declaration.name()))
                    .unwrap_or(false)
                {
                    if let Some(captures) = function_name_captures_regex.captures(function_declaration.name()) {
                        if class.constructor.is_none()
                            && constructor.is_none()
                            && function_declaration.name().ends_with("_new")
                        {
                            class.constructor = Some(function_declaration.to_owned());
                        } else if class.destructor.is_none()
                            && destructor.is_none()
                            && function_declaration.name().ends_with("_free")
                        {
                            class.destructor = Some(function_declaration.to_owned());
                        } else {
                            class.methods.push(Method {
                                name: captures
                                    .get(1)
                                    .expect("Captures at lease 1 part as method name")
                                    .as_str()
                                    .to_owned(),
                                declaration: function_declaration.to_owned(),
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
                    "constructor: {:?}",
                    class.constructor().as_ref().map(|f| f.name())
                ))?;
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
