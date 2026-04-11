/// M170 — `observer` item: parser + codegen tests.
///
/// `observer Name type: T end`
/// generates a `{Name}Observer<T>` struct with subscribe/notify/get.
/// GoF Observer pattern (Gamma et al. 1994).

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

#[test]
fn m170_observer_parses() {
    let out = compile(r#"
module ui
observer TemperatureObserver
  type: Float
end
end
"#);
    assert!(out.contains("TemperatureObserverObserver"), "expected struct\n{out}");
}

#[test]
fn m170_struct_value_field() {
    let out = compile(r#"
module ui
observer CountObserver
  type: Int
end
end
"#);
    assert!(out.contains("pub value: T"), "missing value field\n{out}");
}

#[test]
fn m170_new_takes_initial_value() {
    let out = compile(r#"
module ui
observer PriceObserver
  type: Float
end
end
"#);
    assert!(out.contains("pub fn new(initial: T) -> Self"), "missing new(initial)\n{out}");
}

#[test]
fn m170_get_method_emitted() {
    let out = compile(r#"
module ui
observer StatusObserver
  type: String
end
end
"#);
    assert!(out.contains("pub fn get(&self) -> &T"), "missing get()\n{out}");
}

#[test]
fn m170_notify_method_emitted() {
    let out = compile(r#"
module ui
observer StatusObserver
  type: String
end
end
"#);
    assert!(out.contains("pub fn notify(&mut self, new_value: T)"), "missing notify()\n{out}");
}

#[test]
fn m170_subscribe_method_emitted() {
    let out = compile(r#"
module ui
observer StatusObserver
end
end
"#);
    assert!(out.contains("pub fn subscribe<F: Fn(&T)>"), "missing subscribe()\n{out}");
}

#[test]
fn m170_default_type_is_string() {
    let out = compile(r#"
module ui
observer SimpleObserver
end
end
"#);
    assert!(out.contains("Observer<T = String>"), "default type should be String\n{out}");
}

#[test]
fn m170_audit_comment_emitted() {
    let out = compile(r#"
module ui
observer SimpleObserver
end
end
"#);
    assert!(out.contains("LOOM[observer:behavioral]"), "missing audit comment\n{out}");
    assert!(out.contains("M170"), "missing M170 reference\n{out}");
    assert!(out.contains("GoF"), "missing GoF attribution\n{out}");
}

#[test]
fn m170_struct_derive_attrs() {
    let out = compile(r#"
module ui
observer SimpleObserver
end
end
"#);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

#[test]
fn m170_multiple_observers() {
    let out = compile(r#"
module reactive
observer TemperatureObserver
  type: Float
end
observer StatusObserver
  type: String
end
end
"#);
    assert!(out.contains("TemperatureObserverObserver"), "missing temp\n{out}");
    assert!(out.contains("StatusObserverObserver"), "missing status\n{out}");
}

#[test]
fn m170_get_returns_ref_to_value() {
    let out = compile(r#"
module ui
observer PriceObserver
end
end
"#);
    assert!(out.contains("&self.value"), "get must return &self.value\n{out}");
}

#[test]
fn m170_mixed_with_fallback() {
    let out = compile(r#"
module ui
observer StatusObserver
  type: String
end
fallback DefaultStatus
  value: "unknown"
end
end
"#);
    assert!(out.contains("StatusObserverObserver"), "missing observer\n{out}");
    assert!(out.contains("DefaultStatusFallback"), "missing fallback\n{out}");
}
