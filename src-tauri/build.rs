//! Build-Skript der App.
//!
//! Ausser Tauris eigenem Build-Schritt liest es den Patch-Stand der Repo-Wurzel
//! (`PATCHLEVEL`) und gibt ihn als `AETHER_APP_PATCHLEVEL` an den Compiler
//! weiter. `src/provenance.rs` vergleicht diesen Wert zur Laufzeit gegen den
//! Stempel, den die Engine in ihr eigenes Binary eingebrannt hat -- so kann eine
//! veraltete Engine nicht mehr unbemerkt getestet oder ausgeliefert werden.

use std::path::{Path, PathBuf};

/// Laeuft vom Crate-Verzeichnis aufwaerts und sucht `PATCHLEVEL`.
///
/// `APP_PATCHLEVEL` aus der Umgebung gewinnt immer -- das setzt CI. Wird nichts
/// gefunden, ist das Ergebnis `unstamped`: laut, greppbar und von der App als
/// Warnung behandelt. Niemals eine Version erfinden.
fn discover_patchlevel() -> String {
    if let Ok(v) = std::env::var("APP_PATCHLEVEL") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return v;
        }
    }

    let mut dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(".").to_path_buf());

    for _ in 0..6 {
        let candidate = dir.join("PATCHLEVEL");
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            let v = text.trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
        if !dir.pop() {
            break;
        }
    }

    "unstamped".to_string()
}

fn main() {
    let patchlevel = discover_patchlevel();
    println!("cargo:rustc-env=AETHER_APP_PATCHLEVEL={patchlevel}");
    println!("cargo:rerun-if-env-changed=APP_PATCHLEVEL");
    println!("cargo:rerun-if-changed=../PATCHLEVEL");
    println!("cargo:rerun-if-changed=PATCHLEVEL");

    tauri_build::build();
}
