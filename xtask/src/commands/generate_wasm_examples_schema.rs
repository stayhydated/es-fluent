use crate::{
    util::workspace_root,
    wasm_examples::{write_schema, SCHEMA_REFERENCE},
};

pub fn run() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let schema_path = write_schema(&workspace_root)?;
    println!("Wrote wasm example schema to {}", schema_path.display());
    println!("Manifest should reference {}", SCHEMA_REFERENCE);
    Ok(())
}
