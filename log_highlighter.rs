// logcolor - tiny Rust CLI to colorize log levels in text (ERROR/WARN/INFO/DEBUG)
// Single-file tool (no external crates). Useful for quickly reading logs in terminals.
//
// Usage:
//   cargo run --release -- <path-to-log-file>
//   cat app.log | cargo run --release --
//   cargo build --release && ./target/release/logcolor app.log
//
// Ctrl+C to stop when reading from a never-ending stream.

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::exit;

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

fn color_for_level(level: &str) -> &'static str {
    match level {
        "ERROR" | "ERR" => RED,
        "WARN" | "WARNING" => YELLOW,
        "INFO" => GREEN,
        "DEBUG" => CYAN,
        "TRACE" => MAGENTA,
        _ => RESET,
    }
}

/// attempt to detect a level token in the line.
/// common patterns: "[ERROR]", "ERROR:", "error", "ERR", etc.
/// returns (index_of_token_start, token_string) if found
fn find_level(line: &str) -> Option<(usize, &str)> {
    // We'll do simple checks in order of common formats.
    // Use uppercase matching for case-insensitive detection.
    let upper = line.to_uppercase();
    let tokens = ["ERROR", "ERR", "WARNING", "WARN", "INFO", "DEBUG", "TRACE"];
    // Check bracketed or parenthesized forms first
    for t in tokens.iter() {
        let bracket1 = format!("[{}]", t);
        let bracket2 = format!("({})", t);
        if let Some(pos) = upper.find(&bracket1) {
            return Some((pos, &line[pos..pos + bracket1.len()]));
        }
        if let Some(pos) = upper.find(&bracket2) {
            return Some((pos, &line[pos..pos + bracket2.len()]));
        }
    }
    // Check token followed by ":" or " - " or whitespace
    for t in tokens.iter() {
        if let Some(pos) = upper.find(&format!("{}:", t)) {
            return Some((pos, &line[pos..pos + t.len() + 1]));
        }
        if let Some(pos) = upper.find(&format!("{} -", t)) {
            return Some((pos, &line[pos..pos + t.len() + 2]));
        }
        // standalone token (space padded)
        if let Some(pos) = upper.find(&format!(" {}", t)) {
            return Some((pos + 1, &line[pos + 1..pos + 1 + t.len()]));
        }
    }
    // fallback: contains token anywhere
    for t in tokens.iter() {
        if let Some(pos) = upper.find(t) {
            return Some((pos, &line[pos..pos + t.len()]));
        }
    }
    None
}

fn print_colored_line(mut out: &mut dyn Write, line: &str) -> io::Result<()> {
    if let Some((pos, token)) = find_level(line) {
        // token may include bracket/colon; normalize to raw level text
        let raw = token
            .trim_matches(|c: char| c == '[' || c == ']' || c == '(' || c == ')' || c == ':' || c == '-' || c.is_whitespace())
            .to_uppercase();
        let color = color_for_level(&raw);
        // Write prefix, colored token, then suffix
        write!(out, "{}", &line[..pos])?;
        write!(out, "{}{}{}{}", BOLD, color, &line[pos..pos + token.len()], RESET)?;
        writeln!(out, "{}", &line[pos + token.len()..])?;
    } else {
        writeln!(out, "{}", line)?;
    }
    Ok(())
}

fn process_reader<R: Read>(r: R) -> io::Result<()> {
    let reader = BufReader::new(r);
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for maybe_line in reader.lines() {
        match maybe_line {
            Ok(line) => {
                if let Err(e) = print_colored_line(&mut handle, &line) {
                    eprintln!("write error: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("read error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {} [path-to-log-file]", program);
    eprintln!("Examples:");
    eprintln!("  {} ./app.log", program);
    eprintln!("  tail -f /var/log/syslog | {} -", program);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        print_usage(&args[0]);
        exit(1);
    }

    // If user passes "-" or no args -> read stdin
    if args.len() == 1 || args[1] == "-" {
        if let Err(e) = process_reader(io::stdin()) {
            eprintln!("error processing stdin: {}", e);
            exit(1);
        }
        return;
    }

    let path = &args[1];
    match File::open(path) {
        Ok(file) => {
            if let Err(e) = process_reader(file) {
                eprintln!("error processing '{}': {}", path, e);
                exit(1);
            }
        }
        Err(e) => {
            eprintln!("failed to open '{}': {}", path, e);
            exit(1);
        }
    }
}
