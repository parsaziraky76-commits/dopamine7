//! پورت ۱:۱ از `core/DiagnosticsLog.kt` + وضعیت زندهٔ بررسی‌های `Diagnostics.kt`.
//!
//! رفع نشت حافظهٔ ۱.۲.۲ عیناً حفظ شده:
//!   * رینگ‌بافر کرانمند ۸۰۰ خطی (نه لیست بی‌انتها)
//!   * نوشتن روی دیسک دسته‌ای و روی ترد پس‌زمینه
//!   * سقف فایل ۵۱۲ KiB با چرخش
//!   * به‌روزرسانی UI حداکثر ~۵ بار در ثانیه
//!
//! جدید: همان `ComponentCheck`های پنل عیب‌یابی اندروید (PENDING/RUNNING/
//! PASS/FAIL) این‌جا نگه‌داری می‌شوند تا UI بتواند زنده آن‌ها را بخواند.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LINES: usize = 800;
const MAX_FILE_BYTES: u64 = 512 * 1024;
pub const UI_THROTTLE: Duration = Duration::from_millis(200); // ~۵ بار در ثانیه
/// فقط این مقدار از انتهای فایلِ قبلی خوانده می‌شود (~۸۰۰ خط معمولی).
const RESTORE_TAIL_BYTES: u64 = 128 * 1024;

/// آخرین `max_lines` خطِ کامل از دُمِ فایل. خطِ اولِ نیمه‌بریده دور ریخته می‌شود.
fn read_tail_lines(path: &Path, tail_bytes: u64, max_lines: usize) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(tail_bytes);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut buf = Vec::with_capacity((len - start) as usize);
    if f.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<&str> = text.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let skip = lines.len().saturating_sub(max_lines);
    lines[skip..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect()
}

/// خط‌های بازیابی‌شده *قبل از* هر خطی می‌نشینند که از شروعِ این اجرا آمده، و
/// سقفِ رینگ با اولویتِ خط‌های تازه رعایت می‌شود.
fn merge_restored(ring: &mut VecDeque<String>, restored: Vec<String>) {
    let room = MAX_LINES.saturating_sub(ring.len());
    if room == 0 {
        return;
    }
    let skip = restored.len().saturating_sub(room);
    for line in restored.into_iter().skip(skip).rev() {
        ring.push_front(line);
    }
}

struct Inner {
    ring: VecDeque<String>,
    pending: Vec<String>,
    file: Option<PathBuf>,
    repeat: Repeat,
}

/// چند سطرِ یکسانِ پشتِ هم که نگه داشته می‌شوند؛ بقیه فقط شمرده می‌شوند.
const REPEAT_KEEP: u32 = 2;

/// تشخیصِ سطرهای تکراریِ پشتِ هم (مثلاً هشدارهای ۵-بار-در-ثانیهٔ سایفون).
/// هر سطر حلقه، فایل و رندرِ UI را می‌خرد؛ سیلی از یک پیام هیچ اطلاعی اضافه
/// نمی‌کند و سطرهای مفید را از حلقهٔ ۸۰۰تایی بیرون می‌راند.
#[derive(Default)]
struct Repeat {
    key: String,
    count: u32,
}

/// کلیدِ مقایسه: سطح، تگ و پیام بی مهرِ زمانیِ داخلیِ موتور (`[2026-…Z WARN …]`).
fn repeat_key(level: char, tag: &str, msg: &str) -> String {
    let body = if msg.starts_with('[') {
        match msg.find(']') {
            Some(end) => {
                let head = &msg[1..end];
                // فقط بخشِ بعد از مهرِ زمانی؛ سطح و ماژولِ موتور می‌مانند.
                let rest = head.split_once(' ').map(|(_, r)| r).unwrap_or(head);
                format!("{rest}]{}", &msg[end + 1..])
            }
            None => msg.to_string(),
        }
    } else {
        msg.to_string()
    };
    format!("{level}/{tag}:{body}")
}

impl Repeat {
    /// `Ok(true)` = سطر را بنویس؛ `Ok(false)` = فقط بشمار. `Err(n)` یعنی پیش از
    /// این سطر، خلاصهٔ «n بارِ دیگر تکرار شد» نوشته شود (و این سطر هم).
    fn feed(&mut self, key: String) -> Result<bool, u32> {
        if key == self.key {
            self.count += 1;
            return Ok(self.count <= REPEAT_KEEP);
        }
        let hidden = self.count.saturating_sub(REPEAT_KEEP);
        self.key = key;
        self.count = 1;
        if hidden > 0 {
            Err(hidden)
        } else {
            Ok(true)
        }
    }
}

