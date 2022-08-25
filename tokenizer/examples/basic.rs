use tokenizer::{TokenKind, TokenStream};

pub fn main() {
    let input = "2 == 4";
    let mut stream = TokenStream::new(input.as_bytes());

		let mut last_end: u32 = 0;
    loop {
        let token = stream.next();
        print!(" '{}'\n{:?}:", &input[last_end as usize..token.start as usize], token.kind);
        last_end = token.start;
				if token.kind == TokenKind::Eof {
            break;
        }
    }
}
