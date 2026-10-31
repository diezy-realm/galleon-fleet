# Amandemen `rust-conventions.md` — 2026-10-29 (Rust Single Language)

> [!NOTE]
> **Applied**: Amandemen ini telah diterapkan secara penuh ke [`rust-conventions.md`](./rust-conventions.md) melalui TASK-101 (PR `feat/docs-rust-conventions-amendment`).

**Status:** Applied (diterapkan ke `rust-conventions.md` via TASK-101)
**Dasar:** ODR-2026-10-29, ADR-0018, `board/research/RES-2026-10-29-rust-engineering-standards.md`
**Cara menerapkan:** ganti/ tambah bagian di `standard-docs/tech-standart/tech-stack/rust-conventions.md` persis seperti tabel di bawah (TASK-052, TASK-101). Bagian yang tidak disebut tetap.

| Bagian di `rust-conventions.md` (baris @3f3a729) | Tindakan |
|---|---|
| `## gRPC Server Patterns` (267–341), contoh `tonic` di Workspace (42, 78), `grpc_server.rs` di Naming/Layout (552, 569, 585–588) | **Hapus.** Tidak ada gRPC internal (ODR-26). |
| `## Logging (tracing)` (449–491) dan contoh `tracing::info!` di `## Security` (645) | **Ganti** dengan §1 di bawah. Konvensi lama bertentangan dengan `clippy.toml disallowed-macros`. |
| `## Related Documents` → link `go-conventions.md` (687) | **Ganti** dengan link ke ADR-0018 dan dokumen ini. |
| Setelah `## Crate Boundaries` | **Tambah** §2 Modularitas. |
| Setelah `## Error Handling` | **Tambah** §3 Lints, Unsafe, Panic. |
| `## Testing` | **Tambah** §4. |
| Bagian baru di akhir | **Tambah** §5 Build cache, §6 Pola handler HTTP, §7 File I/O. |

---

## 1. Logging (menggantikan "Logging (tracing)")

Semua event log lewat `::galleon_log::record!` (dipaksa oleh `clippy.toml` `disallowed-macros` + `-D warnings`). Contoh nyata dari `crates/galleon-gateway/src/api_pairing.rs:453`:

```rust
::galleon_log::record!(
    ERROR,
    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
        .with_outcome(::galleon_log::EventOutcome::Failure)
        .with_attrs(::serde_json::json!({ "error": format!("{e}") })),
    "device registry insert failed after successful pairing; rolling back in-process token"
);
```

- Jangan log secret, token, atau body request mentah. Gunakan `galleon-security-scrub` bila perlu menyertakan input pengguna.
- Catatan drift: teks `reason` di `clippy.toml` masih menyebut `::clawcrew_log::record!`; diperbaiki di TASK-093.

## 2. Modularitas Crate

1. Domain baru = crate baru `galleon-<domain>-core` tanpa `axum`, `galleon-runtime`, `galleon-gateway`. Adapter HTTP di `galleon-gateway/src/api_<domain>/`.
2. Arah dependensi: `gateway → runtime → leaf`; `gateway → <domain>-core → leaf`. Leaf tidak boleh mengimpor `runtime`/`gateway`. Cek: `cargo tree -p <leaf> -e normal | rg 'galleon-(runtime|gateway)'` harus kosong.
3. Tidak membuat crate untuk fitur tanpa konsumen (YAGNI). Stub/kerangka kosong dilarang.
4. Nama crate tidak boleh `galleon-fleet` (nama package root).
5. Dependensi pihak ketiga dideklarasikan di `[workspace.dependencies]` bila dipakai ≥ 2 crate.

## 3. Lints, Unsafe, Panic

1. Setiap `Cargo.toml` member: `[lints] workspace = true`.
2. Crate dengan hutang `unwrap/expect`: di `lib.rs`/`main.rs`
   ```rust
   // RATCHET(BL-091): baseline unwrap+expect = <N> pada <commit>. Turunkan, jangan naikkan.
   #![allow(clippy::unwrap_used, clippy::expect_used)]
   ```
   Crate baru **tidak boleh** memakai allow ini. Penambahan `unwrap/expect` baru di jalur produksi dicegah oleh gate diff yang **sudah ada**, `scripts/ci/check_production_unwrap.py` (T-CI-01) — dipasang ke gate di TASK-093, bukan membuat script ratchet baru.
3. Crate baru: `#![forbid(unsafe_code)]`. Crate lama yang memakai `unsafe`: setiap blok wajib komentar `// SAFETY:` (clippy `undocumented_unsafe_blocks` diaktifkan per crate saat crate itu disentuh).
4. Test boleh `unwrap()`/`expect()` (`#[cfg(test)]` + `#[allow(clippy::unwrap_used)]` di modul test).
5. Library tidak memanggil `std::process::exit` atau `panic!` untuk kondisi yang bisa dipulihkan.

## 4. Testing

| Lapisan | Alat | Perintah |
|---|---|---|
| Unit + integrasi | cargo-nextest | `cargo nextest run -p <crate> --locked` |
| Doctest | cargo test | `cargo test --doc -p <crate> --locked` (nextest tidak menjalankan doctest) |
| Kontrak HTTP | `insta` (sudah di `Cargo.lock`) | snapshot JSON per rute di `crates/galleon-gateway/tests/snapshots/` |
| Negatif keamanan | test biasa | 401 tanpa token, 403 di luar allowlist, 400 field asing, 413 body > limit |

- Profil `ci` nextest boleh retry; profil `default` **tidak**. Test flaky dicatat di backlog, bukan ditutup retry.
- Test yang menulis file memakai `tempfile::TempDir`, tidak menulis ke repo.

## 5. Build Cache (sccache)

- `rustc-wrapper` **tidak** ditaruh di `.cargo/config.toml`. Aktifkan via env:
  ```bash
  export RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0
  ```
- Alasan: Cargo gagal bila sccache tidak terpasang; sccache tidak dapat meng-cache build incremental, dan crate `bin`/`dylib`/`cdylib`/`proc-macro` tidak di-cache ([sccache Rust docs](https://github.com/mozilla/sccache/blob/main/docs/Rust.md)).

## 6. Pola Handler HTTP (gateway)

```rust
pub async fn handle_get_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::api::require_auth(&state, &headers).map_err(ApiError::from)?;   // 1. auth dulu
    let name = CollectionName::from_str(&name).map_err(ApiError::bad_request)?; // 2. validasi tipe
    let value = state.fleet.load(name).await.map_err(ApiError::from)?;       // 3. panggil domain
    Ok(Json(value))                                                           // 4. tanpa logika lain
}
```

- `ApiError` memetakan error domain ke status eksplisit; pesan ke klien generik, detail ke `record!`.
- Request body yang mengubah state: `#[serde(deny_unknown_fields)]`.
- Body limit dan timeout dari layer router, bukan per handler (kecuali override eksplisit).

## 7. File I/O

- Tulis atomik: `tempfile::NamedTempFile::new_in(dir)` → `write_all` → `as_file().sync_all()` → `persist(path)`; pola rujukan `crates/galleon-infra/src/session_store.rs:223`.
- Unix: permission 0600 untuk data pengguna.
- I/O di handler async lewat `tokio::fs` atau `spawn_blocking`.
- Nama file dari input pengguna **selalu** lewat enum/newtype tertutup, tidak pernah `format!("{dir}/{user_input}.json")`.
