use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let lines = vec![
        "첫 번째 문장입니다.",
        "두 번째 문장도 형태소 분석합니다.",
        "세 번째 입력입니다.",
    ];

    let options = AnalyzeOptions::default().with_top_n(2);

    let batched = kiwi.analyze_many_via_native(&lines, options)?;
    for (index, candidates) in batched.iter().enumerate() {
        println!("line #{index}: {} candidates", candidates.len());
    }

    if kiwi.supports_analyze_mw() {
        let utf16_lines: Vec<Vec<u16>> = lines
            .iter()
            .map(|line| line.encode_utf16().collect())
            .collect();
        let batched_w = kiwi.analyze_many_utf16_via_native(utf16_lines, options)?;
        println!("utf16 batch count: {}", batched_w.len());
    }

    Ok(())
}
