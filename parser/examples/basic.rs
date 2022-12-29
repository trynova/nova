use parser::{
    lexer::{Lexer, Token},
    parser::Parser,
};

pub fn main() {
    let input = r#" let abc = 5"#;

    let mut parser = Parser::new(input);

    println!("{:?}", parser.parse_scope(true))
}
