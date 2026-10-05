//! Offline CLI: inspect an external CBC witness without opening a public ingress API.
#[path = "../cbc_witness_ingress.rs"]
mod cbc_witness_ingress;

use std::io::Read;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("Usage: cbc_witness_inspect <witness-json-path>")?;
    if args.next().is_some() {
        return Err("Expected exactly one JSON path".into());
    }
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(1_000_001).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 1_000_000 {
        return Err("Input exceeds 1 MB".into());
    }
    let payload: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let result = cbc_witness_ingress::inspect(&payload)?;
    println!("{}", serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("CBC witness inspection failed: {error}");
        std::process::exit(1);
    }
}
