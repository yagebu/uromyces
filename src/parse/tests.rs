use std::fs;

#[test]
fn glob_input_files() {
    insta::glob!("test_inputs/*.beancount", |path| {
        let input = fs::read_to_string(path).unwrap();
        insta::assert_debug_snapshot!(super::parse_string(&input, &None));
    });
}
