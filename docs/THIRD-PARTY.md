# Third-party notices

HERMES is an original client. The MIT licence in the repository covers HERMES code, not upstream projects, images, model weights or services.

| Project | Role | Licence / terms |
| --- | --- | --- |
| Tauri 2 | Native desktop runtime | MIT OR Apache-2.0; bundled dependencies retain their own notices |
| React, Vite, TypeScript, lucide | Interface and development | MIT / ISC / Apache-2.0 as declared by each package |
| Ollama | Separately installed model runtime | MIT; downloaded model licences are independent |
| Open WebUI v0.13.5 | Separately downloaded backend image and original browser interface | Open WebUI licence with additional branding conditions; consult the pinned upstream licence |
| sentence-transformers/all-MiniLM-L6-v2 | Local document embeddings | Consult its model card and Apache-2.0 terms |
| qwen3:4b-instruct | Optional demonstration LLM | Review the exact selected Ollama model’s licence before redistribution or use |
| Docker Desktop | Separately installed container engine | Docker subscription/product terms; not redistributed by HERMES |

HERMES’s DMG includes neither Docker Desktop, Ollama, the Open WebUI image nor model weights. It contains the original HERMES frontend, compiled native dependencies and a Compose definition that references the official image.

Opening Open WebUI shows its original interface and branding without modification. Do not remove its logo or imply that HERMES owns it. Anyone redistributing or modifying Open WebUI must review its exact licence conditions and attribution/branding obligations separately. This document is an inventory, not a legal determination of every intended downstream use.

Sources: [Open WebUI v0.13.5 licence](https://github.com/open-webui/open-webui/blob/v0.13.5/LICENSE), [Tauri licence](https://github.com/tauri-apps/tauri), [Ollama licence](https://github.com/ollama/ollama/blob/main/LICENSE), [MiniLM model card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2), [Docker Desktop terms](https://www.docker.com/legal/docker-subscription-service-agreement/).

Dependency lockfiles identify the source versions used by builds. Review licences/advisories again when updating dependencies. `THIRD-PARTY-LICENSES.txt` includes licence texts for the production npm and Apple Silicon Rust dependency graph and is bundled in the app. Regenerate it with `node scripts/third-party-notices.mjs` after resolving dependencies; omitted crate licence texts are retained with their pinned sources in `upstream-licenses/`. Review the inventory again for every release.
