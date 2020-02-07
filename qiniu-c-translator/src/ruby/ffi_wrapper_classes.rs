use super::{
    ast::{Context, Method, MethodCall, Module, Proc, RawCode},
    utils::{
        is_const_binary_type, is_const_str_list_type, is_const_str_type, is_size_type, normalize_constant,
        try_to_extract_function_type, try_to_extract_receiver_pointer_type_name, try_to_extract_typedef_type_name,
    },
    CORE_FFI_MODULE_NAME,
};
use crate::{
    ast::{FunctionType, ParameterDeclaration, Type, TypeKind},
    classifier::{Class, Classifier},
    utils::{CodeGenerator, RandomIdentifier},
};
use clang::TypeKind as ClangTypeKind;
use matches::matches;
use tap::TapOps;

const CLASS_CONTEXT_INSTANCE_VARIABLE_NAME: &str = "@instance";
const CONSTRUCTOR_METHOD_NAME: &str = "initialize";
const DESTRUCTOR_METHOD_NAME: &str = "finalize";

pub(super) fn insert_ffi_wrapper_classes(classifier: &Classifier, module: &mut Module) {
    for class in classifier.classes().iter() {
        module
            .sub_nodes_mut()
            .push(Box::new(Module::new(class.name().to_owned(), true)).tap(|new_class| {
                new_class.sub_nodes_mut().push(Box::new(RawCode::new(format!(
                    "attr_reader :{}",
                    CLASS_CONTEXT_INSTANCE_VARIABLE_NAME.get(1..).unwrap()
                ))));
                insert_constructor_node(class, new_class.sub_nodes_mut());
                insert_destructor_node(class, new_class.sub_nodes_mut());
                insert_methods_nodes(class, classifier.classes(), new_class.sub_nodes_mut());
            }));
    }
}

fn insert_constructor_node(class: &Class, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    nodes.push(
        Box::new(Method::new(CONSTRUCTOR_METHOD_NAME, false)).tap(|initializer| {
            *initializer.parameter_names_mut() = vec!["instance".to_owned()];
            initializer.insert_asc_node(Box::new(RawCode::new(format!(
                "{} = instance",
                CLASS_CONTEXT_INSTANCE_VARIABLE_NAME
            ))));
            if class.destructor().is_some() {
                initializer.insert_asc_node(Box::new(RawCode::new(format!(
                    "ObjectSpace.define_finalizer(self, self.class.{}({}))",
                    DESTRUCTOR_METHOD_NAME, CLASS_CONTEXT_INSTANCE_VARIABLE_NAME
                ))));
            }
        }),
    );
}

fn insert_destructor_node(class: &Class, nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    if let Some(destructor) = class.destructor().as_ref() {
        nodes.push(Box::new(Method::new(DESTRUCTOR_METHOD_NAME, true)).tap(|finalizer| {
            finalizer.insert_asc_node(Box::new(Proc::new(None, false)).tap(|proc| {
                proc.sub_nodes_mut().push(
                    Box::new(MethodCall::new(
                        Some(Context::Modules(vec![CORE_FFI_MODULE_NAME.to_owned()])),
                        destructor.name(),
                    ))
                    .tap(|finalizer_method_call| {
                        *finalizer_method_call.parameter_names_mut() = vec!["instance".to_owned()];
                    }),
                )
            }));
            *finalizer.parameter_names_mut() = vec!["instance".to_owned()];
        }));
    }
}

fn insert_methods_nodes(class: &Class, classes: &[Class], nodes: &mut Vec<Box<dyn CodeGenerator>>) {
    for method in class.methods().iter() {
        let method_name = if method.name().as_str() == "new" {
            "new!".to_owned()
        } else {
            method.name().to_owned()
        };
        let is_instance_method = if let Some(first_parameter) = method.declaration().parameters().first() {
            is_current_class_matches_type_by_value(first_parameter.parameter_type(), class)
        } else {
            false
        };
        let identifier_generator = RandomIdentifier::new();
        nodes.push(
            Box::new(Method::new(method_name, !is_instance_method)).tap(|new_method| {
                let parameters = if is_instance_method {
                    method.declaration().parameters().get(1..).unwrap_or(&[])
                } else {
                    method.declaration().parameters()
                };
                let return_type = method.declaration().return_type();

                let mut method_call = MethodCall::new(
                    Some(Context::Modules(vec![CORE_FFI_MODULE_NAME.to_owned()])),
                    method.declaration().name(),
                );

                if is_instance_method {
                    // 对于实例方法，在每个方法调用时的第一个参数设置为 `@instance`
                    *method_call.parameter_names_mut() = vec![CLASS_CONTEXT_INSTANCE_VARIABLE_NAME.to_owned()];
                }

                normalize_parameters_and_insert_into_method_and_method_call(
                    new_method,
                    parameters,
                    return_type,
                    method.receive_pointers_parameter_names(),
                    classes,
                    method_call,
                    &identifier_generator,
                );
            }),
        );
    }
}

