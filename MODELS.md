# 로컬 AI 모델 목록과 필요 사양

`local-ai-fit --markdown`으로 생성한 파일이에요. 원본 데이터는 `models.json`이에요.

- **파일 크기**: Ollama 기본 태그 기준(대부분 Q4_K_M 양자화). `--update`로 Ollama 레지스트리에서 최신 값을 받아요.
- 비고가 "(자동 추가)"인 모델은 `--update`가 새로 찾아 넣은 것이라 분야 분류가 틀릴 수 있고, MoE 활성 파라미터는 비어 있어요.
- **필요 메모리**: 파일 크기 + 문맥(약 8K 토큰) 몫. 문맥을 길게 쓰면 더 필요해요.
- **GPU 전체 실행**: 필요 메모리 + 0.7GB(화면·드라이버 몫) 이상의 VRAM이면 모델 전체가 GPU에 올라가 빨라요.
- **CPU 실행**: 필요 메모리 + 3GB(OS 몫) 이상의 RAM이면 GPU 없이도 돌아가요(느림). VRAM이 모자라면 VRAM+RAM에 나눠 올릴 수도 있어요.
- **MoE**: 전체 파라미터는 크지만 토큰마다 일부(활성 파라미터)만 계산해서, CPU나 분할 실행에서도 비교적 빨라요.
- Apple Silicon Mac은 통합 메모리의 약 70%를 GPU가 쓸 수 있다고 보고 계산해요.

