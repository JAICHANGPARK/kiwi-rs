use std::env;
use std::hint::black_box;
use std::time::Instant;

use kiwi_rs::Kiwi;

#[derive(Debug)]
struct Cli {
    iters: usize,
    text_len: usize,
}

fn parse_args() -> Result<Cli, String> {
    let mut iters = 10_000usize;
    let mut text_len = 100usize;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" => {
                iters = args
                    .next()
                    .ok_or_else(|| "--iters requires a value".to_string())?
                    .parse()
                    .map_err(|e| format!("invalid iters: {e}"))?
            }
            "--text-len" => {
                text_len = args
                    .next()
                    .ok_or_else(|| "--text-len requires a value".to_string())?
                    .parse()
                    .map_err(|e| format!("invalid text-len: {e}"))?
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --release --example bench_glue -- [--iters <n>] [--text-len <n>]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(Cli { iters, text_len })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_args()?;
    let kiwi = Kiwi::init()?;

    // Prepare data for glue
    // Create a vector of "token" strings to glue together.
    // e.g. ["아버지가", "방에", "들어가신다"] repeated to match text_len
    let base_chunks = vec!["아버지가", "방에", "들어가신다", "이것은", "테스트입니다"];
    let mut glue_chunks = Vec::new();
    while glue_chunks.len() < cli.text_len {
        glue_chunks.extend_from_slice(&base_chunks);
    }
    glue_chunks.truncate(cli.text_len);

    // Prepare data for join
    // Create a vector of (form, tag) tuples.
    let base_morphs = vec![
        ("아버지", "NNG"),
        ("가", "JKS"),
        ("방", "NNG"),
        ("에", "JKB"),
        ("들어가", "VV"),
        ("시", "EP"),
        ("ㄴ다", "EF"),
        (".", "SF"),
    ];
    let mut join_morphs = Vec::new();
    while join_morphs.len() < cli.text_len {
        join_morphs.extend_from_slice(&base_morphs);
    }
    join_morphs.truncate(cli.text_len);

    println!(
        "Benchmarking with iters={}, text_len={}",
        cli.iters, cli.text_len
    );

    // Bench glue
    let start = Instant::now();
    for _ in 0..cli.iters {
        let res = kiwi.glue(&glue_chunks)?;
        black_box(res);
    }
    let duration = start.elapsed();
    println!(
        "glue: {:.2} ms/iter",
        duration.as_secs_f64() * 1000.0 / cli.iters as f64
    );

    // Bench join
    let start = Instant::now();
    for _ in 0..cli.iters {
        let res = kiwi.join(&join_morphs, true)?;
        black_box(res);
    }
    let duration = start.elapsed();
    println!(
        "join: {:.2} ms/iter",
        duration.as_secs_f64() * 1000.0 / cli.iters as f64
    );

    Ok(())
}
