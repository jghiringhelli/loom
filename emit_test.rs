fn main() {
    let src = std::fs::read_to_string("corpus/pricing_engine.loom").unwrap();
    let module = loom_lang::parse(&src).unwrap();
    let rust_src = loom_lang::compile(&module).unwrap();
    println!("{}", rust_src);
}
