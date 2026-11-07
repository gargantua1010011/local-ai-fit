//! CLI: 내 PC에서 돌릴 수 있는 로컬 AI 모델을 터미널 표로 보여준다. 판정 로직은 lib.rs.

use local_ai_fit::*;

/// 터미널 표시 폭(한글은 2칸)
fn width(s: &str) -> usize {
    s.chars()
        .map(|c| if matches!(c, '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' | '\u{AC00}'..='\u{D7A3}') { 2 } else { 1 })
        .sum()
}

fn pad(s: &str, n: usize) -> String {
    format!("{s}{}", " ".repeat(n.saturating_sub(width(s))))
}

fn print_fit(models: &[Model], s: &Sys, show_all: bool) {
    println!("== 내 PC 사양 ==");
    println!("CPU : {}", if s.cpu.is_empty() { "(알 수 없음)" } else { &s.cpu });
    // 모델 크기가 10진 GB라 계산도 GB로 하고, 흔히 말하는 "16GB 램"(GiB)도 같이 보여준다
    let gib = |gb: f64| gb * 1e9 / 1_073_741_824.0;
    println!("RAM : {:.1} GB = {:.0} GiB (모델에 쓸 수 있는 양 약 {:.1} GB)", s.ram, gib(s.ram), usable_ram(s));
    match &s.gpu {
        Some(g) => println!("GPU : {} / {:.1} GB = {:.0} GiB (모델에 쓸 수 있는 양 약 {:.1} GB)", g.name, g.vram, gib(g.vram), usable_vram(g)),
        None => println!("GPU : 쓸 만한 GPU 없음 → CPU로만 실행"),
    }
    println!("\n기준: 문맥 약 8K 토큰, Ollama 기본 양자화(대부분 Q4_K_M). 속도는 대략적인 체감 등급이에요.\n");

    for cat in CATEGORIES {
        let rows = rows(models, s, cat, show_all);
        println!("■ {cat}");
        if rows.is_empty() {
            println!("  (이 사양에서 돌릴 수 있는 모델이 없어요)\n");
            continue;
        }
        if let Some(m) = best(&rows) {
            println!("  추천: {}", m.tag);
        }
        println!("  {} {} {} {} {} 메모", pad("모델", 24), pad("실행도구", 11), pad("크기/필요(GB)", 14), pad("실행 방식", 12), pad("속도", 9));
        for (m, f) in rows {
            let mut note = m.note.clone();
            if q8_fits(m, f, s) {
                note += " · Q8도 GPU에 들어감";
            }
            println!(
                "  {} {} {} {} {} {}",
                pad(&m.tag, 24),
                pad(m.runtime(), 11),
                pad(&format!("{:.1} / {:.1}", m.size, m.need()), 14),
                pad(&f.label(), 12),
                pad(speed(m, f), 9),
                note
            );
        }
        println!();
    }
    if !show_all {
        println!("돌릴 수 없는 모델까지 보려면 --all, 다른 사양을 가정하려면 --ram 32 --vram 8 처럼 실행하세요.");
    }
}

