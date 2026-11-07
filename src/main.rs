//! PC 사양(RAM, GPU VRAM)을 확인해 그 안에서 돌릴 수 있는 로컬 AI 모델을 보여준다.
//! 모델 목록은 models.json을 빌드 때 넣고, --update로 받은 최신 목록이 exe 옆에 있으면 그걸 쓴다.

mod update;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

const MODELS_JSON: &str = include_str!("../models.json");
/// OS·다른 프로그램 몫으로 남겨두는 RAM(GB)
const RAM_RESERVE: f64 = 3.0;
/// 화면 출력·드라이버 몫으로 남겨두는 VRAM(GB)
const VRAM_RESERVE: f64 = 0.7;
/// Apple Silicon에서 GPU가 쓸 수 있는 통합 메모리 비율
const UNIFIED_GPU_SHARE: f64 = 0.70;
const CATEGORIES: [&str; 8] = ["대화", "추론", "코딩", "비전", "번역", "임베딩", "음성인식", "이미지생성"];

#[derive(Deserialize, Serialize, Clone)]
struct Model {
    tag: String,
    cat: String,
    /// 파일 크기(GB, Ollama 기본 양자화 = 대부분 Q4_K_M)
    size: f64,
    /// 전체 파라미터 수(B)
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<f64>,
    /// MoE 모델의 토큰당 활성 파라미터(B)
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<f64>,
    /// 문맥(KV 캐시) 등 추가 메모리(GB). 없으면 파라미터 수로 추정
    #[serde(skip_serializing_if = "Option::is_none")]
    overhead: Option<f64>,
    /// 없으면 Ollama
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    gpu_only: bool,
    note: String,
}

impl Model {
    /// 실행에 필요한 메모리(GB) = 파일 크기 + 문맥(약 8K 토큰) 몫
    fn need(&self) -> f64 {
        self.size + self.overhead().max(0.0)
    }
    fn overhead(&self) -> f64 {
        self.overhead
            .unwrap_or_else(|| (0.5 + 0.03 * self.params.unwrap_or(self.size / 0.6)).min(4.0))
    }
    /// 같은 모델을 Q8_0(약 8.5bit)로 받았을 때 필요한 메모리
    fn need_q8(&self) -> Option<f64> {
        let p = self.params?;
        let extra = (self.size - p * 0.6).max(0.0); // 비전 인코더 등 양자화와 무관한 부분
        Some(p * 1.07 + extra + self.overhead())
    }
    fn runtime(&self) -> &str {
        self.runtime.as_deref().unwrap_or("Ollama")
    }
}

struct Gpu {
    name: String,
    vram: f64,
    unified: bool,
}

struct Sys {
    cpu: String,
    ram: f64,
    gpu: Option<Gpu>,
}

#[derive(Clone, Copy, PartialEq)]
enum Fit {
    Gpu,
    /// GPU+CPU 나눠 실행. 값은 GPU에 올라가는 비율
    Split(f64),
    Cpu,
    No,
}

impl Fit {
    fn rank(self) -> u8 {
        match self {
            Fit::Gpu => 0,
            Fit::Split(_) => 1,
            Fit::Cpu => 2,
            Fit::No => 3,
        }
    }
    fn label(self) -> String {
        match self {
            Fit::Gpu => "GPU 전체".into(),
            Fit::Split(f) => format!("GPU {:.0}%+CPU", f * 100.0),
            Fit::Cpu => "CPU".into(),
            Fit::No => "불가".into(),
        }
    }
}

