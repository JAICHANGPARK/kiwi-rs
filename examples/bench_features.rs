use std::env;
use std::fs::File;
use std::hint::black_box;
use std::io::{BufRead, BufReader};
use std::time::Instant;

use kiwi_rs::{AnalyzeOptions, Kiwi, Sentence, KIWI_MATCH_ALL};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InputMode {
    Repeated,
    Varied,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InitMode {
    Init,
    New,
}

#[derive(Debug)]
struct Cli {
    text: String,
    warmup: usize,
    iters: usize,
    batch_size: usize,
    batch_iters: usize,
    join_lm_search: bool,
    input_mode: InputMode,
    variant_pool: usize,
    dataset_tsv: Option<String>,
    dataset_category: Option<String>,
    init_mode: InitMode,
}

#[derive(Debug)]
struct BenchResult {
    feature: &'static str,
    avg_ms: f64,
    calls_per_sec: f64,
    sink: usize,
    iters: usize,
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run --release --example bench_features -- [--text <text>] [--warmup <n>] [--iters <n>] [--batch-size <n>] [--batch-iters <n>] [--join-lm-search <true|false>] [--input-mode <repeated|varied>] [--variant-pool <n>] [--init-mode <init|new>] [--dataset-tsv <path>] [--dataset-category <name>]"
    );
}

fn parse_usize_flag(name: &str, value: Option<String>) -> Result<usize, String> {
    let raw = value.ok_or_else(|| format!("{name} requires a value"))?;
    raw.parse::<usize>()
        .map_err(|error| format!("invalid {name} value '{raw}': {error}"))
}

fn parse_bool_flag(name: &str, value: Option<String>) -> Result<bool, String> {
    let raw = value.ok_or_else(|| format!("{name} requires a value"))?;
    match raw.as_str() {
        "1" | "true" | "TRUE" | "yes" | "on" => Ok(true),
        "0" | "false" | "FALSE" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid {name} value '{raw}': expected true/false")),
    }
}

fn parse_input_mode_flag(name: &str, value: Option<String>) -> Result<InputMode, String> {
    let raw = value.ok_or_else(|| format!("{name} requires a value"))?;
    match raw.as_str() {
        "repeated" => Ok(InputMode::Repeated),
        "varied" => Ok(InputMode::Varied),
        _ => Err(format!(
            "invalid {name} value '{raw}': expected repeated|varied"
        )),
    }
}

fn parse_init_mode_flag(name: &str, value: Option<String>) -> Result<InitMode, String> {
    let raw = value.ok_or_else(|| format!("{name} requires a value"))?;
    match raw.as_str() {
        "init" => Ok(InitMode::Init),
        "new" => Ok(InitMode::New),
        _ => Err(format!("invalid {name} value '{raw}': expected init|new")),
    }
}

fn parse_args() -> Result<Cli, String> {
    let mut text = "아버지가방에들어가신다.".to_string();
    let mut warmup = 100usize;
    let mut iters = 5_000usize;
    let mut batch_size = 256usize;
    let mut batch_iters = 500usize;
    let mut join_lm_search = true;
    let mut input_mode = InputMode::Repeated;
    let mut variant_pool = 4096usize;
    let mut dataset_tsv: Option<String> = None;
    let mut dataset_category: Option<String> = None;
    let mut init_mode = InitMode::Init;

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
            "--batch-size" => batch_size = parse_usize_flag("--batch-size", args.next())?,
            "--batch-iters" => batch_iters = parse_usize_flag("--batch-iters", args.next())?,
            "--join-lm-search" => {
                join_lm_search = parse_bool_flag("--join-lm-search", args.next())?
            }
            "--input-mode" => input_mode = parse_input_mode_flag("--input-mode", args.next())?,
            "--variant-pool" => variant_pool = parse_usize_flag("--variant-pool", args.next())?,
            "--init-mode" => init_mode = parse_init_mode_flag("--init-mode", args.next())?,
            "--dataset-tsv" => {
                dataset_tsv = Some(
                    args.next()
                        .ok_or_else(|| "--dataset-tsv requires a value".to_string())?,
                )
            }
            "--dataset-category" => {
                dataset_category = Some(
                    args.next()
                        .ok_or_else(|| "--dataset-category requires a value".to_string())?,
                )
            }
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
    if batch_size == 0 {
        return Err("--batch-size must be >= 1".to_string());
    }
    if batch_iters == 0 {
        return Err("--batch-iters must be >= 1".to_string());
    }
    if variant_pool == 0 {
        return Err("--variant-pool must be >= 1".to_string());
    }
    if dataset_category.is_some() && dataset_tsv.is_none() {
        return Err("--dataset-category requires --dataset-tsv".to_string());
    }

    Ok(Cli {
        text,
        warmup,
        iters,
        batch_size,
        batch_iters,
        join_lm_search,
        input_mode,
        variant_pool,
        dataset_tsv,
        dataset_category,
        init_mode,
    })
}

