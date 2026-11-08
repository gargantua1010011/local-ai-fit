"""benchmarks.json 만들기: Artificial Analysis(artificialanalysis.ai) 리더보드에 실린 오픈 모델 점수를
models.json의 Ollama 태그에 맞춰 모으고, 비교용 상용 모델(COMMERCIAL) 점수도 함께 넣는다.

- 현재 페이지엔 지원 종료(deprecated) 모델이 빠져 있어서, Wayback Machine에 남은 예전 페이지로 채운다.
  벤치마크마다 가장 최근 페이지의 값을 쓴다. 모두 같은 곳(AA)이 같은 방식으로 직접 측정한 값이다.
- 점수는 원본(대부분 BF16) 모델 기준이다. Ollama 기본 양자화(Q4)로 돌리면 조금 낮을 수 있다.
- 실행: python scripts/fetch_benchmarks.py  (표준 라이브러리만 사용, 1~2분)
"""
import datetime
import gzip
import json
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

LIVE = "https://artificialanalysis.ai/leaderboards/models"
# 새 것부터. 페이지 구조가 여러 번 바뀌어서 세 가지 형식을 모두 읽는다.
ARCHIVES = ["20260801032331", "20260601025529", "20260205184229", "20251201192238"]

# (키, 이름, 설명, 새 형식 키, 예전 형식 키)
BENCHMARKS = [
    ("mmlu_pro", "MMLU-Pro", "여러 분야 지식(객관식 10지선다)", None, "mmlu_pro"),
    ("gpqa", "GPQA Diamond", "대학원 수준 과학 문제", "gpqa-diamond", "gpqa"),
    ("hle", "HLE", "전문가도 어려운 문제(Humanity's Last Exam)", "humanitys-last-exam", "hle"),
    ("livecodebench", "LiveCodeBench", "코딩 문제 풀이", "livecodebench", "livecodebench"),
    ("aime25", "AIME 2025", "고등 수학 경시대회", "aime-2025", "aime25"),
    ("ifbench", "IFBench", "지시를 정확히 따르기", "ifbench", "ifbench"),
    ("mmmu_pro", "MMMU-Pro", "이미지를 보고 푸는 문제(비전 모델만)", "mmmu-pro", "mmmu_pro"),
]

