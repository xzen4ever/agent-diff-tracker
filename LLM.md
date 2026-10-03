# DiffTrack - LLM & Developer Maintenance Guide

> **Mục đích tài liệu**: Tài liệu này được thiết kế riêng cho các mô hình AI/LLM và lập trình viên kế thừa tiếp tục phát triển, bảo trì, hoặc mở rộng dự án `diff-track`. Tài liệu cung cấp toàn bộ bối cảnh kiến trúc, quy tắc phân tách module, định dạng dữ liệu của các công cụ AI, quy trình build, và hướng dẫn từng bước khi thêm tính năng mới.

---

## 1. Tổng quan Dự án (Project Overview)

* **Tên dự án**: `diff-track`
* **Ngôn ngữ**: Rust (2021 Edition, Toolchain `x86_64-pc-windows-gnu` trên Windows 10/11)
* **Bản chất**: Ứng dụng độc lập (Standalone tool) quét, tổng hợp và hiển thị trực quan toàn bộ số dòng mã thay đổi (LOC: Lines of Code added/deleted/net) từ lịch sử phiên chat của các trợ lý AI lập trình:
  1. **Google Antigravity CLI** (`google-antigravity/antigravity-cli`)
  2. **Anthropic Claude Code** (`anthropics/claude-code`)
  3. **OpenAI Codex** (`openai/codex` -> `codex-rs`)
  4. **Google Gemini CLI** (`google-gemini/gemini-cli`)
  5. Các tệp nhật ký chứa Unified Diff tiêu chuẩn (`.diff`, `.patch`, git diff).
* **Hai chế độ vận hành song song**:
  * **Fullscreen GUI**: Giao diện Immediate Mode GUI đồ họa mượt mà (60 FPS qua `eframe` / `egui 0.29`), hỗ trợ font Unicode tiếng Việt không lỗi tofu, tìm kiếm thời gian thực, xem diff tô màu trực quan.
  * **CLI Mode**: Giao diện dòng lệnh trong terminal với các cờ lệnh (`stats`, `summary`, `sessions`, `diff`, `scan`), hỗ trợ xuất JSON (`--json`) và cờ siêu tinh gọn (`--simple` / `-s`).

---

## 2. Kiến trúc Hệ thống & Quy tắc Phân tách Module (Architecture)

Dự án áp dụng mô hình phân tách trách nhiệm nghiêm ngặt (Strict Separation of Concerns):

```text
               ┌────────────────────────────────────────────────────────┐
               │                        main.rs                         │
               │  (Router: phân luồng GUI hoặc CLI dựa trên đối số)      │
               └───────────┬────────────────────────────────┬───────────┘
                           │ (Nếu không có tham số / --gui) │ (Nếu có cờ CLI / lệnh)
                           ▼                                ▼
               ┌───────────────────────┐        ┌───────────────────────┐
               │       src/ui/         │        │       src/cli/        │
               │   (Fullscreen GUI)    │        │  (Terminal Interface) │
               │ • app.rs (State/Font) │        │ • mod.rs              │
               │ • sidebar.rs          │        │ • Dashboard, Tables   │
               │ • diff_viewer.rs      │        │ • --simple / --json   │
               └───────────┬───────────┘        └───────────┬───────────┘
                           │                                │
                           └────────────────┬───────────────┘
                                            │ Dùng chung dữ liệu tổng hợp
                                            ▼
                           ┌─────────────────────────────────┐
                           │        src/aggregator.rs        │
                           │  (Business Logic & Analytics)   │
                           │ • compute_stats()               │
                           │ • compute_file_summaries()      │
                           │ • group_sessions_by_agent()     │
                           │ • filter_sessions()             │
                           │ • find_session()                │
                           └────────────────┬────────────────┘
                                            │ Nhận danh sách ChatSession
                                            ▼
                           ┌─────────────────────────────────┐
                           │        src/scanner/             │
                           │  (Discovery & Tool Parsers)     │
                           │ • mod.rs (Coordinator)          │
                           │ • antigravity.rs                │
                           │ • claude.rs                     │
                           │ • codex.rs                      │
                           │ • gemini.rs                     │
                           │ • generic_diff.rs               │
                           └────────────────┬────────────────┘
                                            │ Tuân theo Data Models
                                            ▼
                           ┌─────────────────────────────────┐
                           │         src/models.rs           │
                           │   (Core Data Structures)        │
                           │ ToolType, CodeChange, DiffLine, │
                           │ ChatSession, FileSummary, Stats │
                           └─────────────────────────────────┘
```

