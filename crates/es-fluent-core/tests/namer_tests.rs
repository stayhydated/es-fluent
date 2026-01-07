use es_fluent_core::namer::FluentKey;

fn ident(name: &str) -> syn::Ident {
    syn::parse_str::<syn::Ident>(name).expect("valid ident")
}

#[test]
fn fluent_key_formats_snapshot() {
    let cases: Vec<FluentKey> = vec![
        FluentKey::from(&ident("MyStruct")),
        FluentKey::from(&ident("MyStruct")).join("Field"),
        FluentKey::from(&ident("HTTPServer")),
        FluentKey::from(&ident("MyEnum")).join("Variant"),
        FluentKey::from(&ident("already_snake")).join("field_name"),
        FluentKey::from(&ident("X")),
    ];

    insta::assert_debug_snapshot!("fluent_key_formats_snapshot", &cases);
}

#[test]
fn fluent_key_with_base_allows_custom_roots() {
    let key = FluentKey::from("es-fluent-lang").join("en-US");
    assert_eq!(key.to_string(), "es-fluent-lang-en-US");
    let root_only = FluentKey::from("es-fluent-lang");
    assert_eq!(root_only.to_string(), "es-fluent-lang");
}
