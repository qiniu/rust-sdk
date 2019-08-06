use num::One;
use num::Zero;
use std::cmp::Eq;

pub(crate) fn int_to_bool<T: Zero + Eq>(num: T) -> bool {
    num != num::zero::<T>()
}

pub(crate) fn bool_to_int<T: Zero + One>(yes: bool) -> T {
    if yes {
        num::one::<T>()
    } else {
        num::zero::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_int_to_bool() {
        assert!(int_to_bool(1i8));
        assert!(int_to_bool(1i16));
        assert!(int_to_bool(1i32));
        assert!(int_to_bool(1i64));
        assert!(int_to_bool(1u8));
        assert!(int_to_bool(1u16));
        assert!(int_to_bool(1u32));
        assert!(int_to_bool(1u64));
        assert!(!int_to_bool(0i8));
        assert!(!int_to_bool(0i16));
        assert!(!int_to_bool(0i32));
        assert!(!int_to_bool(0i64));
        assert!(!int_to_bool(0u8));
        assert!(!int_to_bool(0u16));
        assert!(!int_to_bool(0u32));
        assert!(!int_to_bool(0u64));
    }

    #[test]
    fn test_from_bool_to_int() {
        let a: i8 = bool_to_int(true);
        assert_eq!(a, 1i8);
        let a: i16 = bool_to_int(true);
        assert_eq!(a, 1i16);
        let a: i32 = bool_to_int(true);
        assert_eq!(a, 1i32);
        let a: i64 = bool_to_int(true);
        assert_eq!(a, 1i64);
        let a: u8 = bool_to_int(true);
        assert_eq!(a, 1u8);
        let a: u16 = bool_to_int(true);
        assert_eq!(a, 1u16);
        let a: u32 = bool_to_int(true);
        assert_eq!(a, 1u32);
        let a: u64 = bool_to_int(true);
        assert_eq!(a, 1u64);
        let a: i8 = bool_to_int(false);
        assert_eq!(a, 0i8);
        let a: i16 = bool_to_int(false);
        assert_eq!(a, 0i16);
        let a: i32 = bool_to_int(false);
        assert_eq!(a, 0i32);
        let a: i64 = bool_to_int(false);
        assert_eq!(a, 0i64);
        let a: u8 = bool_to_int(false);
        assert_eq!(a, 0u8);
        let a: u16 = bool_to_int(false);
        assert_eq!(a, 0u16);
        let a: u32 = bool_to_int(false);
        assert_eq!(a, 0u32);
        let a: u64 = bool_to_int(false);
        assert_eq!(a, 0u64);
    }
}