fn is_current_class_matches_type_by_value(current_type: &Type, class: &Class) -> bool {
    if let TypeKind::Typedef { subtype } = current_type.type_kind() {
        matches!(subtype.type_kind(), TypeKind::Base(ClangTypeKind::Record))
            && class.ffi_class_name().as_str() == current_type.display_name().as_str()
    } else {
        false
    }
}

fn normalize_parameters_and_insert_into_method_and_method_call(
    method: &mut Method,
    parameters: &[ParameterDeclaration],
    return_type: &Type,
    receive_pointers_parameter_names: &[String],
    classes: &[Class],
    mut method_call: MethodCall,
    identifier_generator: &RandomIdentifier,
) {
    let mut skip = 0usize;
    for (i, parameter) in parameters.iter().enumerate() {
        if skip > 0 {
            skip -= 1;
            continue;
        }

        let cur_param_type = parameter.parameter_type();
        let next_param_type = parameters.get(i + 1).map(|next| next.parameter_type());
        if is_const_str_type(cur_param_type) {
            // 对于参数列表中含有字符串的参数，添加一条语句将参数转换为 UTF-8 编码
            skip += convert_str_encoding_and_insert_into_method_and_method_call(
                parameter,
                method,
                &mut method_call,
                identifier_generator,
            );
        } else if is_const_str_list_type(cur_param_type) && next_param_type.map(is_size_type) == Some(true) {
            // 对于参数列表中含有字符串列表和其尺寸的参数，添加一条语句将参数转换为 FFI 内存指针
            skip += convert_str_list_and_size_to_list_and_insert_into_method_and_method_call(
                parameter,
                method,
                &mut method_call,
                identifier_generator,
            );
        } else if is_const_binary_type(cur_param_type) && next_param_type.map(is_size_type) == Some(true) {
            // 对于参数列表中含有数据和其尺寸的参数，添加一条语句将参数转换为 FFI 内存指针
            skip += convert_data_and_size_to_string_and_insert_into_method_and_method_call(
                parameter,
                method,
                &mut method_call,
                identifier_generator,
            );
        } else if let Some(function_type) = try_to_extract_function_type(cur_param_type) {
            // 对于参数列表中含有回调函数的参数，添加一条创建 proc 的语句，并在 proc 里将参数正常化。
            skip += convert_callback_and_insert_into_method_and_method_call(
                parameter,
                &function_type,
                method,
                &mut method_call,
                identifier_generator,
            );
        } else if let Some(receiver_pointer_type_name) = try_to_extract_receiver_pointer_type_name(cur_param_type) {
            // 对于参数列表中含有接受数据的指针参数
            //   如果在 `receive_pointers_parameter_names` 中列举过，则表示该指针并非用于接受数据，而只是普通的指针传递数据，对其调用 `instance` 方法，以访问到内部的 CoreFFI 类的实例
            //   如果不是，则该指针确实用于接受数据，该方法的参数列表中将不会含有该参数，而在方法内自行创建，创建完毕后先尝试将数据转换为 Bindings 类的实例，然后将其作为方法的返回值。
            skip += if receive_pointers_parameter_names.contains(parameter.name()) {
                convert_bindings_instance_to_coreffi_instance(parameter, method, &mut method_call)
            } else {
                convert_pointer_to_receiver(
                    receiver_pointer_type_name,
                    method,
                    &mut method_call,
                    classes,
                    identifier_generator,
                )
            };
        } else if try_to_extract_typedef_type_name(cur_param_type).is_some() {
            // 对于参数列表中含有传送结构体值的参数，对其调用 `instance` 方法，以访问到内部的 CoreFFI 类的实例
            skip += convert_bindings_instance_to_coreffi_instance(parameter, method, &mut method_call);
        } else {
            method.parameter_names_mut().push(parameter.name().to_owned());
            method_call.parameter_names_mut().push(parameter.name().to_owned());
        }
    }
    if method_call.receiver_names().is_empty() {
        if is_const_str_type(return_type) {
            convert_returned_str_encoding(method, &mut method_call, identifier_generator);
        } else if let Some(typedef_type) = try_to_extract_typedef_type_name(return_type) {
            convert_returned_core_ffi_instance_to_bindings_instance(
                &typedef_type,
                method,
                &mut method_call,
                classes,
                identifier_generator,
            );
        }
    }
    method.insert_asc_node(Box::new(method_call));
}

