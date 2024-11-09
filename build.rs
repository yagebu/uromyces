use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

use regex::{Captures, Regex};

/// Update the constants in the given file to match the numbers in the tree-sitter grammar's
/// parser.c. This is used for both the "kinds" of the nodes as well as the literal names of
/// "fields" on the nodes.
fn update_consts(path: &Path, kind: &str) {
    let parser: PathBuf = ["tree-sitter-beancount", "parser.c"].iter().collect();
    let parser_contents = &read_to_string(parser).unwrap();
    let find_consts = Regex::new(r"const ([A-Z_]+): u16 = (\d+);").unwrap();

    // Update the constants for all node fields.
    let mut changed = false;
    let contents = find_consts
        .replace_all(&read_to_string(path).unwrap(), |caps: &Captures| {
            let const_name = caps.get(1).unwrap().as_str();
            let num_match = caps.get(2).unwrap();
            let num = num_match.as_str();
            let re = &format!(r" {}_{} = (\d+)", kind, &const_name.to_ascii_lowercase());
            let new_num = Regex::new(re)
                .unwrap()
                .captures(parser_contents)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str();
            if new_num != num {
                changed = true;
            }
            format!("const {}: u16 = {};", const_name, new_num)
        })
        .to_string();
    if changed {
        write(path, contents).unwrap();
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/parse/node_fields.rs");
    println!("cargo:rerun-if-changed=src/parse/node_ids.rs");
    println!("cargo:rerun-if-changed=tree-sitter-beancount/parser.c");

    // Update the constants for all node fields.
    let node_fields: PathBuf = ["src", "parse", "node_fields.rs"].iter().collect();
    update_consts(&node_fields, "field");
    // Update the constants for all node ids.
    let node_ids: PathBuf = ["src", "parse", "node_ids.rs"].iter().collect();
    update_consts(&node_ids, "sym");

    let dir: PathBuf = ["tree-sitter-beancount"].iter().collect();
    let parser = &dir.join("parser.c");

    cc::Build::new()
        .include(&dir)
        .file(parser)
        // Enable C99 mode - required for tree-sitter, does not seem to be default in
        // e.g. the aarch64 maturin CI build
        .flag("-std=c99")
        .compile("tree-sitter-beancount");
}