### Quy tắc bất di bất dịch (Architectural Invariants):
1. **Không viết logic tổng hợp / tính toán trong UI**: Mọi phép tính số liệu (`stats`, `summary`, xếp hạng file, lọc session) **bắt buộc** phải nằm trong `src/aggregator.rs`. Module `ui/` chỉ là view layer.
2. **Không để UI phụ thuộc vào CLI hoặc ngược lại**: Cả `src/ui/` và `src/cli/` đều là các consumer độc lập của `src/aggregator.rs` và `src/scanner/`.
3. **Mọi parser công cụ AI phải trả về `Option<ChatSession>`**: Chuẩn hóa dữ liệu thô từ log của từng công cụ về `ChatSession` và `Vec<CodeChange>`.

---

## 3. Bản đồ Lưu trữ & Cấu trúc Log của 4 Công cụ AI

Khi cập nhật hoặc gỡ lỗi đường dẫn quét, hãy đối chiếu với bảng thông số đã xác minh từ mã nguồn gốc:

### A. Google Antigravity CLI
* **Vị trí lưu trữ**:
  * `~/.gemini/antigravity-cli/brain/{conversation-id}/.system_generated/logs/transcript.jsonl` (ưu tiên đọc bản này).
  * `~/.gemini/antigravity-cli/brain/{conversation-id}/.system_generated/logs/transcript_full.jsonl`.
  * `~/.gemini/antigravity-cli/brain/{conversation-id}/transcript.jsonl`.
  * `~/.gemini/antigravity/conversations/`.
* **Cấu trúc JSONL**:
  * Các dòng bước có `type == "PLANNER_RESPONSE"` chứa `tool_calls`.
  * **Tool tạo file**: `name == "write_to_file"` -> tham số `TargetFile`, `CodeContent`, `Overwrite`.
  * **Tool sửa file**: `name == "replace_file_content"` -> tham số `TargetFile`, `TargetContent`, `ReplacementContent`, `StartLine`.
  * **Đặc thù**: Chuỗi tham số đôi khi bị serialize 2 lần (`"\"content\""` hoặc chứa escape `\u003c`), giải mã an toàn qua hàm [`unwrap_nested_str()`](src/scanner/antigravity.rs).

### B. Anthropic Claude Code
* **Vị trí lưu trữ**:
  * `~/.claude/projects/` (lưu phiên làm việc theo từng dự án).
  * `~/.claude/sessions/` và `~/.claude/history/`.
  * Thư mục biến môi trường: `$CLAUDE_CONFIG_DIR`.
  * Trên Windows: `%APPDATA%\Claude`, `%LOCALAPPDATA%\Claude`, `%APPDATA%\claude-code`.
* **Cấu trúc JSON / JSONL**:
  * Các khối `tool_use` có `name == "Edit"` hoặc `"Replace"` (chứa `file_path`, `old_string`, `new_string`).
  * Khối `tool_use` có `name == "Write"` hoặc `"write_file"` (chứa `file_path`, `content`).
  * Bóc tách Unified Diff khi có thao tác bash/git.

### C. OpenAI Codex (`codex-rs`)
* **Vị trí lưu trữ**:
  * Thư mục gốc: `$CODEX_HOME` (nếu không đặt, mặc định là `~/.codex`).
  * Cây thư mục rollout lưu theo ngày tháng: `{CODEX_HOME}/sessions/{YYYY}/{MM}/{DD}/rollout-*.jsonl` (cần độ sâu `max_depth >= 6`).
  * Thư mục lưu trữ cũ: `{CODEX_HOME}/archived_sessions/`.
  * GitHub Copilot CLI: `~/.copilot/logs/`, `~/.config/github-copilot/`.
