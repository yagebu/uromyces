use std::fs;

use crate::types::Filename;

#[test]
fn glob_input_files() {
    let dummy_filename = Filename::new_dummy("string");
    insta::glob!("test_inputs/*.beancount", |path| {
        let input = fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(super::parse_string(&input, &dummy_filename.clone()));
    });
}
