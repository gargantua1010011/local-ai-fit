//! --update: Ollama에서 최신 모델 정보를 받아 목록을 갱신한다.
//! - 크기·파라미터: 공식 레지스트리 API(ollama pull이 쓰는 것)라 정확하고 잘 안 바뀐다.
//! - 새 모델 찾기: 공식 API가 없어 ollama.com/library 페이지를 읽는다(사이트 구조가 바뀌면 이 부분만 고치면 됨).
//! HTTP는 OS 기본 curl을 쓴다(Rust TLS 라이브러리는 C 컴파일러가 필요해서).

use crate::Model;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const LIBRARY: &str = "https://ollama.com/library";
const REGISTRY: &str = "https://registry.ollama.ai/v2/library";
const MANIFEST: &str = "application/vnd.docker.distribution.manifest.v2+json";
/// 새로 추가할 모델의 최소 다운로드 수(잘 안 쓰이는 모델까지 다 넣지 않으려고)
const MIN_PULLS: f64 = 500_000.0;
const WORKERS: usize = 8;

fn curl(url: &str, accept: Option<&str>) -> Option<String> {
    let mut c = crate::command("curl");
    c.args(["-sfL", "--max-time", "30"]);
    if let Some(a) = accept {
        c.args(["-H", &format!("Accept: {a}")]);
    }
    let out = c.arg(url).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    Some(&s[i..i + s[i..].find(end)?])
}

fn all_between<'a>(mut s: &'a str, start: &str, end: &str) -> Vec<&'a str> {
    let mut v = Vec::new();
    while let Some(x) = between(s, start, end) {
        v.push(x.trim());
        s = &s[s.find(start).unwrap() + start.len()..];
    }
    v
}

struct Listing {
    name: String,
    desc: String,
    caps: Vec<String>,
    sizes: Vec<String>,
    pulls: f64,
    recent: bool,
}

/// "28.7M" → 28_700_000
fn parse_count(s: &str) -> Option<f64> {
    let (num, mul) = match s.chars().last()? {
        'K' => (&s[..s.len() - 1], 1e3),
        'M' => (&s[..s.len() - 1], 1e6),
        'B' => (&s[..s.len() - 1], 1e9),
        _ => (s, 1.0),
    };
    Some(num.parse::<f64>().ok()? * mul)
}

fn unescape(s: &str) -> String {
    s.replace("&#39;", "'").replace("&quot;", "\"").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
}

fn parse_library(html: &str) -> Vec<Listing> {
    html.split("<li ")
        .skip(1)
        .filter_map(|it| {
            let name = between(it, "href=\"/library/", "\"")?.to_string();
            let spans = all_between(it, "<span >", "</span>"); // 다운로드 수, 태그 수, 업데이트 시점
            Some(Listing {
                name,
                desc: unescape(between(it, "text-md\">", "</p>").unwrap_or("").trim()),
                caps: all_between(it, "text-indigo-600 sm:text-[13px]\">", "<").into_iter().map(String::from).collect(),
                sizes: all_between(it, "text-blue-600 sm:text-[13px]\">", "<").into_iter().map(String::from).collect(),
                pulls: spans.first().and_then(|s| parse_count(s)).unwrap_or(0.0),
                recent: spans.iter().any(|s| s.ends_with("ago") && !s.contains("year")),
            })
        })
        .collect()
}

struct Pulled {
    size: f64,
    params: Option<f64>,
    digest: String,
}

/// 레지스트리에서 한 태그의 크기(GB)·파라미터 수·본체 digest를 받는다.
fn fetch(tag: &str) -> Option<Pulled> {
    let (name, t) = tag.split_once(':').unwrap_or((tag, "latest"));
    let m: serde_json::Value = serde_json::from_str(&curl(&format!("{REGISTRY}/{name}/manifests/{t}"), Some(MANIFEST))?).ok()?;
    let layers = m["layers"].as_array()?;
    let digest = layers.iter().find(|l| l["mediaType"] == "application/vnd.ollama.image.model")?["digest"].as_str()?.to_string();
    let bytes: u64 = layers.iter().filter_map(|l| l["size"].as_u64()).sum();
    // config의 model_type 예: "11.9B", "567M"
    let params = m["config"]["digest"].as_str()
        .and_then(|d| curl(&format!("{REGISTRY}/{name}/blobs/{d}"), None))
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|c| parse_count(c["model_type"].as_str()?))
        .map(|p| (p / 1e8).round() / 10.0);
    Some(Pulled { size: (bytes as f64 / 1e8).round() / 10.0, params, digest })
}

fn fetch_all(tags: &[String]) -> Vec<Option<Pulled>> {
    let (next, done) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let out: Mutex<Vec<Option<Pulled>>> = Mutex::new(tags.iter().map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..WORKERS {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= tags.len() {
                    break;
                }
                let r = fetch(&tags[i]);
                out.lock().unwrap()[i] = r;
                eprint!("\r  레지스트리 조회 {}/{}", done.fetch_add(1, Ordering::Relaxed) + 1, tags.len());
            });
        }
    });
    eprintln!();
    out.into_inner().unwrap()
}