* **Cấu trúc JSONL**:
  * Từng dòng mang cấu trúc `{"timestamp": "...", "type": "response_item", "payload": { ... }}`.
  * Thao tác code: `payload.name == "apply_patch"` với `arguments` chứa chuỗi JSON hoặc đối tượng mang trường `patch` hoặc `diff`.

### D. Google Gemini CLI
* **Vị trí lưu trữ**:
  * Thư mục temp & runtime: `~/.gemini/tmp/`.
  * Phiên chat dự án: `~/.gemini/tmp/{project_hash_or_slug}/chats/session-*.jsonl`.
  * Phiên chat subagent: `~/.gemini/tmp/{project_hash_or_slug}/chats/{parentId}/{subagentId}.jsonl`.
  * Shadow git repository: `~/.gemini/history/{project_hash_or_slug}/`.
* **Cấu trúc JSONL**:
  * Dòng đầu tiên là `ConversationRecord` chứa `sessionId`, `startTime`, `summary`.
  * Tin nhắn từ Gemini chứa mảng `toolCalls` gồm:
    * `name == "edit"`: tham số `file_path`, `old_str`, `new_str`.
    * `name == "write_file"`: tham số `file_path`, `content`.

---

## 4. Các Cấu trúc Dữ liệu Cốt lõi ([`src/models.rs`](src/models.rs))

```rust
// Loại công cụ AI
pub enum ToolType {
    Antigravity,
    ClaudeCode,
    Codex,
    GeminiCli,
    Generic,
}

// Loại can thiệp file
pub enum ChangeType {
    Create,   // File mới hoặc ghi đè toàn bộ
    Modify,   // Sửa đổi một phần
    Delete,   // Xóa file
}

// Một dòng trong Diff
pub struct DiffLine {
    pub line_type: DiffLineType, // Header (@@), Added (+), Removed (-), Context ( )
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub text: String,
}

// Chi tiết một lần AI thay đổi code
pub struct CodeChange {
    pub id: String,
    pub tool: ToolType,
    pub session_id: String,
    pub session_title: String,
    pub timestamp: String,
    pub file_path: String,
    pub change_type: ChangeType,
    pub description: String,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub diff_lines: Vec<DiffLine>,
    pub target_tool_action: Option<String>,
}

// Một phiên làm việc (chat session)
pub struct ChatSession {
    pub id: String,
    pub tool: ToolType,
    pub title: String,
    pub timestamp: String,
    pub log_path: String,
    pub changes: Vec<CodeChange>,
    pub total_lines_added: usize,
    pub total_lines_deleted: usize,
}

// Bản ghi tổng hợp theo từng file
pub struct FileSummary {
    pub file_path: String,
    pub filename: String,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub change_count: usize,
}
```

---

## 5. Hướng dẫn Dành cho LLM khi Mở rộng Tính năng

### Trường hợp 1: Thêm một Công cụ AI mới (Ví dụ: `Cursor`, `Aider`, `Continue`)
1. **Thêm biến thể vào enum [`ToolType`](src/models.rs)**:
   * Thêm `Cursor` vào enum `ToolType`.
   * Cập nhật `display_name()`, `badge_short()`, `color_rgba()` trong `impl ToolType`.
2. **Tạo parser mới tại `src/scanner/<tool_name>.rs`**:
   * Viết hàm `pub fn parse_<tool>_log(path: &Path, session_id: &str) -> Option<ChatSession>`.
   * Bóc tách các thao tác thêm/xóa dòng thành `Vec<DiffLine>` và `CodeChange`.
3. **Đăng ký module và đường dẫn trong [`src/scanner/mod.rs`](src/scanner/mod.rs)**:
   * `pub mod <tool_name>;`
   * Bổ sung hàm `scan_<tool_name>(&self) -> Vec<ChatSession>` với danh sách các thư mục chứa log trên Windows/Linux.
   * Gọi hàm này trong `scan_all()`.
