use crate::scanner::{Scanner, TokenType};

pub struct Compiler {}

impl Compiler {
    pub fn compile(code: &str) {
        let mut scanner = Scanner::init_scanner(code.as_bytes());

        let line: i32 = -1;
        loop {
            let scanner_start = scanner.start.clone();
            let token = scanner.scan_token();

            // println!("{:?}", token);
            // println!("{:?}", scanner_start);
            // for i in 0..scanner_start.len() {
            //     print!("{}", scanner_start[i] as char);
            // }

            // if token.line as i32 != line {
            //     println!("{:4} ", token.line);
            // } else {
            //     print!("    | ")
            // }

            // println!(
            //     "{:2} '{}'",
            //     token.token_type,
            //     token.length
            // );

            if token.token_type == TokenType::Eof {
                break;
            }
        }
    }
}
