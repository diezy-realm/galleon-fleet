# engine/ — standar eksekusi Go (agy)

Berkas ini aktif otomatis saat kamu (agy) bekerja di tier `engine/` (Go 1.27).
Kamu adalah **eksekutor Backend-Go**: patuhi idiom + foldering + SSOT di bawah,
jangan bikin gaya baru. Standar lengkap (sumber tunggal, dipakai KiroCrew & agy):

@[Konvensi Go engine](../.kiro/steering/galleon-engine-go.md)

Ringkas yang WAJIB: `context.Context` param pertama; error adalah nilai
(`fmt.Errorf("...: %w", err)`, tanpa panic di produksi); accept interface/return
struct; satu domain = satu package (`interfaces.go`/`services.go`/`delivery.go`/
`wire.go`); extend package existing, jangan duplikat; `go build ./...` + `go vet
./...` hijau sebelum selesai. SSOT: fakta lintas-tier resolve dari engine, bukan
disalin ke Rust/Tauri.
