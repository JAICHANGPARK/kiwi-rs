use kiwi_rs::{Kiwi, KIWI_MATCH_ALL};
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (dataset_path, out_path) = parse_args()?;
    let rows = load_dataset_rows(&dataset_path)?;

    let kiwi = Kiwi::init()?;
    let out_file = File::create(&out_path)?;
    let mut writer = BufWriter::new(out_file);

    for (index, (category, text)) in rows.iter().enumerate() {
        let tokens = kiwi.tokenize(text)?;
        let boundaries = kiwi.split_into_sents(text, KIWI_MATCH_ALL)?;

        write!(writer, "{{\"index\":{},\"category\":\"", index)?;
        write_escaped_json(&mut writer, category)?;
        write!(writer, "\",\"text\":\"")?;
        write_escaped_json(&mut writer, text)?;
        write!(writer, "\",\"tokens\":[")?;

        for (token_index, token) in tokens.iter().enumerate() {
            if token_index > 0 {
                write!(writer, ",")?;
            }
            write!(
                writer,
                "{{\"form\":\"{}\",\"tag\":\"{}\",\"start\":{},\"len\":{}}}",
                json_escape(&token.form),
                json_escape(&token.tag),
                token.position,
                token.length
            )?;
        }

        write!(writer, "],\"sents\":[")?;
        for (sent_index, sent) in boundaries.iter().enumerate() {
            if sent_index > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "[{},{}]", sent.begin, sent.end)?;
        }
        writeln!(writer, "]}}")?;
    }

    writer.flush()?;
    println!("wrote {} rows to {}", rows.len(), out_path.display());
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let mut dataset_tsv: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dataset-tsv" => {
                let value = args
                    .next()
                    .ok_or("--dataset-tsv requires a path argument")?;
                dataset_tsv = Some(PathBuf::from(value));
            }
            "--out" => {
                let value = args.next().ok_or("--out requires a path argument")?;
                out_path = Some(PathBuf::from(value));
            }
            _ => {
                return Err(format!(
                    "unknown argument: {arg}. expected --dataset-tsv <path> --out <path>"
                )
                .into());
            }
        }
    }

    let dataset_tsv = dataset_tsv.ok_or("missing --dataset-tsv")?;
    let out_path = out_path.ok_or("missing --out")?;
    Ok((dataset_tsv, out_path))
}

fn load_dataset_rows(path: &PathBuf) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for (line_no, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let category = parts
            .next()
            .ok_or_else(|| format!("line {} missing category", line_no + 1))?;
        let text = parts
            .next()
            .ok_or_else(|| format!("line {} missing text", line_no + 1))?;
        rows.push((category.to_string(), text.to_string()));
    }
    if rows.is_empty() {
        return Err("dataset contains no usable rows".into());
    }
    Ok(rows)
}

fn write_escaped_json<W: Write>(
    writer: &mut W,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    writer.write_all(json_escape(value).as_bytes())?;
    Ok(())
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}
