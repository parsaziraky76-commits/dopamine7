#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Prüft, dass jede .rs-Datei überhaupt noch parsebar ist.

# Warum es diesen Wächter gibt

Am 2026-09-17 fielen `Build x64` und `Build x86` beide beim allerersten
cargo-Schritt:

```text
error: unexpected closing delimiter: `}`
    --> src\\tor_native.rs:1340:1
Error writing files: failed to resolve mod `tor_native`
```

Ursache war eine einzige Zeile, in der ein Textpatch den Doc-Kommentar und die
Signatur der nächsten Funktion zusammengeklebt hatte:

```rust
/// … نه فرض‌شده.    fn socks_port(&mut self, budget: Duration) -> … {
```

Damit lag `fn socks_port` IM Kommentar, seine öffnende Klammer fehlte, und die
Datei war unbalanciert. Der Preflight war grün — jeder seiner Wächter arbeitet
mit `grep` und Regex, und für die ist eine unbalancierte Datei völlig in
Ordnung. Gemerkt hat es erst der Compiler, sieben Minuten später auf einem
Windows-Runner.

Dieser Wächter braucht keine Toolchain und läuft deshalb auch dort, wo `cargo`
fehlt. Er beweist nicht, dass der Code *korrekt* ist — nur, dass er nicht an
einer zerschnittenen Zeile scheitert. Wo `cargo` da ist, bleibt `cargo check`
die Wahrheit.
"""
import sys
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
BAEUME = ["src-tauri/src", "native/aether/aether/src"]

PAARE = {"}": "{", ")": "(", "]": "["}
OEFFNER = set(PAARE.values())

# Ein Kommentar, hinter dem auf derselben Zeile wieder Code beginnt. Genau das
# war der Schaden vom 17.9., und die Klammer-Bilanz erwischt nur die Fälle, in
# denen dabei auch eine Klammer verloren geht.
VERDACHT = ("fn ", "pub fn ", "impl ", "struct ", "enum ", "let ", "match ", "if ")


def zerlege(text: str):
    """Liefert (code, zeilennummern) — Strings und Kommentare entfernt.

    Kein echter Lexer, aber alles, was in Rust einen `{` verstecken kann:
    Zeilen- und (verschachtelte) Blockkommentare, normale Strings mit
    Escapes, rohe Strings mit beliebig vielen `#`, und Zeichenliteralen.
    Lifetimes (`'a`) sind keine Zeichenliterale und dürfen nicht als solche
    gelesen werden, sonst frisst der Parser den halben Rest der Datei.
    """
    code = []
    zeilen = []
    zeile = 1
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "\n":
            zeile += 1
            i += 1
            continue
        # Zeilenkommentar
        if text.startswith("//", i):
            ende = text.find("\n", i)
            i = n if ende < 0 else ende
            continue
        # Blockkommentar, in Rust verschachtelbar
        if text.startswith("/*", i):
            tiefe = 1
            i += 2
            while i < n and tiefe:
                if text.startswith("/*", i):
                    tiefe += 1
                    i += 2
                elif text.startswith("*/", i):
                    tiefe -= 1
                    i += 2
                else:
                    if text[i] == "\n":
                        zeile += 1
                    i += 1
            continue
        # roher String: r"…", r#"…"#, br##"…"##
        if c in "rb" and (roh := _roher_string(text, i)) is not None:
            neu_i, uebersprungen = roh
            zeile += uebersprungen
            i = neu_i
            continue
        # normaler String
        if c == '"':
            i += 1
            while i < n:
                if text[i] == "\\":
                    # `"…\<Zeilenumbruch>` — der Umbruch zählt trotzdem als Zeile
                    if i + 1 < n and text[i + 1] == "\n":
                        zeile += 1
                    i += 2
                    continue
                if text[i] == '"':
                    i += 1
                    break
                if text[i] == "\n":
                    zeile += 1
                i += 1
            continue
        # Zeichenliteral — aber nicht die Lifetime `'a`
        if c == "'":
            if text.startswith("'\\", i):
                ende = text.find("'", i + 2)
                i = n if ende < 0 else ende + 1
                continue
            if i + 2 < n and text[i + 2] == "'":
                i += 3
                continue
            i += 1  # Lifetime: das Apostroph ist bedeutungslos
            continue
        code.append(c)
        zeilen.append(zeile)
        i += 1
    return "".join(code), zeilen


def _roher_string(text: str, i: int):
    """`r"…"`, `r#"…"#`, `br##"…"##` → (neue Position, verschluckte Zeilen)."""
    j = i
    if text[j] == "b":
        j += 1
    if j >= len(text) or text[j] != "r":
        return None
    j += 1
    rauten = 0
    while j < len(text) and text[j] == "#":
        rauten += 1
        j += 1
    if j >= len(text) or text[j] != '"':
        return None
    schluss = '"' + "#" * rauten
    ende = text.find(schluss, j + 1)
    if ende < 0:
        return len(text), text.count("\n", i)
    return ende + len(schluss), text.count("\n", i, ende)



# >>> AETHER-APP-FIX top-level-stray-code
# Am 2026-10-05 fiel `Build x64` erneut beim ersten cargo-Schritt:
#
#   error: expected one of `!` or `::`, found `=`
#      --> src\\engine.rs:918:40
#   918 | AETHER_QUICK_RECONNECT_MAX_HANDSHAKE_MS=0 # Disable handshake timeout for Tor
#
# Eine Shell-Zeile war ans Dateiende geraten. Die Klammer-Bilanz bleibt dabei
# völlig ausgeglichen, deshalb war dieser Wächter grün. Jetzt wird zusätzlich
# geprüft, dass auf oberster Ebene (Klammertiefe 0) nur Items beginnen:
# Attribute, `pub`/`fn`/`use`/`mod`/… oder ein Makroaufruf `name!`.
ITEM_START = {
    "pub", "fn", "use", "mod", "struct", "enum", "union", "impl", "trait",
    "const", "static", "type", "extern", "unsafe", "async", "macro_rules",
    "crate", "self", "super", "auto", "default",
}


def oberste_ebene(code: str, zeilen):
    """Befunde für Text auf Klammertiefe 0, der kein Item beginnt."""
    import re

    befunde = []
    tiefe = 0
    erwartet = True      # wir stehen vor dem Beginn eines Items
    attribut = False     # das laufende Item ist ein `#[...]`
    i, n = 0, len(code)
    while i < n:
        c = code[i]
        if c.isspace():
            i += 1
            continue
        if tiefe == 0 and erwartet:
            if c == ";":
                i += 1
                continue
            if c == "#":
                attribut = True
                erwartet = False
                i += 1
                continue
            m = re.match(r"[A-Za-z_][A-Za-z0-9_]*", code[i:])
            if not m:
                befunde.append(
                    f"Zeile {zeilen[i]}: `{c}` auf oberster Ebene — dort darf nur ein Item beginnen"
                )
                return befunde
            wort = m.group(0)
            j = i + len(wort)
            # Makroaufruf: name! / pfad::name!
            k = j
            while code.startswith("::", k):
                m2 = re.match(r"::[A-Za-z_][A-Za-z0-9_]*", code[k:])
                if not m2:
                    break
                k += len(m2.group(0))
            while k < n and code[k].isspace():
                k += 1
            ist_makro = k < n and code[k] == "!"
            if wort not in ITEM_START and not ist_makro:
                befunde.append(
                    f"Zeile {zeilen[i]}: `{wort}` auf oberster Ebene ist kein Rust-Item "
                    f"(verirrte Zeile, z. B. Shell/Env-Syntax?) — cargo bricht hier ab"
                )
                return befunde
            erwartet = False
            attribut = False
            i = j
            continue
        if c in OEFFNER:
            tiefe += 1
        elif c in PAARE:
            tiefe -= 1
            if tiefe == 0:
                if attribut and c == "]":
                    erwartet = True
                    attribut = False
                elif c == "}" and not attribut:
                    erwartet = True
        elif c == ";" and tiefe == 0:
            erwartet = True
            attribut = False
        i += 1
    if not erwartet and not attribut and tiefe == 0:
        pass
    return befunde
# <<< AETHER-APP-FIX top-level-stray-code

def pruefe(pfad: Path):
    """Alle Befunde einer Datei — leere Liste heisst: parsebar."""
    text = pfad.read_text(encoding="utf-8", errors="replace")
    befunde = []
    code, zeilen = zerlege(text)
    stapel = []
    for zeichen, zeile in zip(code, zeilen):
        if zeichen in OEFFNER:
            stapel.append((zeichen, zeile))
        elif zeichen in PAARE:
            if not stapel:
                befunde.append(f"Zeile {zeile}: `{zeichen}` ohne öffnendes Gegenstück")
                break
            offen, offen_zeile = stapel.pop()
            if offen != PAARE[zeichen]:
                befunde.append(
                    f"Zeile {zeile}: `{zeichen}` schliesst `{offen}` aus Zeile {offen_zeile}"
                )
                break
    if stapel and not befunde:
        offen, offen_zeile = stapel[0]
        befunde.append(f"Zeile {offen_zeile}: `{offen}` wird nie geschlossen")
    if not befunde:
        befunde.extend(oberste_ebene(code, zeilen))

    # Code hinter einem Kommentar auf derselben Zeile.
    for nummer, roh in enumerate(text.split("\n"), 1):
        ohne_string = roh.split('"')[0]
        marke = ohne_string.find("//")
        if marke < 0:
            continue
        rest = ohne_string[marke + 2 :]
        for verdacht in VERDACHT:
            stelle = rest.find(verdacht)
            # Ein Wort mitten im Kommentar ist harmlos; Code ist es erst, wenn
            # danach auch die Klammer einer Signatur folgt.
            if stelle > 0 and "(" in rest[stelle:] and rest[stelle - 1] in " \t":
                if rest[:stelle].strip().endswith((".", "؟", ":", "—")):
                    befunde.append(
                        f"Zeile {nummer}: hinter dem Kommentar beginnt wieder Code "
                        f"(`{verdacht.strip()}`) — ein Textpatch hat zwei Zeilen verklebt"
                    )
                    break
    return befunde


def main() -> int:
    dateien = []
    for baum in BAEUME:
        wurzel = WURZEL / baum
        if not wurzel.exists():
            print(f"✗ fehlt: {baum}")
            return 1
        dateien.extend(sorted(wurzel.rglob("*.rs")))
    if not dateien:
        print("✗ keine .rs-Dateien gefunden")
        return 1

    kaputt = 0
    for pfad in dateien:
        for befund in pruefe(pfad):
            print(f"   - {pfad.relative_to(WURZEL)}: {befund}")
            kaputt += 1

    if kaputt:
        print(f"✗ {kaputt} Datei(en) sind nicht parsebar — cargo bricht darauf ab")
        return 1

    print(f"✓ alle {len(dateien)} .rs-Dateien sind balanciert, kein Code hinter einem Kommentar, keine verirrte Zeile auf oberster Ebene")
    return 0


if __name__ == "__main__":
    sys.exit(main())
