---
title: Agent & Sub-Agent Permission Scoping — Survei Kompetitor
status: research
date: 2026-10-04
author: squad-lead (diskusi dengan Achmad)
---

# Agent & Sub-Agent Permission Scoping — Survei Kompetitor

> **Konteks:** Lahir dari diskusi review `standard-docs/` di `feat/refactoring-rust` (lihat sesi 2026-10-04).
> Pertanyaan asal: Tauri plugin (`shell`/`dialog`/`fs`/`notification`) dengan capability luas (`fs:default`,
> `shell:allow-open`) ditandai berisiko oleh audit awal. Setelah ditelusuri, ini kemungkinan **legacy dari
> tahap desain awal** (flow: desain → scoping API), belum tahap akhir hardening. Owner (Achmad) meminta
> perbandingan konkret: bagaimana kompetitor (Kiro Crew, Claude Code, GitHub Copilot CLI, Google Antigravity,
> Cursor) men-scope kapabilitas agent/sub-agent, sebagai referensi desain untuk Galleon.
>
> **Status:** Riset murni — belum jadi keputusan arsitektur. Tidak mengubah `decisions/` atau `dev-tasks/`.

---

## 1. Ringkasan Eksekutif

Semua platform yang disurvei konvergen ke pola yang sama secara struktural, walau istilah berbeda:

1. **Default-closed, bukan default-open.** Kapabilitas baru (filesystem, shell, network) mulai dari *deny* atau *ask*, naik ke *allow* lewat konfigurasi eksplisit — bukan sebaliknya.
2. **Deny selalu menang di atas allow**, berapa pun spesifiknya rule allow. Ini "fail-closed" yang sudah jadi lesson tersimpan untuk Galleon (`Gate G4`) — konsisten dengan Claude Code (`deny > ask > allow`) dan Kiro Crew sendiri (`governance ceiling: Policy ∩ Profile (tightest-wins) — tidak bisa dilonggarkan agent/app`).
3. **Scope filesystem defaultnya adalah working directory, bukan HOME.** Working-dir boundary + explicit opt-in untuk direktori tambahan adalah pola universal.
4. **Sub-agent TIDAK otomatis mewarisi scope penuh parent** — ini area yang paling banyak insiden/bug report di kompetitor, dan paling relevan untuk `spawn_subagent` Galleon.
5. **OS-level sandbox adalah lapisan terakhir, bukan satu-satunya.** Semua platform mapan menumpuk permission-layer (app-level) di atas sandbox-layer (OS-level) — bukan mengandalkan salah satu saja.

---

## 2. Perbandingan Per Platform

### 2.1 Kiro Crew (lineage proyek ini — paling relevan)

Sumber: `kiro.dev/docs/crew/security/`, `kiro.dev/docs/permissions/`.

**Model: capability-based, bukan tool-based.** Kapabilitas yang didefinisikan: `fs_read`, `fs_write`, `shell`, `web_fetch`, `web_search`, `mcp`, `subagent`, `skill`, `power`, `context`, `diagnostics`, `sandbox_network`. Meta-capability `all`/`builtin`/`filesystem` (= `fs_read`+`fs_write`) untuk shortcut.

**Effect:** `deny` (selalu blokir) > `ask` (minta approval) > `allow` (jalan tanpa prompt). Deny menang mutlak di skala manapun.

**Defense-in-depth 8 layer** (tiap tool call lewat semua, dicatat di security event log):
1. Owner lock (gateway channel tolak user tak sah sebelum masuk sesi)
2. Denied commands (pattern built-in + konfigurasi, dicek SEBELUM minta approval)
3. **Governance ceiling** — `Policy ∩ Profile` (tightest-wins) — **tidak bisa dilonggarkan oleh agent atau app**
4. Sensitive paths + sandbox mask (tool file langsung blokir path terlindungi; OS sandbox sembunyikan lokasi kredensial dari subprocess)
5. Tool approval (interactive, trust escalation, atau Autopilot — hanya jalan SETELAH deny checks lolos)
6. Input validation (skema MCP, type check, panjang, normalisasi unicode)
7. OS sandbox (isolasi filesystem level proses via Linux namespace / macOS Seatbelt)
8. Output redaction (pola kredensial di-scrub sebelum sampai ke chat surface)

