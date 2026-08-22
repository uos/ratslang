use super::*;
use log::{LevelFilter, Log, Metadata, Record};
use std::sync::{
    Mutex, Once,
    atomic::{AtomicBool, Ordering},
};

struct Configs {
    user: VariableHistory,
    defaults: VariableHistory,
}

struct TestLogger {
    records: Mutex<Vec<String>>,
    level: LevelFilter,
    capture: AtomicBool,
    capture_owner: Mutex<Option<std::thread::ThreadId>>,
}

impl TestLogger {
    const fn new(level: LevelFilter) -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            level,
            capture: AtomicBool::new(false),
            capture_owner: Mutex::new(None),
        }
    }

    fn clear(&self) {
        self.records.lock().unwrap().clear();
    }

    fn messages(&self) -> Vec<String> {
        self.records.lock().unwrap().clone()
    }

    fn set_capture(&self, enabled: bool) {
        if enabled {
            *self.capture_owner.lock().unwrap() = Some(std::thread::current().id());
            self.capture.store(true, Ordering::Relaxed);
        } else {
            self.capture.store(false, Ordering::Relaxed);
            *self.capture_owner.lock().unwrap() = None;
        }
    }

    fn capture_guard(&'static self) -> CaptureGuard {
        self.set_capture(true);
        CaptureGuard { logger: self }
    }
}

struct CaptureGuard {
    logger: &'static TestLogger,
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        self.logger.set_capture(false);
    }
}

impl Log for TestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.capture.load(Ordering::Relaxed)
            && self
                .capture_owner
                .lock()
                .unwrap()
                .map_or(false, |owner| owner == std::thread::current().id())
            && self.enabled(record.metadata())
        {
            self.records
                .lock()
                .unwrap()
                .push(format!("{}", record.args()));
        }
    }

    fn flush(&self) {}
}

static TEST_LOGGER: TestLogger = TestLogger::new(LevelFilter::Warn);
static INIT_LOGGER: Once = Once::new();

fn init_test_logger() {
    INIT_LOGGER.call_once(|| {
        log::set_logger(&TEST_LOGGER).unwrap();
        log::set_max_level(LevelFilter::Warn);
    });
}

#[test]
fn empty() {
    const SRC: &str = r"";
    let eval = compile_code(SRC);
    assert!(eval.is_ok());

    const SRC0: &str = r"
        ";
    let eval = compile_code(SRC0);
    assert!(eval.is_ok());

    const SRC1: &str = r"# blub";
    let eval = compile_code(SRC1);
    assert!(eval.is_ok());
}

#[test]
fn extended_length_units() {
    const SRC: &str = r"
        a = 1pm
        b = 2Ym
        c = 3am
        ";

    let eval = compile_code(SRC).unwrap();

    let a = eval.vars.resolve("a").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(l))) = a {
        assert!((l.get::<picometer>() - 1.0).abs() < 1e-12);
    } else {
        panic!("unexpected type for a");
    }

    let b = eval.vars.resolve("b").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(l))) = b {
        assert!((l.get::<yottameter>() - 2.0).abs() < 1e-12);
    } else {
        panic!("unexpected type for b");
    }

    let c = eval.vars.resolve("c").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(l))) = c {
        assert!((l.get::<attometer>() - 3.0).abs() < 1e-12);
    } else {
        panic!("unexpected type for c");
    }
}

#[test]
fn extended_time_units() {
    const SRC: &str = r"
        a = 4ps
        b = 5Ys
        c = 6zs
        ";

    let eval = compile_code(SRC).unwrap();

    let a = eval.vars.resolve("a").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(UnitVal::TimeVal(t))) = a {
        assert!((t.get::<picosecond>() - 4.0).abs() < 1e-12);
    } else {
        panic!("unexpected type for a");
    }

    let b = eval.vars.resolve("b").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(UnitVal::TimeVal(t))) = b {
        assert!((t.get::<yottasecond>() - 5.0).abs() < 1e-12);
    } else {
        panic!("unexpected type for b");
    }

    let c = eval.vars.resolve("c").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(UnitVal::TimeVal(t))) = c {
        assert!((t.get::<zeptosecond>() - 6.0).abs() < 1e-12);
    } else {
        panic!("unexpected type for c");
    }
}

#[test]
fn comments() {
    const SRC: &str = r"
        # bla,.
        # ups /// \masd
        a = 1 # yo
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
}

#[test]
fn var_declare() {
    const SRC: &str = r"
         a = 7
         b = a
         a = 6
         _l = /cloud
        ";

    // let mut a = Token::lexer(SRC);
    // let b = a.next();
    // dbg!(b);

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();

    let res = eval.vars.resolve("_l");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Path("/cloud".to_owned()));

    let res = eval.vars.resolve("a");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(6))));

    let res = eval.vars.resolve("b");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(7))));
}

#[test]
fn var_declare_nested() {
    const SRC: &str = r"
         a.b.c = 7
         a.b = a.b.c
         a.b.c = 6
         wind._l = /cloud

         bla, blub = 2, 4

         wal, du = 9

         empty_range = ..
        ";

    // let mut a = Token::lexer(SRC);
    // let b = a.next();
    // dbg!(b);

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();

    let res = eval.vars.resolve("wind._l");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Path("/cloud".to_owned()));

    let res = eval.vars.resolve("a.b.c");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(6))));

    let res = eval.vars.resolve("a.b");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(7))));

    let res = eval.vars.resolve("bla");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(2))));

    let res = eval.vars.resolve("blub");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(4))));

    let res = eval.vars.resolve("wal");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(9))));

    let res = eval.vars.resolve("du");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(9))));

    let res = eval.vars.resolve("empty_range");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::EmptyRange);
}

#[test]
fn include() {
    const SRC: &str = r"
        # like assigning a file to the current namespace
        = ./test.rl

        # including in namespace blocks will prepend the namespaces to the included AST
        # (excluding rules, they are always global)
        c {
            = ./test.rl
        }
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();

    let res = eval.vars.resolve("test");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(4))));

    let res = eval.vars.resolve("c.test");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(4))));
}

