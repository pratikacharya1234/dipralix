use anyhow::{Context, Result};
use tree_sitter::Parser;

pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn extract_symbols(content: &str) -> Result<Vec<Symbol>> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::language();
    parser.set_language(language)?;

    let tree = parser
        .parse(content, None)
        .context("Failed to parse code")?;
    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Basic DFS to find functions, structs, enums
    let mut stack = vec![root_node];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        match kind {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "impl_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &content[name_node.start_byte()..name_node.end_byte()];
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: kind.to_string(),
                        start_line: node.start_position().row + 1,
                        end_line: node.end_position().row + 1,
                    });
                }
            }
            _ => {}
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }

    Ok(symbols)
}
