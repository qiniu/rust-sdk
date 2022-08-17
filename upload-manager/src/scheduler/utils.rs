use super::super::ReinitializeOptions;

pub(super) fn keep_original_region_options() -> ReinitializeOptions {
    ReinitializeOptions::builder().keep_original_region().build()
}
