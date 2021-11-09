use proc_macro2::TokenStream;

pub(super) trait CodeGenerator {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream;
}
