#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Struktur-Wächter für .github/workflows/build.yml.

Grund für diese Datei — Lauf 94978476672:
Der Schritt »Unit tests« wurde vor die Engine-Builds gezogen, damit ein
kaputter Test in drei statt in vierunddreißig Minuten auffällt. Beide
Build-Jobs starben daraufhin an

    resource path `..\\dist-engine\\server_entries.txt` doesn't exist

Jeder cargo-Aufruf in src-tauri führt build.rs aus, und tauri_build::build()
prüft jeden Pfad aus bundle.resources. Diese Pfade liegen unter dist-engine
und entstehen erst in den Engine-/Wintun-/Psiphon-Schritten. Fail-fast und
diese Prüfung stehen also im Widerspruch — auflösbar nur, indem der frühe
Test-Schritt bundle.resources per TAURI_CONFIG leert.

Der Wächter prüft beides zusammen:
  1. Läuft »Unit tests« vor dem Engine-Build, MUSS der Schritt TAURI_CONFIG
     mit leerem resources setzen.
  2. Der leere Wert muss ein ARRAY sein. `{}` wäre wirkungslos: der
     Config-Merge geht bei Objekten Schlüssel für Schlüssel, ein leeres
     Objekt löscht nichts. Genau darauf ist der erste Reparaturversuch
     hereingefallen.
  3. Der ECHTE Build-Schritt (tauri build) darf den Patch NICHT tragen —
     sonst würde ein Installer ohne Engine-Dateien gebaut, und das wäre
     schlimmer als jeder gescheiterte Lauf.