#[test]
fn var_declare_blocks() {
    const SRC: &str = r"
        a {
            b = 7
        }

        c {
            t = 5

            d {
                o = 1
            }
            i = 95

            e.f {
                bla = 1
            }
        }
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();

    let res = eval.vars.resolve("a.b");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(7))));

    let res = eval.vars.resolve("c.t");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(5))));

    let res = eval.vars.resolve("c.d.o");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(1))));

    let res = eval.vars.resolve("c.i");
    assert!(res.is_ok());
    let res = res.unwrap();
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res, Rhs::Val(Val::NumVal(NumVal::Integer(95))));
}

#[test]
fn in_ns() {
    // other, me
    let res = is_sub_namespace(&["a".to_owned()], &["a".to_owned()]); // true
    assert!(res);
    let res = is_sub_namespace(&["a".to_owned(), "b".to_owned()], &["a".to_owned()]); // a.b is deeper than a, so a.b can not be a sub namespace. false
    assert!(!res);
    let res = is_sub_namespace(&["a".to_owned()], &["a".to_owned(), "b".to_owned()]); // a is in a.b, true
    assert!(res);
    let res = is_sub_namespace(&[], &["a".to_owned()]); // empty is not in a.b, false
    assert!(!res);
}

#[test]
fn resolve_ns() {
    const SRC: &str = r"
        _bag {
            lidar {
                short = l
                type = Pointcloud2
            }
            imu {
                _short = i
                _type = Imu
            }
        }
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();
    let e = eval.vars.resolve_ns(&["_bag"]);
    assert_eq!(e.len(), 4);
}

#[test]
fn usage_tracks_assignments_and_evaluations() {
    const SRC: &str = r"
        a = 1
        b = a
        ";

    let eval = compile_code(SRC).unwrap();
    let (assigned, evaluated) = eval.vars.usage_counts();
    assert_eq!(assigned, 2);
    assert_eq!(evaluated, 0);

    let _ = eval.vars.resolve("a").unwrap();
    let (_assigned_after, evaluated_after) = eval.vars.usage_counts();
    assert_eq!(evaluated_after, 1);
}

#[test]
fn evaluate_stats_only_marks_used_without_resolution() {
    let hist = VariableHistory::new(vec![]);
    let (assigned, evaluated) = hist.usage_counts();
    assert_eq!(assigned, 0);
    assert_eq!(evaluated, 0);

    hist.evaluate_stats_only("_ghost.var").unwrap();
    let (assigned_after, evaluated_after) = hist.usage_counts();
    assert_eq!(assigned_after, 0);
    assert_eq!(evaluated_after, 1);
}

#[test]
fn warns_on_unevaluated_variable_logs_message() {
    init_test_logger();
    TEST_LOGGER.clear();
    let _guard = TEST_LOGGER.capture_guard();

    {
        let _eval = compile_code("a = 1").unwrap();
    } // drop here should log unused warning

    let logs = TEST_LOGGER.messages();
    let found = logs
        .iter()
        .any(|m| m.contains("Variable 'a' was never used."));
    assert!(found, "logs: {:?}", logs);
}

#[test]
fn no_warning_when_marked_evaluated() {
    init_test_logger();
    TEST_LOGGER.clear();
    let _guard = TEST_LOGGER.capture_guard();

    {
        let eval = compile_code("a = 1").unwrap();
        eval.vars.evaluate_stats_only("a").unwrap();

        let (assigned, evaluated) = eval.vars.usage_counts();
        assert_eq!(assigned, 1);
        assert_eq!(evaluated, 1);

        let (assigned_vars, evaluated_vars) = eval.vars.usage_vars();
        assert_eq!(assigned_vars, vec!["a".to_string()]);
        assert_eq!(evaluated_vars, vec!["a".to_string()]);
    }

    let logs = TEST_LOGGER.messages();
    let unwanted = logs
        .iter()
        .filter(|m| m.contains("Variable 'a' was never used."))
        .cloned()
        .collect::<Vec<_>>();
    assert!(unwanted.is_empty(), "logs: {:?}", logs);
}

#[test]
fn strings() {
    const SRC: &str = r#"
        a = "bla with space"
        b = Blub
        c = Lidar::Ouster
            
        "#;

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();

    let e = eval.vars.resolve("a");
    assert!(e.is_ok());
    let e = e.unwrap();
    assert!(e.is_some());
    let rhs = e.unwrap();
    assert_eq!(rhs, Rhs::Val(Val::StringVal("bla with space".to_owned())));

    let e = eval.vars.resolve("b");
    assert!(e.is_ok());
    let e = e.unwrap();
    assert!(e.is_some());
    let rhs = e.unwrap();
    assert_eq!(rhs, Rhs::Val(Val::StringVal("Blub".to_owned())));

    let e = eval.vars.resolve("c");
    assert!(e.is_ok());
    let e = e.unwrap();
    assert!(e.is_some());
    let rhs = e.unwrap();
    assert_eq!(rhs, Rhs::Val(Val::StringVal("Lidar::Ouster".to_owned())));
}

#[test]
fn arrays() {
    const SRC: &str = r#"
        a = [ "bla with space", Blub ]
        "#;

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();

    let e = eval.vars.resolve("a");
    assert!(e.is_ok());
    let e = e.unwrap();
    assert!(e.is_some());
    let rhs = e.unwrap();
    assert_eq!(
        rhs,
        Rhs::Array(vec![
            Box::new(Rhs::Val(Val::StringVal("bla with space".to_owned()))),
            Box::new(Rhs::Val(Val::StringVal("Blub".to_owned()))),
        ])
    );
}

