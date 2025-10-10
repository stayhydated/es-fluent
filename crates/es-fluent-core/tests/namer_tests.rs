use es_fluent_core::namer::FluentKey;

fn ident(name: &str) -> syn::Ident {
    syn::parse_str::<syn::Ident>(name).expect("valid ident")
}

#[test]
fn fluent_key_formats_snapshot() {
    let cases: Vec<FluentKey> = vec![
        FluentKey::new(&ident("MyStruct"), ""),
        FluentKey::new(&ident("MyStruct"), "Field"),
        FluentKey::new(&ident("HTTPServer"), ""),
        FluentKey::new(&ident("MyEnum"), "Variant"),
        FluentKey::new(&ident("already_snake"), "field_name"),
        FluentKey::new(&ident("X"), ""),
    ];

    insta::assert_ron_snapshot!("fluent_key_formats_snapshot", &cases);
}
