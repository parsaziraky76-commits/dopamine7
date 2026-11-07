#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Prüft, dass „Tor allein" wirklich über das offizielle tor.exe läuft.

Der Grund steht im Kopf von `src-tauri/src/tor_native.rs`: der arti-Pfad blieb
im Feldlog vom 2026-09-16 bei 15 % stehen, und WhiteAesther 1.9.4 — gleicher
Aether-Kern 2.0.0 — verbindet auf Windows, weil es das offizielle tor als Kind
startet und den Fortschritt am Control-Port abfragt.

Fünf Dinge müssen zusammen stimmen, und jedes einzelne ist eine Stelle, an der
diese Verdrahtung beim nächsten Umbau still verloren gehen kann:

1. `launch_candidate` fragt den nativen Pfad, BEVOR die Engine gestartet wird.
   Danach gefragt, liefe erst arti und dann tor auf demselben Port.
2. Der Zustandsautomat fragt `carrier_alive()`, nicht `engine.is_alive()` —
   sonst reißt der erste Tick eine Sitzung ab, in der absichtlich keine Engine
   läuft. Betroffen sind drei Stellen: Verbindungsaufbau, Selbsttest und der
   Wächter im verbundenen Zustand.
3. Der SOCKS-Port ist der Port der App (1819). Ein anderer wäre für System-Proxy,
   TUN und Freigabe unsichtbar.
4. Beim Abbauen wird tor gestoppt. Sonst bleibt ein Kind mit einem Tunnel übrig,
   in den nichts mehr geroutet wird.
5. `pt::dirs` kennt `engine/tor` — dort liegt lyrebird im offiziellen Bundle.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATE = ROOT / "src-tauri/src/state.rs"
ENGINE = ROOT / "src-tauri/src/engine.rs"
PT = ROOT / "src-tauri/src/pt.rs"
NATIVE = ROOT / "src-tauri/src/tor_native.rs"
DIAG = ROOT / "src-tauri/src/diagnostics.rs"
PROBE = ROOT / "src-tauri/src/probe.rs"


def code(path: Path) -> str:
    """Kommentare weg, damit eine Erklärung nicht als Verdrahtung zählt."""
    return "\n".join(
        line for line in path.read_text(encoding="utf-8").split("\n")
        if not line.strip().startswith("//")
    )


def block(src: str, kopf: str) -> str:
    """Der Körper GENAU dieser Funktion, bis zur nächsten auf gleicher Ebene.

    Ein fester Zeichenausschnitt tut es nicht: er reicht in die nächste Funktion
    hinein, und dann bezeugt deren Zeile die Verdrahtung dieser hier. Genau so
    blieb die Negativkontrolle "das tor hinter dem Tunnel holt Brücken" grün —
    das `BridgeMode::None` kam aus `start_native_tor`, nicht aus der geprüften
    Funktion.
    """
    at = src.find(kopf)
    if at < 0:
        return ""
    rest = src[at + len(kopf):]
    ende = min(
        (i for i in (rest.find("\n    fn "), rest.find("\n    pub fn ")) if i >= 0),
        default=len(rest),
    )
    return rest[:ende]


