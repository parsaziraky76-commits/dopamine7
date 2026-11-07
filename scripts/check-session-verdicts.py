#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Wächter für die fünf Ursachen aus dem Feldlog vom 2026-09-16, 20:04.

Jede Prüfung hier hat eine Zeile in jenem Log, die sie ausgelöst hat. Wer eine
davon beim nächsten Umbau still entfernt, bekommt genau den alten Fehler zurück
— deshalb steht die Log-Zeile jeweils daneben.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
S = ROOT / "src-tauri/src"


def code(name: str) -> str:
    """Datei ohne Kommentarzeilen: eine Erklärung ist keine Verdrahtung."""
    return "\n".join(
        line for line in (S / name).read_text(encoding="utf-8").split("\n")
        if not line.strip().startswith("//")
    )


def main() -> int:
    fehler = []
    tor = code("tor_native.rs")
    state = code("state.rs")
    guard = code("leakguard.rs")
    diag = code("diagnostics.rs")
    tun = code("tun.rs")

    # 1) „Engine exited before it opened the SOCKS5 port" 200 ms nach dem Start:
    #    Anlaufen ist nicht Totsein.
    if "PHASE_STARTING" not in tor or "fn mark_starting(" not in tor:
        fehler.append("tor_native kennt keine Anlaufphase; der erste Tick killt die Sitzung wieder")
    else:
        m = re.search(r"fn is_alive\(&self\) -> bool \{(.*?)\n    \}", tor, re.S)
        if not m or "PHASE_STARTING => true" not in m.group(1):
            fehler.append("is_alive zählt die Anlaufphase nicht als lebendig")
    # Und die Phase muss VOR dem Thread gesetzt werden, sonst ist das Rennen nur verschoben.
    a, b = state.find("tor.mark_starting()"), state.find('.name("tor-native"')
    if a < 0:
        fehler.append("state.rs markiert den Anlauf nicht (mark_starting fehlt)")
    elif b >= 0 and a > b:
        fehler.append("mark_starting kommt erst nach dem Thread — dasselbe Rennen, nur später")
    if "tor.mark_dead()" not in state:
        fehler.append("ein gescheiterter Start meldet sich nicht (mark_dead fehlt)")

    # 2) „Connection refused: WebRTC can still reach the real IP" nach 100 %:
    #    Das Urteil muss die Reichweite der Sperre kennen, die wir selbst gewählt haben.
    if "udp_browser_scoped" not in guard:
        fehler.append("leakguard meldet die Reichweite der UDP-Sperre nicht")
    if "udp_browser_scoped" not in diag:
        fehler.append(
            "das Leak-Urteil liest die Reichweite nicht — eine Tor-Sitzung wird wieder "
            "abgelehnt, obwohl sie den Selbsttest bestanden hat"
        )
    else:
        m = re.search(r"let leaking = (.*?);", diag, re.S)
        if not m or "udp_open_by_design" not in m.group(1):
            fehler.append("das Urteil rechnet die browser-begrenzte Sperre nicht ein")
        if "guard.udp_browser_scoped && guard.browser_policies > 0" not in diag:
            fehler.append(
                "die Ausnahme hängt nicht an einer aktiven Browser-Policy; ohne diese "
                "Bedingung gilt sie auch ohne jeden Schutz (fail-open)"
            )

    # 3) Fresh installs default to Aether, but a saved backend must survive
    #    restarts and older-settings migrations (regression reported in 1.3.0).
    profile = code("profile.rs")
    store = code("store.rs")
    if 'backend: TransportBackend::Aether' not in profile:
        fehler.append("ein neues Profil beginnt nicht mehr mit Aether")
    if 'changed = migrate(&mut profile)' not in store:
        fehler.append("die Migration fehlt beim Laden des Profils")
    if re.search(r"profile\.backend\s*=\s*TransportBackend::Aether", store):
        fehler.append("gespeicherter Backend-Wunsch wird beim Laden wieder überschrieben")
    if 'load_preserves_saved_backend_across_launches' not in store or 'migration_preserves_legacy_backend' not in store:
        fehler.append("Regressionstests für gespeicherte Backends fehlen")

    # 4) 1,96 s Trennen: die Registry-Wiederherstellung darf nicht seriell sein.
    if "fn restore_policies(" not in guard or "fn reg_spawn(" not in guard:
        fehler.append("die Policies werden wieder seriell zurückgesetzt (~1,9 s beim Trennen)")
    else:
        m = re.search(r"fn restore_policies\(.*?\n\}", guard, re.S)
        if m and "kid.wait()" not in m.group(0):
            fehler.append("restore_policies wartet nicht auf seine Prozesse — die Registry ist beim Rückkehren evtl. noch nicht zurück")
    # und der doppelte Teardown-Log
    if 'if had_session {' not in tun:
        fehler.append("tun.rs meldet den Abbau wieder unbedingt (die Zeile kam doppelt)")

    # 5) Der Fingerprint entscheidet bei „Tor allein" nichts und kostete ~2,4 s.
    if "let fingerprint = kind == Prep::Plan && !tor_only" not in state:
        fehler.append("der Netz-Fingerprint läuft wieder auch bei Tor allein")

    if fehler:
        print("✗ Sitzungslogik (Log 2026-09-16 20:04):")
        for f in fehler:
            print(f"   - {f}")
        return 1

    print(
        "✓ Sitzungslogik: Anlauf zählt als lebendig, Leak-Urteil kennt die Sperr-Reichweite "
        "(fail-closed ohne Schutz), gespeichertes Backend bleibt erhalten, Policy-Rückgabe parallel, "
        "Fingerprint bei Tor allein übersprungen"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