"""
import json
import sys

import yaml

WF = ".github/workflows/build.yml"


def fail(msg):
    print(f"::error::{msg}")
    sys.exit(1)


with open(WF, encoding="utf-8") as fh:
    wf = yaml.safe_load(fh)

for job_name, job in (wf.get("jobs") or {}).items():
    steps = job.get("steps") or []
    names = [(s.get("name") or s.get("uses") or "") for s in steps]

    def index_of(needle):
        for i, n in enumerate(names):
            if needle.lower() in n.lower():
                return i
        return None

    i_test = index_of("Unit tests")
    if i_test is None:
        continue

    # Die Schritte, die dist-engine ERST ANLEGEN, werden über die Skripte
    # erkannt, die sie aufrufen — nicht über ihre Namen. Ein Namensvergleich auf
    # "engine" traf hier den Testschritt selbst (»Unit tests (engine contract
    # parity …)«), womit der Wächter sich an sich selbst maß und alles durchwinkte.
    producers = ("build-engine.ps1", "fetch-wintun.ps1", "build-psiphon.ps1")
    i_engine = None
    for i, s in enumerate(steps):
        run = s.get("run") or ""
        if any(p in run for p in producers):
            i_engine = i
            break
    if i_engine is None:
        fail(
            f"{job_name}: none of the steps calls {', '.join(producers)}, so this guard cannot tell "
            "where dist-engine starts to exist. Either the scripts were renamed — then update this "
            "list — or the build no longer produces the engine at all."
        )

    step = steps[i_test]
    env = step.get("env") or {}
    raw = env.get("TAURI_CONFIG")

    if i_engine is not None and i_test < i_engine:
        if not raw:
            fail(
                f"{job_name}: »{names[i_test]}« runs before the engine build but does not set "
                "TAURI_CONFIG. build.rs will check bundle.resources, the dist-engine files do not "
                "exist yet, and both build jobs die on 'resource path ... doesn't exist' — this is "
                "run 94978476672. Either set TAURI_CONFIG='{\"bundle\":{\"resources\":[]}}' on this "
                "step or move it back behind the Psiphon step."
            )
        try:
            patch = json.loads(raw)
        except json.JSONDecodeError as exc:
            fail(f"{job_name}: TAURI_CONFIG on »{names[i_test]}« is not valid JSON: {exc}")
        res = (patch.get("bundle") or {}).get("resources")
        if not isinstance(res, list):
            fail(
                f"{job_name}: TAURI_CONFIG on »{names[i_test]}« must set bundle.resources to an "
                f"empty ARRAY, got {type(res).__name__}. An empty object is merged key by key and "
                "removes nothing, so the resource check still fires."
            )
        if res:
            fail(f"{job_name}: bundle.resources in TAURI_CONFIG must be empty, got {res}.")

    i_build = index_of("Build Aether (Tauri")
    if i_build is not None:
        benv = (steps[i_build].get("env") or {})
        if "TAURI_CONFIG" in benv:
            fail(
                f"{job_name}: the real build step »{names[i_build]}« carries TAURI_CONFIG. If that "
                "empties bundle.resources, the installer ships without engine, Wintun and Psiphon. "
                "The patch belongs on the test step only."
            )

# ── Budget ───────────────────────────────────────────────────────────────────
# Lauf 95128606820: alle Bau-Schritte grün, danach startete »Publish GitHub
# Release« nicht — GitHub blockierte den Job wegen Abrechnung/Spending-Limit.
# Das ist keine Code-Ursache und lässt sich hier nicht reparieren. Was sich
# reparieren lässt, ist der Verbrauch, der das Limit erreicht hat: Windows-
# Minuten zählen im privaten Repo doppelt, und ohne Zeitlimit kann ein
# hängender Job bis zu 6 Stunden davon verbrennen.
for job_name, job in (wf.get("jobs") or {}).items():
    grenze = job.get("timeout-minutes")
    if not grenze:
        fail(
            f"{job_name}: kein timeout-minutes. Ein hängender Job läuft in GitHubs Standard bis zu "
            "6 Stunden und zieht auf windows-latest die doppelte Minutenzahl vom Kontingent ab — "
            "genau das Kontingent, dessen Ende Lauf 95128606820 gestoppt hat."
        )
    if job.get("runs-on") == "windows-latest" and grenze > 90:
        fail(f"{job_name}: timeout-minutes={grenze} auf Windows ist mehr als 3 Stunden Kontingent.")

# A successful build must never wait on post-job cache pruning / upload.
# GitHub Actions runs post steps after the last visible build step, so a job
# may look successful yet spend tens of minutes at Swatinem/rust-cache@v2.
# Read-only cache restore is allowed: it has no save hook on cache miss.
b = (wf.get("jobs") or {}).get("build") or {}
schritte = b.get("steps") or []
for step in schritte:
    uses = step.get("uses") or ""
    if "rust-cache@" in uses or ("actions/cache@" in uses and "/restore@" not in uses):
        fail(f"build: {uses} registers a post-job cache save; use restore-only or omit it")
cache = [x for x in schritte if "actions/cache/restore@" in (x.get("uses") or "")
         and "go" in str(x.get("with", {}).get("key", ""))]
if not cache:
    fail("build: missing version-pinned, restore-only Go cache step")
schluessel = str(cache[0].get("with", {}).get("key", ""))
for pin in ("PSIPHON_VERSION", "LYREBIRD_VERSION"):
    if pin not in schluessel:
        fail(f"build: restore-only Go cache key must include {pin}")

trigger = (wf.get(True) or wf.get("on") or {}).get("push") or {}
if "**/*.md" not in (trigger.get("paths-ignore") or []):
    fail(
        "push: kein paths-ignore für **/*.md. Ein Push, der nur Text ändert, startet dann beide "
        "Windows-Jobs — rund 80 Kontingent-Minuten für eine geänderte Zeile Dokumentation."
    )

# Und die Artefakte dürfen den Speicher nicht vollaufen lassen: GitHub Free
# enthält 500 MB Artefakt-Speicher, geteilt mit Packages, und die Standard-
# Aufbewahrung sind 90 Tage. Läuft dieser Speicher über, blockiert dieselbe
# Abrechnungsmeldung den nächsten Job, der einen Runner anfordert.
for job_name, job in (wf.get("jobs") or {}).items():
    for st in (job.get("steps") or []):
        if "upload-artifact" not in (st.get("uses") or ""):
            continue
        tage = (st.get("with") or {}).get("retention-days")
        wie = (st.get("with") or {}).get("name")
        if not tage:
            fail(
                f"{job_name}: der Upload »{wie}« setzt kein retention-days und behält die "
                "Installer damit 90 Tage. Auf 500 MB eingeschlossenem Speicher ist das nach "
                "wenigen Läufen voll."
            )
        if int(tage) > 14:
            fail(f"{job_name}: retention-days={tage} für »{wie}« ist zu lang für 500 MB.")

print("  ✓ workflow: fail-fast test step and resource patch are consistent")
print("  ✓ workflow: jedes Job hat ein Zeitlimit; kein Post-Job-Cache-Save, Go-Cache nur lesend")
