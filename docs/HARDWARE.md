# Mac hardware and local model guide

Guidance checked on 26 November 2026. These are conservative engineering estimates for a single user running HERMES, Docker/Open WebUI and native Ollama. They are not measured benchmarks across the listed Macs. The development verification used a **16 GB M5 MacBook Air with `qwen3:4b-instruct`**, including real document retrieval.

Every workload level uses the same HERMES application and features. Choose by unified memory, model quantisation, desired context and sustained workload. A chip generation alone does not determine which models fit.

## Workload levels

| Level | Suggested Macs, with sufficient configured memory | Practical starting models | Starting settings |
| --- | --- | --- | --- |
| Experimental, 8 GB | Existing Apple Silicon Air/mini only, if its OS is still supported by prerequisites | `qwen3:0.6b` or `qwen3:1.7b`; limited quality and memory headroom | 2K–4K context, one request; full Docker/RAG use may swap heavily |
| Everyday study, 16 GB | M1 or newer Air, mini, iMac or Pro; M5 Air 16 GB is the verified development machine | `qwen3:4b-instruct` | 4K context, one request; close memory-heavy apps during indexing |
| Comfortable study/coding, 24–32 GB | M5 Air 24/32 GB for portable occasional use; mini or MacBook Pro for longer sessions | `qwen3:8b`; try `qwen3:14b` on 32 GB after measuring | 4K initially, then 8K if pressure remains low; one request initially |
| Advanced coding, 48–64 GB | Pro/Max MacBook Pro, Pro Mac mini, Max Mac Studio, or older equivalents with this memory | `qwen3:30b` / `qwen3:32b` | 4K–8K initially; evaluate concurrency separately |
| Large-model workstation, 96–128+ GB | Mac Studio Max/Ultra; 96–128 GB Max MacBook Pro where available | `llama3.3:70b` with its separate licence, or smaller models with more context headroom | 4K–8K initially; grow context after measuring actual memory usage |

**Recommendation for the project’s 16 GB M5 Air:** keep `qwen3:4b-instruct`, 4K context and one active request as the baseline. Establish textbook retrieval accuracy first, then compare an 8B model if enough memory remains. Larger models can improve some answers but do not repair missing text extraction or poor retrieval automatically.

Air is fanless. For frequent long sessions, sustained ingestion and repeated load tests, an actively cooled MacBook Pro or desktop is a sensible choice. This is a thermal/workload recommendation, not a measured speed ranking. [Apple describes the Air’s fanless design](https://www.apple.com/macbook-air/).

Current examples include the M5 Air (16/24/32 GB), Pro/Max MacBook Pro configurations, Pro Mac mini configurations, and Max/Ultra Mac Studio configurations. Check the exact memory option before buying. Official specifications: [Air](https://www.apple.com/macbook-air/specs/), [MacBook Pro](https://www.apple.com/macbook-pro/specs/), [Mac mini](https://www.apple.com/mac-mini/specs/), [Mac Studio](https://www.apple.com/mac-studio/specs/). Older models with adequate memory can remain useful if they meet current OS/prerequisite support.

## Model weights are only part of memory use

Approximate downloads for the named Ollama tags:

| Ollama tag | Approximate weight download | Comment |
| --- | --- | --- |
| `qwen3:0.6b` | 0.5 GB | Very small baseline; limited reasoning |
| `qwen3:1.7b` | 1.4 GB | Lightweight experiments |
| `qwen3:4b-instruct` | 2.5 GB | HERMES demonstration model; tested |
| `qwen3:8b` | 5.2 GB | A useful next comparison when memory permits |
| `qwen3:14b` | 9.3 GB | Prefer 32 GB for the complete stack |
| `qwen3:30b` | 19 GB | MoE; the full weight set still needs memory despite fewer active parameters |
| `qwen3:32b` | 20 GB | Larger dense model |
| `llama3.3:70b` | 43 GB | Large quantised model; its licence differs from HERMES |

These are the registry’s current quantised downloads, not required RAM totals. Different tags, Q8 and FP16 builds can be substantially larger. Reserve additional unified memory for **macOS + other apps + Docker + embeddings + model runtime + KV/context cache**. Do not allocate nearly all system RAM to Docker: Ollama runs outside Docker and shares the same physical memory.

Sources: [Qwen3 tag sizes and quantisations](https://ollama.com/library/qwen3/tags), [Llama 3.3 70B weight details](https://ollama.com/library/llama3.3:70b). The workload-to-Mac recommendations above are our estimates from those weights and the complete application overhead, rather than claims made by Apple or Ollama.

## Context, concurrency and eight textbooks

A large advertised context window is a model capability, not a promise it will fit your Mac. Increasing context increases memory, and parallel sequences increase it further. Begin with explicit 4K requests when comparing models. `ollama ps` shows the actual loaded model, processor allocation and context. See [Ollama context guidance](https://docs.ollama.com/context-length) and [concurrency behaviour](https://docs.ollama.com/faq#how-does-ollama-handle-concurrent-requests).

Eight indexed textbooks remain in the local knowledge store. Open WebUI retrieves passages for each question. It does not place every page of every book into every model request. Page count, extraction quality and chunking affect ingestion/index size; retrieved passages and chat history affect per-request context. Do not raise context to 256K merely because eight books are indexed.

For long textbooks, first test a known fact from each book and check the quoted source. Scanned PDFs need OCR; missing source content requires fixing ingestion. Use [the stress-test procedure](STRESS-TESTING.md) to distinguish runtime throughput from retrieval behaviour.

## Storage and compatibility

Allow **at least 30–50 GB free for a small-model starting installation**, and more for multiple models, many textbooks, Docker’s image/cache and backups. For repeated model experiments, 100 GB or more of free headroom is a practical planning allowance. These are estimates; source PDF size alone does not predict vector-store/cache growth. Inspect Docker Desktop’s disk usage and your Ollama model list before downloading more weights.

The distributed DMG targets Apple Silicon (`aarch64`). Intel, Windows and Linux packages are not provided in the initial release. HERMES’s native app target is macOS 13+, while [Ollama currently requires macOS 14+](https://docs.ollama.com/macos), and [Docker Desktop supports its current and two previous major macOS releases](https://docs.docker.com/desktop/setup/install/mac-install/). Satisfy the strictest prerequisite; do not buy an older Mac based only on HERMES’s bundle minimum.
