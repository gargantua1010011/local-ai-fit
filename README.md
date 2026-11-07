# local-ai-fit

PC 사양(RAM, GPU VRAM)을 자동으로 확인해서, 그 안에서 돌릴 수 있는 로컬 AI 모델을 분야별로 보여주는 Rust 프로그램.

- 모델 목록·필요 사양 표: [MODELS.md](MODELS.md) (`models.json`에서 생성)
- 모델 데이터: [models.json](models.json) — 빌드 때 실행 파일에 포함됨(내장 목록)
- `--update`로 받은 최신 목록은 exe 옆 `models.json`에 저장되고, 있으면 내장 목록 대신 쓴다. 지우면 내장 목록으로 돌아감.

## 실행

```bash
~/.cargo/bin/cargo build --release
./target/release/local-ai-fit.exe            # 내 PC 기준
./target/release/local-ai-fit.exe --all      # 못 돌리는 모델도 표시
./target/release/local-ai-fit.exe --ram 32 --vram 8   # 다른 사양 가정 (--vram 0 = GPU 없음)
./target/release/local-ai-fit.exe --markdown > MODELS.md
./target/release/local-ai-fit.exe --update   # Ollama에서 최신 목록 받기(약 15초)
```

## --update 동작 (src/update.rs)

1. `ollama.com/library` HTML을 읽어 모델 이름·설명·기능 태그·크기 태그·다운로드 수·업데이트 시점을 뽑는다(공식 API 없음 → 사이트 구조가 바뀌면 `parse_library`만 고치면 됨).
2. 기존 Ollama 모델 + 새 후보(최근 1년 안 업데이트, 다운로드 50만 이상, `MIN_PULLS`)의 태그마다 공식 레지스트리 `registry.ollama.ai/v2/library/<이름>/manifests/<태그>`로 레이어 크기 합(GB)을, config blob의 `model_type`으로 파라미터 수를 받는다. 8개 병렬.
3. 기존 항목은 크기만 갱신(분야·메모·활성 파라미터는 유지). 새 항목은 분야를 이름·기능 태그로 추정하고, 활성 파라미터는 설명의 `A3B`/`3B active`/`activating 12B` 표기에서 찾는다. 메모는 영어 설명 앞부분에 "(자동 추가)"를 붙임.
4. 본체 digest가 같은 별칭 태그(예: `llama4:16x17b` = `llama4:scout`)와 클라우드 전용 모델(매니페스트 없음)은 건너뛴다.
5. 연결이 전혀 안 되면 저장하지 않고 종료 코드 1.

HTTP는 OS 기본 `curl`을 호출한다(Windows 10+ 기본 포함). Rust TLS crate는 C 컴파일러가 필요한데 이 PC의 GNU 툴체인엔 없어서.
레지스트리 크기는 모든 레이어 합이라 ollama.com 표시 범위의 위쪽 값과 같다(예: gemma4:26b 16→18.7GB). 안전한 쪽으로 계산된다.

exe 하나로 동작(models.json 따로 필요 없음). 탐색기에서 더블클릭하면 결과를 보여주고 Enter를 기다린다.

## 판정 방식

- 필요 메모리 = 파일 크기(Ollama 기본 태그, 대부분 Q4_K_M) + 문맥 몫(`0.5 + 0.03×파라미터B`, 최대 4GB ≈ 8K 토큰). 임베딩·음성·이미지 모델은 `overhead` 값 직접 지정.
- 여유분: RAM은 OS 몫 3GB, VRAM은 화면·드라이버 몫 0.7GB를 뺀다.
- GPU 전체: 필요 ≤ VRAM → 빠름 / GPU+CPU 분할: 필요 ≤ VRAM+RAM / CPU: GPU 없음 + 필요 ≤ RAM / `gpu_only`(이미지 생성)는 VRAM에 들어가야만 가능.
- 속도 등급: MoE는 활성 파라미터만 읽으므로 분할·CPU에서도 한 단계 빠르게 본다. 실측 아님.
- 추천: 분야별로 GPU에 전부 올라가는 모델 중 가장 큰 것.
- "Q8도 GPU에 들어감": 파라미터×1.07GB(+비전 인코더 등)로 Q8_0 크기를 추정해 VRAM에 들어가면 표시.

## GPU 감지

| 환경 | 방법 |
|---|---|
| NVIDIA | `nvidia-smi` |
| Windows AMD/Intel | 레지스트리 `HardwareInformation.qwMemorySize` (WMI AdapterRAM은 4GB에서 잘림) |
| Linux AMD | `/sys/class/drm/*/device/mem_info_vram_total` |
| Apple Silicon | 통합 메모리의 70%를 GPU 몫으로 계산 |

VRAM 2GB 미만(내장 그래픽)은 GPU 없음으로 본다. GPU가 여러 개면 가장 큰 것 하나만 쓴다.

## 모델 추가

Ollama 모델은 `--update`가 알아서 추가한다. 손으로 넣을 때(Ollama 밖 모델, 분류·메모 다듬기)는 `models.json`에 한 줄 추가 후 다시 빌드. 자동 추가 항목을 다듬어 내장 목록에 반영하려면 exe 옆 파일에서 해당 줄을 옮겨오면 된다. 필드: `tag`, `cat`(대화/추론/코딩/비전/번역/임베딩/음성인식/이미지생성), `size`(GB), `params`·`active`(B, MoE만 active), `overhead`(GB, 선택), `runtime`(기본 Ollama), `gpu_only`, `note`.
크기는 `https://ollama.com/library/<이름>/tags`에서 확인. 데이터 기준일 2026-11-07.