static LOG: OnceLock<Mutex<Inner>> = OnceLock::new();

fn inner() -> &'static Mutex<Inner> {
    LOG.get_or_init(|| {
        Mutex::new(Inner {
            ring: VecDeque::with_capacity(MAX_LINES),
            pending: Vec::new(),
            file: None,
            repeat: Repeat::default(),
        })
    })
}

/// معادل `ComponentCheck` اندروید — یک ردیف پنل عیب‌یابی.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentCheck {
    pub id: String,
    pub label: String,
    /// "PENDING" | "RUNNING" | "PASS" | "FAIL"
    pub state: String,
    pub detail: String,
}

static CHECKS: OnceLock<Mutex<Vec<ComponentCheck>>> = OnceLock::new();

fn checks_inner() -> &'static Mutex<Vec<ComponentCheck>> {
    CHECKS.get_or_init(|| Mutex::new(Vec::new()))
}

pub struct DiagnosticsLog;

impl DiagnosticsLog {
    pub fn init(data_dir: &Path) {
        let path = data_dir.join("logs").join("aether.log");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        inner().lock().file = Some(path.clone());
        Self::spawn_flusher();
        // >>> AETHER-APP-PATCH log-restore-off-main (port of mobile 1.3.1/1.4.0 #33)
        // موبایل ۱.۳.۱ نشان داد بازیابیِ لاگِ نشستِ قبل روی ترد اصلی، باز شدنِ
        // برنامه را تا ده ثانیه نگه می‌داشت. این‌جا لاگ رمزنگاری‌نشده است و تا امروز
        // اصلاً بازیابی نمی‌شد (پنل عیب‌یابی بعد از هر اجرا خالی بود). حالا بازیابی
        // می‌شود، ولی با همان دو قاعدهٔ اصلاحِ موبایل:
        //   * روی یک ترد پس‌زمینه — `init` هرگز منتظرِ دیسک نمی‌ماند؛
        //   * فقط دُمِ فایل (`RESTORE_TAIL_BYTES`) — نه کلِ ۵۱۲ KiB برای نگه‌داشتنِ ۸۰۰ خط.
        std::thread::Builder::new()
            .name("aether-log-restore".into())
            .spawn(move || {
                let restored = read_tail_lines(&path, RESTORE_TAIL_BYTES, MAX_LINES);
                if !restored.is_empty() {
                    merge_restored(&mut inner().lock().ring, restored);
                }
            })
            .ok();
        // <<< AETHER-APP-PATCH log-restore-off-main
    }

    pub fn d(tag: &str, msg: &str) {
        Self::push('D', tag, msg)
    }
    pub fn i(tag: &str, msg: &str) {
        Self::push('I', tag, msg)
    }
    pub fn w(tag: &str, msg: &str) {
        Self::push('W', tag, msg)
    }
    pub fn e(tag: &str, msg: &str) {
        Self::push('E', tag, msg)
    }

