# Models

All models are downloaded only when you click **Download**, from the official [ggml-org](https://huggingface.co/ggml-org) GGUF repositories on Hugging Face.

| Model | Publisher | Download | Recommended RAM | Vision | License |
|---|---|---|---|---|---|
| Gemma 4 E4B | Google | 5.2 GB | 8 GB | ✅ | Apache 2.0 |
| **Gemma 4 12B** | Google | 7.4 GB | 16 GB | ✅ | Apache 2.0 |
| gpt-oss 20B | OpenAI | 12.1 GB | 16 GB | — | Apache 2.0 |
| Gemma 4 26B-A4B | Google | 15.4 GB | 24 GB | ✅ | Apache 2.0 |
| Gemma 4 31B | Google | 18.8 GB | 32 GB | ✅ | Apache 2.0 |
| Qwen 3.8 27B | Alibaba | 19.6 GB | 32 GB | ✅ | Apache 2.0 |
| Qwen 3.6 35B-A3B | Alibaba | 21.0 GB | 32 GB | ✅ | Apache 2.0 |

## Speed tips
- **Fast mode** (default) disables the reasoning phase: ~70× faster first word on Gemma 4.
- Enable **Deep thinking** for maths, logic and tricky questions.
- The first question in a long conversation re-reads the history; the next ones reuse the cache.
- Smaller models write faster (Gemma 4 E4B ≈ 2× Gemma 4 12B).

## Nova 1
NOVA's own model is not a chat model: it is a tiny network (1.6 M parameters, 1.9 MB) trained from scratch to understand everyday commands instantly. It ships with the app.
