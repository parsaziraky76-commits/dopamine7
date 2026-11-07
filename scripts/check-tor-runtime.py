#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Prüft, dass eine gescheiterte Tor-Sprosse restlos verschwindet.

Belegt durch loge1.txt (2026-09-16, 13:11-13:16): fünf
`Memory quota tracking initialised max=4.30 GiB` auf einer Maschine mit
5865 MB RAM, drei gleichzeitige Consensus-Zustandsmaschinen, und 43 Sekunden
nach dem Ende der snowflake-Sprosse noch deren `Rejected 2/2 as down` mitten
in der obfs4-Sprosse — die sechs Brücken hat.

Vier Dinge müssen dafür zusammen stimmen, und jedes einzelne kann beim
Umbauen verloren gehen:

1. Der Client bekommt eine eigene Laufzeit (`TorClient::with_runtime`),
   nicht die des ganzen Prozesses (`TorClient::builder`).
2. Die Laufzeit gehört einem Wächter mit `Drop`, damit auch ein Abbruch von
   außen — `stage()` setzt ein `timeout` darüber — sie freigibt.
3. Freigegeben wird auf einem eigenen Thread. Eine Tokio-Laufzeit im
   async-Kontext fallen zu lassen, paniert.
4. Der Wächter wird VOR dem Client angelegt. Rust gibt in umgekehrter
   Reihenfolge frei, also fällt der Client zuerst und ist nie der Letzte,
   der die Laufzeit loslässt.
"""
import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "native/aether/aether/src/tor.rs"


def ohne_kommentare(text: str) -> str:
    """Kommentare weg, damit eine Erklärung nicht als Code zählt."""
    aus = []
    for zeile in text.split("\n"):
        kern = zeile.strip()
        if kern.startswith("//"):
            continue
        aus.append(zeile)
    return "\n".join(aus)


def main() -> int:
    roh = SRC.read_text(encoding="utf-8")
    code = ohne_kommentare(roh)
    fehler = []

    if "TorClient::with_runtime(" not in code:
        fehler.append(
            "der Client hängt an der Prozess-Laufzeit (TorClient::builder), "
            "also überlebt eine gescheiterte Sprosse ihren Abgang"
        )

    grave = re.search(r"impl Drop for OwnRuntime \{(.*?)\n    \}", code, re.S)
    if not grave:
        fehler.append("OwnRuntime hat kein Drop — ein Abbruch von außen gibt nichts frei")
    else:
        block = grave.group(1)
        if "thread::Builder" not in block and "thread::spawn" not in block:
            fehler.append(
                "OwnRuntime::drop gibt die Laufzeit nicht auf einem Thread frei — "
                "im async-Kontext paniert das"
            )

    stelle_grave = code.find("let grave = OwnRuntime::new()")
    stelle_client = code.find("let client = TorClient::with_runtime(")
    if stelle_grave < 0:
        fehler.append("boot() legt keinen OwnRuntime-Wächter an")
    elif stelle_client < 0:
        fehler.append("boot() baut den Client nicht auf der eigenen Laufzeit")
    elif stelle_grave > stelle_client:
        fehler.append(
            "der Wächter wird nach dem Client angelegt — dann fällt er zuerst, "
            "und die letzte Kopie der Laufzeit stirbt im async-Kontext"
        )

    if "still at {at}%" not in roh:
        fehler.append(
            "die Stall-Prüfung schweigt wieder — dann sagt Stille im Log nichts "
            "darüber, ob die Schleife noch läuft"
        )

    if fehler:
        print("✗ Tor-Sprossen-Laufzeit:")
        for f in fehler:
            print(f"   - {f}")
        return 1

    print(
        "✓ Tor-Sprosse: eigene Laufzeit, Freigabe per Drop auf eigenem Thread, "
        "Wächter vor dem Client, Stall-Prüfung protokolliert"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
