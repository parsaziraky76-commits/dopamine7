#!/usr/bin/env python3
"""حلِ تعارض‌های merge هستهٔ ۱.۹.۰ — هر تصمیم با دلیلش.

یافتهٔ اصلیِ این ارتقا: آپ‌استریمِ ۱.۹.۰ بخش بزرگی از پچ‌های ۱.۲.۳ دسکتاپ را
**خودش جذب کرده** — تفکیک بافر ارسال/دریافتِ TCP، پنجره‌های بزرگ‌تر HTTP/2 و
دسته‌بندیِ کپسول‌ها. جایی که آپ‌استریم همان کار را می‌کند، نسخهٔ آپ‌استریم
می‌ماند و پچ دسکتاپ **حذف** می‌شود: نگه‌داشتن نسخهٔ ما یعنی جنگیدن با APIهایی که
دیگر مالِ ما نیستند (`append_datagram_capsule`، `SenderMsg`، `log_or_debug`) و
هزینهٔ نگه‌داری آن هر ارتقای بعدی دوباره پرداخت می‌شود.

آنچه آپ‌استریم جذب **نکرده** و باید بماند:
  1. کنترل ازدحام CUBIC در smoltcp (آپ‌استریم هنوز هیچ کنترل‌کننده‌ای انتخاب
     نمی‌کند؛ با `default-features = false` نتیجه `NoControl` است).
  2. سقفِ صفِ تحویلِ بسته در lib.rs (ضد bufferbloat).
  3. تفکیک SO_RCVBUF/SO_SNDBUF سوکت‌های UDP — quic.rs و upstream.rs دسکتاپ
     همین حالا صداش می‌زنند.
  4. پچ‌های dns.rs، prober.rs، wg_prober.rs، wireguard.rs، netstack.rs که تمیز
     merge شدند.

از این پس هر پچ با `AETHER-APP-PATCH <نام>` علامت‌گذاری می‌شود — همان قراردادی
که مخزن موبایل در ۱.۲.۹ گذاشت، تا merge بعدی بداند کدام خط مالِ برنامه است.
"""
import pathlib, re, shutil, sys

OUT = pathlib.Path('/tmp/core19')
MOB = pathlib.Path('/tmp/mob/Aether-main/native/aether')

def pristine(rel: str) -> pathlib.Path:
    """بکرِ ۱.۹.۰: اگر موبایل فایل را پچ کرده، baseline‌اش؛ وگرنه خودش.

    `masque_h2.rs` در موبایل baseline ندارد چون موبایل هرگز پچش نکرده —
    پس فایل کاریِ خودش همان بکرِ آپ‌استریم است.
    """
    b = MOB / '.upstream-baseline' / rel
    return b if b.exists() else MOB / rel

def blocks(text):
    """(pre, ours, base, theirs) برای هر تعارض + دُم."""
    parts, i, lines = [], 0, text.splitlines(keepends=True)
    buf = []
    while i < len(lines):
        if lines[i].startswith('<<<<<<<'):
            m = next(j for j in range(i, len(lines)) if lines[j].startswith('|||||||'))
            e = next(j for j in range(m, len(lines)) if lines[j].startswith('======='))
            z = next(j for j in range(e, len(lines)) if lines[j].startswith('>>>>>>>'))
            parts.append((''.join(buf), ''.join(lines[i+1:m]), ''.join(lines[m+1:e]), ''.join(lines[e+1:z])))
            buf = []
            i = z + 1
            continue
        buf.append(lines[i])
        i += 1
    return parts, ''.join(buf)

def resolve(rel, chooser):
    """`chooser(n, ours, base, theirs) -> str` متنِ نهاییِ هر تعارض."""
    path = OUT / rel
    parts, tail = blocks(path.read_text())
    out = []
    for n, (pre, ours, base, theirs) in enumerate(parts, 1):
        out.append(pre)
        out.append(chooser(n, ours, base, theirs))
    out.append(tail)
    path.write_text(''.join(out))
    left = path.read_text().count('<<<<<<<')
    print(f'{rel:<26} {len(parts)} تعارض حل شد، باقی‌مانده: {left}')
    return left

left = 0