fn guess_cat(name: &str, caps: &[String]) -> &'static str {
    let has = |ws: &[&str]| ws.iter().any(|w| name.contains(w));
    if caps.iter().any(|c| c == "embedding") {
        "임베딩"
    } else if has(&["coder", "code", "devstral", "codestral"]) {
        "코딩"
    } else if has(&["ocr", "-vl", "vision", "llava", "moondream"]) || name.ends_with("vl") {
        "비전"
    } else if has(&["translate"]) {
        "번역"
    } else if has(&["-r1", "reason", "magistral", "qwq"]) {
        "추론"
    } else {
        "대화"
    }
}

/// MoE 활성 파라미터를 이름·설명에서 찾는다. "30B-A3B", "3B active", "activating just 12B" 형식.
fn guess_active(text: &str) -> Option<f64> {
    let num_b = |w: &str| w.strip_suffix(['b', 'B'])?.parse::<f64>().ok();
    let words: Vec<&str> = text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.')).filter(|w| !w.is_empty()).collect();
    words.iter().find_map(|w| num_b(w.strip_prefix(['a', 'A'])?))
        .or_else(|| {
            let i = words.iter().position(|w| w.to_lowercase().starts_with("activ"))?;
            i.checked_sub(1).and_then(|j| num_b(words[j]))
                .or_else(|| words[i + 1..].iter().take(4).find_map(|w| num_b(w)))
        })
}

fn short(s: &str, n: usize) -> String {
    let first = s.split(". ").next().unwrap_or(s).trim_end_matches('.');
    if first.chars().count() > n { format!("{}…", first.chars().take(n).collect::<String>()) } else { first.to_string() }
}

/// 기존 목록을 받아 크기를 갱신하고 새 모델을 덧붙인 목록을 돌려준다. 연결 실패면 None.
pub fn update(models: &[Model]) -> Option<Vec<Model>> {
    let mut models = models.to_vec();
    eprintln!("ollama.com 라이브러리 읽는 중…");
    let listings = curl(LIBRARY, None).map(|h| parse_library(&h)).unwrap_or_default();
    if listings.is_empty() {
        eprintln!("  라이브러리 페이지를 읽지 못했어요(인터넷 연결 또는 사이트 구조 변경). 기존 모델 크기만 갱신해요.");
    }

    let is_ollama = |m: &Model| m.runtime.is_none();
    let base = |t: &str| t.split_once(':').map_or(t.to_string(), |(n, _)| n.to_string());
    let existing: Vec<String> = models.iter().filter(|m| is_ollama(m)).map(|m| m.tag.clone()).collect();
    let mut candidates: Vec<(String, &Listing)> = Vec::new();
    for l in listings.iter().filter(|l| l.recent && l.pulls >= MIN_PULLS) {
        let tags: Vec<String> = if l.sizes.is_empty() { vec![l.name.clone()] } else { l.sizes.iter().map(|s| format!("{}:{s}", l.name)).collect() };
        for t in tags {
            if !existing.contains(&t) && !candidates.iter().any(|(c, _)| *c == t) {
                candidates.push((t, l));
            }
        }
    }

    let all: Vec<String> = existing.iter().cloned().chain(candidates.iter().map(|(t, _)| t.clone())).collect();
    let pulled = fetch_all(&all);
    if pulled.iter().all(Option::is_none) {
        eprintln!("레지스트리에 연결하지 못했어요. 인터넷 연결을 확인해 주세요. 목록은 바꾸지 않았어요.");
        return None;
    }
    let (old, new) = pulled.split_at(existing.len());

    // 1) 기존 모델 갱신
    let mut digests = Vec::new();
    let (mut changed, mut missing) = (Vec::new(), Vec::new());
    for (tag, p) in existing.iter().zip(old) {
        let m = models.iter_mut().find(|m| &m.tag == tag).unwrap();
        match p {
            Some(p) => {
                if (p.size - m.size).abs() >= 0.1 {
                    changed.push(format!("{tag} {:.1}→{:.1}GB", m.size, p.size));
                    m.size = p.size;
                }
                if m.params.is_none() {
                    m.params = p.params;
                }
                digests.push(p.digest.clone());
            }
            None => missing.push(tag.clone()),
        }
    }

    // 2) 새 모델 추가(같은 파일을 가리키는 별칭 태그는 건너뜀)
    let mut added = Vec::new();
    for ((tag, l), p) in candidates.iter().zip(new) {
        let Some(p) = p else { continue }; // 클라우드 전용 등
        if digests.contains(&p.digest) {
            continue;
        }
        digests.push(p.digest.clone());
        added.push(tag.clone());
        models.push(Model {
            tag: tag.clone(),
            cat: guess_cat(&base(tag), &l.caps).into(),
            size: p.size,
            params: p.params,
            active: guess_active(&format!("{tag} {}", l.desc)),
            overhead: None,
            runtime: None,
            gpu_only: false,
            note: format!("(자동 추가) {}", short(&l.desc, 50)),
        });
    }

    println!("크기가 바뀐 모델 {}개{}", changed.len(), if changed.is_empty() { String::new() } else { format!(": {}", changed.join(", ")) });
    println!("새로 추가한 모델 {}개{}", added.len(), if added.is_empty() { String::new() } else { format!(": {}", added.join(", ")) });
    if !missing.is_empty() {
        println!("레지스트리에서 못 찾은 모델(목록엔 그대로 둠): {}", missing.join(", "));
    }
    Some(models)
}