fn print_markdown(models: &[Model]) {
    println!("# 로컬 AI 모델 목록과 필요 사양\n");
    println!("`local-ai-fit --markdown`으로 생성한 파일이에요. 원본 데이터는 `models.json`이에요.\n");
    println!("- **파일 크기**: Ollama 기본 태그 기준(대부분 Q4_K_M 양자화). `--update`로 Ollama 레지스트리에서 최신 값을 받아요.");
    println!("- 비고가 \"(자동 추가)\"인 모델은 `--update`가 새로 찾아 넣은 것이라 분야 분류가 틀릴 수 있고, MoE 활성 파라미터는 비어 있어요.");
    println!("- **필요 메모리**: 파일 크기 + 문맥(약 8K 토큰) 몫. 문맥을 길게 쓰면 더 필요해요.");
    println!("- **GPU 전체 실행**: 필요 메모리 + {VRAM_RESERVE}GB(화면·드라이버 몫) 이상의 VRAM이면 모델 전체가 GPU에 올라가 빨라요.");
    println!("- **CPU 실행**: 필요 메모리 + {RAM_RESERVE}GB(OS 몫) 이상의 RAM이면 GPU 없이도 돌아가요(느림). VRAM이 모자라면 VRAM+RAM에 나눠 올릴 수도 있어요.");
    println!("- **MoE**: 전체 파라미터는 크지만 토큰마다 일부(활성 파라미터)만 계산해서, CPU나 분할 실행에서도 비교적 빨라요.");
    println!("- Apple Silicon Mac은 통합 메모리의 약 {:.0}%를 GPU가 쓸 수 있다고 보고 계산해요.\n", UNIFIED_GPU_SHARE * 100.0);
    for cat in CATEGORIES {
        println!("## {cat}\n");
        println!("| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |");
        println!("|---|---|---|---|---|---|---|---|");
        for m in models.iter().filter(|m| m.cat == cat) {
            let params = match (m.params, m.active) {
                (Some(p), Some(a)) => format!("{p}B (활성 {a}B)"),
                (Some(p), None) => format!("{p}B"),
                _ => "-".into(),
            };
            let cpu = if m.gpu_only { "GPU 필요".into() } else { format!("{:.0} GB", (m.need() + RAM_RESERVE).ceil()) };
            println!(
                "| `{}` | {} | {} | {:.1} GB | {:.1} GB | {:.0} GB | {} | {} |",
                m.tag, m.runtime(), params, m.size, m.need(), (m.need() + VRAM_RESERVE).ceil(), cpu, m.note
            );
        }
        println!();
    }
}

const HELP: &str = "local-ai-fit: 내 PC에서 돌릴 수 있는 로컬 AI 모델 보기
(창으로 보려면 local-ai-fit-gui.exe)

사용법: local-ai-fit [옵션]
  --all          돌릴 수 없는 모델도 함께 표시
  --ram <GB>     RAM 용량을 직접 지정(다른 PC 사양 가정)
  --vram <GB>    GPU VRAM을 직접 지정(0이면 GPU 없음)
  --markdown     전체 모델 목록과 필요 사양을 마크다운 표로 출력
  --update       Ollama에서 최신 모델·크기를 받아 exe 옆 models.json에 저장
                 (다음 실행부터 이 목록을 씀. 지우면 내장 목록으로 돌아감)
  -h, --help     도움말";

/// 탐색기에서 더블클릭으로 실행했는지(콘솔 창에 이 프로그램 하나뿐인지)
#[cfg(windows)]
fn launched_alone() -> bool {
    extern "system" {
        fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
    }
    let mut buf = [0u32; 4];
    unsafe { GetConsoleProcessList(buf.as_mut_ptr(), buf.len() as u32) == 1 }
}
#[cfg(not(windows))]
fn launched_alone() -> bool {
    false
}

fn main() {
    let (models, source) = load_models();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |flag: &str| -> Option<f64> {
        let i = args.iter().position(|a| a == flag)?;
        let v = args.get(i + 1).and_then(|v| v.parse().ok());
        if v.is_none() {
            eprintln!("{flag} 뒤에 숫자(GB)를 적어주세요.");
            std::process::exit(2);
        }
        v
    };

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{HELP}");
        return;
    }
    if args.iter().any(|a| a == "--markdown") {
        print_markdown(&models);
        return;
    }
    if args.iter().any(|a| a == "--update") {
        let Some(updated) = update::update(&models) else { std::process::exit(1) };
        match saved_list_path().map(|p| save_models(&updated, &p).map(|_| p)) {
            Some(Ok(p)) => println!("모델 {}개를 {}에 저장했어요.", updated.len(), p.display()),
            Some(Err(e)) => eprintln!("저장 실패: {e}"),
            None => eprintln!("실행 파일 위치를 알 수 없어 저장하지 못했어요."),
        }
        return;
    }

    let mut sys = detect();
    if let Some(r) = num("--ram") {
        sys.ram = r;
    }
    if let Some(v) = num("--vram") {
        sys.gpu = (v > 0.0).then(|| Gpu { name: "직접 입력".into(), vram: v, unified: false });
    }
    println!("모델 목록: {source}");
    print_fit(&models, &sys, args.iter().any(|a| a == "--all"));

    if launched_alone() {
        println!("\nEnter를 누르면 닫혀요.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}
