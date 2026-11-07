#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Prüft, dass das offizielle Tor für JEDE gebaute Architektur beschafft wird.

Der Anlass ist Run 95081207888 vom 2026-09-16: x64 lief komplett durch, x86
starb an meinem eigenen Widerspruch. `stage-tor.ps1` sprang mit der Begründung
heraus, Tor veröffentliche für x86 kein Bundle — was falsch ist, es gibt
`tor-expert-bundle-windows-i686-<version>.tar.gz` — und das Publish-Gate im
Workflow verlangte die Binärdatei trotzdem. Eine Hälfte übersprang also, was
die andere Hälfte zur Bedingung machte.

Beide Digests stammen aus Tors eigener `sha256sums-signed-build.txt` und sind
zusätzlich gegen einen unabhängigen Download geprüft:

    x86_64  231dad6b9cb401a54c260db7046965ef04e4f72ff071b140d423fb5da281ab1e
    i686    1e4de9a4f1d99b8f40b5e0c75f3dcc3ea51b0aeab040d48fd23881e9fa94979a
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STAGE = ROOT / "scripts/stage-tor.ps1"
WORKFLOW = ROOT / ".github/workflows/build.yml"

# Architektur → (Bezeichner im Dateinamen bei Tor, gepinnter Digest)
ERWARTET = {
    "x64": ("x86_64", "231dad6b9cb401a54c260db7046965ef04e4f72ff071b140d423fb5da281ab1e"),
    "x86": ("i686", "1e4de9a4f1d99b8f40b5e0c75f3dcc3ea51b0aeab040d48fd23881e9fa94979a"),
}


def main() -> int:
    fehler = []
    stage = STAGE.read_text(encoding="utf-8")
    workflow = WORKFLOW.read_text(encoding="utf-8")

    for arch, (slug, digest) in ERWARTET.items():
        if digest not in stage:
            fehler.append(f"{arch}: der gepinnte Digest für {slug} fehlt in stage-tor.ps1")
        if slug not in stage:
            fehler.append(f"{arch}: der Bezeichner {slug} kommt in stage-tor.ps1 nicht vor")

    matrix = re.findall(r"^\s*-\s*arch:\s*(\S+)\s*$", workflow, re.M)
    if not matrix:
        fehler.append("in build.yml ist keine Architektur-Matrix zu finden")
    for arch in matrix:
        if arch not in ERWARTET:
            fehler.append(
                f"der Workflow baut {arch}, aber für {arch} ist kein Tor-Bundle gepinnt"
            )
    if "stage-tor.ps1 -Arch '${{ matrix.arch }}'" not in workflow:
        fehler.append("der Workflow ruft stage-tor.ps1 nicht je Matrix-Architektur auf")
    # Und das Publish-Gate muss für dieselbe Matrix-Architektur greifen, sonst
    # verlangt es die Datei für eine Architektur, die niemand beschafft hat.
    if "dist/payload-${{ matrix.arch }}/engine/tor/tor.exe" not in workflow:
        fehler.append("das Publish-Gate prüft tor.exe nicht je Matrix-Architektur")

    # Ein pauschales Überspringen aller Nicht-x64-Architekturen ist genau der
    # Fehler von Run 95081207888 und darf nicht zurückkommen.
    if re.search(r"\$Arch\s+-ne\s+'x64'", stage):
        fehler.append(
            "stage-tor.ps1 überspringt pauschal alles außer x64 — Tor liefert auch i686"
        )

    # Beide Digests dürfen nicht identisch sein: das wäre ein Copy-Paste, das
    # erst beim Digest-Vergleich im Build auffällt.
    digests = {d for _, d in ERWARTET.values()}
    if len(digests) != len(ERWARTET):
        fehler.append("die gepinnten Digests sind nicht unterschiedlich")

    # درختِ *نصب‌شده* هم باید سنجیده شود، وگرنه نصب‌کننده‌ای بدونِ تور سبز
    # می‌گذرد و فقط روی کامپیوترِ کاربر لو می‌رود.
    smoke = (ROOT / "scripts/smoke-test.ps1").read_text(encoding="utf-8")
    for needle, was in (
        ("[switch]$RequireTor", "پارامترِ -RequireTor را نمی‌شناسد"),
        ("if ($RequireTor) {", "روی آن پارامتر شرط نمی‌گذارد"),
        ("engine\\tor\\tor.exe", "خودِ tor.exe را در فهرستِ سنجه ندارد"),
        ("engine\\tor\\pt_config.json", "پیکربندیِ پل‌ها را نمی‌سنجد"),
    ):
        if needle not in smoke:
            fehler.append(f"اسموک‌تست {was}")
    if "-RequireTor" not in workflow or "github.event_name != 'pull_request' && '-RequireTor'" not in workflow:
        fehler.append("ورک‌فلو، -RequireTor را با شرطِ انتشار به اسموک‌تست نمی‌دهد")

    if fehler:
        print("✗ Tor-Beschaffung:")
        for f in fehler:
            print(f"   - {f}")
        return 1

    print(
        "✓ offizielles Tor für beide Architekturen: x64 → x86_64, x86 → i686, "
        "je eigener gepinnter Digest, Gate und Beschaffung passen zusammen"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
