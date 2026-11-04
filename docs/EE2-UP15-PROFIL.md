# Optionales EE2-UP15-Datenprofil

Cargo-Feature `ee2-up15` hängt von `ee2-reference` ab und besitzt Ruleset-ID **2**. Es verwendet die belegten CSV5-Tabellen und genau die beiden aktiven CFG5-Dateilisten aus dem lokal aufgelösten Original-Namensraum. 55 der 57 bisherigen Rollen sind numerisch gebunden; die unaufgelösten Rollen 28/43 sind in diesem Profil gesperrt. Die begrenzte Rollenzuordnung und nicht umgesetzte Originalmechaniken stehen in `ee2-up15-profile.json`, `ee2-up15-research.json` und `ee2-up15-unit-refs.json`.

Die bestehenden Profile 0/1 bleiben getrennt. Neue Saves beginnen mit `OEU5` (0x3555454f); fremde Profile und alte Layouts werden atomar abgewiesen. Das Frontend verwendet getrennte Profilwahl, Multiplayer-Rulesetprüfung und die Datenbank `openepoch-ee2-up15-v1`. `?rules=ee2` startet dieses Profil; `?rules=ee2&profile=base` behält Basisversion und ältere Basis-Spielstände. Das Profil bildet nicht das gesamte durch Laufzeitpatches veränderte UP1.6-Original ab.

## Reproduktion

Aus `outputs/OpenEpoch` ausführen. Voraussetzung sind die persönlichen Originaldateien, der lokale Modell-Metadatenimport und der geprüfte Source-Resolver; diese Originalquellen werden nicht veröffentlicht.

```powershell
python tools/extract_up_static_sources.py --install 'C:\Program Files (x86)\GOG.com\Empire Earth II' --out ../../work/up-resolved-sources --game ee2
python tools/regenerate_ee2_up15.py
python tools/regenerate_ee2_up15.py --verify
cargo test --offline --features ee2-up15
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-up15 --target-dir ../../work/up15-target
```

Nur die neue Builddatei als `docs/openepoch-ee2-up15.wasm` übernehmen. Die beiden bestehenden WASMs nicht ersetzen. `regenerate_ee2_up15.py` führt nacheinander Units, Combat, Research, Profile/Economy, unabhängige Rohdaten-Orakel und den vollständigen UpgradeRef-Audit aus. `--verify` verlangt bytegleiche Wiederholung aller zwölf erzeugten Dateien.

Der WASM-Prüfer akzeptiert Baseline-WASM0, Baseline-WASM1 und einen separaten Buildordner mit `openepoch-base.wasm`, `openepoch-ee2-reference.wasm`, `openepoch-up15.wasm`:

```powershell
node tools/test_ee2_up15_wasm.mjs docs/openepoch.wasm docs/openepoch-ee2.wasm ../../work/up15-profile-candidate/builds
```

Die aktuelle geprüfte Variante besteht 76 UP-Rusttests und 92.660 unabhängige Rohdatenprüfungen. 318.228 WASM-Prüfungen decken alle 75.625 Matchup-Paare, 15 Epochen, Profilisolation sowie BASE-/Referenz-Savebytes und Simulationshashes ab. Laden setzt wie bisher den flüchtigen Flow-Cache zurück; Projektile bleiben flüchtig. Originalmedien, Archive, EXE/DLL und Ghidra-Datenbanken gehören nicht zum Quellpaket.
