use tokenizer::{Lexer, Token};

pub fn main() {
    let input = r#"asdf23 == "hello" "#;
    let mut lex = Lexer::new(input);

    loop {
        lex.next();
        println!("{:4} {:?}", lex.start, lex.token);
        if lex.token == Token::EOF {
            break;
        }
    }
}
