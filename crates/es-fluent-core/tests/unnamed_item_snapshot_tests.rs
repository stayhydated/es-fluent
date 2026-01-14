use es_fluent_derive_core::namer::UnnamedItem;

#[test]
fn unnamed_item_formats_snapshot() {
    let indices = [
        0usize, 1, 2, 3, 4, 5, 7, 8, 9, 10, 15, 31, 32, 63, 64, 99, 100, 101, 255, 256, 511, 512,
        1023, 1024,
    ];

    let cases: Vec<(usize, String, String)> = indices
        .iter()
        .map(|&i| {
            let item: UnnamedItem = i.into();
            let display = item.to_string();
            let ident_str = item.to_ident().to_string();
            (i, display, ident_str)
        })
        .collect();

    insta::assert_debug_snapshot!("unnamed_item_formats_snapshot", &cases);
}