#[test]
fn unbounded_ranges() {
    const SRC: &str = r"
        open_upper = 1..
        open_lower = ..2
        open_upper_unit = 3m..
        open_lower_unit = ..4s
        empty = ..
        num_float = 2.2..6.5
        num_int = -2..6
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let mut eval = eval.unwrap();
    eval.vars.populate_cache();

    let res_upper = eval.vars.resolve("open_upper").unwrap().unwrap();
    assert_eq!(
        res_upper,
        Rhs::Range {
            from: Some(Val::NumVal(NumVal::Integer(1))),
            to: None
        }
    );

    let res_lower = eval.vars.resolve("open_lower").unwrap().unwrap();
    assert_eq!(
        res_lower,
        Rhs::Range {
            from: None,
            to: Some(Val::NumVal(NumVal::Integer(2)))
        }
    );

    // 3m = 3.0 meters
    let res_upper_unit = eval.vars.resolve("open_upper_unit").unwrap().unwrap();
    if let Rhs::Range {
        from: Some(Val::UnitedVal(uv)),
        to: None,
    } = res_upper_unit
    {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<meter>() - 3.0).abs() < 1e-9,
            "Expected 3.0 meters, got {}",
            uv.as_length().unwrap().get::<meter>()
        );
    } else {
        panic!("Expected Range with Length UnitedVal");
    }

    // 4s = 4.0 seconds
    let res_lower_unit = eval.vars.resolve("open_lower_unit").unwrap().unwrap();
    if let Rhs::Range {
        from: None,
        to: Some(Val::UnitedVal(uv)),
    } = res_lower_unit
    {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<second>() - 4.0).abs() < 1e-9,
            "Expected 4.0 seconds, got {}",
            uv.as_time().unwrap().get::<second>()
        );
    } else {
        panic!("Expected Range with Time UnitedVal");
    }

    let res_empty = eval.vars.resolve("empty").unwrap().unwrap();
    assert_eq!(res_empty, Rhs::EmptyRange);

    let num_floatrange = eval.vars.resolve("num_float").unwrap().unwrap();
    assert_eq!(
        num_floatrange,
        Rhs::Range {
            from: Some(Val::NumVal(NumVal::Floating(2.2))),
            to: Some(Val::NumVal(NumVal::Floating(6.5))),
        }
    );

    let num_intrange = eval.vars.resolve("num_int").unwrap().unwrap();
    assert_eq!(
        num_intrange,
        Rhs::Range {
            from: Some(Val::NumVal(NumVal::Integer(-2))),
            to: Some(Val::NumVal(NumVal::Integer(6))),
        }
    );
}

#[test]
fn all() {
    const SRC: &str = r#"
        # a comment
variable = true

time = 1s
time_is_running = 1ms..2min # ranges convert automatically

len = 1cm..1m

# _ signals a variable the interpreter is looking for.
_internal = time_is_running

my.super.long.prefix.var = 0..100 # ranges on namespaced variable "var"

# nested
my.super {

    long.prefix {
        next_var = "UTF-🎱 Strings"
    }

    something_else = -99.018
}

mat = [ [ 6, 1, 9 ],
        [ 3, 1, 8 ] ]

        "#;
    let eval = compile_code(SRC);
    assert!(eval.is_ok());
}

// #[test]
// fn enums() {
//     // in your code
//     #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
//     enum Lidar {
//         Ouster,
//         Sick,
//     }

//     #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]

#[test]
fn error_int_range_with_float_values() {
    let code = "count = 1.5..2.5";
    let ast = compile_code(code).unwrap();
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };
    let res = (|| -> anyhow::Result<(i64, i64)> { resolve_int_range!(configs, "count", 0, 10) })();
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Expected integer value for 'from'"));
}

#[test]
fn error_float_range_with_integer_values() {
    let code = "ratio = 1..2";
    let ast = compile_code(code).unwrap();
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };
    let res =
        (|| -> anyhow::Result<(f64, f64)> { resolve_float_range!(configs, "ratio", 0.0, 10.0) })();
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Expected floating value for 'from'"));
}

#[test]
fn error_string_with_bool() {
    let code = "name = true";
    let ast = compile_code(code).unwrap();
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };
    let res: anyhow::Result<String> =
        (|| -> anyhow::Result<String> { resolve_string!(configs, "name") })();
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Pattern mismatch"));
}

#[test]
fn error_path_with_string_val() {
    let code = "_file = \"/path/to/file\""; // StringVal, not Path
    let ast = compile_code(code).unwrap();
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };
    let res: anyhow::Result<String> =
        (|| -> anyhow::Result<String> { resolve_path!(configs, "_file") })();
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Pattern mismatch"));
}

#[test]
fn error_length_range_wrong_unit() {
    // Provide a united value with incorrect unit (e.g., milliseconds)
    let code = "distance = 1000ms..2000ms";
    let ast = compile_code(code).unwrap();
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };
    let res = (|| -> anyhow::Result<(f64, f64)> {
        resolve_length_range_meters_float!(configs, "distance", 0.5, 10.0)
    })();
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Expected a united length value"));
}

#[test]
fn error_time_range_non_time_unit() {
    // Provide a united value with wrong unit (e.g., millimeter)
    let code = "timeout = 1000mm..2000mm";
    let ast = compile_code(code).unwrap();
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };
    let res = (|| -> anyhow::Result<(f64, f64)> {
        resolve_time_range_seconds_float!(configs, "timeout", 0.0, 10.0)
    })();
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Expected a united time value"));
}
//     enum Sensor {
//         Lidar(Lidar),
//         Imu,
//     }

//     // for rl
//     #[derive(Debug)]
//     enum Enums {
//         Sensor(Sensor),
//     }
//     let a = Enums::Sensor(Sensor::Lidar(Lidar::Ouster));
//     let b = format!("{:?}", a);
//     let r = convert_to_path(&b);
//     println!("{:?}", r);
//     const SRC: &str = "a = Sensor::Lidar";