# Ollama 태그 → AA 모델 slug. 생각(reasoning) 모드가 기본인 모델은 reasoning 쪽 점수를 쓴다.
# Ollama 기본 태그가 어느 판인지는 레지스트리 digest로 확인함(2026-11-08):
#   qwen3:4b/30b/235b = thinking-2507, qwen3-vl(235b 포함) = thinking, qwen3-next:80b = thinking, magistral:24b = 2506(Small 1)
MAPPING = {
    "gemma4:e2b": "gemma-4-e2b",
    "gemma4:e4b": "gemma-4-e4b",
    "gemma4:12b": "gemma-4-12b",
    "gemma4:26b": "gemma-4-26b-a4b",
    "gemma4:31b": "gemma-4-31b",
    "qwen3.5:0.8b": "qwen3-5-0-8b",
    "qwen3.5:2b": "qwen3-5-2b",
    "qwen3.5:4b": "qwen3-5-4b",
    "qwen3.5:9b": "qwen3-5-9b",
    "qwen3.5:27b": "qwen3-5-27b",
    "qwen3.5:35b": "qwen3-5-35b-a3b",
    "qwen3.5:122b": "qwen3-5-122b-a10b",
    "qwen3.8:27b": "qwen3-8-27b-medium",
    "qwen3:4b": "qwen3-4b-2507-instruct-reasoning",
    "qwen3:8b": "qwen3-8b-instruct-reasoning",
    "qwen3:14b": "qwen3-14b-instruct-reasoning",
    "qwen3:30b": "qwen3-30b-a3b-2507-reasoning",
    "qwen3:32b": "qwen3-32b-instruct-reasoning",
    "qwen3:235b": "qwen3-235b-a22b-instruct-2507-reasoning",
    "gpt-oss:20b": "gpt-oss-20b",
    "gpt-oss:120b": "gpt-oss-120b",
    "llama3.2:1b": "llama-3-2-instruct-1b",
    "llama3.2:3b": "llama-3-2-instruct-3b",
    "llama3.1:8b": "llama-3-1-instruct-8b",
    "llama3.3:70b": "llama-3-3-instruct-70b",
    "llama4:scout": "llama-4-scout",
    "gemma3:1b": "gemma-3-1b",
    "gemma3:4b": "gemma-3-4b",
    "gemma3:12b": "gemma-3-12b",
    "gemma3:27b": "gemma-3-27b",
    "mistral-small3.2:24b": "mistral-small-3-2",
    "ministral-3:3b": "ministral-3-3b",
    "ministral-3:8b": "ministral-3-8b",
    "ministral-3:14b": "ministral-3-14b",
    "phi4-mini:3.8b": "phi-4-mini",
    "phi4:14b": "phi-4",
    "lfm2:24b": "lfm2-24b-a2b",
    "glm-4.7-flash": "glm-4-7-flash",
    "nemotron-3-super:120b": "nvidia-nemotron-3-super-120b-a12b",
    "deepseek-r1:8b": "deepseek-r1-qwen3-8b",
    "deepseek-r1:14b": "deepseek-r1-distill-qwen-14b",
    "deepseek-r1:32b": "deepseek-r1-distill-qwen-32b",
    "deepseek-r1:70b": "deepseek-r1-distill-llama-70b",
    "deepseek-r1:671b": "deepseek-r1",
    "magistral:24b": "magistral-small",
    "qwen2.5-coder:7b": "qwen2-5-coder-7b-instruct",
    "qwen2.5-coder:32b": "qwen2-5-coder-32b-instruct",
    "qwen3-coder:30b": "qwen3-coder-30b-a3b-instruct",
    "qwen3-coder-next": "qwen3-coder-next",
    "qwen3.6:27b": "qwen3-6-27b",
    "qwen3.6:35b": "qwen3-6-35b-a3b",
    "devstral-small-2:24b": "devstral-small-2",
    "qwen3-vl:4b": "qwen3-vl-4b-reasoning",
    "qwen3-vl:8b": "qwen3-vl-8b-reasoning",
    "qwen3-vl:30b": "qwen3-vl-30b-a3b-reasoning",
    "qwen3-vl:32b": "qwen3-vl-32b-reasoning",
    "llama3.2-vision:11b": "llama-3-2-instruct-11b-vision",
    # 내장 목록엔 없고 "최신 목록 받기"(--update)로 추가되는 모델(2026-11-08 기준)
    "gemma3n:e2b": "gemma-3n-e2b",
    "gemma3n:e4b": "gemma-3n-e4b",
    "gemma3:270m": "gemma-3-270m",
    "qwen3-vl:235b": "qwen3-vl-235b-a22b-reasoning",
    "qwen3-next:80b": "qwen3-next-80b-a3b-reasoning",
    "lfm2.5-thinking:1.2b": "lfm2-5-1-2b-thinking",
    "nemotron-3-nano:4b": "nvidia-nemotron-3-nano-4b",
    "nemotron-3-nano:30b": "nvidia-nemotron-3-nano-30b-a3b-reasoning",
    "nemotron-3.5-lightning:30b": "nemotron-3-5-lightning",
    "granite4.1:3b": "granite-4-1-3b",
    "granite4.1:8b": "granite-4-1-8b",
    "granite4.1:30b": "granite-4-1-30b",
    "deepseek-ocr:3b": "deepseek-ocr",
}

# 비교용 상용(클라우드) 모델: (묶음, 화면에 보일 이름, AA slug). 생각 수준을 고를 수 있는 모델은 가장 높은 설정.
COMMERCIAL = [
    ("최신", "Claude Opus 5.5", "claude-opus-5-5"),
    ("최신", "Claude Fable 5.1", "claude-fable-5-1"),
    ("최신", "Claude Sonnet 5.5", "claude-sonnet-5-5"),
    ("최신", "Claude Haiku 5.5", "claude-haiku-5-5"),
    ("최신", "GPT-6 Astra", "gpt-6-astra"),
    ("최신", "GPT-6.1 Sol", "gpt-6-1-sol"),
    ("최신", "GPT-6.1 Terra", "gpt-6-1-terra"),
    ("최신", "GPT-6 Luna", "gpt-6-luna"),
    ("최신", "Gemini 4 Argon", "gemini-4-argon"),
    ("최신", "Gemini 3.1 Pro", "gemini-3-1-pro-preview"),
    ("최신", "Gemini 3.8 Flash", "gemini-3-8-flash"),
    ("최신", "Grok 4.7", "grok-4-7"),
    ("예전 참고", "GPT-5", "gpt-5"),
    ("예전 참고", "o3", "o3"),
    ("예전 참고", "GPT-4o", "gpt-4o"),
    ("예전 참고", "GPT-4o mini", "gpt-4o-mini"),
    ("예전 참고", "Claude 4 Sonnet", "claude-4-sonnet-thinking"),
    ("예전 참고", "Claude 3.5 Sonnet", "claude-35-sonnet"),
    ("예전 참고", "Gemini 2.5 Pro", "gemini-2-5-pro"),
    ("예전 참고", "Gemini 2.5 Flash", "gemini-2-5-flash-reasoning"),
]


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0", "Accept-Encoding": "gzip"})
    for wait in (20, 60, None):  # Wayback은 연달아 받으면 429(요청 과다)를 준다
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                data = r.read()
            break
        except urllib.error.HTTPError as e:
            if e.code != 429 or wait is None:
                raise
            time.sleep(wait)
    if data[:2] == b"\x1f\x8b":
        data = gzip.decompress(data)
    return data.decode("utf-8", "ignore").replace('\\"', '"')


