use proc_macro2::TokenStream;

pub(crate) fn pretty_file_tokens(tokens: TokenStream) -> String {
    let file = syn::parse2(tokens).expect("generated tokens should parse as a Rust file");
    prettyplease::unparse(&file).trim().to_string()
}

pub(crate) fn with_i18n_domains<T>(domains: &[&str], f: impl FnOnce() -> T) -> T {
    let temp_dir = tempfile::TempDir::new().expect("create temporary manifest directory");
    let domains = domains
        .iter()
        .map(|domain| format!("\"{domain}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs_err::write(
        temp_dir.path().join("i18n.toml"),
        format!("fallback_language = \"en-US\"\nassets_dir = \"i18n\"\ndomains = [{domains}]\n"),
    )
    .expect("write i18n.toml");

    temp_env::with_var("CARGO_MANIFEST_DIR", Some(temp_dir.path()), f)
}