**Sandbox mode** (`kirocrew config set agent.sandbox <mode>`):

| Mode | Disembunyikan | Bisa diakses | Dipakai untuk |
|---|---|---|---|
| `auto` (default) | `.gnupg`, `.gcloud`, `.azure`, `.docker` | `.aws`, `.ssh`, `.kube` | Mayoritas user — tetap bisa git-over-SSH & AWS CLI |
| `strict` | semua di atas + `.aws`, `.ssh`, `.kube` | hanya `~/.ssh/known_hosts` | Deployment terkunci |
| `off` | tidak ada | semua | Hanya kalau trade-off dipahami penuh |

**Fail-closed by design:** di platform tanpa dukungan sandbox (`armv7l`, `riscv64`, Windows tanpa sandbox internal Kiro CLI), **Crew menolak start agent** kecuali user eksplisit set `sandbox=off` atau `sandbox_allow_unsandboxed_exec=true`. Tidak diam-diam jalan tanpa proteksi.

**Catatan penting:** command-gate TIDAK mendeteksi path kredensial dari teks shell (gampang dielakkan lewat alias/encoding) — proteksi nyata ada di sandbox bind-mask (level OS), bukan di pattern matching teks.

### 2.2 Claude Code (Anthropic)

Sumber: `code.claude.com/docs/en/security`.

**Permission mode:** `Auto` (default interaktif — classifier model terpisah mengevaluasi tiap action, blokir yang dinilai tidak aman) vs `Manual` (mulai read-only, minta izin eksplisit untuk edit/exec, dengan command read-only seperti `ls`/`cat`/`git status` otomatis lolos).

**Working-directory boundary:** di Manual mode, Claude Code **minta izin SEBELUM file tool baca/tulis di luar folder tempat ia dijalankan + subfoldernya.** Ini default scope — identik dengan prinsip yang Anda usulkan ("default ke workspace dir, bukan HOME"). Direktori tambahan ditambah via `additional directory` config eksplisit, bukan default luas.

**Catatan tajam:** boundary itu adalah *permission prompt*, bukan sandbox OS — command Bash yang sekali di-approve tetap bisa menulis ke mana pun user punya akses. Untuk proteksi OS-level sungguhan, mereka punya `/sandbox` command terpisah (filesystem+network isolation).

