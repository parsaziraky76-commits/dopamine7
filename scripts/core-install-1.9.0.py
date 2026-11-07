#!/usr/bin/env python3
"""نصبِ هستهٔ merge‌شدهٔ ۱.۹.۰ در مخزن + به‌روزکردن هر جایی که نسخه را می‌داند.

این مرحله بعد از `core-upgrade-1.9.0.py` و `core-upgrade-resolve.py` اجرا می‌شود و
تا وقتی `cargo check` روی درختِ merge‌شده سبز نشده نباید اجرا شود: جای هستهٔ
قدیمی را می‌گیرد.
"""
import pathlib, shutil, re, sys, subprocess

ROOT = pathlib.Path('.')
NEW = pathlib.Path('/tmp/core19')
DST = ROOT / 'native/aether'
OLD_SNAP = pathlib.Path('/tmp/core18-backup')

if not (NEW / 'aether/src/lib.rs').exists():
    sys.exit('!! درخت merge‌شده در /tmp/core19 نیست')
if any(NEW.rglob('*.rs')) and subprocess.run(
        ['grep', '-rq', '<<<<<<<', str(NEW / 'aether/src')], capture_output=True).returncode == 0:
    sys.exit('!! نشانهٔ تعارض در درخت merge‌شده — نصب متوقف شد')

# پشتیبان از ۱.۸.۰: نه برای آرشیو، برای مقایسه در گزارش و برای rollback دستی.
if OLD_SNAP.exists():
    shutil.rmtree(OLD_SNAP)
shutil.copytree(DST, OLD_SNAP, symlinks=True)

# دو چیزی که در درختِ جدید نیست و باید بماند: پوشهٔ .git (اگر بود) — و هیچ چیز
# دیگر. baseline تازه را خودِ اسکریپت merge ساخته است.
shutil.rmtree(DST)
shutil.copytree(NEW, DST, symlinks=True)

# --- نسخه‌ها ---------------------------------------------------------------
(ROOT / 'CORE_VERSION').write_text('1.9.0\n')
(DST / 'CORE_VERSION').write_text('1.9.0\n')

# --- sync-core.sh ---------------------------------------------------------
sc = ROOT / 'scripts/sync-core.sh'
text = sc.read_text()
text = text.replace('BASELINE="1.8.0"', 'BASELINE="1.9.0"', 1)

# `masque_h2.rs` دیگر پچ محلی ندارد: آپ‌استریمِ ۱.۹.۰ همان کار را می‌کند. اگر در
# این فهرست بماند، هر sync بعدی یک merge سه‌طرفهٔ بی‌معنی روی فایلی می‌زند که
# پچی ندارد — و اولین تعارضِ کاذب، sync را بی‌دلیل متوقف می‌کند.
old_block = re.search(r'PATCHED_FILES=\(\n(?:.*?\n)*?\)\n', text)
if not old_block:
    sys.exit('!! فهرست PATCHED_FILES پیدا نشد')
new_block = '''PATCHED_FILES=(
  aether/src/prober.rs
  aether/src/wg_prober.rs
  aether/src/netstack.rs
  aether/src/wireguard.rs
  aether/src/lib.rs
  aether/src/sysprofile.rs
  aether/src/upstream.rs
  aether/src/quic.rs
  aether/src/dns.rs
  aether/Cargo.toml
)
'''
text = text[:old_block.start()] + new_block + text[old_block.end():]

# توضیح ارتقا، بالای فهرست — تاریخِ تصمیم‌ها جایی می‌ماند که دفعهٔ بعد خوانده
# می‌شود، نه فقط در یادداشت انتشار.
note = '''# ۱.۲.۴ (هستهٔ ۱.۹.۰): آپ‌استریم پچ‌های توان‌عبوریِ ۱.۲.۳ را خودش جذب کرد —
# تفکیک بافر rx/tx نتستک، پنجره‌های HTTP/2 و دسته‌بندی کپسول‌ها. پس دو فایل از
# این فهرست بیرون رفتند:
#   * masque_h2.rs — آپ‌استریم مسیر ارسال را با تسک pump_outbound از نو نوشت.
#   * (سایر پچ‌ها ماندند و حالا با نشانهٔ AETHER-APP-PATCH علامت‌دار هستند.)
# چیزی که آپ‌استریم جذب نکرد و پچش سرِ جایش است: انتخاب CUBIC در smoltcp،
# سقفِ صفِ تحویل بسته در lib.rs و تفکیک SO_RCVBUF/SO_SNDBUF در sysprofile.rs.
'''
text = text.replace('PATCHED_FILES=(', note + 'PATCHED_FILES=(', 1)
sc.write_text(text)

# --- نشانهٔ پچِ CUBIC در netstack.rs -------------------------------------
# جفتِ ویژگیِ socket-tcp-cubic در Cargo.toml. بدون نشانه، merge بعدی نمی‌فهمد
# این خط مالِ برنامه است و نه آپ‌استریم.
ns = DST / 'aether/src/netstack.rs'
t = ns.read_text()
needle = '            socket.set_congestion_control(tcp::CongestionControl::Cubic);'
if needle in t and 'AETHER-APP-PATCH netstack-congestion-control' not in t:
    t = t.replace(needle,
        '            // >>> AETHER-APP-PATCH netstack-congestion-control\n'
        '            // ویژگی socket-tcp-cubic در Cargo.toml جفتِ همین خط است؛\n'
        '            // بی آن، AnyController::new() به NoControl می‌رسد.\n'
        + needle + '\n'
        '            // <<< AETHER-APP-PATCH netstack-congestion-control', 1)
    ns.write_text(t)
    print('netstack.rs: نشانهٔ پچ CUBIC گذاشته شد')
else:
    print('!! خطِ set_congestion_control پیدا نشد یا از قبل نشانه‌دار بود — بررسی شود')

print('CORE_VERSION:', (ROOT / 'CORE_VERSION').read_text().strip(),
      '| vendored:', (DST / 'CORE_VERSION').read_text().strip())
print('baseline files:', sum(1 for _ in (DST / '.upstream-baseline').rglob('*') if _.is_file()))