// TODO Give Enum to parser function that has all possible variants as trait. so fn variants(self--impl display) -> [impl FromStr]. Then call that in parser and pass type through to AST. If parser finds Sensor type, that has the same name as one of the enums in display. so it calls the parse function on that variant.
#[test]
fn vec_mat_vals() {
    const SRC: &str = r"
        vec = [3, 2, 4]

        mat = [ [3, 1, 1],
                [3, 2, 4] ]
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok());
    let eval = eval.unwrap();
    let vec = eval.vars.resolve("vec").unwrap().unwrap();
    assert_eq!(
        vec,
        Rhs::Array(vec![
            Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(3)))),
            Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(2)))),
            Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(4)))),
        ])
    );
    let mat = eval.vars.resolve("mat").unwrap().unwrap();
    assert_eq!(
        mat,
        Rhs::Array(vec![
            Box::new(Rhs::Array(vec![
                Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(3)))),
                Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(1)))),
                Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(1)))),
            ])),
            Box::new(Rhs::Array(vec![
                Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(3)))),
                Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(2)))),
                Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(4)))),
            ])),
        ])
    );
}

#[test]
fn macro_resolve_length_range_meters_float_defaults() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with empty range
    let code_empty = "distance = ..";
    let ast = compile_code(code_empty)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_length_range_meters_float!(configs, "distance", 0.5, 10.0)?;
    assert_eq!(min, 0.5);
    assert_eq!(max, 10.0);

    // Test with partial lower bound
    let code_lower = "distance = 2000mm..";
    let ast = compile_code(code_lower)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_length_range_meters_float!(configs, "distance", 0.0, 15.0)?;
    assert_eq!(min, 2.0);
    assert_eq!(max, 15.0);

    // Test with partial upper bound
    let code_upper = "distance = ..8000mm";
    let ast = compile_code(code_upper)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_length_range_meters_float!(configs, "distance", 1.0, 20.0)?;
    assert_eq!(min, 1.0);
    assert_eq!(max, 8.0);

    Ok(())
}

#[test]
fn macro_resolve_float_force_range_defaults() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with empty range
    let code_empty = "range = ..";
    let ast = compile_code(code_empty)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_float_force_range!(configs, "range", -5.0, 100.0)?;
    assert_eq!(min, -5.0);
    assert_eq!(max, 100.0);

    // Test with partial lower bound
    let code_lower = "range = 25..";
    let ast = compile_code(code_lower)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_float_force_range!(configs, "range", 0.0, 200.0)?;
    assert_eq!(min, 25.0);
    assert_eq!(max, 200.0);

    // Test with partial upper bound
    let code_upper = "range = ..75";
    let ast = compile_code(code_upper)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_float_force_range!(configs, "range", 10.0, 150.0)?;
    assert_eq!(min, 10.0);
    assert_eq!(max, 75.0);

    Ok(())
}

#[test]
fn macro_resolve_float_range_defaults() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with empty range
    let code_empty = "ratio = ..";
    let ast = compile_code(code_empty)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_float_range!(configs, "ratio", 0.0, 1.0)?;
    assert_eq!(min, 0.0);
    assert_eq!(max, 1.0);

    // Test with partial lower bound
    let code_lower = "ratio = 0.1..";
    let ast = compile_code(code_lower)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_float_range!(configs, "ratio", 0.0, 1.0)?;
    assert_eq!(min, 0.1);
    assert_eq!(max, 1.0);

    // Test with partial upper bound
    let code_upper = "ratio = ..0.9";
    let ast = compile_code(code_upper)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_float_range!(configs, "ratio", 0.0, 1.0)?;
    assert_eq!(min, 0.0);
    assert_eq!(max, 0.9);

    Ok(())
}

#[test]
fn macro_resolve_time_range_seconds_float_defaults() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with empty range
    let code_empty = "timeout = ..";
    let ast = compile_code(code_empty)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_time_range_seconds_float!(configs, "timeout", 0.5, 10.0)?;
    assert_eq!(min, 0.5);
    assert_eq!(max, 10.0);

    // Test with bounded range
    let code_bounded = "timeout = 1000ms..5s";
    let ast = compile_code(code_bounded)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_time_range_seconds_float!(configs, "timeout", 0.0, 10.0)?;
    assert_eq!(min, 1.0);
    assert_eq!(max, 5.0);

    // Test with partial lower bound
    let code_lower = "timeout = 2s..";
    let ast = compile_code(code_lower)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_time_range_seconds_float!(configs, "timeout", 0.0, 15.0)?;
    assert_eq!(min, 2.0);
    assert_eq!(max, 15.0);

    Ok(())
}

#[test]
fn macro_resolve_int_range_defaults() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with empty range
    let code_empty = "count = ..";
    let ast = compile_code(code_empty)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_int_range!(configs, "count", 0, 100)?;
    assert_eq!(min, 0);
    assert_eq!(max, 100);

    // Test with bounded range
    let code_bounded = "count = 10..50";
    let ast = compile_code(code_bounded)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_int_range!(configs, "count", 0, 100)?;
    assert_eq!(min, 10);
    assert_eq!(max, 50);

    // Test with partial lower bound
    let code_lower = "count = 25..";
    let ast = compile_code(code_lower)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let (min, max) = resolve_int_range!(configs, "count", 0, 200)?;
    assert_eq!(min, 25);
    assert_eq!(max, 200);

    Ok(())
}

#[test]
fn macro_resolve_string() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with explicit string
    let code = "name = \"example\"";
    let ast = compile_code(code)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let name: String = resolve_string!(configs, "name")?;
    assert_eq!(name, "example");

    // Test with implicit string (identifier)
    let code_implicit = "type = Lidar";
    let ast = compile_code(code_implicit)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let type_name: String = resolve_string!(configs, "type")?;
    assert_eq!(type_name, "Lidar");

    Ok(())
}

#[test]
fn macro_resolve_bool() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with true
    let code_true = "enabled = true";
    let ast = compile_code(code_true)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let enabled: bool = resolve_bool!(configs, "enabled")?;
    assert_eq!(enabled, true);

    // Test with false
    let code_false = "disabled = false";
    let ast = compile_code(code_false)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let disabled: bool = resolve_bool!(configs, "disabled")?;
    assert_eq!(disabled, false);

    Ok(())
}

