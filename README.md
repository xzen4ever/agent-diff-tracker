# ⚡ DiffTrack - AI Code Change Aggregator

> **Ứng dụng Rust giao diện toàn màn hình (Fullscreen GUI) giúp quét, tổng hợp và phân tích toàn bộ các dòng mã nguồn thay đổi (Lines of Code Changes) từ lịch sử chat của các công cụ AI: Antigravity, Claude Code, OpenAI Codex / Copilot CLI, và Gemini CLI.**

---

## 📌 Mục lục
- [Giới thiệu](#-giới-thiệu)
- [Tính năng nổi bật](#-tính-năng-nổi-bật)
- [Cấu trúc dự án](#-cấu-trúc-dự-án)
- [Phương pháp tính toán (Calculation Methodology)](#-phương-pháp-tính-toán-calculation-methodology)
- [Công nghệ sử dụng](#-công-nghệ-sử-dụng)
- [Hướng dẫn cài đặt & Khởi chạy](#-hướng-dẫn-cài-đặt--khởi-chạy)
- [Hướng dẫn sử dụng chi tiết](#-hướng-dẫn-sử-dụng-chi-tiết)
- [Xử lý sự cố (Troubleshooting)](#-xử-lý-sự-cố-troubleshooting)

---

## 📖 Giới thiệu

Trong quá trình lập trình cùng các trợ lý AI (AI Coding Assistants), rất nhiều thay đổi code diễn ra tự động thông qua các tool call hoặc diff patches rải rác ở hàng chục session khác nhau. 

**DiffTrack** được xây dựng bằng ngôn ngữ **Rust** nhằm:
1. Tự động truy vết và phát hiện các session chat từ các công cụ AI phổ biến trên máy.
2. Phân tích ngữ cảnh, bóc tách chính xác từng thao tác can thiệp tệp tin (`write_to_file`, `replace_file_content`, `Edit`, `Write`, `patch`).
3. Tổng hợp số dòng code thêm mới (`+`), số dòng bị xóa (`-`), tính toán Net Change và hiển thị trực quan dưới dạng Unified Diff có số dòng và highlight màu sắc.
4. Giao diện trực quan, mặc định mở **Toàn màn hình (Fullscreen)** giúp lập trình viên có cái nhìn toàn cảnh về mọi thay đổi mã nguồn do AI thực hiện.

---

## ✨ Tính năng nổi bật

* 🖥️ **Toàn màn hình mặc định (Fullscreen Mode)**: Tối ưu không gian hiển thị diff code, hỗ trợ bật/tắt toàn màn hình qua phím tắt `F11`.
* 🤖 **Hỗ trợ đa công cụ AI**:
  * **Antigravity**: Tự động quét toàn bộ `brain` sessions (`~/.gemini/antigravity-cli/brain/`), trích xuất tool call JSONL và prompt `<USER_REQUEST>`.
  * **Claude Code**: Quét dự án và session logs (`~/.claude/projects/`, `~/.claude/sessions/`, `AppData/Roaming/Claude/`).
  * **OpenAI Codex / Copilot CLI**: Quét lịch sử command và logs (`~/.copilot/logs/`, `~/.codex/`).
  * **Gemini CLI**: Quét lịch sử lệnh và transcript (`~/.gemini/gemini-cli/`, `~/.gemini/history/`).
  * **Custom Folder Scanner**: Quét bất kỳ thư mục nào trên máy tính do người dùng chỉ định.
* 🇻🇳 **Hỗ trợ Unicode & Tiếng Việt 100%**: Nhúng trực tiếp bộ font cao cấp **JetBrains Mono** (`JetBrainsMono-Regular`, `JetBrainsMono-Bold`) vào file thực thi, đồng nhất hiển thị trên cả Windows và Linux (Ubuntu, Debian...) mà không cần cài font ngoài, giải mã chuỗi `\uXXXX`, không bao giờ bị lỗi font ô vuông ("tofu").
* 📊 **Dashboard số liệu & Bảng tổng hợp không nhấp nháy**:
  * Thống kê: Tổng session, tổng thay đổi, tổng số dòng thêm (`+`), xóa (`-`), số file độc nhất bị sửa đổi.
  * Tab **Summary**: Gom nhóm theo file, sắp xếp ổn định tuyệt đối (Deterministic Sorting), hiển thị dạng lưới `egui::Grid` có sọc ngang rõ ràng.
* 🔍 **Tìm kiếm & Lọc thời gian thực**: Lọc theo từng công cụ, tìm kiếm nhanh theo tên file, tiêu đề prompt hoặc đường dẫn.
* ⚡ **Hiệu năng cao & Không đơ UI**: Thực hiện quét nền qua luồng riêng (`std::thread` + channel `mpsc`), phím tắt `F5` để quét lại tức thì.

---

## 🏗️ Cấu trúc dự án

Dự án được tổ chức dạng module hóa rõ ràng theo chuẩn Rust:

```text
.diff-track/
├── Cargo.toml                  # Khai báo dependencies (eframe, egui, serde, walkdir)
├── Cargo.lock                  # Lock file phiên bản phụ thuộc
├── LICENSE                     # Giấy phép nguồn mở MIT & OFL-1.1
├── LLM.md                      # Tài liệu kỹ thuật kiến trúc cho AI / Developer
├── README.md                   # Tài liệu hướng dẫn dự án
└── src/
    ├── main.rs                 # Router điều phối: Khởi chạy GUI hoặc CLI tùy theo đối số đầu vào
    ├── models.rs               # Định nghĩa các cấu trúc dữ liệu cốt lõi (ChatSession, CodeChange, FileSummary, ...)
    ├── aggregator.rs           # Module TỔNG HỢP: Tính toán stats, nhóm file, xếp hạng LOC, lọc session độc lập
    ├── cli/
    │   └── mod.rs              # Module GIAO DIỆN CLI: Terminal dashboard, bảng ANSI, xuất JSON, xem diff trực tiếp
    ├── scanner/
    │   ├── mod.rs              # Module QUÉT: Điều phối quét đa nguồn đệ quy
    │   ├── antigravity.rs      # Parser chuyên sâu cho Antigravity JSONL transcripts
    │   ├── claude.rs           # Parser cho Claude Code session logs (Edit, Write, MultiEdit)
    │   ├── codex.rs            # Parser cho Codex rollout JSONL (apply_patch, patch, diff)
    │   ├── gemini.rs           # Parser cho Gemini CLI history & transcripts (edit, write_file)
    │   └── generic_diff.rs     # Parser trích xuất unified diff format chuẩn (---, +++, @@)
    └── ui/
        ├── mod.rs              # Re-export các UI components
        ├── app.rs              # Module GIAO DIỆN ĐỒ HỌA: State chính của App, Top Metric Bar, Tab Summary
        ├── diff_viewer.rs      # Widget hiển thị diff màu sắc, số dòng và nút sao chép
        └── sidebar.rs          # Widget thanh bên (Bộ lọc công cụ, Search box, Danh sách sessions)
```

### Chi tiết các thành phần chính:

| File / Thư mục | Trách nhiệm |
| :--- | :--- |
| [`src/models.rs`](src/models.rs) | Định nghĩa `ToolType`, `CodeChange`, `ChatSession`, `DiffLine`, `DiffLineType`, `FileSummary`, `ScanStats`. |
| [`src/aggregator.rs`](src/aggregator.rs) | **Module Tổng hợp độc lập**: Bóc tách toàn bộ logic tính toán (`compute_stats`, `compute_file_summaries`, `filter_sessions`, `find_session`) ra khỏi UI, dùng chung cho cả GUI và CLI. |
| [`src/cli/mod.rs`](src/cli/mod.rs) | **Module Giao diện CLI**: Hiển thị bảng tổng hợp trên terminal, dashboard màu ANSI, tìm kiếm, xuất JSON và in diff chi tiết. |
| [`src/scanner/`](src/scanner/) | **Module Quét dữ liệu**: Đọc stream và bóc tách các tệp nhật ký chat từ Antigravity, Claude Code, Codex, Gemini CLI. |
| [`src/ui/`](src/ui/) | **Module Giao diện Đồ họa (GUI)**: Chỉ tập trung vào việc render màn hình Fullscreen qua `eframe`/`egui 0.29`, không chứa logic xử lý dữ liệu thô. |

---

## 🔍 Đường dẫn lưu trữ & Định dạng log (Đã xác minh qua mã nguồn target CLI)

DiffTrack đã được rà soát và đối chiếu trực tiếp với mã nguồn của 4 công cụ AI hàng đầu trong `src-of-target-cli/`:

| Công cụ | Đường dẫn lưu trữ mặc định & Biến môi trường | Định dạng tệp tin & Cấu trúc phiên chat |
| :--- | :--- | :--- |
| **Google Antigravity CLI** | • `~/.gemini/antigravity-cli/brain/{conversation-id}/.system_generated/logs/transcript.jsonl`<br>• `~/.gemini/antigravity-cli/brain/{conversation-id}/transcript.jsonl`<br>• `~/.gemini/antigravity/conversations/` | • Định dạng JSON Lines (`.jsonl`).<br>• Ghi lại từng bước `PLANNER_RESPONSE` chứa mảng `tool_calls`.<br>• Tool tạo file: `write_to_file` (`TargetFile`, `CodeContent`).<br>• Tool sửa file: `replace_file_content` (`TargetFile`, `TargetContent`, `ReplacementContent`, `StartLine`). |
| **Claude Code** (Anthropic) | • `~/.claude/projects/` (lưu phiên làm việc theo từng dự án làm việc)<br>• `~/.claude/sessions/`<br>• `~/.claude/history/`<br>• `~/.claude/`<br>• Biến `CLAUDE_CONFIG_DIR`<br>• Windows: `%APPDATA%\Claude`, `%LOCALAPPDATA%\Claude`, `%APPDATA%\claude-code` | • Tệp JSON hoặc JSONL ghi lại các sự kiện chat và tool execution.<br>• Phân tích khối `tool_use` với `name: "Edit"` hoặc `"Replace"` (chứa `file_path`, `old_string`, `new_string`).<br>• Khối `tool_use` với `name: "Write"` (chứa `file_path`, `content`).<br>• Bóc tách Unified Diff khi có thao tác bash/git. |
| **OpenAI Codex** (`codex-rs`) | • `{CODEX_HOME}/sessions/{YYYY}/{MM}/{DD}/rollout-*.jsonl`<br>• `{CODEX_HOME}/archived_sessions/`<br>• Biến `CODEX_HOME` (nếu không đặt, mặc định là `~/.codex`)<br>• Copilot CLI: `~/.copilot/logs/`, `~/.config/github-copilot/` | • Định dạng `rollout-*.jsonl` lưu theo cấu trúc ngày tháng năm.<br>• Mỗi dòng là một record: `{"type": "response_item", "payload": { ... }}`.<br>• Bóc tách hàm thực thi: `name: "apply_patch"` với tham số `arguments` chứa patch hoặc diff.<br>• Bóc tách tiêu đề session từ `payload.role == "user"`. |
| **Google Gemini CLI** | • `~/.gemini/tmp/{project_hash_or_slug}/chats/session-*.jsonl`<br>• `~/.gemini/tmp/{project_hash_or_slug}/chats/{parentId}/{subagentId}.jsonl`<br>• `~/.gemini/history/{project_hash_or_slug}/` (shadow git repo)<br>• `~/.gemini/tmp/{project_hash_or_slug}/logs/` | • Tệp `session-{timestamp}-{shortId}.jsonl` lưu trong thư mục `tmp/{project}/chats/`.<br>• Dòng đầu tiên chứa metadata `ConversationRecord` (`sessionId`, `startTime`, `summary`).<br>• Các tin nhắn của Gemini chứa `toolCalls` gồm: `edit` (`file_path`, `old_str`, `new_str`) và `write_file` (`file_path`, `content`). |

---

## 📐 Phương pháp tính toán (Calculation Methodology)

DiffTrack áp dụng các quy chuẩn bóc tách và tính toán dòng mã thay đổi (LOC - Lines of Code) cụ thể cho từng loại thao tác:

### 1. Thao tác Tạo mới / Ghi đè file (`write_to_file` / `Write`)
* **Bản chất**: AI cung cấp toàn bộ nội dung của tệp tin đích (`CodeContent` hoặc `content`).
* **Phương pháp tính**:
  * `lines_added = count(lines in content)`
  * `lines_deleted = 0`
  * `change_type = ChangeType::Create`
* **Cấu trúc Diff**: Sinh ra hunk header `@@ +1,N @@` và toàn bộ các dòng được đánh dấu dấu cộng `+` kèm số dòng mới từ `1` đến `N`.

### 2. Thao tác Thay thế nội dung (`replace_file_content` / `Edit`)
* **Bản chất**: AI chỉ định khối code cũ cần bỏ (`TargetContent`) và khối code mới thay vào (`ReplacementContent`) tại vị trí `StartLine` đến `EndLine`.
* **Phương pháp tính**:
  * `lines_deleted = count(lines in TargetContent)`
  * `lines_added = count(lines in ReplacementContent)`
  * `change_type = ChangeType::Modify`
* **Cấu trúc Diff**: 
  * Header: `@@ -StartLine,lines_deleted +StartLine,lines_added @@ (Description)`
  * Các dòng trong `TargetContent` được gán tiền tố `-` kèm số dòng gốc `StartLine + i`.
  * Các dòng trong `ReplacementContent` được gán tiền tố `+` kèm số dòng mới `StartLine + i`.

### 3. Thao tác Unified Diff (`diff --git`, `.patch`)
* Quét từng dòng của đoạn văn bản:
  * Bắt đầu bằng `+` (loại trừ `+++`): tăng `lines_added`, gán `DiffLineType::Added`.
  * Bắt đầu bằng `-` (loại trừ `---`): tăng `lines_deleted`, gán `DiffLineType::Removed`.
  * Bắt đầu bằng dấu cách ` `: dòng ngữ cảnh (Context), tăng đồng thời con trỏ dòng cũ và mới.

### 4. Thuật toán Tổng hợp & Ổn định hiển thị (Flicker-Free Aggregation)
* Để loại bỏ hiện tượng nhấp nháy dòng khi render ở 60 FPS:
  1. **Không tính toán lại trong hàm vẽ**: Dữ liệu tổng hợp được tính toán sẵn một lần vào `summary_files` khi khởi động hoặc sau khi luồng quét nền trả kết quả.
  2. **Gom nhóm bằng `BTreeMap`**: Đảm bảo thứ tự key luôn cố định, không bị phụ thuộc vào seed hash ngẫu nhiên như `HashMap`.
  3. **Sắp xếp ổn định (Stable Sort với Tie-Breaker)**:
     * **Ưu tiên 1**: `lines_added + lines_deleted` (giảm dần theo tổng biến động LOC).
     * **Ưu tiên 2**: `file_path` (tăng dần theo thứ tự bảng chữ cái để chống nhảy thứ tự hàng).
  4. **Dựng hình bằng `egui::Grid`**: Các cột có chiều rộng cố định, không dùng `right_to_left` lồng nhau để loại bỏ hoàn toàn việc dao động sub-pixel làm vỡ dòng.

---

## 💻 Công nghệ sử dụng

* **Ngôn ngữ**: [Rust 1.80+ / 1.99](https://www.rust-lang.org/) (Toolchain `x86_64-pc-windows-gnu`).
* **Giao diện (GUI)**: [`eframe`](https://crates.io/crates/eframe) & [`egui 0.29`](https://crates.io/crates/egui) (Immediate mode GUI thuần Rust, rendering qua OpenGL / `glow`).
* **Xử lý dữ liệu**:
  * [`serde`](https://crates.io/crates/serde) & [`serde_json`](https://crates.io/crates/serde_json): Đọc stream (`BufRead`) và parse JSON / JSONL dung lượng lớn.
  * [`walkdir`](https://crates.io/crates/walkdir): Quét đệ quy cây thư mục tốc độ cao, an toàn với symlink và giới hạn độ sâu (`max_depth`).
  * Pure Rust Diff Engine: Bóc tách unified diff, patch và line counting thuần Standard Library không phụ thuộc thư viện ngoài cồng kềnh.

---

## 🚀 Hướng dẫn cài đặt & Khởi chạy

### Yêu cầu hệ thống
* Hệ điều hành: Windows 10/11 (64-bit) hoặc Linux (Ubuntu, Debian, Fedora...).
* Rust Toolchain đã được cài đặt (`cargo` và `rustc` trong PATH).

### 1. Khởi chạy trực tiếp bản Release đã build sẵn
Nếu bạn đã clone/build dự án, chỉ cần chạy:
```powershell
.\target\release\diff-track.exe
```

### 2. Build từ mã nguồn

```powershell
# 1. Clone repository
git clone https://github.com/xzen4ever/agent-diff-tracker.git
cd agent-diff-tracker

# 2. Khởi chạy trực tiếp (Debug mode)
cargo run

# 3. Build & Chạy bản tối ưu hiệu năng cao (Release mode)
cargo run --release

# 4. Build file thực thi độc lập (Standalone .exe ~13.7 MB)
cargo build --release
```
File `.exe` thành phẩm sẽ nằm tại `target/release/diff-track.exe` (có thể copy và chạy độc lập trên bất kỳ máy Windows nào mà không cần cài thêm font hay phụ thuộc ngoài).

---

## 💻 Giao diện dòng lệnh (CLI Interface)

Bên cạnh giao diện đồ họa toàn màn hình, DiffTrack cung cấp giao diện dòng lệnh (CLI) cực nhanh, thích hợp cho làm việc trong PowerShell / Windows Terminal hoặc tích hợp vào CI/CD script:

### Các lệnh thông dụng:

| Lệnh | Ý nghĩa & Mô tả |
| :--- | :--- |
| `.\diff-track.exe` | Mở ứng dụng ở chế độ **Fullscreen GUI** (mặc định khi không truyền tham số). |
| `.\diff-track.exe --gui` | Khởi chạy giao diện đồ họa GUI. |
| `.\diff-track.exe stats` | In dashboard số liệu thống kê tổng hợp (tổng sessions, LOC `+ thêm / - xóa`, số file, số lượng phiên từng tool). |
| `.\diff-track.exe summary` | In bảng xếp hạng các tệp tin có số dòng thay đổi nhiều nhất (Top files changed). |
| `.\diff-track.exe summary -n 10` | Giới hạn in 10 file thay đổi nhiều nhất. |
| `.\diff-track.exe sessions` | Liệt kê danh sách tất cả các phiên trò chuyện AI kèm prompt, thời gian, số dòng biến động. |
| `.\diff-track.exe agents` (hoặc `by-agent`) | **Liệt kê toàn bộ session theo từng Agent** (Antigravity, Claude, Codex, Gemini), kèm tổng LOC và danh sách phiên chi tiết. |
| `.\diff-track.exe sessions --by-agent` (hoặc `-b`) | Nhóm các phiên theo Agent trực tiếp từ lệnh `sessions`. |
| `.\diff-track.exe agents --simple` (hoặc `-s`) | Liệt kê session theo từng Agent ở định dạng rút gọn 1 dòng/phiên, phân cách bởi tiêu đề `# AGENT: ...`. |
| `.\diff-track.exe --json agents` | Xuất toàn bộ dữ liệu session gom nhóm theo từng Agent dưới dạng JSON. |
| `.\diff-track.exe sessions -t antigravity` | Lọc phiên theo công cụ (`antigravity`, `claude`, `codex`, `gemini`). |
| `.\diff-track.exe sessions -q "MainActivity"` | Tìm kiếm phiên chat có chứa từ khóa trong prompt hoặc tên file. |
| `.\diff-track.exe diff <SESSION_ID>` | Hiển thị toàn bộ Unified Diff của một phiên chat với cú pháp tô màu trực tiếp trên console. |
| `.\diff-track.exe --simple` (hoặc `-s`) | In thông tin **cô đặc, có cấu trúc**, loại bỏ toàn bộ viền trang trí, cực kỳ ngắn gọn và dễ parse bằng script. |
| `.\diff-track.exe stats --simple` | In các dòng key-value số liệu thống kê rút gọn (sessions, changes, files, added, deleted, net). |
| `.\diff-track.exe summary --simple -n 10` | In bảng tóm tắt 10 file thay đổi nhiều nhất, mỗi file trên 1 dòng đơn giản. |
| `.\diff-track.exe sessions --simple` | In danh sách các phiên chat dạng 1 dòng/phiên (ID, thời gian, Tool, +Add/-Del, Prompt). |
| `.\diff-track.exe diff <ID> --simple` | In unified diff thuần túy, không kèm thông tin banner bao quanh. |
| `.\diff-track.exe --json stats` | Xuất dữ liệu thống kê ra định dạng JSON cho tool tự động hóa. |
| `.\diff-track.exe --help` | In tài liệu hướng dẫn nhanh các tùy chọn dòng lệnh. |

---

## 🖥️ Hướng dẫn sử dụng Giao diện Đồ họa (GUI)

### Các phím tắt tiện ích
| Phím tắt | Thao tác | Mô tả |
| :---: | :--- | :--- |
| **`F11`** | **Toggle Fullscreen** | Bật hoặc tắt chế độ toàn màn hình tức thì. |
| **`F5`** | **Rescan Logs** | Chạy quét lại toàn bộ thư mục log của các công cụ AI trong nền mà không làm đơ ứng dụng. |

---

### Các khu vực giao diện chính

#### 1. Thanh tiêu đề & Dashboard thống kê (Top Panel)
* **Metric Badges**:
  * `Sessions`: Tổng số phiên trò chuyện AI tìm thấy có chứa thay đổi code.
  * `Changes`: Tổng số thao tác sửa đổi file.
  * `Lines Added`: Tổng số dòng code mới được thêm vào (màu xanh lá `+N`).
  * `Lines Deleted`: Tổng số dòng code bị xóa đi (màu đỏ `-N`).
  * `Files`: Tổng số file phân biệt (unique files) bị can thiệp.
* **Scan Directory**: Ô nhập đường dẫn thư mục tùy biến (ví dụ: `C:\ws\Prj\my-project`) -> bấm **Scan Custom Folder** để quét thêm dự án ngoài.
* **Nút bấm tiện ích**: Bật tắt `Fullscreen` / `Windowed`, nút quét lại `Rescan`, và nút chuyển đổi tab `Diff View` / `Summary`.

#### 2. Thanh bên trái (Sidebar)
* **Bộ lọc công cụ (Tool Pills)**: Bấm vào badge `All Tools`, `AGY` (Antigravity), `CLAUDE`, `CODEX`, hoặc `GEMINI` để lọc danh sách session theo công cụ tương ứng.
* **Search Box**: Gõ từ khóa bất kỳ để lọc session theo:
  * Tên tệp tin (ví dụ: `MainActivity.java`, `Cargo.toml`).
  * Nội dung yêu cầu prompt của người dùng (ví dụ: `fix error`, `tạo giao diện`).
  * ID của session.
* **Danh sách Session Cards**: Mỗi card hiển thị huy hiệu công cụ, nhãn thời gian, tiêu đề yêu cầu người dùng và biến động `+thêm / -xóa`.

#### 3. Khu vực giữa: Danh sách File thay đổi trong Session
* Khi chọn một session ở Sidebar, danh sách các file được AI tạo mới hoặc sửa đổi trong phiên đó sẽ hiển thị tại đây kèm huy hiệu `CREATED` hoặc `MODIFIED`.

#### 4. Khu vực chính: Trình xem Diff (Diff Viewer)
* Hiển thị đường dẫn file đầy đủ, hành động thực hiện và timestamp.
* **Bảng so khớp Diff**:
  * Cột 1: Số dòng cũ.
  * Cột 2: Số dòng mới.
  * Cột 3: Nội dung dòng code (Dòng xanh lá `+` là thêm mới, dòng đỏ `-` là xóa đi, dòng xanh lam là header hunk).
* **Các nút tiện ích nhanh**:
  * 📋 **Copy Diff**: Sao chép toàn bộ unified diff vào Clipboard.
  * 📋 **Copy Path**: Sao chép đường dẫn file.
  * 📁 **Open Folder**: Mở thư mục chứa file trực tiếp trong Windows Explorer.

#### 5. Tab Bảng tổng hợp (📊 Summary Tab)
* Chuyển sang tab này để xem bảng phân tích tất cả các file bị tác động nhiều nhất trên toàn hệ thống.
* Có thanh tìm kiếm file riêng (`Filter files...`).
* Bảng hiển thị cố định: Tên file, Đường dẫn thu gọn, Số lần sửa đổi, Tổng số dòng thêm, Tổng số dòng xóa.

---

## 🛠️ Xử lý sự cố (Troubleshooting)

### 1. Gặp lỗi `error calling dlltool 'dlltool.exe'` khi build
* **Nguyên nhân**: Crate `parking_lot_core` hoặc `windows-targets` gọi `dlltool` để tạo thư viện liên kết tạm thời trên Windows MinGW.
* **Khắc phục**: Shim `dlltool.exe` có thể đặt tại `%USERPROFILE%\.cargo\bin\dlltool.exe` (Windows) hoặc `~/.cargo/bin/dlltool.exe`. Đảm bảo biến môi trường `PATH` của người dùng chứa đường dẫn này.

### 2. Ký tự tiếng Việt hiển thị thành ô vuông ("tofu")
* **Khắc phục**: Ứng dụng đã nhúng trực tiếp bộ font **JetBrains Mono** vào file thực thi nên không phụ thuộc vào font cài sẵn của hệ điều hành. Dù chạy trên Windows hay Linux (Ubuntu, Debian...), toàn bộ ký tự tiếng Việt và ký hiệu lập trình đều hiển thị chuẩn xác 100%.

### 3. Cửa sổ mở lên bị đen màn hình
* **Khắc phục**: DiffTrack sử dụng backend OpenGL (`glow`). Hãy đảm bảo driver đồ họa (GPU Driver) trên máy tính hỗ trợ OpenGL 2.1 trở lên.

---

## 📄 Giấy phép (License)
* Mã nguồn dự án được phát hành theo giấy phép [MIT](LICENSE).
* Bộ font nhúng **JetBrains Mono** được phát hành theo giấy phép [SIL Open Font License 1.1](assets/fonts/OFL.txt) (Copyright 2020 The JetBrains Mono Project Authors).