## 대화

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `gemma4:e2b` | Ollama | 4.6B | 4.6 GB | 5.2 GB | 6 GB | 9 GB | Gemma 4 소형. 이미지·오디오 입력 |
| `gemma4:e4b` | Ollama | 7.5B | 6.6 GB | 7.3 GB | 9 GB | 11 GB | Gemma 4 소형. 이미지·오디오 입력 |
| `gemma4:12b` | Ollama | 12B | 8.0 GB | 8.9 GB | 10 GB | 12 GB | Gemma 4. 이미지 입력, 256K 문맥 |
| `gemma4:26b` | Ollama | 26B (활성 4B) | 18.7 GB | 20.0 GB | 21 GB | 23 GB | Gemma 4 MoE(활성 4B) |
| `gemma4:31b` | Ollama | 31B | 20.4 GB | 21.8 GB | 23 GB | 25 GB | Gemma 4 최상위 dense |
| `qwen3.5:0.8b` | Ollama | 0.8B | 1.3 GB | 1.8 GB | 3 GB | 5 GB | 이미지 입력 |
| `qwen3.5:2b` | Ollama | 2B | 2.7 GB | 3.3 GB | 4 GB | 7 GB | 이미지 입력 |
| `qwen3.5:4b` | Ollama | 4B | 3.3 GB | 3.9 GB | 5 GB | 7 GB | 이미지 입력 |
| `qwen3.5:9b` | Ollama | 9B | 6.6 GB | 7.4 GB | 9 GB | 11 GB | 이미지 입력 |
| `qwen3.5:27b` | Ollama | 27B | 17.5 GB | 18.8 GB | 20 GB | 22 GB | 이미지 입력 |
| `qwen3.5:35b` | Ollama | 35B (활성 3B) | 22.1 GB | 23.7 GB | 25 GB | 27 GB | MoE(활성 3B), 이미지 입력 |
| `qwen3.5:122b` | Ollama | 122B (활성 10B) | 81.4 GB | 85.4 GB | 87 GB | 89 GB | MoE(활성 10B), 이미지 입력 |
| `qwen3.8:27b` | Ollama | 27B | 17.7 GB | 19.0 GB | 20 GB | 23 GB | Qwen 최신. 코딩·에이전트 강화 |
| `qwen3:4b` | Ollama | 4B | 2.5 GB | 3.1 GB | 4 GB | 7 GB | 생각 모드 지원 |
| `qwen3:8b` | Ollama | 8B | 5.2 GB | 5.9 GB | 7 GB | 9 GB | 생각 모드 지원 |
| `qwen3:14b` | Ollama | 14B | 9.3 GB | 10.2 GB | 11 GB | 14 GB | 생각 모드 지원 |
| `qwen3:30b` | Ollama | 30B (활성 3B) | 18.6 GB | 20.0 GB | 21 GB | 23 GB | MoE(활성 3B) |
| `qwen3:32b` | Ollama | 32B | 20.2 GB | 21.7 GB | 23 GB | 25 GB | 생각 모드 지원 |
| `qwen3:235b` | Ollama | 235B (활성 22B) | 142.2 GB | 146.2 GB | 147 GB | 150 GB | MoE(활성 22B) |
| `gpt-oss:20b` | Ollama | 21B (활성 3.6B) | 13.8 GB | 14.9 GB | 16 GB | 18 GB | OpenAI 공개 모델, MoE |
| `gpt-oss:120b` | Ollama | 117B (활성 5.1B) | 65.4 GB | 69.4 GB | 71 GB | 73 GB | OpenAI 공개 모델, MoE |
| `llama3.2:1b` | Ollama | 1.2B | 1.3 GB | 1.8 GB | 3 GB | 5 GB | 초경량 |
| `llama3.2:3b` | Ollama | 3.2B | 2.0 GB | 2.6 GB | 4 GB | 6 GB | 초경량 |
| `llama3.1:8b` | Ollama | 8B | 4.9 GB | 5.6 GB | 7 GB | 9 GB | 범용 표준 |
| `llama3.3:70b` | Ollama | 70B | 42.5 GB | 45.1 GB | 46 GB | 49 GB | 대형 dense |
| `llama4:scout` | Ollama | 109B (활성 17B) | 67.4 GB | 71.2 GB | 72 GB | 75 GB | MoE(활성 17B), 이미지 입력 |
| `gemma3:1b` | Ollama | 1B | 0.8 GB | 1.3 GB | 3 GB | 5 GB | 초경량 |
| `gemma3:4b` | Ollama | 4B | 3.4 GB | 4.0 GB | 5 GB | 8 GB | 이미지 입력 |
| `gemma3:12b` | Ollama | 12B | 8.1 GB | 9.0 GB | 10 GB | 12 GB | 이미지 입력 |
| `gemma3:27b` | Ollama | 27B | 17.4 GB | 18.7 GB | 20 GB | 22 GB | 이미지 입력 |
| `mistral-small3.2:24b` | Ollama | 24B | 15.2 GB | 16.4 GB | 18 GB | 20 GB | 이미지 입력, 도구 호출 |
| `ministral-3:3b` | Ollama | 3B | 3.0 GB | 3.6 GB | 5 GB | 7 GB | 이미지 입력 |
| `ministral-3:8b` | Ollama | 8B | 6.0 GB | 6.7 GB | 8 GB | 10 GB | 이미지 입력 |
| `ministral-3:14b` | Ollama | 14B | 9.1 GB | 10.0 GB | 11 GB | 14 GB | 이미지 입력 |
| `phi4-mini:3.8b` | Ollama | 3.8B | 2.5 GB | 3.1 GB | 4 GB | 7 GB | MS 소형 |
| `phi4:14b` | Ollama | 14.7B | 9.1 GB | 10.0 GB | 11 GB | 14 GB | MS 중형 |
| `exaone3.5:2.4b` | Ollama | 2.4B | 1.6 GB | 2.2 GB | 3 GB | 6 GB | LG, 한국어 특화 |
| `exaone3.5:7.8b` | Ollama | 7.8B | 4.8 GB | 5.5 GB | 7 GB | 9 GB | LG, 한국어 특화 |
| `exaone3.5:32b` | Ollama | 32B | 19.3 GB | 20.8 GB | 22 GB | 24 GB | LG, 한국어 특화 |
| `lfm2:24b` | Ollama | 24B (활성 2B) | 14.4 GB | 15.6 GB | 17 GB | 19 GB | MoE(활성 2B), 기기용 |
| `glm-4.7-flash` | Ollama | 30B (활성 3B) | 19.0 GB | 20.4 GB | 22 GB | 24 GB | MoE(활성 3B) |
| `nemotron-3-super:120b` | Ollama | 120B (활성 12B) | 86.8 GB | 90.8 GB | 92 GB | 94 GB | NVIDIA MoE(활성 12B) |
| `gemma3:270m` | Ollama | 0.3B | 0.3 GB | 0.8 GB | 2 GB | 4 GB | (자동 추가) The current, most capable model that runs on a sin… |
| `gemma3n:e2b` | Ollama | 4.5B | 7.8 GB | 8.4 GB | 10 GB | 12 GB | (자동 추가) Gemma 3n models are designed for efficient executi… |
| `gemma3n:e4b` | Ollama | 6.9B | 9.8 GB | 10.5 GB | 12 GB | 14 GB | (자동 추가) Gemma 3n models are designed for efficient executi… |
| `lfm2.5-thinking:1.2b` | Ollama | 1.2B | 0.7 GB | 1.2 GB | 2 GB | 5 GB | (자동 추가) LFM2.5 is a new family of hybrid models designed f… |
| `nemotron-3-nano:4b` | Ollama | 4B | 2.8 GB | 3.4 GB | 5 GB | 7 GB | (자동 추가) Nemotron-3-Nano is a new Standard for Efficient, O… |
| `nemotron-3-nano:30b` | Ollama | 31.6B | 24.3 GB | 25.7 GB | 27 GB | 29 GB | (자동 추가) Nemotron-3-Nano is a new Standard for Efficient, O… |
| `nemotron-3.5-lightning:30b` | Ollama | 32.9B (활성 3B) | 25.4 GB | 26.9 GB | 28 GB | 30 GB | (자동 추가) NVIDIA Nemotron 3.5 Lightning is an open 30B mixtu… |
| `ornith:9b` | Ollama | 9B | 5.6 GB | 6.4 GB | 8 GB | 10 GB | (자동 추가) A self-improving family of open-source models for … |
| `ornith:35b` | Ollama | 34.7B | 21.2 GB | 22.7 GB | 24 GB | 26 GB | (자동 추가) A self-improving family of open-source models for … |
| `nemotron3:33b` | Ollama | 33B | 27.6 GB | 29.1 GB | 30 GB | 33 GB | (자동 추가) NVIDIA Nemotron 3 Nano Omni is a multimodal large … |
| `granite4.1:3b` | Ollama | 3.4B | 2.1 GB | 2.7 GB | 4 GB | 6 GB | (자동 추가) IBM Granite Models are a family of enterprise-read… |
| `granite4.1:8b` | Ollama | 8.8B | 5.3 GB | 6.1 GB | 7 GB | 10 GB | (자동 추가) IBM Granite Models are a family of enterprise-read… |
| `granite4.1:30b` | Ollama | 28.9B | 17.5 GB | 18.9 GB | 20 GB | 22 GB | (자동 추가) IBM Granite Models are a family of enterprise-read… |
| `qwen3-next:80b` | Ollama | 79.7B | 50.4 GB | 53.3 GB | 54 GB | 57 GB | (자동 추가) The first installment in the Qwen3-Next series wit… |
| `rnj-1:8b` | Ollama | 8.3B | 5.1 GB | 5.8 GB | 7 GB | 9 GB | (자동 추가) Rnj-1 is a family of 8B parameter open-weight, den… |