def parse_new(h):
    """2026-06 이후 형식: {"slug":..,"name":..,...,"scores":{"gpqa-diamond":0.8,...}}"""
    out = {}
    pat = r'\{"slug":"([^"]+)","name":"([^"]+)"[^{}]*?"scores":\{([^{}]*)\}'
    for slug, name, scores in re.findall(pat, h):
        try:
            s = json.loads("{" + scores + "}")
        except json.JSONDecodeError:
            continue
        out[slug] = (name, {k: s[new] for k, _, _, new, _ in BENCHMARKS if new and isinstance(s.get(new), (int, float))})
    return out


def parse_mid(h):
    """2026-06~08 형식: {"id":..,"name":..,"slug":..,"gpqa":0.8,"mmmuPro":..,"isOpenWeights":true,..}"""
    out = {}
    keys = {"mmlu_pro": "mmluPro", "mmmu_pro": "mmmuPro"}
    for chunk in h.split('{"id":"')[1:]:
        if '"intelligenceIndex"' not in chunk:
            continue
        slug = re.search(r'"slug":"([^"]+)"', chunk)
        name = re.search(r'"name":"([^"]+)"', chunk)
        if not slug:
            continue
        scores = {}
        for k, *_ in BENCHMARKS:
            m = re.search(r'"%s":([-0-9.e]+)[,}]' % keys.get(k, k), chunk)
            if m:
                scores[k] = float(m.group(1))
        out.setdefault(slug.group(1), (name.group(1) if name else None, scores))
    return out


def parse_old(h):
    """2026-02 이전 형식: {"additional_text":..,"gpqa":0.8,...,"slug":..}"""
    out = {}
    for chunk in h.split('{"additional_text":')[1:]:
        chunk = chunk.split('"model_creators"')[0]

        def get(k):
            m = re.search(r'"%s":("[^"]*"|[-0-9.e]+|null)' % k, chunk)
            return json.loads(m.group(1)) if m else None

        slug = get("slug")
        if not slug:
            continue
        out[slug] = (get("name"), {k: get(old) for k, _, _, _, old in BENCHMARKS if isinstance(get(old), (int, float))})
    return out


def main():
    root = Path(__file__).resolve().parent.parent
    pages = [("live", LIVE)] + [(ts, f"https://web.archive.org/web/{ts}id_/{LIVE}") for ts in ARCHIVES]
    snapshots = []  # 새 것부터
    for label, url in pages:
        try:
            h = fetch(url)
        except Exception as e:
            print(f"{label}: 받지 못함({e})", file=sys.stderr)
            continue
        data = parse_new(h) or parse_mid(h) or parse_old(h)
        print(f"{label}: 모델 {len(data)}개")
        snapshots.append(data)

    order = [b[0] for b in BENCHMARKS]

    def collect(slug):
        """여러 페이지에서 벤치마크마다 가장 최근 값을 모은다. 없으면 None."""
        name, scores = None, {}
        for snap in snapshots:
            if slug in snap:
                n, s = snap[slug]
                name = name or n
                for k, v in s.items():
                    scores.setdefault(k, round(v * 100, 1))
        if not scores:
            missing.append(slug)
            return None
        return {
            "aa_name": name,
            "url": f"https://artificialanalysis.ai/models/{slug}",
            "scores": dict(sorted(scores.items(), key=lambda kv: order.index(kv[0]))),
        }

    missing = []
    models = {tag: e for tag, slug in MAPPING.items() if (e := collect(slug))}
    commercial = [{"name": n, "group": g, **e} for g, n, slug in COMMERCIAL if (e := collect(slug))]

    out = {
        "source": "Artificial Analysis (artificialanalysis.ai) 독립 측정값",
        "updated": datetime.date.today().isoformat(),
        "benchmarks": [{"key": k, "name": n, "desc": d} for k, n, d, _, _ in BENCHMARKS],
        "models": models,
        "commercial": commercial,
    }
    path = root / "benchmarks.json"
    path.write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{path}: 로컬 {len(models)}개, 상용 {len(commercial)}개")
    if missing:
        print("점수를 찾지 못한 모델:", ", ".join(missing))


if __name__ == "__main__":
    main()
