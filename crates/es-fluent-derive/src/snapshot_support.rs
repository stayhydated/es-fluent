use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn pretty_file_tokens(tokens: TokenStream) -> String {
    let file = syn::parse2(tokens).expect("generated tokens should parse as a Rust file");
    prettyplease::unparse(&file).trim().to_string()
}

pub(crate) fn pretty_block_tokens(tokens: TokenStream) -> String {
    let file: syn::File = syn::parse2(quote! {
        fn __snapshot() {
            #tokens
        }
    })
    .expect("generated tokens should parse inside a function body");

    extract_wrapped_body(prettyplease::unparse(&file), 1, 1, "    ")
}

pub(crate) fn pretty_match_arm_tokens(tokens: TokenStream) -> String {
    let file: syn::File = syn::parse2(quote! {
        fn __snapshot() {
            match __snapshot_value {
                #tokens
            }
        }
    })
    .expect("generated tokens should parse as match arms");

    extract_wrapped_body(prettyplease::unparse(&file), 2, 2, "        ")
}

fn extract_wrapped_body(
    rendered: String,
    leading_lines: usize,
    trailing_lines: usize,
    indent: &str,
) -> String {
    let lines: Vec<_> = rendered.lines().collect();
    let body = &lines[leading_lines..lines.len() - trailing_lines];

    body.iter()
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}