4. **Cập nhật [`src/aggregator.rs`](src/aggregator.rs)**:
   * Đếm số lượng phiên của công cụ trong `compute_stats()`.
5. **Cập nhật UI & CLI**:
   * Trong [`src/ui/sidebar.rs`](src/ui/sidebar.rs): Thêm công cụ vào danh sách nút lọc `tools = [...]`.
   * Trong [`src/cli/mod.rs`](src/cli/mod.rs): Thêm nhãn nhận diện cho cờ `--tool <name>` và in badge màu.

### Trường hợp 2: Thay đổi Phương pháp Tính toán / Sắp xếp
* Chỉ sửa tại [`src/aggregator.rs`](src/aggregator.rs):
  * Thuật toán sắp xếp `compute_file_summaries`: Hiện tại đang sắp xếp theo `lines_added + lines_deleted` giảm dần, tiếp theo là `file_path` theo thứ tự chữ cái để đảm bảo danh sách ổn định tuyệt đối, không nhấp nháy khung hình (flickering).
  * Chạy `cargo test` để xác nhận test `test_compute_stats_and_summaries` vẫn pass.

### Trường hợp 3: Thêm lệnh hoặc Cờ mới cho CLI
* Thêm vào [`src/cli/mod.rs`](src/cli/mod.rs):
  * Bổ sung nhánh so khớp trong vòng lặp `while i < args.len()`.
  * Khi người dùng truyền cờ `--simple` hoặc `-s`, hãy tuân thủ nguyên tắc: **in dữ liệu trực tiếp 1 dòng cho mỗi bản ghi, không dùng khung viền trang trí**.

---

## 6. Lưu ý Kỹ thuật & Môi trường Build (Build Environment Quirks)

### ⚠️ Shim `dlltool.exe` trên Windows MinGW
* **Vấn đề**: Crate `parking_lot_core` (phụ thuộc của `eframe`) trên toolchain MinGW (`x86_64-pc-windows-gnu`) gọi công cụ `dlltool.exe` để tạo thư viện import tạm. Nếu thiếu, build sẽ báo lỗi `error calling dlltool 'dlltool.exe'`.
* **Giải pháp đã thiết lập**: Một shim `dlltool.exe` có thể đặt tại `%USERPROFILE%\.cargo\bin\dlltool.exe` (Windows) hoặc `~/.cargo/bin/dlltool.exe`. LLM khi thực thi lệnh cần đảm bảo thư mục `.cargo\bin` nằm trong `PATH`.

### ⚠️ Cấu hình Windows Long Paths cho Git
* Khi clone các repository Node.js/TypeScript monorepo lớn (như `claude-code`, `gemini-cli`), Windows có thể bị lỗi đường dẫn vượt quá 260 ký tự (`Filename too long`).
* Luôn kích hoạt: `git config --global core.longpaths true`.

### ⚠️ Font chữ Unicode tiếng Việt trên giao diện GUI
* `egui` mặc định **không** có glyph tiếng Việt.
* Hàm `setup_custom_fonts()` trong [`src/ui/app.rs`](src/ui/app.rs) tự động nạp font `C:\Windows\Fonts\segoeui.ttf` và `consola.ttf`. Khi mở rộng hoặc thay đổi theme font, phải luôn duy trì `segoeui.ttf` hoặc một font Unicode hoàn chỉnh để không gây lỗi tofu (`□`).

---

## 7. Quy trình Kiểm thử & Xác thực (Verification Workflow)

Trước khi bàn giao bất kỳ thay đổi nào, LLM **bắt buộc** phải chạy chuỗi lệnh kiểm tra sau:

```powershell
# 1. Chạy toàn bộ Unit Tests (phải pass 100%)
cargo test

# 2. Build bản Release tối ưu hóa
cargo build --release

# 3. Kiểm tra lệnh CLI chạy tốt với dữ liệu thực tế
.\target\release\diff-track.exe stats --simple
.\target\release\diff-track.exe summary --simple -n 5

# 4. Kiểm tra Git Status (đảm bảo không để lại file rác tạm)
git status
```
