use std::env;
use std::hint::black_box;
use std::time::Instant;

use kiwi_rs::{Kiwi, KIWI_MATCH_ALL};

#[derive(Debug)]
struct Cli {
    text: String,
    warmup: usize,
    iters: usize,
    python_default_options: bool,
    utf16: bool,
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run --release --example bench_tokenize -- [--text <text>] [--warmup <n>] [--iters <n>] [--python-default-options] [--utf16]"
    );
}

fn parse_usize_flag(name: &str, value: Option<String>) -> Result<usize, String> {
    let raw = value.ok_or_else(|| format!("{name} requires a value"))?;
    raw.parse::<usize>()
        .map_err(|error| format!("invalid {name} value '{raw}': {error}"))
}

fn parse_args() -> Result<Cli, String> {
    let mut text = "아버지가방에들어가신다.".to_string();
    let mut warmup = 50usize;
    let mut iters = 1_000usize;
    let mut python_default_options = false;
    let mut utf16 = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--text" => {
                text = args
                    .next()
                    .ok_or_else(|| "--text requires a value".to_string())?
            }
            "--warmup" => warmup = parse_usize_flag("--warmup", args.next())?,
            "--iters" => iters = parse_usize_flag("--iters", args.next())?,
            "--python-default-options" => python_default_options = true,
            "--utf16" => utf16 = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if iters == 0 {
        return Err("--iters must be >= 1".to_string());
    }

    Ok(Cli {
        text,
        warmup,
        iters,
        python_default_options,
        utf16,
    })
}

fn tokenize_once(
    kiwi: &Kiwi,
    text: &str,
    utf16: bool,
    python_default_options: bool,
) -> kiwi_rs::Result<usize> {
    let tokens = if utf16 {
        let text16: Vec<u16> = text.encode_utf16().collect();
        if python_default_options {
            kiwi.tokenize_utf16_with_match_options(&text16, KIWI_MATCH_ALL)?
        } else {
            kiwi.tokenize_utf16(&text16)?
        }
    } else if python_default_options {
        kiwi.tokenize_with_match_options(text, KIWI_MATCH_ALL)?
    } else {
        kiwi.tokenize(text)?
    };
    Ok(tokens.len())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_args().map_err(|message| {
        print_usage();
        message
    })?;

    let init_start = Instant::now();
    let kiwi = Kiwi::init()?;
    let init_elapsed = init_start.elapsed();

    let first_start = Instant::now();
    let first_tokens = tokenize_once(&kiwi, &cli.text, cli.utf16, cli.python_default_options)?;
    let first_elapsed = first_start.elapsed();
    let first_token_count = first_tokens;
    black_box(first_tokens);

    for _ in 0..cli.warmup {
        let token_count = tokenize_once(&kiwi, &cli.text, cli.utf16, cli.python_default_options)?;
        black_box(token_count);
    }

    let bench_start = Instant::now();
    let mut total_tokens = 0usize;
    for _ in 0..cli.iters {
        let token_count = tokenize_once(&kiwi, &cli.text, cli.utf16, cli.python_default_options)?;
        total_tokens += token_count;
        black_box(token_count);
    }
    let bench_elapsed = bench_start.elapsed();

    let bench_secs = bench_elapsed.as_secs_f64();
    let avg_ms = (bench_secs * 1_000.0) / cli.iters as f64;
    let calls_per_sec = cli.iters as f64 / bench_secs;
    let tokens_per_sec = total_tokens as f64 / bench_secs;

    println!("engine=kiwi-rs");
    println!("text={}", cli.text);
    println!("warmup={}", cli.warmup);
    println!("iters={}", cli.iters);
    println!(
        "options_mode={}",
        if cli.python_default_options {
            "python_default"
        } else {
            "rust_default"
        }
    );
    println!("init_mode=init");
    println!("input_mode={}", if cli.utf16 { "utf16" } else { "utf8" });
    println!("first_token_count={first_token_count}");
    println!("init_ms={:.3}", init_elapsed.as_secs_f64() * 1_000.0);
    println!(
        "first_tokenize_ms={:.3}",
        first_elapsed.as_secs_f64() * 1_000.0
    );
    println!("bench_total_ms={:.3}", bench_secs * 1_000.0);
    println!("bench_avg_ms={avg_ms:.6}");
    println!("calls_per_sec={calls_per_sec:.2}");
    println!("tokens_per_sec={tokens_per_sec:.2}");

    Ok(())
}