#[test]
fn macro_resolve_int() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with positive integer
    let code_pos = "count = 42";
    let ast = compile_code(code_pos)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let count: i64 = resolve_int!(configs, "count")?;
    assert_eq!(count, 42);

    // Test with negative integer
    let code_neg = "offset = -10";
    let ast = compile_code(code_neg)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let offset: i64 = resolve_int!(configs, "offset")?;
    assert_eq!(offset, -10);

    Ok(())
}

#[test]
#[allow(clippy::approx_constant)]
fn macro_resolve_float() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with explicit float
    let code_float = "ratio = 3.14";
    let ast = compile_code(code_float)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let ratio: f64 = resolve_float!(configs, "ratio")?;
    assert_eq!(ratio, 3.14);

    // Test with integer that gets converted to float
    let code_int = "count = 42";
    let ast = compile_code(code_int)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let count: f64 = resolve_float!(configs, "count")?;
    assert_eq!(count, 42.0);

    Ok(())
}

#[test]
fn macro_resolve_path() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test with absolute path
    let code = "_file = /path/to/file";
    let ast = compile_code(code)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let path: String = resolve_path!(configs, "_file")?;
    assert_eq!(path, "/path/to/file");

    // Test with relative path
    let code_rel = "_config = ./config.toml";
    let ast = compile_code(code_rel)?;
    let configs = Configs {
        user: ast.vars,
        defaults: VariableHistory::new(vec![]),
    };

    let config_path: String = resolve_path!(configs, "_config")?;
    assert_eq!(config_path, "./config.toml");

    Ok(())
}

#[test]
fn macro_fallback_to_defaults() -> anyhow::Result<()> {
    struct Configs {
        user: VariableHistory,
        defaults: VariableHistory,
    }

    // Test that macros fall back to defaults when not in user config
    let user_code = "enabled = true";
    let default_code = "count = 100\nname = \"default\"";

    let user_ast = compile_code(user_code)?;
    let default_ast = compile_code(default_code)?;

    let configs = Configs {
        user: user_ast.vars,
        defaults: default_ast.vars,
    };

    // This should come from user config
    let enabled: bool = resolve_bool!(configs, "enabled")?;
    assert_eq!(enabled, true);

    // These should come from default config
    let count: i64 = resolve_int!(configs, "count")?;
    assert_eq!(count, 100);

    let name: String = resolve_string!(configs, "name")?;
    assert_eq!(name, "default");

    Ok(())
}

#[test]
fn error_variable_not_found_message() {
    // No variables defined; attempt to resolve a missing one
    let ast = compile_code("").unwrap();
    let defaults = VariableHistory::new(vec![]);
    let configs = Configs {
        user: ast.vars,
        defaults,
    };

    // Use identifier form to validate the exact error message path
    let res: anyhow::Result<i64> = (|| -> anyhow::Result<i64> {
        resolve_var!(configs, missing_var, as i64,
            Rhs::Val(Val::NumVal(NumVal::Integer(i))) => { i }
        )
    })();

    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("Required variable 'missing_var' not found in any configuration."));
}

#[test]
fn micrometer_units() {
    const SRC: &str = r"
        micro_int = 1000um
        micro_float = 1.5um
        nano_int = 1000000nm
        nano_float = 1.5nm
        range_micro = 500um..1000um
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let eval = eval.unwrap();

    // 1000um = 1mm = 0.001m (exact with uom)
    let micro_int = eval.vars.resolve("micro_int").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = micro_int {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<micrometer>() - 1000.0).abs() < 1e-9,
            "Expected 1000 micrometers, got {}",
            uv.as_length().unwrap().get::<micrometer>()
        );
        assert!(
            (uv.as_length().unwrap().get::<millimeter>() - 1.0).abs() < 1e-9,
            "Expected 1 mm, got {}",
            uv.as_length().unwrap().get::<millimeter>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 1.5um preserved exactly (no rounding!)
    let micro_float = eval.vars.resolve("micro_float").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = micro_float {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<micrometer>() - 1.5).abs() < 1e-9,
            "Expected 1.5 micrometers, got {}",
            uv.as_length().unwrap().get::<micrometer>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 1000000nm = 1mm = 0.001m (exact with uom)
    let nano_int = eval.vars.resolve("nano_int").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = nano_int {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<nanometer>() - 1_000_000.0).abs() < 1e-6,
            "Expected 1000000 nanometers, got {}",
            uv.as_length().unwrap().get::<nanometer>()
        );
        assert!(
            (uv.as_length().unwrap().get::<millimeter>() - 1.0).abs() < 1e-9,
            "Expected 1 mm, got {}",
            uv.as_length().unwrap().get::<millimeter>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 1.5nm preserved exactly (no rounding!)
    let nano_float = eval.vars.resolve("nano_float").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = nano_float {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<nanometer>() - 1.5).abs() < 1e-9,
            "Expected 1.5 nanometers, got {}",
            uv.as_length().unwrap().get::<nanometer>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // Range: 500um..1000um preserved exactly
    let range_micro = eval.vars.resolve("range_micro").unwrap().unwrap();
    if let Rhs::Range {
        from: Some(Val::UnitedVal(from_uv)),
        to: Some(Val::UnitedVal(to_uv)),
    } = range_micro
    {
        assert!(
            (from_uv.as_length().unwrap().get::<micrometer>() - 500.0).abs() < 1e-9,
            "Expected 500 um, got {}",
            from_uv.as_length().unwrap().get::<micrometer>()
        );
        assert!(
            (to_uv.as_length().unwrap().get::<micrometer>() - 1000.0).abs() < 1e-9,
            "Expected 1000 um, got {}",
            to_uv.as_length().unwrap().get::<micrometer>()
        );
    } else {
        panic!("Expected Range with UnitedVal bounds");
    }
}