## 추론

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `deepseek-r1:7b` | Ollama | 7.6B | 4.7 GB | 5.4 GB | 7 GB | 9 GB | R1 증류(Qwen 기반) |
| `deepseek-r1:8b` | Ollama | 8B | 5.2 GB | 5.9 GB | 7 GB | 9 GB | R1 증류 |
| `deepseek-r1:14b` | Ollama | 14.8B | 9.0 GB | 9.9 GB | 11 GB | 13 GB | R1 증류(Qwen 기반) |
| `deepseek-r1:32b` | Ollama | 32.8B | 19.9 GB | 21.4 GB | 23 GB | 25 GB | R1 증류(Qwen 기반) |
| `deepseek-r1:70b` | Ollama | 70.6B | 42.5 GB | 45.1 GB | 46 GB | 49 GB | R1 증류(Llama 기반) |
| `deepseek-r1:671b` | Ollama | 671B (활성 37B) | 404.4 GB | 408.4 GB | 410 GB | 412 GB | R1 원본, MoE |
| `magistral:24b` | Ollama | 24B | 14.3 GB | 15.5 GB | 17 GB | 19 GB | Mistral 추론 모델 |
| `phi4-reasoning:14b` | Ollama | 14.7B | 11.0 GB | 11.9 GB | 13 GB | 15 GB | MS 추론 모델 |

## 코딩

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `qwen2.5-coder:7b` | Ollama | 7.6B | 4.7 GB | 5.4 GB | 7 GB | 9 GB | 코드 완성·생성 |
| `qwen2.5-coder:14b` | Ollama | 14.8B | 9.0 GB | 9.9 GB | 11 GB | 13 GB | 코드 완성·생성 |
| `qwen2.5-coder:32b` | Ollama | 32.8B | 19.9 GB | 21.4 GB | 23 GB | 25 GB | 코드 완성·생성 |
| `qwen3-coder:30b` | Ollama | 30.5B (활성 3.3B) | 18.6 GB | 20.0 GB | 21 GB | 24 GB | MoE(활성 3.3B), 에이전트 코딩 |
| `qwen3-coder-next` | Ollama | 80B (활성 3B) | 51.7 GB | 54.6 GB | 56 GB | 58 GB | MoE(활성 3B), 에이전트 코딩 |
| `qwen3.6:27b` | Ollama | 27B | 17.8 GB | 19.1 GB | 20 GB | 23 GB | 에이전트 코딩, 이미지 입력 |
| `qwen3.6:35b` | Ollama | 35B (활성 3B) | 22.6 GB | 24.2 GB | 25 GB | 28 GB | MoE(활성 3B), 에이전트 코딩 |
| `devstral-small-2:24b` | Ollama | 24B | 15.2 GB | 16.4 GB | 18 GB | 20 GB | 코딩 에이전트용 |

