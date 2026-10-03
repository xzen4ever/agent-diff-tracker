# DiffTrack

DiffTrack là công cụ desktop và dòng lệnh viết bằng Rust, dùng để quét, phân tích và tổng hợp các thay đổi mã nguồn (Lines of Code: added/deleted/net) từ lịch sử phiên làm việc của các trợ lý AI lập trình: Google Antigravity CLI, Anthropic Claude Code, OpenAI Codex và Google Gemini CLI.

Công cụ cung cấp hai giao diện sử dụng:
- **Giao diện đồ họa (GUI)**: Dựng trên nền `eframe`/`egui`, hiển thị unified diff có đánh số dòng, tìm kiếm theo session/file và bảng thống kê tổng hợp.
- **Giao diện dòng lệnh (CLI)**: Phục vụ truy vấn nhanh trên terminal hoặc tích hợp kịch bản tự động hóa (hỗ trợ cờ `--simple` và `--json`).

---

## Cài đặt & Build

Yêu cầu: Rust toolchain (cargo).

```powershell
# Chạy trực tiếp chế độ phát triển
cargo run

# Build bản Release (file thực thi tại target/release/diff-track.exe)
cargo build --release
```

---

## Tính năng

- **Quét đa nguồn**: Tự động nhận diện nhật ký phiên từ Antigravity (`brain/*.jsonl`), Claude Code (`projects/`, `sessions/`), Codex (`rollout-*.jsonl`), Gemini CLI (`tmp/*/chats/`), hoặc thư mục tùy chỉnh do người dùng chỉ định.
- **Phân tích thay đổi mã nguồn**: Trích xuất các thao tác tạo file mới (`write_to_file`, `Write`), sửa đổi nội dung (`replace_file_content`, `Edit`) và Unified Diff.
- **Giao diện đồ họa (GUI)**:
  - Phím tắt `F11`: Bật/tắt chế độ toàn màn hình.
  - Phím tắt `F5`: Quét lại dữ liệu trong nền.
  - Nhúng trực tiếp font JetBrains Mono, hỗ trợ hiển thị đầy đủ bộ ký tự tiếng Việt và ký hiệu lập trình.
- **Giao diện dòng lệnh (CLI)**: Xuất dữ liệu thống kê dạng bảng terminal, định dạng thô tối giản (`--simple`), hoặc định dạng JSON (`--json`).

---

## Cú pháp dòng lệnh (CLI)

```text
diff-track.exe [COMMAND] [OPTIONS]
```

| Lệnh | Chức năng |
| :--- | :--- |
| `diff-track.exe` | Khởi chạy giao diện đồ họa (mặc định nếu không truyền tham số) |
| `diff-track.exe stats` | Hiển thị bảng tổng hợp: tổng session, file thay đổi, số dòng thêm/xóa |
| `diff-track.exe summary [-n <N>]` | Liệt kê các file có khối lượng thay đổi dòng code lớn nhất |
| `diff-track.exe sessions [-t <TOOL>] [-q <QUERY>]` | Liệt kê danh sách phiên làm việc, hỗ trợ lọc theo công cụ hoặc từ khóa |
| `diff-track.exe agents` | Thống kê số lượng phiên và dòng mã thay đổi theo từng trợ lý AI |
| `diff-track.exe diff <SESSION_ID>` | In chi tiết Unified Diff của một phiên làm việc cụ thể |

### Tùy chọn định dạng đầu ra
- `--simple` hoặc `-s`: In kết quả dạng text rút gọn không viền bảng, phù hợp cho xử lý qua script.
- `--json`: Xuất toàn bộ dữ liệu cấu trúc dưới dạng JSON.

---

## Tài liệu kỹ thuật

Chi tiết về cấu trúc dữ liệu, định dạng file log của từng trợ lý AI và kiến trúc hệ thống được ghi nhận tại [LLM.md](LLM.md).

---

## Giấy phép

- Mã nguồn: [MIT](LICENSE)
- Font JetBrains Mono: [SIL Open Font License 1.1](assets/fonts/OFL.txt)
