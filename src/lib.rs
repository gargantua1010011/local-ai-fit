//! PC 사양(RAM, GPU VRAM)을 확인해 그 안에서 돌릴 수 있는 로컬 AI 모델을 판정한다.
//! CLI(main.rs)와 GUI(bin/local-ai-fit-gui.rs)가 같이 쓴다.
//! 모델 목록은 models.json을 빌드 때 넣고, --update로 받은 최신 목록이 exe 옆에 있으면 그걸 쓴다.

pub mod update;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

const MODELS_JSON: &str = include_str!("../models.json");
/// OS·다른 프로그램 몫으로 남겨두는 RAM(GB)
pub const RAM_RESERVE: f64 = 3.0;
/// 화면 출력·드라이버 몫으로 남겨두는 VRAM(GB)
pub const VRAM_RESERVE: f64 = 0.7;
/// Apple Silicon에서 GPU가 쓸 수 있는 통합 메모리 비율
pub const UNIFIED_GPU_SHARE: f64 = 0.70;
pub const CATEGORIES: [&str; 8] = ["대화", "추론", "코딩", "비전", "번역", "임베딩", "음성인식", "이미지생성"];

#[derive(Deserialize, Serialize, Clone)]
pub struct Model {
    pub tag: String,
    pub cat: String,
    /// 파일 크기(GB, Ollama 기본 양자화 = 대부분 Q4_K_M)
    pub size: f64,
    /// 전체 파라미터 수(B)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<f64>,
    /// MoE 모델의 토큰당 활성 파라미터(B)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<f64>,
    /// 문맥(KV 캐시) 등 추가 메모리(GB). 없으면 파라미터 수로 추정
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overhead: Option<f64>,
    /// 없으면 Ollama
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gpu_only: bool,
    pub note: String,
}

impl Model {
    /// 실행에 필요한 메모리(GB) = 파일 크기 + 문맥(약 8K 토큰) 몫
    pub fn need(&self) -> f64 {
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
    pub fn runtime(&self) -> &str {
        self.runtime.as_deref().unwrap_or("Ollama")
    }
}

#[derive(Clone)]
pub struct Gpu {
    pub name: String,
    pub vram: f64,
    pub unified: bool,
}

#[derive(Clone)]
pub struct Sys {
    pub cpu: String,
    pub ram: f64,
    pub gpu: Option<Gpu>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Fit {
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
    pub fn label(self) -> String {
        match self {
            Fit::Gpu => "GPU 전체".into(),
            Fit::Split(f) => format!("GPU {:.0}%+CPU", f * 100.0),
            Fit::Cpu => "CPU".into(),
            Fit::No => "불가".into(),
        }
    }
}

// ---------- 사양 감지 ----------

/// 외부 명령. GUI에서 실행해도 콘솔 창이 깜빡이지 않게 한다.
pub(crate) fn command(cmd: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(cmd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = command(cmd).args(args).output().ok()?;
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

pub fn detect() -> Sys {
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

pub fn usable_vram(g: &Gpu) -> f64 {
    if g.unified { g.vram } else { (g.vram - VRAM_RESERVE).max(0.0) }
}

pub fn usable_ram(s: &Sys) -> f64 {
    (s.ram - RAM_RESERVE).max(0.0)
}

pub fn fit(m: &Model, s: &Sys) -> Fit {
    let need = m.need();
    let ram = usable_ram(s);
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
pub fn speed(m: &Model, f: Fit) -> &'static str {
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

/// 한 분야의 모델을 판정해 실행 방식(GPU→분할→CPU→불가), 크기 큰 순으로 정렬
pub fn rows<'a>(models: &'a [Model], s: &Sys, cat: &str, show_all: bool) -> Vec<(&'a Model, Fit)> {
    let mut rows: Vec<(&Model, Fit)> = models
        .iter()
        .filter(|m| m.cat == cat)
        .map(|m| (m, fit(m, s)))
        .filter(|(_, f)| show_all || *f != Fit::No)
        .collect();
    rows.sort_by(|a, b| a.1.rank().cmp(&b.1.rank()).then(b.0.size.total_cmp(&a.0.size)));
    rows
}

/// 추천: GPU에 전부 올라가는 것 중 가장 큰 모델, 없으면 실행 가능한 것 중 가장 큰 모델
pub fn best<'a>(rows: &[(&'a Model, Fit)]) -> Option<&'a Model> {
    let biggest = |want: &dyn Fn(Fit) -> bool| {
        rows.iter().filter(|r| want(r.1)).max_by(|a, b| a.0.size.total_cmp(&b.0.size)).map(|r| r.0)
    };
    biggest(&|f| f == Fit::Gpu).or_else(|| biggest(&|f| f != Fit::No))
}

/// 더 정확한 Q8 양자화도 GPU에 다 들어가는지
pub fn q8_fits(m: &Model, f: Fit, s: &Sys) -> bool {
    let room = s.gpu.as_ref().map(usable_vram).unwrap_or(0.0);
    f == Fit::Gpu && m.need_q8().is_some_and(|q| q <= room)
}

// ---------- 목록 저장·불러오기 ----------

/// --update로 받은 목록을 두는 곳: 실행 파일 옆 models.json
pub fn saved_list_path() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("models.json"))
}

/// 저장된 목록이 있으면 그것, 없거나 깨졌으면 내장 목록
pub fn load_models() -> (Vec<Model>, String) {
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

pub fn save_models(models: &[Model], path: &std::path::Path) -> std::io::Result<()> {
    let lines: Vec<String> = models.iter().map(|m| format!("  {}", serde_json::to_string(m).unwrap())).collect();
    std::fs::write(path, format!("[\n{}\n]\n", lines.join(",\n")))
}

// ── 벤치마크 점수(비교 탭) ──────────────────────────────────────────────

const BENCHMARKS_JSON: &str = include_str!("../benchmarks.json");

#[derive(Deserialize)]
pub struct Bench {
    pub key: String,
    pub name: String,
    pub desc: String,
}

#[derive(Deserialize)]
pub struct BenchModel {
    /// Artificial Analysis에서 쓰는 모델 이름(생각 모드 여부 등이 붙어 있음)
    pub aa_name: String,
    pub url: String,
    /// 벤치마크 키 → 점수(0~100). 측정되지 않은 벤치마크는 빠져 있다.
    pub scores: std::collections::HashMap<String, f64>,
}

/// scripts/fetch_benchmarks.py가 만든 benchmarks.json(빌드 때 exe에 포함)
#[derive(Deserialize)]
pub struct Benchmarks {
    pub source: String,
    pub updated: String,
    pub benchmarks: Vec<Bench>,
    pub models: std::collections::HashMap<String, BenchModel>,
    /// 비교용 상용(클라우드) 모델
    #[serde(default)]
    pub commercial: Vec<CommercialModel>,
}

#[derive(Deserialize)]
pub struct CommercialModel {
    pub name: String,
    /// "최신" / "예전 참고"
    pub group: String,
    #[serde(flatten)]
    pub bench: BenchModel,
}

pub fn load_benchmarks() -> Benchmarks {
    serde_json::from_str(BENCHMARKS_JSON).expect("내장 benchmarks.json 형식 오류")
}