## 비전

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `qwen3-vl:2b` | Ollama | 2B | 1.9 GB | 2.5 GB | 4 GB | 6 GB | 이미지·문서 이해 |
| `qwen3-vl:4b` | Ollama | 4B | 3.3 GB | 3.9 GB | 5 GB | 7 GB | 이미지·문서 이해 |
| `qwen3-vl:8b` | Ollama | 8B | 6.2 GB | 6.9 GB | 8 GB | 10 GB | 이미지·문서 이해 |
| `qwen3-vl:30b` | Ollama | 30B (활성 3B) | 19.6 GB | 21.0 GB | 22 GB | 24 GB | MoE(활성 3B) |
| `qwen3-vl:32b` | Ollama | 32B | 20.9 GB | 22.4 GB | 24 GB | 26 GB | 이미지·문서 이해 |
| `minicpm-v:8b` | Ollama | 8B | 5.5 GB | 6.2 GB | 7 GB | 10 GB | 이미지 이해, 손글씨 OCR |
| `llama3.2-vision:11b` | Ollama | 11B | 7.8 GB | 8.6 GB | 10 GB | 12 GB | 이미지 이해 |
| `glm-ocr` | Ollama | 0.9B | 2.7 GB | 3.2 GB | 4 GB | 7 GB | 문서 OCR 전용 |
| `qwen3-vl:235b` | Ollama | 235.7B | 143.3 GB | 147.3 GB | 148 GB | 151 GB | (자동 추가) The most powerful vision-language model in the Qwe… |
| `deepseek-ocr:3b` | Ollama | 3.3B | 6.7 GB | 7.3 GB | 8 GB | 11 GB | (자동 추가) DeepSeek-OCR is a vision-language model that can p… |

## 번역

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `translategemma:4b` | Ollama | 4B | 3.3 GB | 3.9 GB | 5 GB | 7 GB | 55개 언어 번역 |
| `translategemma:12b` | Ollama | 12B | 8.1 GB | 9.0 GB | 10 GB | 12 GB | 55개 언어 번역 |
| `translategemma:27b` | Ollama | 27B | 17.4 GB | 18.7 GB | 20 GB | 22 GB | 55개 언어 번역 |

## 임베딩

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `nomic-embed-text` | Ollama | 0.1B | 0.3 GB | 0.6 GB | 2 GB | 4 GB | 영어 중심 임베딩 |
| `embeddinggemma:300m` | Ollama | 0.3B | 0.6 GB | 0.9 GB | 2 GB | 4 GB | 다국어 임베딩 |
| `bge-m3` | Ollama | 0.6B | 1.2 GB | 1.5 GB | 3 GB | 5 GB | 다국어(한국어 양호) 임베딩 |
| `qwen3-embedding:0.6b` | Ollama | 0.6B | 0.6 GB | 0.9 GB | 2 GB | 4 GB | 다국어 임베딩 |
| `qwen3-embedding:4b` | Ollama | 4B | 2.5 GB | 3.0 GB | 4 GB | 6 GB | 다국어 임베딩 |
| `qwen3-embedding:8b` | Ollama | 7.6B | 4.7 GB | 5.2 GB | 6 GB | 9 GB | 다국어 임베딩 |
| `nomic-embed-text-v2-moe` | Ollama | 0.5B | 1.0 GB | 1.5 GB | 3 GB | 5 GB | (자동 추가) nomic-embed-text-v2-moe is a multilingual MoE text… |

## 음성인식

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `whisper small` | whisper.cpp | - | 0.5 GB | 1.0 GB | 2 GB | 4 GB | 빠름, 정확도 보통 |
| `whisper medium` | whisper.cpp | - | 1.5 GB | 2.0 GB | 3 GB | 5 GB | 한국어 무난 |
| `whisper large-v3-turbo` | whisper.cpp | - | 1.6 GB | 2.1 GB | 3 GB | 6 GB | large급 정확도, 빠름 |
| `whisper large-v3` | whisper.cpp | - | 3.1 GB | 3.9 GB | 5 GB | 7 GB | 최고 정확도 |

## 이미지생성

| 모델 | 실행 도구 | 파라미터 | 파일 크기 | 필요 메모리 | GPU 전체 실행 VRAM | CPU 실행 RAM | 비고 |
|---|---|---|---|---|---|---|---|
| `Stable Diffusion 1.5` | ComfyUI | - | 4.0 GB | 4.0 GB | 5 GB | GPU 필요 | 512px, 가벼움 |
| `SDXL` | ComfyUI | - | 8.0 GB | 8.0 GB | 9 GB | GPU 필요 | 1024px 표준 |
| `FLUX.1 (GGUF Q4)` | ComfyUI | - | 8.0 GB | 8.0 GB | 9 GB | GPU 필요 | FLUX 경량판 |
| `FLUX.1 dev (fp8)` | ComfyUI | - | 12.0 GB | 12.0 GB | 13 GB | GPU 필요 | 고품질 |