# ---------------------------------------------------------------- Cargo.toml
# آپ‌استریم smoltcp را به 0.14 برد ولی هنوز هیچ کنترل ازدحامی انتخاب نمی‌کند.
# نسخه از آپ‌استریم می‌آید و فقط ویژگیِ cubic اضافه می‌شود (در 0.14 هم با همین
# نام وجود دارد — از فهرست ویژگی‌های crates.io بررسی شد).
CUBIC = '''# >>> AETHER-APP-PATCH netstack-congestion-control
# `default-features = false` کنترل ازدحام smoltcp را هم خاموش می‌کند، پس
# `AnyController::new()` به `NoControl` می‌رسید: فرستندهٔ TCP نتستک بی هیچ پنجرهٔ
# ازدحامی کار می‌کرد. اِتِر TCP را داخل smoltcp خاتمه می‌دهد، پس همین پشته —
# نه TCP خودِ ویندوز — مالک کنترل ازدحامِ هر چیزی است که از تونل بیرون می‌رود.
# ویژگی اینجا و انتخاب صریح در netstack.rs (`set_congestion_control`) با هم
# هستند تا حذفِ ویژگی، بیلد را بشکند و نه اینکه بی‌صدا این باگ را برگرداند.
smoltcp = { version = "0.14", default-features = false, features = ["std", "medium-ip", "proto-ipv4", "proto-ipv6", "socket-tcp", "socket-udp", "socket-tcp-cubic"] }
# <<< AETHER-APP-PATCH netstack-congestion-control
'''
left += resolve('aether/Cargo.toml', lambda n, o, b, t: CUBIC)

# ------------------------------------------------------------------- lib.rs
# هر دو طرف *افزودنی*اند و به هم کاری ندارند: سقفِ صف مالِ ما، ماژول تست‌ها
# مالِ آپ‌استریم. هر دو می‌مانند.
def lib_choice(n, ours, base, theirs):
    ours = ours.replace(
        '\n/// Depth of the two PACKET handoff queues',
        '\n// >>> AETHER-APP-PATCH packet-queue-depth\n/// Depth of the two PACKET handoff queues', 1)
    return ours.rstrip('\n') + '\n// <<< AETHER-APP-PATCH packet-queue-depth\n' + theirs
left += resolve('aether/src/lib.rs', lib_choice)

# ---------------------------------------------------------------- netstack.rs
# هر دو تعارض «یک کار، دو نگارش»اند: آپ‌استریم دقیقاً همان تفکیک rx/tx را
# پذیرفته. متن آپ‌استریم می‌ماند تا فایل با کدی که آپ‌استریم نگه می‌دارد یکی
# باشد؛ انتخابِ CUBIC چند خط پایین‌تر پچِ ما است و دست‌نخورده ماند.
left += resolve('aether/src/netstack.rs', lambda n, o, b, t: t)

# --------------------------------------------------------------- masque_h2.rs
# آپ‌استریم مسیر ارسال را از نو نوشت: یک تسک `pump_outbound` با دسته‌بندی
# (`H2_SEND_BATCH_BYTES`) و `append_datagram_capsule`. این همان کاری است که پچ
# ما می‌کرد، ولی در کدی که آپ‌استریم نگه می‌دارد. پس پچِ ما کنار می‌رود و فایل
# دیگر «پچ‌خورده» نیست. تلمتریِ H2Stats هم با آن می‌رود؛ همان اعداد از
# `[h2] flow control` و لاگ‌های آپ‌استریم درمی‌آید.
shutil.copy2(pristine('aether/src/masque_h2.rs'), OUT / 'aether/src/masque_h2.rs')
print(f'{"aether/src/masque_h2.rs":<26} پچ دسکتاپ کنار گذاشته شد → نسخهٔ بکرِ ۱.۹.۰')

# --------------------------------------------------------------- sysprofile.rs
# ۹ تعارض داشت و همه‌شان یک ریشه: آپ‌استریم همان تفکیک‌ها را با نام‌های دیگر
# انجام داده. پس از نسخهٔ بکرِ ۱.۹.۰ شروع می‌کنیم و تنها پچی که آپ‌استریم جذب
# نکرده — تفکیک SO_RCVBUF/SO_SNDBUF — دوباره روی آن اعمال می‌شود. quic.rs و
# upstream.rs همین حالا این دو تابع را صدا می‌زنند، پس اگر جا بیفتد کامپایل
# می‌شکند و بی‌صدا نمی‌ماند.
sp = pristine('aether/src/sysprofile.rs').read_text()

def sub1(pattern, repl, text, what):
    new, n = re.subn(pattern, repl, text, count=1)
    if n != 1:
        sys.exit(f'!! جای «{what}» در sysprofile بکر پیدا نشد — دستی بررسی شود')
    return new