#[test]
fn microsecond_units() {
    const SRC: &str = r"
        micro_int = 1000us
        micro_float = 1.5us
        nano_int = 1000000ns
        nano_float = 1.5ns
        range_micro = 500us..1000us
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let eval = eval.unwrap();

    // 1000us = 1ms = 0.001s (exact with uom)
    let micro_int = eval.vars.resolve("micro_int").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = micro_int {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<microsecond>() - 1000.0).abs() < 1e-9,
            "Expected 1000 microseconds, got {}",
            uv.as_time().unwrap().get::<microsecond>()
        );
        assert!(
            (uv.as_time().unwrap().get::<millisecond>() - 1.0).abs() < 1e-9,
            "Expected 1 ms, got {}",
            uv.as_time().unwrap().get::<millisecond>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 1.5us preserved exactly (no rounding!)
    let micro_float = eval.vars.resolve("micro_float").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = micro_float {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<microsecond>() - 1.5).abs() < 1e-9,
            "Expected 1.5 microseconds, got {}",
            uv.as_time().unwrap().get::<microsecond>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 1000000ns = 1ms = 0.001s (exact with uom)
    let nano_int = eval.vars.resolve("nano_int").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = nano_int {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<nanosecond>() - 1_000_000.0).abs() < 1e-3,
            "Expected 1000000 nanoseconds, got {}",
            uv.as_time().unwrap().get::<nanosecond>()
        );
        assert!(
            (uv.as_time().unwrap().get::<millisecond>() - 1.0).abs() < 1e-9,
            "Expected 1 ms, got {}",
            uv.as_time().unwrap().get::<millisecond>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 1.5ns preserved exactly (no rounding!)
    let nano_float = eval.vars.resolve("nano_float").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = nano_float {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<nanosecond>() - 1.5).abs() < 1e-9,
            "Expected 1.5 nanoseconds, got {}",
            uv.as_time().unwrap().get::<nanosecond>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // Range: 500us..1000us preserved exactly
    let range_micro = eval.vars.resolve("range_micro").unwrap().unwrap();
    if let Rhs::Range {
        from: Some(Val::UnitedVal(from_uv)),
        to: Some(Val::UnitedVal(to_uv)),
    } = range_micro
    {
        assert!(
            (from_uv.as_time().unwrap().get::<microsecond>() - 500.0).abs() < 1e-9,
            "Expected 500 us, got {}",
            from_uv.as_time().unwrap().get::<microsecond>()
        );
        assert!(
            (to_uv.as_time().unwrap().get::<microsecond>() - 1000.0).abs() < 1e-9,
            "Expected 1000 us, got {}",
            to_uv.as_time().unwrap().get::<microsecond>()
        );
    } else {
        panic!("Expected Range with UnitedVal bounds");
    }
}

#[test]
fn accuracy_conversions_no_rounding() {
    const SRC: &str = r"
        um_500 = 500um
        nm_1 = 1nm
        us_500 = 500us
        ns_1 = 1ns
        um_999999 = 999999um
        us_999999 = 999999us
        ";

    let eval = compile_code(SRC).expect("Failed to compile");

    // 500um = 0.5mm exactly
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("um_500").unwrap().unwrap() {
        assert!((uv.as_length().unwrap().get::<micrometer>() - 500.0).abs() < 1e-9);
        assert!((uv.as_length().unwrap().get::<millimeter>() - 0.5).abs() < 1e-9);
    } else {
        panic!("Expected UnitedVal");
    }

    // 1nm = 0.000001mm exactly
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("nm_1").unwrap().unwrap() {
        assert!((uv.as_length().unwrap().get::<nanometer>() - 1.0).abs() < 1e-9);
        assert!((uv.as_length().unwrap().get::<millimeter>() - 0.000001).abs() < 1e-12);
    } else {
        panic!("Expected UnitedVal");
    }

    // 500us = 0.5ms exactly
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("us_500").unwrap().unwrap() {
        assert!((uv.as_time().unwrap().get::<microsecond>() - 500.0).abs() < 1e-9);
        assert!((uv.as_time().unwrap().get::<millisecond>() - 0.5).abs() < 1e-9);
    } else {
        panic!("Expected UnitedVal");
    }

    // 1ns = 0.000001ms exactly
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("ns_1").unwrap().unwrap() {
        assert!((uv.as_time().unwrap().get::<nanosecond>() - 1.0).abs() < 1e-9);
        assert!((uv.as_time().unwrap().get::<millisecond>() - 0.000001).abs() < 1e-12);
    } else {
        panic!("Expected UnitedVal");
    }

    // 999999um = 999.999mm exactly
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("um_999999").unwrap().unwrap() {
        assert!((uv.as_length().unwrap().get::<micrometer>() - 999999.0).abs() < 1e-6);
        assert!((uv.as_length().unwrap().get::<millimeter>() - 999.999).abs() < 1e-9);
    } else {
        panic!("Expected UnitedVal");
    }

    // 999999us = 999.999ms exactly
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("us_999999").unwrap().unwrap() {
        assert!((uv.as_time().unwrap().get::<microsecond>() - 999999.0).abs() < 1e-6);
        assert!((uv.as_time().unwrap().get::<millisecond>() - 999.999).abs() < 1e-9);
    } else {
        panic!("Expected UnitedVal");
    }
}

