# Third-Party Software

VPet is closed-source. It includes the following open-source components, whose licenses must be respected in every distributed build.

| Component | License | Use |
|---|---|---|
| Tauri (tauri, tauri-build, wry, tao) | MIT / Apache-2.0 | Desktop application shell |
| SQLite (via rusqlite `bundled`) | Public domain | Local database |
| rusqlite | MIT | SQLite bindings |
| serde, serde_json | MIT / Apache-2.0 | Serialization |
| chrono | MIT / Apache-2.0 | Local time |
| windows-sys, windows | MIT / Apache-2.0 | Win32 input state, audio peak meter |
| ureq | MIT / Apache-2.0 | HTTP client (netgate only, permission-gated) |
| @tauri-apps/api | MIT / Apache-2.0 | UI ↔ Rust IPC |
| Microsoft Edge WebView2 Runtime | Microsoft license | Web rendering (system component) |
| NSIS | zlib/libpng | Installer |
| llama.cpp / ggml (via llama-cpp-sys-2) | MIT | Local LLM inference |
| llama-cpp-2 | MIT / Apache-2.0 | llama.cpp bindings |
| encoding_rs | MIT / Apache-2.0 | Token text decoding |
| Qwen3-4B / Qwen3-0.6B (model weights, separate files) | Apache-2.0 | Conversation model (full / light) |
| Windows.Media.Ocr (part of Windows) | Windows license | Offline text recognition for screen sharing |
| Rainyhearts font (`src/assets/rainyhearts.ttf`) by Camellina | Free ("100% Free" on dafont; author: "feel free to use") | Pixel UI font |

Before each release, regenerate the full transitive list, for example with `cargo about generate` and `npx license-checker`, and ship the license texts with the installer.
