/// M172 — `scheduler` item: parser + codegen tests.
///
/// `scheduler Name interval: N unit: ms|s|min end`
/// generates a `{Name}Scheduler` struct with run/stop methods.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

#[test]
fn m172_scheduler_parses() {
    let out = compile(r#"
module tasks
scheduler HeartbeatScheduler
  interval: 30
  unit: s
end
end
"#);
    assert!(out.contains("HeartbeatSchedulerScheduler"), "expected struct\n{out}");
}

#[test]
fn m172_struct_fields_emitted() {
    let out = compile(r#"
module tasks
scheduler CleanupScheduler
  interval: 5
  unit: min
end
end
"#);
    assert!(out.contains("pub interval: u64"), "missing interval\n{out}");
    assert!(out.contains("pub unit: &'static str"), "missing unit\n{out}");
}

#[test]
fn m172_new_uses_configured_values() {
    let out = compile(r#"
module tasks
scheduler PollScheduler
  interval: 500
  unit: ms
end
end
"#);
    assert!(out.contains("interval: 500"), "interval not 500\n{out}");
    assert!(out.contains("unit: \"ms\""), "unit not ms\n{out}");
}

#[test]
fn m172_default_values_when_omitted() {
    let out = compile(r#"
module tasks
scheduler SimpleScheduler
end
end
"#);
    assert!(out.contains("interval: 1"), "default interval should be 1\n{out}");
    assert!(out.contains("unit: \"s\""), "default unit should be s\n{out}");
}

#[test]
fn m172_run_method_emitted() {
    let out = compile(r#"
module tasks
scheduler HeartbeatScheduler
  interval: 30
  unit: s
end
end
"#);
    assert!(out.contains("pub fn run<F: Fn()>(&self, _task: F)"), "missing run()\n{out}");
}

#[test]
fn m172_stop_method_emitted() {
    let out = compile(r#"
module tasks
scheduler HeartbeatScheduler
  interval: 30
end
end
"#);
    assert!(out.contains("pub fn stop(&mut self)"), "missing stop()\n{out}");
}

#[test]
fn m172_audit_comment_emitted() {
    let out = compile(r#"
module tasks
scheduler SimpleScheduler
end
end
"#);
    assert!(out.contains("LOOM[scheduler:behavioral]"), "missing audit comment\n{out}");
    assert!(out.contains("M172"), "missing M172 reference\n{out}");
}

#[test]
fn m172_struct_derive_attrs() {
    let out = compile(r#"
module tasks
scheduler SimpleScheduler
end
end
"#);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

#[test]
fn m172_ms_unit_parses() {
    let out = compile(r#"
module tasks
scheduler FastPoller
  interval: 100
  unit: ms
end
end
"#);
    assert!(out.contains("unit: \"ms\""), "unit should be ms\n{out}");
    assert!(out.contains("interval: 100"), "interval not 100\n{out}");
}

#[test]
fn m172_min_unit_parses() {
    let out = compile_check(r#"
module tasks
scheduler SlowCleaner
  interval: 60
  unit: min
end
end
"#);
    assert!(out.is_ok(), "min unit should parse\n{:?}", out.err());
    assert!(out.unwrap().contains("unit: \"min\""), "unit should be min");
}

#[test]
fn m172_multiple_schedulers() {
    let out = compile(r#"
module tasks
scheduler HeartbeatScheduler
  interval: 30
  unit: s
end
scheduler CleanupScheduler
  interval: 1
  unit: min
end
end
"#);
    assert!(out.contains("HeartbeatSchedulerScheduler"), "missing heartbeat\n{out}");
    assert!(out.contains("CleanupSchedulerScheduler"), "missing cleanup\n{out}");
}

#[test]
fn m172_mixed_with_observer() {
    let out = compile(r#"
module reactive
scheduler MetricsScheduler
  interval: 60
  unit: s
end
observer MetricsObserver
  type: String
end
end
"#);
    assert!(out.contains("MetricsSchedulerScheduler"), "missing scheduler\n{out}");
    assert!(out.contains("MetricsObserverObserver"), "missing observer\n{out}");
}
