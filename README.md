# ⚡ DiffTrack

> Ứng dụng Rust độc lập quét, tổng hợp và hiển thị trực quan các dòng mã thay đổi (+/- LOC) từ lịch sử chat của các trợ lý AI: **Antigravity**, **Claude Code**, **OpenAI Codex**, và **Gemini CLI**.

Hỗ trợ 2 chế độ song song: **Fullscreen GUI** (mượt mà 60 FPS, nhúng font JetBrains Mono hiển thị tiếng Việt hoàn hảo) và **Fast CLI** (hỗ trợ cờ `--simple` và `--json` cho script/CI).

---

## 🚀 Cài đặt & Khởi chạy

Yêu cầu: [Rust Toolchain](https://www.rust-lang.org/) (cargo).

```powershell
# 1. Chạy ngay chế độ Debug
cargo run

# 2. Build bản Release tối ưu (target/release/diff-track.exe ~13.7 MB)
cargo build --release

# 3. Khởi chạy trực tiếp file build
.\target\release\diff-track.exe
```

---

## ✨ Tính năng chính

- **Đa nền tảng AI**: Tự động phát hiện session logs từ Antigravity, Claude Code, Codex, Gemini CLI hoặc thư mục bất kỳ.
- **Tính toán LOC chuẩn xác**: Bóc tách chính xác các thao tác tạo file, sửa file (`replace_file_content`, `Edit`) và Unified Diff.
- **Fullscreen GUI (F11)**: Xem diff tô màu thời gian thực, bảng metrics, tìm kiếm và lọc theo agent/prompt/file.
- **Giao diện CLI mạnh mẽ**: Bảng màu ANSI, xuất JSON, xem diff trực tiếp trên terminal.
- **Font tiếng Việt nhúng sẵn**: Tích hợp JetBrains Mono, hiển thị chuẩn xác 100% không cần cài font ngoài.

---

## 💻 Lệnh CLI thông dụng

| Lệnh | Mô tả |
| :--- | :--- |
| `.\diff-track.exe` | Mở giao diện Fullscreen GUI (mặc định) |
| `.\diff-track.exe stats` | Dashboard thống kê tổng quan (sessions, LOC +/-, files) |
| `.\diff-track.exe summary [-n 10]` | Top file có nhiều dòng code biến động nhất |
| `.\diff-track.exe sessions [-t <tool>] [-q <query>]` | Danh sách session chat, hỗ trợ lọc theo tool và từ khóa |
| `.\diff-track.exe agents` (hoặc `by-agent`) | Thống kê và gom nhóm session theo từng Agent |
| `.\diff-track.exe diff <SESSION_ID>` | Xem Unified Diff chi tiết có tô màu của một session |

> **Mẹo CLI**: Thêm cờ `--simple` (hoặc `-s`) để lấy dữ liệu 1 dòng không viền cho shell script, hoặc `--json` để xuất dữ liệu JSON.

---

## ⌨️ Phím tắt GUI

- **`F11`**: Bật / tắt chế độ toàn màn hình (Fullscreen).
- **`F5`**: Quét lại toàn bộ logs trong nền (không đơ UI).

---

## 📖 Tài liệu kỹ thuật

Chi tiết kiến trúc module, parser logs từng AI và quy chuẩn tính toán: xem [LLM.md](LLM.md).

---

## 📄 License

- Mã nguồn: [MIT](LICENSE)
- Font JetBrains Mono: [SIL Open Font License 1.1](assets/fonts/OFL.txt)
