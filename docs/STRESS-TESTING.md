# Stress testing HERMES with programming textbooks

Start with the 4B model on a 16 GB Mac, a 4,096-token context, and one concurrent client. Wait for all eight textbooks to finish indexing. Keep HERMES open to observe responsiveness, and use a second Terminal window for monitoring. This runner requires Node.js 22.12+; it is a developer tool rather than a packaged-app prerequisite.

The runner never downloads models, changes service settings, starts another model runtime, syncs files or deletes indexed content. Requests use direct loopback HTTP; environment proxies are bypassed. Default runs stop after three errors, have per-request and whole-run time limits, and support Ctrl-C cancellation.

## 1. Check the current system

Run from your HERMES repository:

```sh
./scripts/doctor.sh
ollama list
ollama ps
docker ps --filter label=com.docker.compose.project=hermes-llm
```

`ollama ps` lists the actual allocated context and CPU/GPU placement. A loaded model and a small request working are prerequisites for a useful load test.

## 2. Preview before sending requests

```sh
node scripts/stress-test.mjs --target ollama --requests 3 --dry-run
node scripts/stress-test.mjs --knowledge --requests 3 --dry-run
```

Dry-run makes no network requests and asks for no password. It reports model, endpoint, context, request count and case IDs. The knowledge test uses the KB currently selected in HERMES Settings. It never automatically chooses a different KB.

## 3. Measure Ollama alone

```sh
# Small baseline, excluding one warmup request from reported metrics.
node scripts/stress-test.mjs --target ollama --requests 6 --concurrency 1 --tokens 256

# Compare an 8K context setting, using the same installed model.
node scripts/stress-test.mjs --target ollama --requests 6 --context 8192 --tokens 256
```

These commands test model execution, **without querying the textbooks**. `--context` is requested capacity; it does not itself fill the window with a long prompt. Check `prompt_tokens` and `ollama ps` to see actual processing/allocation. A thinking model can use its generation budget before producing visible answer text; increase `--tokens` deliberately or use the instruct model for a comparable baseline.

To measure a cold start, quit unrelated model work, then use `--warmup 0`. The runner does not unload other users’/applications’ models or change keep-alive settings.

## 4. Exercise your eight-book knowledge base

Make sure HERMES is signed in and the correct KB is selected:

```sh
# Smoke test: same authenticated chat API and collection format as HERMES.
node scripts/stress-test.mjs --knowledge --requests 3 --concurrency 1 --tokens 256

# Longer serial retrieval test.
node scripts/stress-test.mjs --knowledge --requests 20 --concurrency 1 --tokens 256 --max-seconds 900

# Two clients: a modest queue/concurrency test after the serial baseline works.
node scripts/stress-test.mjs --knowledge --requests 12 --concurrency 2 --tokens 256 --max-seconds 900
```

The runner connects to Open WebUI, attaches the configured knowledge collection and requires a retrieved-source event. A stream with no answer, missing completion marker, missing RAG sources or an API error fails. A source event is evidence that retrieval ran; it does **not** prove the answer or its citation is correct.