fn convert_str_encoding_and_insert_into_method_and_method_call(
    parameter: &ParameterDeclaration,
    method: &mut Method,
    method_call: &mut MethodCall,
    identifier_generator: &RandomIdentifier,
) -> usize {
    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
    method.insert_asc_node(
        Box::new(MethodCall::new(
            Some(Context::Instance(parameter.name().to_owned())),
            "encode",
        ))
        .tap(|encode_method_call| {
            *encode_method_call.parameter_names_mut() = vec!["Encoding::UTF_8".to_owned()];
            *encode_method_call.receiver_names_mut() = vec![temp_pointer_variable_name.clone()];
        }),
    );
    method.parameter_names_mut().push(parameter.name().to_owned());
    method_call.parameter_names_mut().push(temp_pointer_variable_name);
    0
}

fn convert_str_list_and_size_to_list_and_insert_into_method_and_method_call(
    parameter: &ParameterDeclaration,
    method: &mut Method,
    method_call: &mut MethodCall,
    identifier_generator: &RandomIdentifier,
) -> usize {
    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
    method.insert_asc_node(
        Box::new(MethodCall::new(
            Some(Context::Modules(vec!["FFI".to_owned(), "MemoryPointer".to_owned()])),
            "new",
        ))
        .tap(|new_ffi_memory_call| {
            *new_ffi_memory_call.parameter_names_mut() =
                vec![":string".to_owned(), format!("{}.size", parameter.name())];
            *new_ffi_memory_call.receiver_names_mut() = vec![temp_pointer_variable_name.to_owned()];
        }),
    );
    method.insert_asc_node(Box::new(RawCode::new(format!(
        "{}.write_array_of_pointer({}.map {{|s| FFI::MemoryPointer.from_string(s) }})",
        temp_pointer_variable_name,
        parameter.name(),
    ))));
    method.parameter_names_mut().push(parameter.name().to_owned());
    method_call.parameter_names_mut().push(temp_pointer_variable_name);
    method_call
        .parameter_names_mut()
        .push(format!("{}.size", parameter.name()));
    1
}

fn convert_data_and_size_to_string_and_insert_into_method_and_method_call(
    parameter: &ParameterDeclaration,
    method: &mut Method,
    method_call: &mut MethodCall,
    identifier_generator: &RandomIdentifier,
) -> usize {
    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
    method.insert_asc_node(
        Box::new(MethodCall::new(
            Some(Context::Modules(vec!["FFI".to_owned(), "MemoryPointer".to_owned()])),
            "from_string",
        ))
        .tap(|ffi_memory_from_string| {
            *ffi_memory_from_string.parameter_names_mut() = vec![parameter.name().to_owned()];
            *ffi_memory_from_string.receiver_names_mut() = vec![temp_pointer_variable_name.to_owned()];
        }),
    );
    method.parameter_names_mut().push(parameter.name().to_owned());
    method_call.parameter_names_mut().push(temp_pointer_variable_name);
    method_call
        .parameter_names_mut()
        .push(format!("{}.bytesize", parameter.name()));
    1
}

fn convert_callback_and_insert_into_method_and_method_call(
    parameter: &ParameterDeclaration,
    function_type: &FunctionType,
    method: &mut Method,
    method_call: &mut MethodCall,
    identifier_generator: &RandomIdentifier,
) -> usize {
    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
    method.insert_asc_node(
        Box::new(Proc::new(Some(temp_pointer_variable_name.to_owned()), false)).tap(|in_proc| {
            *in_proc.parameter_names_mut() = function_type
                .parameter_types()
                .iter()
                .enumerate()
                .map(|(i, _)| format!("__{}", i))
                .collect();
            in_proc.sub_nodes_mut().push(
                Box::new(MethodCall::new(
                    Some(Context::Instance(parameter.name().to_owned())),
                    "call",
                ))
                .tap(|proc_call| {
                    *proc_call.parameter_names_mut() = function_type
                        .parameter_types()
                        .iter()
                        .enumerate()
                        .map(|(i, t)| {
                            if is_const_str_type(t) {
                                format!("__{}&.force_encoding(Encoding::UTF_8)", i)
                            } else {
                                format!("__{}", i)
                            }
                        })
                        .collect();
                }),
            )
        }),
    );
    method.parameter_names_mut().push(parameter.name().to_owned());
    method_call.parameter_names_mut().push(temp_pointer_variable_name);
    0
}