fn run_bench(
    feature: &'static str,
    warmup: usize,
    iters: usize,
    mut f: impl FnMut() -> kiwi_rs::Result<usize>,
) -> kiwi_rs::Result<BenchResult> {
    for _ in 0..warmup {
        black_box(f()?);
    }

    let start = Instant::now();
    let mut sink = 0usize;
    for _ in 0..iters {
        sink = sink.wrapping_add(f()?);
        black_box(sink);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let avg_ms = (elapsed * 1_000.0) / iters as f64;
    let calls_per_sec = iters as f64 / elapsed;

    Ok(BenchResult {
        feature,
        avg_ms,
        calls_per_sec,
        sink,
        iters,
    })
}

fn print_result_named(feature: &'static str, result: &BenchResult) {
    println!(
        "feature={} avg_ms={:.6} calls_per_sec={:.2} sink={} iters={}",
        feature, result.avg_ms, result.calls_per_sec, result.sink, result.iters
    );
}

fn print_result(result: &BenchResult) {
    print_result_named(result.feature, result);
}

fn first_candidate_token_len(candidates: &[kiwi_rs::AnalysisCandidate]) -> usize {
    candidates
        .first()
        .map(|candidate| candidate.tokens.len())
        .unwrap_or(0)
}

fn sentence_payload_size(sentences: &[Sentence]) -> usize {
    fn visit(sentence: &Sentence) -> usize {
        let token_count = sentence.tokens.as_ref().map_or(0, Vec::len);
        let subs = sentence.subs.as_ref();
        let sub_count = subs.map_or(0, Vec::len);
        let nested = subs
            .map(|subs| subs.iter().map(visit).sum::<usize>())
            .unwrap_or(0);
        1 + token_count + sub_count + nested
    }

    sentences.iter().map(visit).sum()
}

fn text_sink(value: &str) -> usize {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return 0;
    };
    let last = chars.next_back().unwrap_or(first);
    (first as usize).wrapping_add(last as usize)
}

fn build_text_variants(base: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("{base} [{index}]"))
        .collect()
}

fn load_dataset_texts(path: &str, category_filter: Option<&str>) -> Result<Vec<String>, String> {
    let file = File::open(path)
        .map_err(|error| format!("failed to open dataset file '{path}': {error}"))?;
    let reader = BufReader::new(file);
    let mut texts = Vec::new();
    for (index, line_result) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line_result
            .map_err(|error| format!("failed to read dataset line {line_number}: {error}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (category, text) = if let Some((cat, value)) = trimmed.split_once('\t') {
            (cat.trim(), value.trim())
        } else {
            ("default", trimmed)
        };
        if category_filter
            .map(|wanted| !wanted.eq_ignore_ascii_case(category))
            .unwrap_or(false)
        {
            continue;
        }
        if text.is_empty() {
            return Err(format!("dataset line {line_number} has empty text"));
        }
        texts.push(text.to_string());
    }
    if texts.is_empty() {
        return Err("dataset selection produced zero texts".to_string());
    }
    Ok(texts)
}

fn cycle_text_pool(seed: &[String], total: usize) -> Vec<String> {
    (0..total)
        .map(|index| seed[index % seed.len()].clone())
        .collect()
}

fn pick_text<'a>(mode: InputMode, fallback: &'a str, pool: &'a [String], index: usize) -> &'a str {
    match mode {
        InputMode::Repeated => fallback,
        InputMode::Varied => pool
            .get(index % pool.len())
            .map(String::as_str)
            .unwrap_or(fallback),
    }
}

