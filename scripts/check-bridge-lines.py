#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Prüft die eingebauten Brückenzeilen und die Reihenfolge pro Land.

Warum als eigener Wächter und nicht nur als `#[test]` in `bridges.rs`:
Die Kern-Crate braucht arti und lässt sich weder in der Sandbox noch im
`npm test`-Schritt kompilieren, also würde der Rust-Test erst im
Windows-Build laufen — zu spät. Diese Prüfung liest den Quelltext als Text
und läuft überall in einer Sekunde.

Die Regeln stammen aus zwei Abrufen der Circumvention-API am 2026-09-16
(beide HTTP 200):

  POST https://bridges.torproject.org/moat/circumvention/builtin
  GET  https://bridges.torproject.org/moat/circumvention/map

1. Jede Zeile braucht eine Identität (40 Hex-Zeichen, mit oder ohne
   `fingerprint=`/`$`). arti verlangt sie pro Brücke, C-tor nicht — die von
   Tor Browser übernommene meek_lite-Zeile hat auch heute keine und wurde im
   Log vom 2026-09-16 nach zwei Millisekunden verworfen: "none of the bridges
   could be read by tor".

2. snowflake braucht Domain-Fronting (`front=` ODER `fronts=` — die API
   benutzt beide Formen), eine Broker-URL und mindestens drei ICE-Server.
   NICHT geprüft wird ein ICE-Server jenseits von 3478: die offiziellen
   Zeilen für `ir` haben neun Server, alle auf 3478.

3. Die Landestabelle muss die Länder enthalten, um die es geht, und für `ir`
   muss snowflake vor obfs4 stehen — so steht es in der map.
"""
import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "native/aether/aether/src/bridges.rs"

HEX40 = re.compile(r"^[0-9A-Fa-f]{40}$")


def hat_identitaet(zeile: str) -> bool:
    for wort in zeile.split():
        kern = wort
        if kern.startswith("fingerprint="):
            kern = kern[len("fingerprint="):]
        kern = kern.lstrip("$")
        if HEX40.match(kern) or wort.startswith("ed25519:"):
            return True
    return False


def eingebaute_zeilen(quelle: str) -> list[str]:
    start = quelle.find("const EMBEDDED: &[&str] = &[")
    if start < 0:
        sys.exit("✗ EMBEDDED nicht gefunden — wurde bridges.rs umgebaut?")
    ende = quelle.find("\n];", start)
    if ende < 0:
        sys.exit("✗ Ende der EMBEDDED-Liste nicht gefunden")
    return re.findall(r'^\s*"(.+?)",\s*$', quelle[start:ende], re.MULTILINE)


def landestabelle(quelle: str) -> dict[str, list[str]]:
    start = quelle.find("const BY_COUNTRY:")
    if start < 0:
        return {}
    ende = quelle.find("\n];", start)
    tabelle = {}
    for land, order in re.findall(r'\("(\w\w)",\s*&\[(.*?)\]\)', quelle[start:ende], re.S):
        tabelle[land] = re.findall(r'"(\w+)"', order)
    return tabelle


def main() -> int:
    quelle = SRC.read_text(encoding="utf-8")
    zeilen = eingebaute_zeilen(quelle)
    fehler: list[str] = []

    if not zeilen:
        fehler.append("die EMBEDDED-Liste ist leer")

    for zeile in zeilen:
        kurz = zeile[:60]
        if not hat_identitaet(zeile):
            fehler.append(f"ohne Identität, arti kann sie nicht lesen: {kurz}…")
        if zeile.split()[0] != "snowflake":
            continue
        if "front=" not in zeile and "fronts=" not in zeile:
            fehler.append(f"snowflake ohne Domain-Fronting: {kurz}…")
        if "url=https://" not in zeile:
            fehler.append(f"snowflake ohne Broker-URL: {kurz}…")
        ice = re.search(r"ice=(\S+)", zeile)
        if not ice:
            fehler.append(f"snowflake ohne ice=: {kurz}…")
        elif len(ice.group(1).split(",")) < 3:
            fehler.append(f"snowflake mit weniger als drei ICE-Servern: {kurz}…")

    tabelle = landestabelle(quelle)
    for land in ("ir", "ru", "cn"):
        if land not in tabelle:
            fehler.append(f"BY_COUNTRY hat keinen Eintrag für '{land}'")
    if "ir" in tabelle:
        order = tabelle["ir"]
        if "snowflake" not in order or "obfs4" not in order:
            fehler.append("ir: snowflake und obfs4 müssen beide in der Reihenfolge stehen")
        elif order.index("snowflake") > order.index("obfs4"):
            fehler.append(
                "ir: obfs4 steht vor snowflake — die map sagt das Gegenteil, und im Log "
                "vom 2026-09-16 hat obfs4 dort das ganze Budget verbraucht"
            )

    if fehler:
        print("✗ Brückenzeilen:")
        for f in fehler:
            print(f"   - {f}")
        return 1

    arten: dict[str, int] = {}
    for zeile in zeilen:
        arten[zeile.split()[0]] = arten.get(zeile.split()[0], 0) + 1
    zusammen = ", ".join(f"{n}× {a}" for a, n in sorted(arten.items()))
    print(
        f"✓ {len(zeilen)} eingebaute Brückenzeile(n) sind für arti brauchbar ({zusammen}); "
        f"Landesreihenfolge für {len(tabelle)} Länder, ir = {' → '.join(tabelle.get('ir', []))}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