Concurrent HTTP clients may be queued by Ollama. The script does not change `OLLAMA_NUM_PARALLEL`. True parallel generation depends on server configuration and available memory; increasing clients does not guarantee a throughput increase. Longer context and parallel sequences raise memory demand. See [Ollama concurrency guidance](https://docs.ollama.com/faq#how-does-ollama-handle-concurrent-requests).

## Authentication

The default is the session inside HERMES's encrypted local vault. The runner asks once in Terminal for **the HERMES password you chose**, using hidden input. The Rust helper decrypts the credential vault for this invocation. No Keychain or Mac password is requested. Create the password and connect your account in the app first. See [passwords](PASSWORDS.md).

If you prefer a temporary token, obtain a **local Open WebUI** API key from its Account settings (administrator API-key settings may need enabling). In the standard macOS zsh shell:

```sh
read -s "HERMES_STRESS_TOKEN?Local Open WebUI token: "
export HERMES_STRESS_TOKEN
printf '\n'
node scripts/stress-test.mjs --knowledge --requests 3
unset HERMES_STRESS_TOKEN
```

The hidden prompt avoids putting the token in shell history or a command-line argument. Do not paste tokens into source files or commit them. The tool never prints or saves the token. A native desktop session and an API key can expire/revoke independently.

## 5. Check accuracy across all eight textbooks

Generic programming questions may be answerable from the model’s training. For a grounding test, create one question per textbook about a distinctive example, exercise, local definition or stated constraint. Keep the expected answer out of the prompt.

Create a private JSON file outside the repository, for example `/tmp/my-textbook-cases.json`:

```json
[
  {
    "id": "book-1-specific-example",
    "prompt": "In [your exact book title], what name is used for the example class in chapter [N]? Use the indexed text, name the source and say if the passage is unavailable.",
    "expected": "the actual class name you verified in the book"
  },
  {
    "id": "book-2-specific-constraint",
    "prompt": "In [another exact title], what precondition is stated for the algorithm in section [N]? Cite the source and do not infer a missing passage."
  }
]
```

Replace bracketed text with real references and add cases for books 3–8. Optional `expected` is checked against the answer, case-insensitively, and is **never included in the API prompt**. This is a simple phrase check, not a semantic correctness judge. A valid paraphrase can fail it, and a phrase appearing does not prove sound reasoning.

```sh
node scripts/stress-test.mjs --knowledge --cases /tmp/my-textbook-cases.json --requests 8 --concurrency 1 --tokens 512 --show-answers
```

`--show-answers` prints answers locally for manual checking; it may reveal private excerpts in your terminal. Without it, only metrics/case IDs are printed. Inspect source titles, the relevant passages and any claimed page numbers against the original books. Page numbers are not always available in extraction metadata; the model should state that instead of inventing them.

Also try these prompts in HERMES Chat with **Use knowledge base** enabled:

1. “Find the treatment of binary search in two indexed books. Name each source and compare their stated preconditions. Say explicitly if two sources are unavailable.”
2. “Compare two books’ recursion examples. Cite short supporting passages and separate source facts from your own explanation.”
3. “Which of the uploaded books contains [a real distinctive term]? Quote the definition and identify its source.”
4. “What does the textbook collection say about [a topic you verified is absent]? Say you cannot find support when it is absent.”

Score at least one known fact per book, one cross-book comparison, and one abstention test. Eight books indexed successfully does not imply every question retrieves passages from all eight at once.

## 6. Longer run and metrics-only reports

```sh
mkdir -p .artifacts/stress
node scripts/stress-test.mjs --knowledge --requests 60 --concurrency 1 --tokens 256 --max-seconds 1800 --output .artifacts/stress/textbooks-serial.json
```

Choose a fresh report filename each run; existing reports are never overwritten. `.artifacts/` is ignored by Git. Reports contain metrics only, without prompts, answers, credentials or source names, even when `--show-answers` is used.

Interpretation:

| Metric | Meaning |
| --- | --- |
| First-token latency | Time until first visible answer text, including queueing/retrieval/prompt processing |
| Total latency | Full request time until stream completion |
| p50 / p95 | Median and high-percentile successful-request times; tiny samples are only smoke tests |
| Sources | Emitted retrieval source entries; inspect actual evidence separately |
| Completion/prompt tokens | Reported by the server when available; absent values are not estimated |
| Tokens/second | Computed only when the server reports actual generation token count and duration |
| Failed / stopped early | API, stream, timeout, missing-source or expected-phrase failure; error threshold/time budget can stop scheduling |

Exit code 0 means all requested measured checks passed. Exit code 1 indicates failure, cancellation, early stopping or an operational problem. A successful load test alone does not establish factual accuracy.

## 7. Monitor and recover

In separate terminals while a run is active:

```sh
# Only HERMES's managed backend; Ctrl-C stops monitoring.
hermes_container_id="$(docker ps -q --filter label=com.docker.compose.project=hermes-llm --filter label=com.docker.compose.service=open-webui)"
if [ -n "$hermes_container_id" ]; then
  docker stats "$hermes_container_id"
else
  echo 'HERMES backend is not running.'
fi

# macOS memory-pressure snapshot and swap allocation.
memory_pressure
sysctl vm.swapusage

# Recheck GPU placement and active context.
ollama ps
```

Activity Monitor → Memory is a convenient continuous view. A growing swap allocation by itself is not an error; look for growing pressure, sustained slowdown, timeouts, backend restarts or a rising failure rate.

On a 16 GB Air, start with one client and move to two only after the baseline. Stop the runner with Ctrl-C if pressure becomes excessive, let active work settle and reduce context/concurrency/model size. It cancels only this tool’s requests and leaves services, documents and indexes intact. Thermal slowdown during repeated long runs is a reason to compare shorter runs or an actively cooled machine.

## Development verification

`npm test` includes loopback-mocked stress-tool tests for authentication, split UTF-8/SSE, source checking, timeouts, concurrency limits, early stopping and metrics privacy. The real fixture integration test is separately documented in [TESTING.md](TESTING.md). The eight-book stress workload is supplied for the owner to run; it is not presented as an already measured benchmark.