#[test]
fn accuracy_large_values_floating_point_precision() {
    // Test large values - f64 has about 15-17 significant decimal digits
    const SRC: &str = r"
        large_um = 123456789012um
        large_ns = 123456789012345ns
        ";

    let eval = compile_code(SRC).expect("Failed to compile");

    // 123456789012um = 123456.789012m
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("large_um").unwrap().unwrap() {
        let expected_meters = 123456.789012;
        let tolerance = expected_meters * 1e-12;
        assert!(
            (uv.as_length().unwrap().get::<meter>() - expected_meters).abs() < tolerance,
            "large_um: expected {} m, got {} m",
            expected_meters,
            uv.as_length().unwrap().get::<meter>()
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // 123456789012345ns = 123456.789012345s (ns to s: divide by 10^9)
    if let Rhs::Val(Val::UnitedVal(uv)) = eval.vars.resolve("large_ns").unwrap().unwrap() {
        let expected_seconds = 123456.789012345;
        let tolerance = expected_seconds * 1e-10;
        assert!(
            (uv.as_time().unwrap().get::<second>() - expected_seconds).abs() < tolerance,
            "large_ns: expected {} s, got {} s",
            expected_seconds,
            uv.as_time().unwrap().get::<second>()
        );
    } else {
        panic!("Expected UnitedVal");
    }
}

#[test]
fn imperial_length_units() {
    const SRC: &str = r"
        feet_val = 5ft
        inches_val = 12in
        miles_val = 1mi
        yards_val = 3yd
        chains_val = 2ch
        rods_val = 1rd
        fathom_val = 1fathom
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let eval = eval.unwrap();

    // Test feet
    let feet_val = eval.vars.resolve("feet_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = feet_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<foot>() - 5.0).abs() < 1e-9,
            "Expected 5 feet, got {}",
            uv.as_length().unwrap().get::<foot>()
        );
    } else {
        panic!("Expected UnitedVal for feet");
    }

    // Test inches
    let inches_val = eval.vars.resolve("inches_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = inches_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<inch>() - 12.0).abs() < 1e-9,
            "Expected 12 inches, got {}",
            uv.as_length().unwrap().get::<inch>()
        );
    } else {
        panic!("Expected UnitedVal for inches");
    }

    // Test miles
    let miles_val = eval.vars.resolve("miles_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = miles_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<mile>() - 1.0).abs() < 1e-9,
            "Expected 1 mile, got {}",
            uv.as_length().unwrap().get::<mile>()
        );
    } else {
        panic!("Expected UnitedVal for miles");
    }

    // Test yards
    let yards_val = eval.vars.resolve("yards_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = yards_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<yard>() - 3.0).abs() < 1e-9,
            "Expected 3 yards, got {}",
            uv.as_length().unwrap().get::<yard>()
        );
    } else {
        panic!("Expected UnitedVal for yards");
    }

    // Test chains
    let chains_val = eval.vars.resolve("chains_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = chains_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<chain>() - 2.0).abs() < 1e-9,
            "Expected 2 chains, got {}",
            uv.as_length().unwrap().get::<chain>()
        );
    } else {
        panic!("Expected UnitedVal for chains");
    }

    // Test rods
    let rods_val = eval.vars.resolve("rods_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = rods_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<rod>() - 1.0).abs() < 1e-9,
            "Expected 1 rod, got {}",
            uv.as_length().unwrap().get::<rod>()
        );
    } else {
        panic!("Expected UnitedVal for rods");
    }

    // Test fathom
    let fathom_val = eval.vars.resolve("fathom_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = fathom_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<fathom>() - 1.0).abs() < 1e-9,
            "Expected 1 fathom, got {}",
            uv.as_length().unwrap().get::<fathom>()
        );
    } else {
        panic!("Expected UnitedVal for fathom");
    }
}

#[test]
fn astronomical_and_small_length_units() {
    const SRC: &str = r"
        au_val = 1.5au
        light_year_val = 4.37light_years
        parsec_val = 1pc
        nautical_val = 10nautical_miles
        angstrom_val = 5angstrom
        micron_val = 0.5microns
        mil_val = 10mil
        bohr_val = 1bohr_radius
        fermi_val = 10fermi
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let eval = eval.unwrap();

    // Test astronomical unit
    let au_val = eval.vars.resolve("au_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = au_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<astronomical_unit>() - 1.5).abs() < 1e-9,
            "Expected 1.5 AU, got {}",
            uv.as_length().unwrap().get::<astronomical_unit>()
        );
    } else {
        panic!("Expected UnitedVal for AU");
    }

    // Test light year
    let light_year_val = eval.vars.resolve("light_year_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = light_year_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<light_year>() - 4.37).abs() < 1e-9,
            "Expected 4.37 light years, got {}",
            uv.as_length().unwrap().get::<light_year>()
        );
    } else {
        panic!("Expected UnitedVal for light_year");
    }

    // Test parsec
    let parsec_val = eval.vars.resolve("parsec_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = parsec_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<parsec>() - 1.0).abs() < 1e-9,
            "Expected 1 parsec, got {}",
            uv.as_length().unwrap().get::<parsec>()
        );
    } else {
        panic!("Expected UnitedVal for parsec");
    }

    // Test nautical mile
    let nautical_val = eval.vars.resolve("nautical_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = nautical_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<nautical_mile>() - 10.0).abs() < 1e-9,
            "Expected 10 nautical miles, got {}",
            uv.as_length().unwrap().get::<nautical_mile>()
        );
    } else {
        panic!("Expected UnitedVal for nautical_mile");
    }

    // Test angstrom
    let angstrom_val = eval.vars.resolve("angstrom_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = angstrom_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<angstrom>() - 5.0).abs() < 1e-9,
            "Expected 5 angstroms, got {}",
            uv.as_length().unwrap().get::<angstrom>()
        );
    } else {
        panic!("Expected UnitedVal for angstrom");
    }

    // Test micron
    let micron_val = eval.vars.resolve("micron_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = micron_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<micron>() - 0.5).abs() < 1e-9,
            "Expected 0.5 microns, got {}",
            uv.as_length().unwrap().get::<micron>()
        );
    } else {
        panic!("Expected UnitedVal for micron");
    }

    // Test mil
    let mil_val = eval.vars.resolve("mil_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = mil_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<mil>() - 10.0).abs() < 1e-9,
            "Expected 10 mils, got {}",
            uv.as_length().unwrap().get::<mil>()
        );
    } else {
        panic!("Expected UnitedVal for mil");
    }

    // Test bohr radius
    let bohr_val = eval.vars.resolve("bohr_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = bohr_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<bohr_radius>() - 1.0).abs() < 1e-9,
            "Expected 1 bohr radius, got {}",
            uv.as_length().unwrap().get::<bohr_radius>()
        );
    } else {
        panic!("Expected UnitedVal for bohr_radius");
    }

    // Test fermi
    let fermi_val = eval.vars.resolve("fermi_val").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = fermi_val {
        assert_eq!(uv.unit(), Unit::Length);
        assert!(
            (uv.as_length().unwrap().get::<fermi>() - 10.0).abs() < 1e-9,
            "Expected 10 fermis, got {}",
            uv.as_length().unwrap().get::<fermi>()
        );
    } else {
        panic!("Expected UnitedVal for fermi");
    }
}