fn round_batch_pool_size(value: usize, batch_size: usize) -> usize {
    let chunks = value.div_ceil(batch_size);
    chunks * batch_size
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_args().map_err(|message| {
        print_usage();
        message
    })?;

    let init_start = Instant::now();
    let kiwi = match cli.init_mode {
        InitMode::Init => Kiwi::init()?,
        InitMode::New => Kiwi::new()?,
    };
    let init_elapsed = init_start.elapsed().as_secs_f64() * 1_000.0;

    let options_top1 = AnalyzeOptions::default()
        .with_top_n(1)
        .with_match_options(KIWI_MATCH_ALL);
    let mut single_variants = if let Some(path) = cli.dataset_tsv.as_deref() {
        load_dataset_texts(path, cli.dataset_category.as_deref())?
    } else {
        build_text_variants(&cli.text, cli.variant_pool)
    };
    if single_variants.len() > cli.variant_pool {
        single_variants.truncate(cli.variant_pool);
    }
    let split_text = if cli.dataset_tsv.is_some() {
        single_variants
            .first()
            .cloned()
            .unwrap_or_else(|| cli.text.clone())
    } else {
        format!("{} 두 번째 문장입니다.", cli.text)
    };
    let split_variants: Vec<String> = if cli.dataset_tsv.is_some() {
        single_variants.clone()
    } else {
        single_variants
            .iter()
            .map(|text| format!("{text} 두 번째 문장입니다."))
            .collect()
    };
    let glue_chunks = vec![
        "오늘은".to_string(),
        "날씨가".to_string(),
        "좋다.".to_string(),
    ];
    let glue_variant_pool: Vec<Vec<String>> = single_variants
        .iter()
        .map(|text| vec![text.clone(), "날씨가".to_string(), "좋다.".to_string()])
        .collect();

    let join_seed_text = single_variants
        .first()
        .map(String::as_str)
        .unwrap_or(cli.text.as_str());
    let join_tokens = kiwi.tokenize_with_match_options(join_seed_text, KIWI_MATCH_ALL)?;
    let join_owned: Vec<(String, String)> = join_tokens
        .iter()
        .map(|token| (token.form.clone(), token.tag.clone()))
        .collect();
    let join_pairs: Vec<(&str, &str)> = join_owned
        .iter()
        .map(|(form, tag)| (form.as_str(), tag.as_str()))
        .collect();
    let join_prepared = kiwi.prepare_join_morphs(&join_pairs)?;
    let joiner_reuse = kiwi.prepare_joiner(&join_prepared, cli.join_lm_search)?;

    let batch_texts: Vec<String> = if cli.dataset_tsv.is_some() {
        cycle_text_pool(&single_variants, cli.batch_size)
    } else {
        (0..cli.batch_size)
            .map(|index| format!("{} {}", cli.text, index))
            .collect()
    };
    let batch_pool_size =
        round_batch_pool_size(cli.variant_pool.max(cli.batch_size * 64), cli.batch_size);
    let batch_text_pool: Vec<String> = if cli.dataset_tsv.is_some() {
        cycle_text_pool(&single_variants, batch_pool_size)
    } else {
        (0..batch_pool_size)
            .map(|index| format!("{} {} {}", cli.text, index, index % 17))
            .collect()
    };
    let batch_round_count = batch_text_pool.len() / cli.batch_size;

    println!("engine=kiwi-rs");
    println!("text={}", cli.text);
    println!("warmup={}", cli.warmup);
    println!("iters={}", cli.iters);
    println!("batch_size={}", cli.batch_size);
    println!("batch_iters={}", cli.batch_iters);
    println!("join_lm_search={}", cli.join_lm_search);
    println!(
        "init_mode={}",
        match cli.init_mode {
            InitMode::Init => "init",
            InitMode::New => "new",
        }
    );
    println!(
        "input_mode={}",
        match cli.input_mode {
            InputMode::Repeated => "repeated",
            InputMode::Varied => "varied",
        }
    );
    println!("variant_pool={}", cli.variant_pool);
    println!("dataset_tsv={}", cli.dataset_tsv.as_deref().unwrap_or(""));
    println!(
        "dataset_category={}",
        cli.dataset_category.as_deref().unwrap_or("")
    );
    println!("dataset_entries={}", single_variants.len());
    println!("init_ms={:.3}", init_elapsed);

    let mut tokenize_round = 0usize;
    let tokenize = run_bench("tokenize", cli.warmup, cli.iters, || {
        let text = pick_text(cli.input_mode, &cli.text, &single_variants, tokenize_round);
        tokenize_round = tokenize_round.wrapping_add(1);
        Ok(kiwi
            .tokenize_with_match_options(text, KIWI_MATCH_ALL)?
            .len())
    })?;
    print_result(&tokenize);

    let mut analyze_top1_round = 0usize;
    let analyze_top1 = run_bench("analyze_top1", cli.warmup, cli.iters, || {
        let text = pick_text(
            cli.input_mode,
            &cli.text,
            &single_variants,
            analyze_top1_round,
        );
        analyze_top1_round = analyze_top1_round.wrapping_add(1);
        let candidates = kiwi.analyze_with_options(text, options_top1)?;
        Ok(first_candidate_token_len(&candidates))
    })?;
    print_result(&analyze_top1);

    let mut split_round = 0usize;
    let split_into_sents = run_bench("split_into_sents", cli.warmup, cli.iters, || {
        let text = pick_text(cli.input_mode, &split_text, &split_variants, split_round);
        split_round = split_round.wrapping_add(1);
        Ok(kiwi.split_into_sents(text, KIWI_MATCH_ALL)?.len())
    })?;
    print_result(&split_into_sents);

    let mut split_with_tokens_round = 0usize;
    let split_into_sents_with_tokens = run_bench(
        "split_into_sents_with_tokens",
        cli.warmup,
        cli.iters,
        || {
            let text = pick_text(
                cli.input_mode,
                &split_text,
                &split_variants,
                split_with_tokens_round,
            );
            split_with_tokens_round = split_with_tokens_round.wrapping_add(1);
            let sentences = kiwi.split_into_sents_with_options(text, options_top1, true, true)?;
            Ok(sentence_payload_size(&sentences))
        },
    )?;
    print_result(&split_into_sents_with_tokens);

    let mut space_round = 0usize;
    let space = run_bench("space", cli.warmup, cli.iters, || {
        let text = pick_text(cli.input_mode, &split_text, &split_variants, space_round);
        space_round = space_round.wrapping_add(1);
        Ok(text_sink(&kiwi.space(text, true)?))
    })?;
    print_result(&space);

    let join = run_bench("join", cli.warmup, cli.iters, || {
        Ok(text_sink(&kiwi.join(&join_pairs, cli.join_lm_search)?))
    })?;
    print_result(&join);

    let join_prepared_bench = run_bench("join_prepared", cli.warmup, cli.iters, || {
        Ok(text_sink(
            &kiwi.join_prepared(&join_prepared, cli.join_lm_search)?,
        ))
    })?;
    print_result(&join_prepared_bench);

    let join_prepared_utf16_bench =
        run_bench("join_prepared_utf16", cli.warmup, cli.iters, || {
            Ok(text_sink(
                &kiwi.join_prepared_utf16(&join_prepared, cli.join_lm_search)?,
            ))
        })?;
    print_result(&join_prepared_utf16_bench);

    let joiner_reuse_bench = run_bench("joiner_reuse", cli.warmup, cli.iters, || {
        Ok(text_sink(&joiner_reuse.get()?))
    })?;
    print_result(&joiner_reuse_bench);

    let joiner_reuse_utf16_bench = run_bench("joiner_reuse_utf16", cli.warmup, cli.iters, || {
        Ok(text_sink(&joiner_reuse.get_utf16()?))
    })?;
    print_result(&joiner_reuse_utf16_bench);

    let mut glue_round = 0usize;
    let glue = run_bench("glue", cli.warmup, cli.iters, || {
        let chunks = if cli.input_mode == InputMode::Varied {
            let chunks = &glue_variant_pool[glue_round % glue_variant_pool.len()];
            glue_round = glue_round.wrapping_add(1);
            chunks
        } else {
            &glue_chunks
        };
        Ok(text_sink(&kiwi.glue(chunks)?))
    })?;
    print_result(&glue);

    let mut analyze_many_loop_round = 0usize;
    let analyze_many_loop = run_bench("analyze_many_loop", cli.warmup, cli.batch_iters, || {
        let batch = if cli.input_mode == InputMode::Varied {
            let start = (analyze_many_loop_round % batch_round_count) * cli.batch_size;
            analyze_many_loop_round = analyze_many_loop_round.wrapping_add(1);
            &batch_text_pool[start..start + cli.batch_size]
        } else {
            &batch_texts
        };
        let mut total = 0usize;
        for text in batch {
            let candidates = kiwi.analyze_with_options(text, options_top1)?;
            total = total.wrapping_add(first_candidate_token_len(&candidates));
        }
        Ok(total)
    })?;
    print_result(&analyze_many_loop);

    let mut analyze_many_native_round = 0usize;
    let analyze_many_native =
        run_bench("analyze_many_native", cli.warmup, cli.batch_iters, || {
            let batch = if cli.input_mode == InputMode::Varied {
                let start = (analyze_many_native_round % batch_round_count) * cli.batch_size;
                analyze_many_native_round = analyze_many_native_round.wrapping_add(1);
                &batch_text_pool[start..start + cli.batch_size]
            } else {
                &batch_texts
            };
            let results = kiwi.analyze_many_via_native(batch, options_top1)?;
            Ok(results
                .iter()
                .map(|candidates| first_candidate_token_len(candidates))
                .sum())
        })?;
    print_result(&analyze_many_native);
    print_result_named("batch_analyze_native", &analyze_many_native);

    let mut tokenize_many_loop_round = 0usize;
    let tokenize_many_loop = run_bench("tokenize_many_loop", cli.warmup, cli.batch_iters, || {
        let batch = if cli.input_mode == InputMode::Varied {
            let start = (tokenize_many_loop_round % batch_round_count) * cli.batch_size;
            tokenize_many_loop_round = tokenize_many_loop_round.wrapping_add(1);
            &batch_text_pool[start..start + cli.batch_size]
        } else {
            &batch_texts
        };
        let mut total = 0usize;
        for text in batch {
            total = total.wrapping_add(kiwi.tokenize(text)?.len());
        }
        Ok(total)
    })?;
    print_result(&tokenize_many_loop);

    let mut tokenize_many_batch_round = 0usize;
    let tokenize_many_batch =
        run_bench("tokenize_many_batch", cli.warmup, cli.batch_iters, || {
            let batch = if cli.input_mode == InputMode::Varied {
                let start = (tokenize_many_batch_round % batch_round_count) * cli.batch_size;
                tokenize_many_batch_round = tokenize_many_batch_round.wrapping_add(1);
                &batch_text_pool[start..start + cli.batch_size]
            } else {
                &batch_texts
            };
            let results = kiwi.tokenize_many(batch)?;
            Ok(results.iter().map(Vec::len).sum())
        })?;
    print_result(&tokenize_many_batch);

    let mut split_many_loop_round = 0usize;
    let split_many_loop = run_bench("split_many_loop", cli.warmup, cli.batch_iters, || {
        let batch = if cli.input_mode == InputMode::Varied {
            let start = (split_many_loop_round % batch_round_count) * cli.batch_size;
            split_many_loop_round = split_many_loop_round.wrapping_add(1);
            &batch_text_pool[start..start + cli.batch_size]
        } else {
            &batch_texts
        };
        let mut total = 0usize;
        for text in batch {
            total = total.wrapping_add(kiwi.split_into_sents(text, KIWI_MATCH_ALL)?.len());
        }
        Ok(total)
    })?;
    print_result(&split_many_loop);

    let mut space_many_loop_round = 0usize;
    let space_many_loop = run_bench("space_many_loop", cli.warmup, cli.batch_iters, || {
        let batch = if cli.input_mode == InputMode::Varied {
            let start = (space_many_loop_round % batch_round_count) * cli.batch_size;
            space_many_loop_round = space_many_loop_round.wrapping_add(1);
            &batch_text_pool[start..start + cli.batch_size]
        } else {
            &batch_texts
        };
        let mut total = 0usize;
        for text in batch {
            total = total.wrapping_add(text_sink(&kiwi.space(text, true)?));
        }
        Ok(total)
    })?;
    print_result(&space_many_loop);

    let mut space_many_batch_round = 0usize;
    let space_many_batch = run_bench("space_many_batch", cli.warmup, cli.batch_iters, || {
        let batch = if cli.input_mode == InputMode::Varied {
            let start = (space_many_batch_round % batch_round_count) * cli.batch_size;
            space_many_batch_round = space_many_batch_round.wrapping_add(1);
            &batch_text_pool[start..start + cli.batch_size]
        } else {
            &batch_texts
        };
        let results = kiwi.space_many(batch, true)?;
        Ok(results.iter().map(|text| text_sink(text)).sum())
    })?;
    print_result(&space_many_batch);

    Ok(())
}
