use proc_macro2::TokenStream;

pub(crate) fn pretty_file_tokens(tokens: TokenStream) -> String {
    let file = syn::parse2(tokens).expect("generated tokens should parse as a Rust file");
    prettyplease::unparse(&file).trim().to_string()
}