def main() -> int:
    fehler = []
    state = code(STATE)
    engine = code(ENGINE)

    # 1) vor der Engine gefragt
    native = state.find("self.start_native_tor(")
    started = state.find("self.engine.start(")
    if native < 0:
        fehler.append(
            "launch_candidate ruft start_native_tor nicht auf; Tor allein liefe weiter über arti"
        )
    elif started >= 0 and native > started:
        fehler.append("start_native_tor wird erst nach engine.start gefragt; dann laufen beide auf Port 1819")

    # 2) Lebendigkeit über den Carrier
    if "fn carrier_alive(" not in state:
        fehler.append("carrier_alive fehlt; eine Sitzung ohne Engine gilt sofort als tot")
    else:
        rufe = state.count("self.carrier_alive()")
        if rufe < 3:
            fehler.append(
                f"nur {rufe} von 3 Lebendigkeits-Prüfungen fragen den Carrier "
                "(Aufbau, Selbsttest, Wächter; Psiphon-Ketten gehören jetzt zum Kern)"
            )
        # Und der Carrier sind BEIDE Hälften: "nie gestartet" ist nicht "tot".
        # Genau diese Verwechslung riss im Feldlog vom 17.09. die
        # `Tor → Psiphon`-Sitzung 200 ms nach "tor is ready" ab.
        if "was_started()" not in state or "fn was_started(" not in engine:
            fehler.append(
                "carrier_alive kann 'nie gestartet' nicht von 'tot' unterscheiden "
                "(engine.was_started fehlt)"
            )

    # 2b) Tor → Psiphon läuft jetzt mit --tor-only --psiphon IM Kern. Daher
    # gehört die Engine zur Kette, und poll_chain darf ihr Ende erkennen.
    # Der native Tor-Pfad bleibt für Tor alleine / Tor → Aether unverändert.
    profile = code(ROOT / "src-tauri/src/profile.rs")
    if 'args.push("--psiphon".into())' not in profile or '"--tor-only"' not in profile:
        fehler.append("Tor → Psiphon übergibt Tor/Psiphon nicht an den Kern")
    if "TransportBackend::TorPsiphon" not in block(state, "fn start_native_tor("):
        fehler.append("Tor → Psiphon könnte tor.exe und Kern-Tor zugleich starten")

    # 3) der Port der App — für "Tor allein", das die Sitzung ALLEIN trägt.
    #    Die Kettenmodi binden absichtlich 1820 und speisen die Engine damit.
    if not re.search(r"Some\(TorMode::Only\)\s*=>\s*\{?\s*engine::LOCAL_SOCKS_PORT", state):
        fehler.append("der native tor bindet für 'Tor allein' nicht den Port der App (1819)")

    # 3b) Tor → Aether: tor.exe VOR der Engine, und die Engine als sein Kunde.
    #
    # Vorher lief hier `--tor-reverse`, also arti im Kern. Dasselbe Feldlog zeigt
    # beide Wege am selben Netz: tor.exe 22 s bis zum Kreis, arti zehn Minuten
    # auf 15 % ("Can't bootstrap a Tor directory").
    if not re.search(r"Some\(TorMode::Reverse\)\s*=>\s*self\.tor_listener_port\(\)", state):
        fehler.append(
            "Tor → Aether läuft nicht über das native tor.exe (kein Reverse-Zweig "
            "in start_native_tor)"
        )
    if "fn start_engine_behind_tor(" not in state:
        fehler.append("niemand startet die Engine hinter dem fertigen tor")
    else:
        behind = block(state, "fn start_engine_behind_tor(")
        if "set_upstream_socks(Some(" not in behind:
            fehler.append(
                "die Engine hinter Tor bekommt keinen Upstream-Proxy; sie würde direkt "
                "hinauswählen und Tor umgehen"
            )
        if "TransportBackend::Aether" not in behind:
            fehler.append(
                "die Engine hinter Tor behält ihr Tor-Backend und brächte damit arti "
                "ein zweites Mal hoch"
            )

    # 3c) Aether → Tor: erst die Engine, dann tor.exe DURCH deren Tunnel.
    if "fn start_tor_behind_engine(" not in state:
        fehler.append("niemand startet tor hinter dem Tunnel (Aether → Tor)")
    else:
        tail = block(state, "fn start_tor_behind_engine(")
        if "LOCAL_SOCKS_PORT" not in tail:
            fehler.append("das tor hinter dem Tunnel kennt den Ausgang der Engine nicht")
        if "BridgeMode::None" not in tail:
            fehler.append(
                "das tor hinter dem Tunnel holt Brücken; dort ist die Brücke sinnlos "
                "und lyrebird wählte sie am Proxy vorbei"
            )
    if "TorStage::EngineThenTor" not in state or "TorStage::TorThenEngine" not in state:
        fehler.append("die zwei Hälften einer Tor-Kette sind nicht auseinandergehalten")

    # 3d) und die Engine bekommt den Upstream wirklich als Umgebung mit.
    if "AETHER_UPSTREAM" not in engine or "AETHER_MASQUE_HTTP2" not in engine:
        fehler.append(
            "engine.rs setzt AETHER_UPSTREAM/AETHER_MASQUE_HTTP2 nicht; Tor trägt nur "
            "TCP, also muss MASQUE auf HTTP/2 laufen"
        )
    if "self.upstream_env()" not in engine:
        fehler.append("die Upstream-Umgebung wird beim Start der Engine nicht übergeben")

    # 4) beim Abbauen gestoppt
    if "native_tor.take()" not in state or ".stop()" not in state:
        fehler.append("cleanup stoppt den nativen tor nicht — das Kind überlebt die Sitzung")

    # 5) engine.rs kennt die Binärdatei, und pt::dirs die Transporte daneben
    if "fn native_tor(" not in engine or "fn native_tor_home(" not in engine:
        fehler.append("engine.rs liefert die tor-Pfade nicht")
    if 'join("tor")' not in code(PT):
        fehler.append("pt::dirs kennt engine/tor nicht; lyrebird liegt dort im offiziellen Bundle")

    # und das Modul selbst behält seine gemessenen Eigenheiten
    modul = code(NATIVE)
    if not re.search(r'get_info\(\s*"status/bootstrap-phase"', modul):
        fehler.append("das Modul prüft nicht mehr die Bootstrap-Phase am Control-Port")
    if re.search(r'exec \{"', modul) or 'exec "' in modul:
        fehler.append(
            "der lyrebird-Pfad ist in ClientTransportPlugin gequotet — tor gibt das "
            "Argument unverändert an CreateProcess weiter und findet die Datei dann nicht"
        )

    # 6) ein Lesetimeout am Control-Port darf die Sitzung NICHT töten.
    #
    # Feldlog 17.09. (loge2.txt): dreimal exakt 10,3 s bis zum Abbruch, jedes Mal
    # "os error 10060" — das Lesetimeout lief mitten in den ~7 s, in denen tor laut
    # seinem EIGENEN Log (Lücke zwischen "Bootstrapped 0%" und "guard context")
    # gar nichts beantwortet. Ein kurzes Slice ist erlaubt, aber nur mit Wiederholung.
    if "fn is_timeout(" not in modul or "CONTROL_READ_SLICE" not in modul:
        fehler.append(
            "das Modul unterscheidet ein Lesetimeout nicht von einem Fehler; "
            "ein tor, der gerade hochkommt, wird wieder nach 10 s abgeschossen"
        )
    if "CONTROL_HANDSHAKE_TIMEOUT" not in modul:
        fehler.append("kein eigenes Handshake-Budget für den Control-Port")
    if not re.search(r"fn read_line\(&mut self, deadline: Instant\)", modul):
        fehler.append("read_line kennt keine Deadline und kann daher nicht geduldig sein")

    # 7) zweiter Pfad: der Fortschritt steht auch im Log von tor selbst
    if "fn parse_log_progress(" not in modul or "log_progress" not in modul:
        fehler.append(
            "der Bootstrap-Fortschritt hängt allein am Control-Port; fällt der aus, "
            "ist die Sitzung blind"
        )
    if "following its own log" not in modul:
        fehler.append("ein verlorener Control-Port beendet die Sitzung statt auf das Log auszuweichen")

    # 8) das Budget kommt aus der App, nicht aus einer Konstante im Modul
    if "launch.budget.unwrap_or(BOOTSTRAP_TIMEOUT)" not in modul:
        fehler.append("das Modul ignoriert das Sitzungsbudget der App")
    # Die Schreibweise ist offen (Feld im Literal ODER nachträgliche Zuweisung),
    # der Inhalt nicht: es muss DAS Budget der App sein, das ins Modul geht.
    if not re.search(r"budget:?\s*=?\s*Some\(Duration::from_millis\(budget_ms\)\)", state):
        fehler.append(
            "state.rs gibt das Budget nicht weiter; das Log meldet dann 600 s und "
            "das Modul bricht nach 180 s ab"
        )

    # 9) Transport-Wellen: obfs4 zuerst, die domain-fronted danach
    if "fn plan_waves(" not in modul:
        fehler.append(
            "keine Transport-Wellen; mit eingebauten Brücken bleibt es bei einem "
            "obfs4-Versuch (Feldlog: 'Plan ready (1 attempt)')"
        )
    else:
        reihen = re.search(r'PREFERENCE:\s*\[&str;\s*\d+\]\s*=\s*\[([^\]]*)\]', modul)
        if not reihen:
            fehler.append("die Reihenfolge der Wellen ist nicht ablesbar")
        else:
            namen = re.findall(r'"([^"]+)"', reihen.group(1))
            if "obfs4" not in namen or "snowflake" not in namen:
                fehler.append("obfs4 und snowflake stehen nicht in der Wellen-Reihenfolge")
            elif namen.index("obfs4") > namen.index("snowflake"):
                fehler.append("snowflake vor obfs4: das teuerste Transport zuerst")

    # 10) ein Transport wird nur angeboten, wenn seine Binärdatei mitgeliefert ist
    if "meek_lite,obfs4,webtunnel,snowflake" in modul:
        fehler.append(
            "eine ClientTransportPlugin-Zeile behauptet, lyrebird führe snowflake aus; "
            "das endet immer in 'No running bridges'"
        )
    if "fn available_plugins(" not in modul or "SNOWFLAKE_FILENAME" not in modul:
        fehler.append("die Transporte werden nicht gegen die vorhandenen Binärdateien geprüft")

    # 11) die Stufe-1-Sonde fragt nach NAMEN, nicht nach einer Adresse.
    #
    # Feldlog 17.09.: "Your application (using socks5 to port 80) is giving Tor
    # only an IP address" — die Sonde wählte 1.1.1.1 und tor beklagte genau das.
    # Ein Name geht als ATYP=DOMAIN hinaus und wird am fernen Ende aufgelöst.
    diag = code(DIAG)
    if re.search(r'tcp_via_proxy_on\(\s*port\s*,\s*"\d+\.\d+\.\d+\.\d+"', diag):
        fehler.append(
            "die Stufe-1-Sonde wählt eine rohe IP durch Tor; tor warnt darüber und "
            "die Sonde fällt auf eine Exit-Policy statt auf die Wahrheit"
        )
    if "STAGE_ONE_TARGETS" not in diag:
        fehler.append("die Stufe-1-Sonde hat keine benannten Ziele")

    # 12) das Land kommt von der Platte, nicht aus dem Netz.
    probe = code(PROBE)
    lokal = probe.find("crate::geoip::lookup(")
    fern = probe.find('socks5_stream("ip-api.com"')
    if lokal < 0:
        fehler.append(
            "refine_country fragt die lokale geoip-Datei nicht; für jede Flagge geht "
            "ein Klartext-Request hinaus, obwohl die Antwort neben tor.exe liegt"
        )
    elif fern >= 0 and lokal > fern:
        fehler.append("die lokale geoip-Abfrage kommt erst nach dem Netz-Request")
    if "geoip::use_dir(" not in state:
        fehler.append("niemand sagt dem geoip-Modul, wo die Dateien von tor liegen")

    # 13) ein schlechter Kreis wird gewechselt, nicht als schlechtes Netz gemeldet.
    if "SIGNAL NEWNYM" not in modul:
        fehler.append("tor_native kann keinen neuen Kreis anfordern (SIGNAL NEWNYM)")
    if "fn consider_new_circuit(" not in state:
        fehler.append("niemand entscheidet, ob ein Kreis gewechselt werden soll")
    else:
        rot = block(state, "fn consider_new_circuit(")
        if "ping::reset()" not in rot:
            fehler.append(
                "nach dem Kreiswechsel bleibt die warme Ping-Sitzung stehen und misst "
                "weiter den ALTEN Kreis"
            )
        # Die Schranke selbst, nicht ihr Name: `MAX_ROTATIONS` steht auch in der
        # Logzeile, und ein Name in einer Meldung begrenzt nichts.
        if not re.search(r"circuit_tries\s*>=\s*MAX_ROTATIONS", rot) or not re.search(
            r"const MAX_ROTATIONS:\s*u8\s*=\s*[1-5];", rot
        ):
            fehler.append(
                "der Kreiswechsel hat keine Obergrenze; ein langsames Netz drehte dem "
                "Benutzer den Kreis endlos unter den Füßen weg"
            )
        if "circuit_hunt_until" not in rot:
            fehler.append("der Kreiswechsel ist nicht auf das Fenster nach dem Verbinden begrenzt")

    # 14) AETHER-APP-FIX tor-auto-like-mobile (Logs vom 8. Oktober): `Auto` heißt wie
    # auf dem Handy „direkt UND Brücken“, nicht „nur Brücken“.
    if "TorBridges::Auto => crate::tor_native::BridgeMode::Auto" not in state:
        fehler.append("TorBridges::Auto wird nicht auf BridgeMode::Auto abgebildet (nie direkt versucht)")
    if "fn race(" not in modul or "return self.race(launch);" not in modul:
        fehler.append("BridgeMode::Auto startet kein Rennen direkt gegen Brücken")
    if "SETCONF SocksPort=" not in modul:
        fehler.append("der Gewinner des Rennens zieht nicht auf den festen SOCKS-Port um")
    # 15) Kein CircuitStreamTimeout 30 mehr: eine schlechte Exit hielt jede Probe 30 s fest.
    if re.search(r'push_str\("CircuitStreamTimeout', modul):
        fehler.append("CircuitStreamTimeout steht wieder in der torrc (Scan durch Tor verhungert)")
    # 16) Der Exit-Beweis läuft parallel auf isolierten Kreisen.
    if "PROOF_PARALLEL_ATTEMPT" not in modul or "[0x05, 0x01, 0x02]" not in modul:
        fehler.append("der Exit-Beweis läuft nicht parallel auf isolierten Kreisen")

    if fehler:
        print("✗ nativer Tor-Carrier:")
        for f in fehler:
            print(f"   - {f}")
        return 1

    print(
        "✓ Tor: offizielles tor.exe in allen drei Formen (allein auf 1819, vor der "
        "Engine mit Upstream-SOCKS, hinter dem Tunnel ohne Brücken), "
        "Carrier-Lebendigkeit an 3 Stellen und beide Hälften; Tor → Psiphon "
        "wird vom Kern verwaltet. Stufe-1-Sonde fragt nach Namen, Land aus der lokalen geoip-Datei, "
        "schlechter Kreis wird begrenzt gewechselt, Abbau stoppt tor, engine/tor als "
        "Transportpfad, geduldiger Control-Port mit Log als zweitem Pfad, Budget aus "
        "der App, Wellen obfs4 → snowflake nur mit vorhandener Binärdatei"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
