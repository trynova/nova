use tokenizer::{TokenKind, TokenStream};

pub fn main() {
    let input = "function foo() { return 'bar'; }";
    let mut stream = TokenStream::new(input.as_bytes());

    loop {
        let token = stream.next();
        println!("{:?}", token);
        if token.kind == TokenKind::Eof {
            break;
        }
    }
}
