use crate::{
    ast::{FunctionType, Type, TypeKind},
    utils::RandomIdentifier,
};
use clang::TypeKind as ClangTypeKind;
use heck::CamelCase;
use lazy_static::lazy_static;
use matches::matches;
use std::{collections::HashMap, sync::Mutex};

#[derive(Copy, Clone)]
pub(super) enum ConstantType {
    Enum,
    Struct,
}

lazy_static! {
    static ref TYPE_CONSTANTS: Mutex<HashMap<String, ConstantType>> = Default::default();
    static ref FUNCTION_POINTER_DEFS: Mutex<HashMap<String, String>> = Default::default();
    static ref IDENTIFIER_GENERATOR: RandomIdentifier = Default::default();
}

pub(super) fn insert_type_constants(name: impl Into<String>, constant_type: ConstantType) {
    TYPE_CONSTANTS.lock().unwrap().insert(name.into(), constant_type);
}

pub(super) fn find_type_constants(name: impl AsRef<str>) -> Option<ConstantType> {
    TYPE_CONSTANTS.lock().unwrap().get(name.as_ref()).cloned()
}

pub(super) fn insert_function_pointer_type_callback_name_map(
    function_type: &FunctionType,
    callback_name: impl Into<String>,
) {
    FUNCTION_POINTER_DEFS
        .lock()
        .unwrap()
        .insert(function_type.display_name().to_owned(), callback_name.into());
}

pub(super) fn find_function_pointer_type_for_callback_name(type_name: impl AsRef<str>) -> Option<String> {
    FUNCTION_POINTER_DEFS.lock().unwrap().get(type_name.as_ref()).cloned()
}

pub(super) fn get_random_constant_identifier() -> String {
    IDENTIFIER_GENERATOR.upper_camel_case()
}

pub(super) fn get_random_field_identifier() -> String {
    IDENTIFIER_GENERATOR.snack_case()
}

pub(super) fn normalize_constant(name: impl AsRef<str>) -> String {
    name.as_ref().split(' ').last().unwrap().to_camel_case()
}

pub(super) fn try_to_extract_function_type(t: &Type) -> Option<&FunctionType> {
    if let TypeKind::Pointer { subtype } = t.type_kind() {
        if let TypeKind::Function { subtype } = subtype.type_kind() {
            return Some(&subtype);
        }
    }
    None
}
pub(super) fn is_const_str_type(t: &Type) -> bool {
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

pub(super) fn is_const_binary_type(t: &Type) -> bool {
    if let TypeKind::Pointer { subtype } = t.type_kind() {
        match subtype.type_kind() {
            TypeKind::Base(ClangTypeKind::Void) if subtype.is_const() => true,
            _ => false,
        }
    } else {
        false
    }
}

pub(super) fn is_const_str_list_type(t: &Type) -> bool {
    if let TypeKind::Pointer { subtype } = t.type_kind() {
        subtype.is_const() && is_const_str_type(&subtype)
    } else {
        false
    }
}

pub(super) fn is_size_type(t: &Type) -> bool {
    if let TypeKind::Typedef { subtype } = t.type_kind() {
        matches!(subtype.type_kind(), TypeKind::Base(ClangTypeKind::ULong)) && t.display_name().as_str() == "size_t"
    } else {
        matches!(t.type_kind(), TypeKind::Base(ClangTypeKind::Int))
    }
}

pub(super) fn try_to_extract_pointer_type_name(t: &Type) -> Option<String> {
    if let TypeKind::Pointer { subtype: pointer_type } = t.type_kind() {
        return try_to_extract_typedef_type_name(&pointer_type);
    }
    None
}

pub(super) fn try_to_extract_typedef_type_name(t: &Type) -> Option<String> {
    if let TypeKind::Typedef { subtype: def_type } = t.type_kind() {
        if matches!(def_type.type_kind(), TypeKind::Base(ClangTypeKind::Record)) {
            return Some(normalize_constant(t.display_name()));
        }
    }
    None
}
