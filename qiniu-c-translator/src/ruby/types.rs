use super::utils::{
    find_function_pointer_type_for_callback_name, find_type_constants, normalize_constant, ConstantType,
};
use crate::ast::{Type, TypeKind};
use clang::TypeKind as ClangTypeKind;
use matches::matches;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub(super) enum BaseType {
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

pub(super) enum StructFieldType {
    BaseType(BaseType),
    ByVal(String),
    ByPtr(String),
    ByCallback(String),
}

impl StructFieldType {
    pub(super) fn from(c_type: Type) -> Self {
        return match c_type.type_kind() {
            TypeKind::Typedef { .. } => Self::new_type_by_val(normalize_constant(c_type.display_name())),
            TypeKind::Pointer { subtype: pointer_type } => match pointer_type.type_kind() {
                TypeKind::Typedef { .. } => Self::ByPtr(normalize_constant(pointer_type.display_name())),
                TypeKind::Function { .. } => {
                    if let Some(callback_name) =
                        find_function_pointer_type_for_callback_name(pointer_type.display_name())
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

    pub(super) fn new_type_by_val(t: String) -> Self {
        if find_type_constants(&t).is_none() {
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
}

impl fmt::Display for StructFieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BaseType(type_name) => type_name.to_symbol().fmt(f),
            Self::ByVal(type_name) => match find_type_constants(type_name) {
                Some(ConstantType::Struct) => write!(f, "{}.by_value", type_name),
                _ => type_name.fmt(f),
            },
            Self::ByPtr(type_name) => match find_type_constants(type_name) {
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