// ---------- 사양 감지 ----------

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn nvidia_gpu() -> Option<Gpu> {
    let out = run("nvidia-smi", &["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])?;
    out.lines()
        .filter_map(|l| {
            let (name, mib) = l.rsplit_once(',')?;
            Some(Gpu {
                name: name.trim().into(),
                vram: mib.trim().parse::<f64>().ok()? * 1_048_576.0 / 1e9,
                unified: false,
            })
        })
        .max_by(|a, b| a.vram.total_cmp(&b.vram))
}

/// NVIDIA 외 GPU(AMD·Intel): 드라이버가 레지스트리에 적어둔 VRAM 크기를 읽는다.
/// (WMI의 AdapterRAM은 4GB에서 잘려서 쓰지 않음)
#[cfg(windows)]
fn other_gpu() -> Option<Gpu> {
    let ps = r#"Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*' -EA SilentlyContinue | ForEach-Object { "$($_.DriverDesc)|$($_.'HardwareInformation.qwMemorySize')" }"#;
    let out = run("powershell", &["-NoProfile", "-Command", ps])?;
    out.lines()
        .filter_map(|l| {
            let (name, bytes) = l.trim().rsplit_once('|')?;
            Some(Gpu { name: name.into(), vram: bytes.parse::<f64>().ok()? / 1e9, unified: false })
        })
        .max_by(|a, b| a.vram.total_cmp(&b.vram))
}

/// Linux AMD: amdgpu 드라이버가 sysfs에 VRAM 크기를 노출한다.
#[cfg(not(windows))]
fn other_gpu() -> Option<Gpu> {
    std::fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .filter_map(|e| {
            let s = std::fs::read_to_string(e.path().join("device/mem_info_vram_total")).ok()?;
            Some(Gpu { name: "AMD GPU".into(), vram: s.trim().parse::<f64>().ok()? / 1e9, unified: false })
        })
        .max_by(|a, b| a.vram.total_cmp(&b.vram))
}

fn detect() -> Sys {
    let mut s = sysinfo::System::new();
    s.refresh_memory();
    s.refresh_cpu_all();
    let ram = s.total_memory() as f64 / 1e9;
    let cpu = s.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();

    let gpu = if cfg!(target_os = "macos") && cpu.starts_with("Apple") {
        Some(Gpu { name: format!("{cpu} (통합 메모리)"), vram: ram * UNIFIED_GPU_SHARE, unified: true })
    } else {
        // 2GB 미만은 내장 그래픽으로 보고 GPU 없음 취급
        nvidia_gpu().or_else(other_gpu).filter(|g| g.vram >= 2.0)
    };
    Sys { cpu, ram, gpu }
}

// ---------- 판정 ----------

fn usable_vram(g: &Gpu) -> f64 {
    if g.unified { g.vram } else { (g.vram - VRAM_RESERVE).max(0.0) }
}

fn fit(m: &Model, s: &Sys) -> Fit {
    let need = m.need();
    let ram = (s.ram - RAM_RESERVE).max(0.0);
    match &s.gpu {
        Some(g) if g.unified => if need <= usable_vram(g) { Fit::Gpu } else { Fit::No },
        Some(g) => {
            let v = usable_vram(g);
            if need <= v {
                Fit::Gpu
            } else if !m.gpu_only && need <= v + ram {
                Fit::Split(v / need)
            } else {
                Fit::No
            }
        }
        None if !m.gpu_only && need <= ram => Fit::Cpu,
        None => Fit::No,
    }
}

/// 대략적인 체감 속도. 생성 속도는 토큰마다 읽는 가중치 양(활성 파라미터)과 그게 어디 있느냐로 정해진다.
fn speed(m: &Model, f: Fit) -> &'static str {
    let active_gb = m.active.map(|a| a * 0.6).unwrap_or(m.size);
    match f {
        Fit::Gpu => "빠름",
        Fit::Split(r) if m.active.is_some() || r >= 0.7 => "보통",
        Fit::Split(r) if r >= 0.3 => "느림",
        Fit::Split(_) => "매우 느림",
        Fit::Cpu if active_gb <= 2.5 => "보통",
        Fit::Cpu if active_gb <= 6.0 => "느림",
        Fit::Cpu => "매우 느림",
        Fit::No => "-",
    }
}

// ---------- 출력 ----------

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
    println!("RAM : {:.1} GB = {:.0} GiB (모델에 쓸 수 있는 양 약 {:.1} GB)", s.ram, gib(s.ram), (s.ram - RAM_RESERVE).max(0.0));
    match &s.gpu {
        Some(g) => println!("GPU : {} / {:.1} GB = {:.0} GiB (모델에 쓸 수 있는 양 약 {:.1} GB)", g.name, g.vram, gib(g.vram), usable_vram(g)),
        None => println!("GPU : 쓸 만한 GPU 없음 → CPU로만 실행"),
    }
    println!("\n기준: 문맥 약 8K 토큰, Ollama 기본 양자화(대부분 Q4_K_M). 속도는 대략적인 체감 등급이에요.\n");

    for cat in CATEGORIES {
        let mut rows: Vec<(&Model, Fit)> = models
            .iter()
            .filter(|m| m.cat == cat)
            .map(|m| (m, fit(m, s)))
            .filter(|(_, f)| show_all || *f != Fit::No)
            .collect();
        rows.sort_by(|a, b| a.1.rank().cmp(&b.1.rank()).then(b.0.size.total_cmp(&a.0.size)));

        println!("■ {cat}");
        if rows.is_empty() {
            println!("  (이 사양에서 돌릴 수 있는 모델이 없어요)\n");
            continue;
        }
        // 추천: GPU에 전부 올라가는 것 중 가장 큰 모델, 없으면 실행 가능한 것 중 가장 큰 모델
        let best = rows.iter().filter(|r| r.1 == Fit::Gpu).max_by(|a, b| a.0.size.total_cmp(&b.0.size))
            .or_else(|| rows.iter().filter(|r| r.1 != Fit::No).max_by(|a, b| a.0.size.total_cmp(&b.0.size)));
        if let Some((m, _)) = best {
            println!("  추천: {}", m.tag);
        }
        println!("  {} {} {} {} {} 메모", pad("모델", 24), pad("실행도구", 11), pad("크기/필요(GB)", 14), pad("실행 방식", 12), pad("속도", 9));
        for (m, f) in rows {
            let mut note = m.note.clone();
            let room = s.gpu.as_ref().map(usable_vram).unwrap_or(0.0);
            if f == Fit::Gpu && m.need_q8().is_some_and(|q| q <= room) {
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

사용법: local-ai-fit [옵션]
  --all          돌릴 수 없는 모델도 함께 표시
  --ram <GB>     RAM 용량을 직접 지정(다른 PC 사양 가정)
  --vram <GB>    GPU VRAM을 직접 지정(0이면 GPU 없음)
  --markdown     전체 모델 목록과 필요 사양을 마크다운 표로 출력
  --update       Ollama에서 최신 모델·크기를 받아 exe 옆 models.json에 저장
                 (다음 실행부터 이 목록을 씀. 지우면 내장 목록으로 돌아감)
  -h, --help     도움말";

/// --update로 받은 목록을 두는 곳: 실행 파일 옆 models.json
fn saved_list_path() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("models.json"))
}

/// 저장된 목록이 있으면 그것, 없거나 깨졌으면 내장 목록
fn load_models() -> (Vec<Model>, String) {
    if let Some(p) = saved_list_path() {
        if let Ok(text) = std::fs::read_to_string(&p) {
            match serde_json::from_str(&text) {
                Ok(m) => return (m, format!("{} (--update로 받은 목록)", p.display())),
                Err(e) => eprintln!("{} 형식 오류({e}), 내장 목록을 써요.", p.display()),
            }
        }
    }
    (serde_json::from_str(MODELS_JSON).expect("내장 models.json 형식 오류"), "내장 목록".into())
}

fn save_models(models: &[Model], path: &std::path::Path) -> std::io::Result<()> {
    let lines: Vec<String> = models.iter().map(|m| format!("  {}", serde_json::to_string(m).unwrap())).collect();
    std::fs::write(path, format!("[\n{}\n]\n", lines.join(",\n")))
}

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