fn convert_bindings_instance_to_coreffi_instance(
    parameter: &ParameterDeclaration,
    method: &mut Method,
    method_call: &mut MethodCall,
) -> usize {
    method.parameter_names_mut().push(parameter.name().to_owned());
    method_call
        .parameter_names_mut()
        .push(format!("{}.instance", parameter.name()));
    0
}

fn convert_pointer_to_receiver(
    receiver_pointer_type_name: impl Into<String>,
    method: &mut Method,
    method_call: &mut MethodCall,
    classes: &[Class],
    identifier_generator: &RandomIdentifier,
) -> usize {
    let receiver_pointer_type_name = receiver_pointer_type_name.into();
    let mut temp_pointer_variable_name = identifier_generator.lower_camel_case();
    method.insert_asc_node(
        Box::new(MethodCall::new(
            Some(Context::Modules(vec![
                CORE_FFI_MODULE_NAME.to_owned(),
                receiver_pointer_type_name.to_owned(),
            ])),
            "new",
        ))
        .tap(|new_method_call| {
            *new_method_call.receiver_names_mut() = vec![temp_pointer_variable_name.to_owned()];
        }),
    );

    method_call
        .parameter_names_mut()
        .push(temp_pointer_variable_name.to_owned());

    if let Some(bindings_class_name) =
        try_to_convert_core_ffi_class_name_to_bindings_class_name(&receiver_pointer_type_name, classes)
    {
        method.insert_desc_node(
            Box::new(MethodCall::new(
                Some(Context::Modules(vec![bindings_class_name])),
                "new",
            ))
            .tap(|new_method_call| {
                *new_method_call.parameter_names_mut() = vec![temp_pointer_variable_name.to_owned()];
                temp_pointer_variable_name = identifier_generator.lower_camel_case();
                *new_method_call.receiver_names_mut() = vec![temp_pointer_variable_name.to_owned()];
            }),
        );
    }

    method.return_names_mut().push(temp_pointer_variable_name);
    0
}

fn convert_returned_str_encoding(
    method: &mut Method,
    method_call: &mut MethodCall,
    identifier_generator: &RandomIdentifier,
) {
    let temp_pointer_variable_name = identifier_generator.lower_camel_case();
    *method_call.receiver_names_mut() = vec![temp_pointer_variable_name.to_owned()];
    method.insert_desc_node(
        Box::new(MethodCall::new(
            Some(Context::Instance(temp_pointer_variable_name)),
            "force_encoding",
        ))
        .tap(|encode_method_call| {
            *encode_method_call.parameter_names_mut() = vec!["Encoding::UTF_8".to_owned()];
        }),
    );
}

fn convert_returned_core_ffi_instance_to_bindings_instance(
    typedef_type_name: &str,
    method: &mut Method,
    method_call: &mut MethodCall,
    classes: &[Class],
    identifier_generator: &RandomIdentifier,
) {
    if let Some(typedef_type_name) =
        try_to_convert_core_ffi_class_name_to_bindings_class_name(typedef_type_name, classes)
    {
        let tmp_var_name_for_method_call_return_value = identifier_generator.lower_camel_case();
        let tmp_var_name_for_method_return_value = identifier_generator.lower_camel_case();
        *method_call.receiver_names_mut() = vec![tmp_var_name_for_method_call_return_value.to_owned()];
        method.insert_desc_node(
            Box::new(MethodCall::new(Some(Context::Modules(vec![typedef_type_name])), "new")).tap(|new_method_call| {
                *new_method_call.parameter_names_mut() = vec![tmp_var_name_for_method_call_return_value];
                *new_method_call.receiver_names_mut() = vec![tmp_var_name_for_method_return_value.to_owned()];
            }),
        );
        *method.return_names_mut() = vec![tmp_var_name_for_method_return_value];
    }
}

fn try_to_convert_core_ffi_class_name_to_bindings_class_name(
    core_ffi_class_name: &str,
    classes: &[Class],
) -> Option<String> {
    classes.iter().find_map(|class| {
        if normalize_constant(class.ffi_class_name()).as_str() == core_ffi_class_name {
            Some(class.name().to_owned())
        } else {
            None
        }
    })
}