    fn push(level: char, tag: &str, msg: &str) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let key = repeat_key(level, tag, msg);
        let mut g = inner().lock();
        match g.repeat.feed(key) {
            Ok(false) => return,
            Ok(true) => {}
            Err(hidden) => {
                let note = format!("{ts} I/log: (previous line repeated {hidden} more times)");
                Self::append(&mut g, note);
            }
        }
        let line = format!("{ts} {level}/{tag}: {msg}");
        Self::append(&mut g, line);
    }

    fn append(g: &mut Inner, line: String) {
        if g.ring.len() == MAX_LINES {
            g.ring.pop_front();
        }
        g.ring.push_back(line.clone());
        g.pending.push(line);
    }

    /// خوانده می‌شود توسط پنل Diagnostics — فقط وقتی کنسول باز است
    /// (معادل بهینه‌سازی ۱.۲.۲: فقط کنسولِ باز subscribe می‌کند).
    pub fn tail(limit: usize) -> Vec<String> {
        let g = inner().lock();
        g.ring.iter().rev().take(limit).rev().cloned().collect()
    }

    /// معادل `DiagnosticsLog.exportText()` — خوراک دکمهٔ «Copy logs».
    pub fn export_text() -> String {
        Self::tail(MAX_LINES).join("\n")
    }

    pub fn clear() {
        let mut g = inner().lock();
        g.ring.clear();
        g.pending.clear();
    }

    // ---- وضعیت زندهٔ بررسی‌ها (معادل Diagnostics.checks در اندروید) ----

    /// بازنشانی فهرست بررسی‌ها — معادل `Diagnostics.resetChecks()`.
    pub fn set_checks(items: Vec<(&str, String)>) {
        let mut g = checks_inner().lock();
        *g = items
            .into_iter()
            .map(|(id, label)| ComponentCheck {
                id: id.to_string(),
                label,
                state: "PENDING".to_string(),
                detail: String::new(),
            })
            .collect();
    }

    pub fn update_check(id: &str, state: &str, detail: Option<&str>) {
        let mut g = checks_inner().lock();
        if let Some(c) = g.iter_mut().find(|c| c.id == id) {
            c.state = state.to_string();
            if let Some(d) = detail {
                c.detail = d.to_string();
            }
        }
    }

    pub fn checks() -> Vec<ComponentCheck> {
        checks_inner().lock().clone()
    }

    /// نوشتن دسته‌ای روی ترد پس‌زمینه — مسیر بحرانی هرگز دیسک را لمس نمی‌کند.
    fn spawn_flusher() {
        std::thread::Builder::new()
            .name("aether-log-flush".into())
            .spawn(|| loop {
                std::thread::sleep(UI_THROTTLE);
                let (batch, path) = {
                    let mut g = inner().lock();
                    if g.pending.is_empty() {
                        continue;
                    }
                    (std::mem::take(&mut g.pending), g.file.clone())
                };
                let Some(path) = path else { continue };

                // چرخش فایل با سقف ۵۱۲ KiB.
                if std::fs::metadata(&path)
                    .map(|m| m.len() > MAX_FILE_BYTES)
                    .unwrap_or(false)
                {
                    let _ = std::fs::rename(&path, path.with_extension("log.1"));
                }
                // یک write برای کلِ دسته، نه یک syscall برای هر سطر.
                let mut buf = String::with_capacity(batch.iter().map(|l| l.len() + 1).sum());
                for line in &batch {
                    buf.push_str(line);
                    buf.push('\n');
                }
                if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
                    let _ = f.write_all(buf.as_bytes());
                }
            })
            .ok();
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;

    #[test]
    fn restored_lines_go_before_new_ones_and_respect_the_cap() {
        let mut ring: VecDeque<String> = (0..MAX_LINES - 2).map(|i| format!("new {i}")).collect();
        merge_restored(
            &mut ring,
            vec!["old a".into(), "old b".into(), "old c".into()],
        );
        assert_eq!(ring.len(), MAX_LINES);
        assert_eq!(ring[0], "old b");
        assert_eq!(ring[1], "old c");
        assert_eq!(ring[2], "new 0");
    }

    #[test]
    fn a_flood_of_one_line_is_collapsed() {
        let mut r = Repeat::default();
        let line = |t: &str| {
            repeat_key('D', "engine", &format!("[2026-10-11T01:05:{t}Z WARN  aether::psiphon] [-] psiphon: SOCKS proxy accept error: EOF"))
        };
        assert_eq!(r.feed(line("55.062")), Ok(true));
        assert_eq!(r.feed(line("55.258")), Ok(true));
        for t in ["55.473", "55.692", "55.913"] {
            assert_eq!(r.feed(line(t)), Ok(false));
        }
        assert_eq!(r.feed(repeat_key('I', "state", "Disconnected")), Err(3));
        assert_eq!(r.feed(repeat_key('I', "state", "Connecting")), Ok(true));
        // پیام‌های متفاوت هرگز یکی نمی‌شوند.
        assert_ne!(
            repeat_key('I', "tor", "Bootstrapped 10%"),
            repeat_key('I', "tor", "Bootstrapped 14%")
        );
    }

    #[test]
    fn tail_drops_the_cut_first_line() {
        let dir = std::env::temp_dir().join(format!("aether-log-tail-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.log");
        std::fs::write(&path, "aaaaaaaaaa\nbbbb\ncccc\n").unwrap();
        let lines = read_tail_lines(&path, 12, 10);
        assert_eq!(lines, vec!["bbbb".to_string(), "cccc".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