sp = sub1(r'    pub udp_socket_buf: usize,\n',
'''    // >>> AETHER-APP-PATCH udp-socket-buffer-split
    /// `SO_RCVBUF` هر سوکت دیتاگرامِ مسیر داده.
    ///
    /// بافر دریافت برای هیچ‌کس هزینهٔ تأخیر ندارد: پرشدنش یعنی بسته‌های
    /// رسیده‌ای که هنوز خوانده نشده‌اند، و سرریزش را طرف مقابل به‌صورت گم‌شدن
    /// بسته می‌بیند و پنجره‌اش را می‌بندد — همان چیزی که کاربر «دانلود کند»
    /// می‌خواندش.
    pub udp_socket_rcv_buf: usize,
    /// `SO_SNDBUF` هر سوکت دیتاگرامِ مسیر داده.
    ///
    /// عمداً خیلی کوچک‌تر از سمت دریافت، و به دلیل معکوس: این آخرین صف بین
    /// کنترل‌کنندهٔ ازدحام و کارت شبکه است و `send()` روی سوکت دیتاگرام فقط
    /// وقتی این بافر پر باشد فشار برمی‌گرداند. اندازه‌اش را با بودجهٔ تأخیر
    /// بگیر، نه با حجم RAM: بافر ارسالی که هرگز پر نشود، هر throttle بالای
    /// خودش را بی‌صدا خاموش می‌کند.
    pub udp_socket_snd_buf: usize,
    // <<< AETHER-APP-PATCH udp-socket-buffer-split
''', sp, 'فیلد udp_socket_buf')

sp = sub1(r'    let \(scan_concurrency_cap, udp_socket_buf, netstack_udp_buf, channel_capacity\) = match tier \{\n'
          r'        Tier::Low => \(4usize, (\d+) \* 1024, 32 \* 1024, 128usize\),\n'
          r'        Tier::Medium => \(10usize, 2 \* 1024 \* 1024, 64 \* 1024, 512usize\),\n'
          r'        Tier::High => \(usize::MAX, 7 \* 1024 \* 1024, 128 \* 1024, 1024usize\),\n'
          r'    \};\n',
'''    // >>> AETHER-APP-PATCH udp-socket-buffer-split
    let (scan_concurrency_cap, udp_socket_rcv_buf, udp_socket_snd_buf, netstack_udp_buf, channel_capacity) =
        match tier {
            Tier::Low => (4usize, 512 * 1024, 64 * 1024, 32 * 1024, 128usize),
            Tier::Medium => (10usize, 2 * 1024 * 1024, 192 * 1024, 64 * 1024, 512usize),
            Tier::High => (usize::MAX, 7 * 1024 * 1024, 384 * 1024, 128 * 1024, 1024usize),
        };
    // <<< AETHER-APP-PATCH udp-socket-buffer-split
''', sp, 'جدول tier')

sp = sub1(r'        scan_concurrency_cap,\n        udp_socket_buf,\n',
'''        scan_concurrency_cap,
        // >>> AETHER-APP-PATCH udp-socket-buffer-split
        udp_socket_rcv_buf,
        udp_socket_snd_buf,
        // <<< AETHER-APP-PATCH udp-socket-buffer-split
''', sp, 'ساختِ Tuning')

sp = sub1(r'udp socket buffer=\{\}KB', 'udp socket rcv/snd={}KB/{}KB', sp, 'متن لاگ')
sp = sub1(r'        t\.udp_socket_buf / 1024,\n',
'''        // >>> AETHER-APP-PATCH udp-socket-buffer-split
        t.udp_socket_rcv_buf / 1024,
        t.udp_socket_snd_buf / 1024,
        // <<< AETHER-APP-PATCH udp-socket-buffer-split
''', sp, 'آرگومان‌های لاگ')

sp = sub1(r'pub fn udp_socket_buf_bytes\(\) -> usize \{\n    tuning\(\)\.udp_socket_buf\n\}\n',
'''// >>> AETHER-APP-PATCH udp-socket-buffer-split
pub fn udp_socket_rcv_buf_bytes() -> usize {
    tuning().udp_socket_rcv_buf
}

pub fn udp_socket_snd_buf_bytes() -> usize {
    tuning().udp_socket_snd_buf
}
// <<< AETHER-APP-PATCH udp-socket-buffer-split
''', sp, 'تابع دسترسی')

(OUT / 'aether/src/sysprofile.rs').write_text(sp)
print(f'{"aether/src/sysprofile.rs":<26} از بکرِ ۱.۹.۰ + پچ تفکیک بافر UDP بازساخته شد')

print('\nتعارض باقی‌مانده در کل درخت:', left)
sys.exit(0 if left == 0 else 2)
