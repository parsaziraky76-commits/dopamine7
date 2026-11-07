#!/usr/bin/env python3
"""ارتقای هستهٔ دسکتاپ از 1.8.0 به 1.9.0 با merge سه‌طرفهٔ واقعی.

چرا این‌طور و نه یک کپیِ ساده از هستهٔ ۱.۹.۰ موبایل:

  * هستهٔ موبایل، آپ‌استریمِ ۱.۹.۰ **به‌علاوهٔ پچ‌های خودِ موبایل** است.
  * هستهٔ دسکتاپ، آپ‌استریمِ ۱.۸.۰ به‌علاوهٔ پچ‌های خودِ دسکتاپ است — که در دو
    فایل (`dns.rs` و `masque_h2.rs`) هیچ معادلی در موبایل ندارند.
  * پس کپی کردن، ۱۹۶۳ خط کار دسکتاپ را بی‌صدا پاک می‌کرد؛ همان چیزی که
    خودِ `sync-core.sh` با `PATCHED_FILES` جلویش را گرفته بود.

پایهٔ merge، نسخهٔ **بکرِ** آپ‌استریم است و نه فایلِ پچ‌خوردهٔ موبایل:
  base   = native/aether/.upstream-baseline/<f>            (بکرِ ۱.۸.۰)
  ours   = native/aether/<f>                               (دسکتاپِ پچ‌خورده)
  theirs = بکرِ ۱.۹.۰ (baseline موبایل، وگرنه خودِ فایل موبایل)
"""
import pathlib, subprocess, sys, shutil

D = pathlib.Path('native/aether')
M = pathlib.Path('/tmp/mob/Aether-main/native/aether')
OUT = pathlib.Path('/tmp/core19')

PATCHED = sorted(
    str(p.relative_to(D / '.upstream-baseline'))
    for p in (D / '.upstream-baseline').rglob('*') if p.is_file()
)

def pristine19(rel: str) -> pathlib.Path | None:
    """بکرِ ۱.۹.۰ برای یک مسیر: اگر موبایل پچش کرده، baseline‌اش؛ وگرنه خودش."""
    b = M / '.upstream-baseline' / rel
    if b.exists():
        return b
    f = M / rel
    return f if f.exists() else None

def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    # نقطهٔ شروع: کل درختِ ۱.۹.۰ موبایل، بعد هر فایلِ پچ‌شده با نسخهٔ merge‌شده
    # جا عوض می‌کند و هر فایلی که فقط پچِ موبایل است حذف می‌شود.
    shutil.copytree(M, OUT, symlinks=True)
    shutil.rmtree(OUT / '.upstream-baseline', ignore_errors=True)
    shutil.rmtree(OUT / '.git', ignore_errors=True)

    # پچ‌های فقط-موبایل که آپ‌استریم نیستند: باید کنار گذاشته شوند تا تصمیمِ
    # پذیرش‌شان جداگانه و آگاهانه گرفته شود.
    mobile_only = []
    for rel in ('aether/build.rs',):
        p = OUT / rel
        if p.exists():
            p.unlink()
            mobile_only.append(rel)

    conflicts = {}
    for rel in PATCHED:
        base, ours = D / '.upstream-baseline' / rel, D / rel
        theirs = pristine19(rel)
        if theirs is None:
            print(f'!! {rel}: در ۱.۹.۰ پیدا نشد — دستی بررسی شود')
            continue
        # git merge-file همان merge سه‌طرفهٔ diff3 را می‌دهد و نشانگرِ تعارض
        # می‌گذارد؛ خروجی روی stdout می‌آید تا فایل اصلی دست‌نخورده بماند.
        res = subprocess.run(
            ['git', 'merge-file', '-p', '--diff3',
             '-L', 'desktop-1.8.0-patched', '-L', 'upstream-1.8.0', '-L', 'upstream-1.9.0',
             str(ours), str(base), str(theirs)],
            capture_output=True, text=True)
        merged = res.stdout
        n = merged.count('<<<<<<<')
        (OUT / rel).write_text(merged)
        status = 'OK' if n == 0 else f'{n} CONFLICT'
        conflicts[rel] = n
        print(f'{rel:<26} {status}')

    # baseline تازه: بکرِ ۱.۹.۰ همان فایل‌ها، تا sync بعدی پایهٔ درست داشته باشد.
    nb = OUT / '.upstream-baseline'
    for rel in PATCHED:
        src = pristine19(rel)
        if src is None:
            continue
        dst = nb / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)

    print('\nmobile-only patches set aside:', mobile_only)
    total = sum(conflicts.values())
    print('total conflicts:', total)
    return 0 if total == 0 else 2

sys.exit(main())
