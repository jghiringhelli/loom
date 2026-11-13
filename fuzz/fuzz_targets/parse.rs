#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if let Ok(tokens) = loom::lexer::Lexer::tokenize(data) {
        let _ = loom::parser::Parser::new(&tokens).parse_module();
    }
});