#[test]
fn sidereal_and_tropical_time_units() {
    const SRC: &str = r"
        sidereal_sec = 1second_sidereal
        sidereal_hour = 2hours_sidereal
        sidereal_day = 1day_sidereal
        sidereal_year = 1year_sidereal
        tropical_year = 1.00000611year_tropical
        mixed_range = 0.5day_sidereal..2day_sidereal
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let eval = eval.unwrap();

    // Test sidereal second
    let sidereal_sec = eval.vars.resolve("sidereal_sec").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = sidereal_sec {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<second_sidereal>() - 1.0).abs() < 1e-9,
            "Expected 1 sidereal second, got {}",
            uv.as_time().unwrap().get::<second_sidereal>()
        );
    } else {
        panic!("Expected UnitedVal for sidereal_sec");
    }

    // Test sidereal hour
    let sidereal_hour = eval.vars.resolve("sidereal_hour").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = sidereal_hour {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<hour_sidereal>() - 2.0).abs() < 1e-9,
            "Expected 2 sidereal hours, got {}",
            uv.as_time().unwrap().get::<hour_sidereal>()
        );
    } else {
        panic!("Expected UnitedVal for sidereal_hour");
    }

    // Test sidereal day
    let sidereal_day = eval.vars.resolve("sidereal_day").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = sidereal_day {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<day_sidereal>() - 1.0).abs() < 1e-9,
            "Expected 1 sidereal day, got {}",
            uv.as_time().unwrap().get::<day_sidereal>()
        );
    } else {
        panic!("Expected UnitedVal for sidereal_day");
    }

    // Test sidereal year
    let sidereal_year = eval.vars.resolve("sidereal_year").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = sidereal_year {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<year_sidereal>() - 1.0).abs() < 1e-9,
            "Expected 1 sidereal year, got {}",
            uv.as_time().unwrap().get::<year_sidereal>()
        );
    } else {
        panic!("Expected UnitedVal for sidereal_year");
    }

    // Test tropical year
    let tropical_year = eval.vars.resolve("tropical_year").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = tropical_year {
        assert_eq!(uv.unit(), Unit::Time);
        assert!(
            (uv.as_time().unwrap().get::<year_tropical>() - 1.00000611).abs() < 1e-9,
            "Expected 1.00000611 tropical years, got {}",
            uv.as_time().unwrap().get::<year_tropical>()
        );
    } else {
        panic!("Expected UnitedVal for tropical_year");
    }

    // Test mixed range with sidereal days
    let mixed_range = eval.vars.resolve("mixed_range").unwrap().unwrap();
    if let Rhs::Range { from, to } = mixed_range {
        let min = from.unwrap();
        let max = to.unwrap();
        if let Val::UnitedVal(uv_min) = min {
            if let Val::UnitedVal(uv_max) = max {
                assert_eq!(uv_min.unit(), Unit::Time);
                assert_eq!(uv_max.unit(), Unit::Time);
                assert!(
                    (uv_min.as_time().unwrap().get::<day_sidereal>() - 0.5).abs() < 1e-9,
                    "Expected 0.5 sidereal days min, got {}",
                    uv_min.as_time().unwrap().get::<day_sidereal>()
                );
                assert!(
                    (uv_max.as_time().unwrap().get::<day_sidereal>() - 2.0).abs() < 1e-9,
                    "Expected 2 sidereal days max, got {}",
                    uv_max.as_time().unwrap().get::<day_sidereal>()
                );
            } else {
                panic!("Expected UnitedVal for max");
            }
        } else {
            panic!("Expected UnitedVal for min");
        }
    } else {
        panic!("Expected Range for mixed_range");
    }
}

#[test]
fn length_unit_conversions() {
    const SRC: &str = r"
        feet_to_meters = 1ft
        inches_to_meters = 12in
        miles_to_meters = 1mi
        au_to_meters = 1au
        light_year_to_meters = 1light_years
        angstrom_to_meters = 1angstrom
        ";

    let eval = compile_code(SRC);
    assert!(eval.is_ok(), "Failed to compile: {:?}", eval.err());
    let eval = eval.unwrap();

    // Verify feet to meters: 1 foot = 0.3048 meters
    let feet_res = eval.vars.resolve("feet_to_meters").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = feet_res {
        let meters = uv.as_length().unwrap().get::<meter>();
        assert!(
            (meters - 0.3048).abs() < 1e-9,
            "1 ft should be 0.3048 m, got {}",
            meters
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // Verify inches to meters: 1 inch = 0.0254 meters
    let inches_res = eval.vars.resolve("inches_to_meters").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = inches_res {
        let meters = uv.as_length().unwrap().get::<meter>();
        assert!(
            (meters - 0.3048).abs() < 1e-9,
            "12 in should be 0.3048 m, got {}",
            meters
        );
    } else {
        panic!("Expected UnitedVal");
    }

    // Verify angstrom to meters: 1 angstrom = 1e-10 meters
    let angstrom_res = eval.vars.resolve("angstrom_to_meters").unwrap().unwrap();
    if let Rhs::Val(Val::UnitedVal(uv)) = angstrom_res {
        let meters = uv.as_length().unwrap().get::<meter>();
        assert!(
            (meters - 1e-10).abs() < 1e-18,
            "1 angstrom should be 1e-10 m, got {}",
            meters
        );
    } else {
        panic!("Expected UnitedVal");
    }
}
