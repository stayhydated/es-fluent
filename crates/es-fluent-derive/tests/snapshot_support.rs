pub fn normalized_debug_snapshot(value: &impl std::fmt::Debug) -> String {
    let debug = format!("{:#?}", value);
    let mut normalized = Vec::new();
    let lines: Vec<_> = debug.lines().collect();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();

        if trimmed.ends_with("Ident {")
            && let (Some(sym_line), Some(close_line)) = (lines.get(index + 1), lines.get(index + 2))
            && let Some(sym) = sym_line
                .trim()
                .strip_prefix("sym: ")
                .and_then(|value| value.strip_suffix(','))
        {
            let prefix = line.split("Ident {").next().unwrap_or_default();
            let sym_indent = sym_line
                .chars()
                .take_while(|char| char.is_whitespace())
                .collect::<String>();
            let close_indent = close_line
                .chars()
                .take_while(|char| char.is_whitespace())
                .collect::<String>();
            let trailing = if close_line.trim().ends_with(',') {
                ","
            } else {
                ""
            };

            normalized.push(format!("{prefix}Ident("));
            normalized.push(format!("{sym_indent}{sym},"));
            normalized.push(format!("{close_indent}){trailing}"));
            index += 3;
            continue;
        }

        if trimmed.starts_with("bytes(") {
            let prefix = line
                .chars()
                .take_while(|char| char.is_whitespace())
                .collect::<String>();
            let trailing = if trimmed.ends_with(',') { "," } else { "" };
            normalized.push(format!("{prefix}Span{trailing}"));
            index += 1;
            continue;
        }

        normalized.push(line.to_string());
        index += 1;
    }

    normalized.join("\n")
}
