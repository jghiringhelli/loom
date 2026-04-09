fn main() {
    let src = std::fs::read_to_string("corpus/pricing_engine.loom").unwrap();
    let rust_src = loom::compile(&src).unwrap();
    println!("{}", rust_src);
}
