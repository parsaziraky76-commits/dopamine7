#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Hält den No-Headway-Wächter des Tor-Bootstraps fest.

Die Kern-Crate braucht arti und kompiliert nur im Windows-Build, also hat
`tor.rs` sonst überhaupt kein Gate vor CI. Genau dort saß der Fehler, der im
Log vom 2026-09-16 fünf Minuten stumme Stille auf 15 % erzeugte: der Wächter
maß Fortschritt an `as_frac()` mal 1000, also an jedem Bruchteil. Ein Consensus,
der nur teilweise ankommt ("Partial response"), hebt diesen Bruch endlos um
Kleinstbeträge und setzt den Wächter jedes Mal zurück, während die angezeigte
Stufe unverändert bleibt. Die Sprosse fiel nie, und Sprosse 2 und snowflake
kamen nie an die Reihe.

Zwei Invarianten, beide als Text geprüft:
  1. Fortschritt wird in GANZEN Prozent gemessen (`* 100.0` und `floor`).
  2. Es gibt eine absolute Obergrenze pro Sprosse (HARD_STALL_FACTOR), die
     unabhängig vom Fortschrittssignal zuschlägt.
"""
import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "native/aether/aether/src/tor.rs"


def ohne_kommentare(quelle: str) -> str:
    """Zeilenkommentare weg, bevor geprüft wird.

    Sonst löst die Begründung des Fixes ihn selbst aus: im Kommentar steht,
    dass dort früher `* 1000.0` stand, und der Wächter würde genau darauf
    anschlagen.
    """
    zeilen = []
    for zeile in quelle.splitlines():
        gestrichen = re.sub(r"//.*$", "", zeile)
        zeilen.append(gestrichen)
    return "\n".join(zeilen)


def main() -> int:
    quelle = ohne_kommentare(SRC.read_text(encoding="utf-8"))
    fehler = []

    if "* 1000.0" in quelle:
        fehler.append(
            "Fortschritt wird wieder in Promille gemessen — jedes Bruchstück eines "
            "Consensus zählt dann als Fortschritt und der Wächter fällt nie"
        )
    if not re.search(r"as_frac\(\)[^;]*\*\s*100\.0[^;]*floor\(\)", quelle, re.S):
        fehler.append(
            "kein `as_frac() * 100.0 … floor()` gefunden — Fortschritt muss ein "
            "ganzer Prozentpunkt sein"
        )

    faktor = re.search(r"const HARD_STALL_FACTOR: u32 = (\d+);", quelle)
    if not faktor:
        fehler.append("HARD_STALL_FACTOR fehlt — eine Sprosse hätte keine harte Obergrenze")
    elif int(faktor.group(1)) < 2:
        fehler.append(f"HARD_STALL_FACTOR={faktor.group(1)} ist zu knapp (≥ 2 erwartet)")

    if "saturating_mul(HARD_STALL_FACTOR)" not in quelle:
        fehler.append("HARD_STALL_FACTOR ist definiert, wird aber nicht angewandt")

    if fehler:
        print("✗ Tor-Wächter:")
        for f in fehler:
            print(f"   - {f}")
        return 1

    print(
        f"✓ Tor-Wächter: Fortschritt in ganzen Prozent, harte Obergrenze "
        f"{faktor.group(1)}× des Stall-Limits"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