**Sub-agent scope — AREA PALING RAWAN** (berdasarkan multiple GitHub issue):
- Default: sub-agent **TIDAK mewarisi** allowlist project/user-scoped milik parent (issue #61315).
- Ada permintaan fitur eksplisit agar sub-agent **restricted toolset** (`tools: ["Read", "Grep", "Glob"]`) dipaksa oleh runtime, bukan cuma instruksi prompt — karena prompt compliance saja terbukti bisa diabaikan sub-agent (issue #89277, "Fork subagents ignore prompt-level write restrictions due to unconstrained inherited tool access").
- Ada juga permintaan agar sub-agent **mewarisi** worktree cwd + permission scope lintas tool-call round (issue #69943) — menunjukkan behaviour saat ini tidak konsisten/predictable.

**Pelajaran untuk Galleon:** jangan asumsikan "sub-agent otomatis lebih aman karena scope lebih kecil" — justru defaultnya sering longgar/inkonsisten kecuali dipaksa eksplisit oleh runtime (bukan prompt).

### 2.3 GitHub Copilot CLI

Sumber: `docs.github.com/.../understanding-local-sandboxing`.

**Model: filesystem policy eksplisit** — "a set of rules that decide which paths a sandboxed process or operation can read, which it can write, and which it cannot touch at all." Tiga state per path: read / write / no-touch — bukan biner allow/deny.

**Local sandbox = OS-level isolation**, dijalankan di tiap command yang Copilot CLI eksekusi atas nama user, bukan opsional add-on.

### 2.4 Google Antigravity — STUDI KASUS KEGAGALAN YANG RELEVAN

Sumber: CyberScoop (Pillar Security disclosure), Pillar Security blog ("Escaping Antigravity's Allow-Default Seatbelt").

**Secure Mode** Antigravity: menjalankan semua operasi command lewat sandbox virtual, membatasi network, dan **melarang agent menulis kode di luar working directory** — persis prinsip "scope ke workspace" yang Anda usulkan.

**Tapi ada celah nyata yang ditemukan peneliti (2026):** salah satu tool pencarian file (`find_by_name`) diklasifikasikan sebagai *native tool* — agent bisa eksekusi langsung **sebelum** lapisan proteksi Secure Mode sempat mengevaluasi command-level operation. Kombinasi dengan prompt injection (lewat file/web content yang di-ingest agent) menghasilkan **RCE penuh, bahkan dalam konfigurasi paling aman yang direkomendasikan Google sendiri.**

> *"The security boundary that Secure Mode enforces simply never sees this call. This means an attacker achieves arbitrary code execution under the exact configuration a security-conscious user would rely on to prevent it."* — Dan Lisichkin, Pillar Security

**Pelajaran paling tajam untuk Galleon:** celah ini bukan karena desain allow-list-nya salah — tapi karena **ada satu kategori tool ("native tool") yang diam-diam BYPASS gate yang sama yang mengatur tool lain.** Ini preseden langsung untuk ULANGI pengecekan di Galleon: apakah ada tool/path serupa di `clawcrew-gateway` atau `engine/tool/` yang "native"/"builtin" dan secara tidak sengaja lolos dari `ALLOWED_TOOLS` gate yang sama yang mengatur tool lain? (Catatan: audit awal menemukan 7 tool Go→Rust yang mismatch nama — ini kasus BERBEDA dari Antigravity, tapi sama-sama menunjukkan pentingnya memverifikasi *setiap* jalur tool secara eksplisit, bukan asumsi semua lewat gate yang sama.)

**Mode "Turbo":** mematikan Terminal Sandbox sepenuhnya, full read-write filesystem tanpa prompt. Google eksplisit mendokumentasikan ini sebagai anti-pattern ("prefer scoped allow rules unless you fully trust the prompt and environment") — bukan mode yang direkomendasikan untuk default.

### 2.5 Cursor

Sumber: startdebugging.net (perbandingan langsung dengan Claude Code subagents).

Model sub-agent lebih sederhana: hanya **2 level nesting** (vs Claude Code 5 level), **satu toggle read-only** per sub-agent (vs per-tool allowlist granular Claude Code). Trade-off: lebih mudah dikonfigurasi, tapi kontrol lebih kasar — cocok untuk tim kecil/kasus sederhana, bukan untuk delegasi kompleks lintas-worktree seperti pola squad Galleon.

---

## 3. Sintesis — Pola Umum yang Berulang di Semua Platform

| Pola | Kiro Crew | Claude Code | Copilot CLI | Antigravity | Cursor |
|---|---|---|---|---|---|
| Default filesystem scope | workspace-root (implisit via sandbox mask) | **working dir + subfolder** | path policy eksplisit (R/W/no-touch) | **working directory** (Secure Mode) | tidak terdokumentasi detail |
| Deny menang di atas allow | ✅ eksplisit (governance ceiling) | ✅ eksplisit (`deny > ask > allow`) | ✅ (no-touch state) | ✅ (Secure Mode intent) | tidak terdokumentasi |
| OS-level sandbox terpisah dari app-permission | ✅ (namespace/Seatbelt) | ✅ (`/sandbox` terpisah dari prompt) | ✅ (built-in, selalu jalan) | ✅ (virtual sandbox) | tidak terdokumentasi |
| Sub-agent scope lebih kecil dari parent (dipaksa runtime) | capability `subagent` sendiri, tunduk governance ceiling yang sama | ❌ **tidak otomatis** — banyak bug report | — | — | hanya 1 toggle read-only |
| Celah tool "native"/bypass gate ditemukan | tidak ada laporan publik | — | — | ✅ **ADA, RCE nyata** | — |

**Kesimpulan 1 — soal `fs`/`shell` plugin Tauri:** Default ke workspace-dir (bukan HOME) itu **konsisten dengan SEMUA platform** yang disurvei, bukan preferensi unik Galleon. Validasi penuh atas arah yang Anda tetapkan ("tanggung jawab user, tapi scoped ke workspace yang dibuka"). Yang perlu ditambahkan: pola Kiro Crew sendiri soal *governance ceiling yang tidak bisa dilonggarkan oleh agent/app* — berarti meski user secara default dapat kontrol penuh (local-first, tanggung jawab user), ada baiknya tetap ada *ceiling* yang dikonfigurasi app (mis. tidak pernah boleh menulis ke luar `$APPDATA/galleon` + workspace yang dibuka), supaya "tanggung jawab user" tidak berarti "tanpa batas sama sekali" — konsisten dengan prinsip Local AI 1st tapi tetap defense-in-depth.

**Kesimpulan 2 — soal tool contract Go↔Rust (temuan #12):** Pola sub-agent Claude Code (`#89277`) menunjukkan **restriksi di level prompt saja tidak cukup** — harus dipaksa oleh runtime/gate. `ALLOWED_TOOLS` di Rust gateway sudah benar sebagai *runtime gate*, bukan sekadar instruksi. Masalahnya BUKAN desain gate-nya (itu sudah benar secara prinsip, align dengan semua kompetitor), tapi *daftar nama tool di dua sisi (Go 8 tool vs Rust 4 tool) belum disinkronkan* — ini murni bug implementasi, bukan kesalahan arsitektur.

**Kesimpulan 3 — risiko tersembunyi yang perlu diperiksa:** Insiden Antigravity (`find_by_name` native tool bypass) adalah warning langsung: **audit apakah ada tool di `engine/tool/{services,sandbox,plugin,mcp_client}.go` atau native binding Rust yang eksekusi LANGSUNG tanpa lewat `ExecuteNativeTool`/`ALLOWED_TOOLS` gate yang sama.** Ini pertanyaan verifikasi konkret yang belum pernah ditanyakan di audit `standard-docs/` sebelumnya — layak jadi item baru di `BG-A3` (architecture gate) atau audit terpisah.

---

## 4. Rekomendasi Konkret untuk Galleon (bahan diskusi, bukan keputusan final)

1. **Filesystem scope Tauri (`fs` plugin):** default ke workspace-dir yang user buka (bukan `fs:default` = HOME penuh). Konsisten dengan semua kompetitor. Tetap simpan opsi "buka folder lain" sebagai aksi eksplisit user (align Claude Code `additional directory` + Kiro Crew sandbox mode `auto`).
2. **Shell plugin:** tetap relevan untuk terminal lokal (ODR-07), tapi harus lewat Go sebagai gate (sudah diputuskan) — selaras pola Copilot CLI (built-in local sandbox selalu jalan) dan Kiro Crew (OS sandbox sebagai lapisan terakhir, bukan satu-satunya).
3. **Audit tool "native"/bypass serupa insiden Antigravity:** cek apakah ada jalur tool Rust/Go yang eksekusi tanpa lewat gate `ALLOWED_TOOLS`/`ExecuteNativeTool` yang sama — jadikan item verifikasi eksplisit, bukan asumsi.
4. **Sub-agent scope (`spawn_subagent` di `dispatcher.go`):** saat ini sub-agent didelegasikan sebagai task tanpa definisi scope eksplisit terpisah dari parent. Pertimbangkan pola Kiro Crew (`capability: subagent` sendiri, tunduk governance ceiling yang sama dengan parent) ketimbang pola Claude Code (yang justru bermasalah — sub-agent kadang dapat scope lebih luas dari yang dimaksud karena tidak dipaksa runtime).
5. **Governance ceiling eksplisit:** tambahkan konsep "ceiling" yang tidak bisa dilonggarkan oleh app/agent manapun (termasuk user-facing Tauri config) — mis. tidak pernah boleh baca `~/.ssh`, `~/.aws`, vault Rust, dari jalur manapun, terlepas dari permission yang user atur di level UI.

---

## 5. Sumber

- Kiro Crew Security: https://kiro.dev/docs/crew/security/
- Kiro Permissions (capability-based): https://kiro.dev/docs/permissions/
- Claude Code Security: https://code.claude.com/docs/en/security
- Claude Code subagent issues: anthropics/claude-code #61315, #89277, #69943, #27661 (GitHub)
- GitHub Copilot CLI local sandboxing: https://docs.github.com/en/copilot/concepts/agents/copilot-cli/understanding-local-sandboxing
- Google Antigravity Secure Mode escape (Pillar Security / CyberScoop, 2026): https://cyberscoop.com/google-antigravity-pillar-security-agent-sandbox-escape-remote-code-execution/
- Cursor vs Claude Code subagents comparison: startdebugging.net, 2026-07
