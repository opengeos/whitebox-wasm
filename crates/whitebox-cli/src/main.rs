//! WASI runner for whitebox_next_gen tools, operating on regular files.
//!
//! Build:  cargo build -p whitebox-cli --target wasm32-wasip1 --release
//! List:   wasmtime run whitebox.wasm -- list
//! Run:    wasmtime run --dir DATA::/work whitebox.wasm -- \
//!             slope --input=/work/dem.tif --output=/work/slope.tif --units=degrees
//!
//! The tools read and write rasters by path; under WASI those paths resolve to
//! real files in the host directories mapped with `--dir`.
use std::collections::BTreeMap;
use serde_json::Value;
use wbtools_oss::{ToolRegistry, register_default_tools};
use wbcore::{ToolContext, AllowAllCapabilities, RecordingProgressSink};

fn parse_value(s: &str) -> Value {
    if let Ok(i) = s.parse::<i64>() { return Value::from(i); }
    if let Ok(f) = s.parse::<f64>() { return Value::from(f); }
    match s {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(s.to_string()),
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let mut reg = ToolRegistry::new();
    register_default_tools(&mut reg);

    let cmd = argv.get(1).map(|s| s.as_str()).unwrap_or("list");
    if cmd == "list" {
        let mut ids: Vec<String> = reg.list().into_iter().map(|m| m.id.to_string()).collect();
        ids.sort();
        println!("{} tools:", ids.len());
        for id in ids { println!("  {id}"); }
        return;
    }
    if cmd == "help" {
        if let Some(id) = argv.get(2) {
            for m in reg.list() {
                if m.id == id.as_str() { println!("{}: {}", m.id, m.display_name); }
            }
        }
        return;
    }

    // cmd is a tool id; remaining args are --key=value
    let mut args: BTreeMap<String, Value> = BTreeMap::new();
    for a in &argv[2..] {
        let kv = a.strip_prefix("--").unwrap_or(a);
        if let Some((k, v)) = kv.split_once('=') {
            args.insert(k.to_string(), parse_value(v));
        } else {
            args.insert(kv.to_string(), Value::Bool(true));
        }
    }

    let progress = RecordingProgressSink::new();
    let caps = AllowAllCapabilities;
    let ctx = ToolContext { progress: &progress, capabilities: &caps };

    match reg.run(cmd, &args, &ctx) {
        Ok(_) => println!("OK: tool '{cmd}' completed"),
        Err(e) => { eprintln!("ERROR running '{cmd}': {e:?}"); std::process::exit(1); }
    }
}
