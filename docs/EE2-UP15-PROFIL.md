# EE2-UP15-Datenprofil (Combat5)

Cargo-Feature `ee2-up15` haengt von `ee2-reference` ab und besitzt Ruleset-ID **2**. Es verwendet die belegten CSV5-Tabellen und genau die beiden aktiven CFG5-Dateilisten aus dem lokal aufgeloesten Original-Namensraum. Alle **57 bisherigen Rollen** sind jetzt an Quellen gebunden: **279 Klassenintervalle, 129 Visualintervalle, 313 RPS-Profile**. Darunter sind Paladin und LightHerc mit ihren wirklichen UP-Modellen sowie HumveeStriker als spaetere Flugabwehrstufe. 14 Rollen bleiben ausdrueckliche Zuordnungen unseres kleineren Kaders; dies ist noch keine vollstaendige Original-Produktionsliste.

Die Profile 0/1 bleiben getrennt und ihre bestehenden WASM-Dateien unveraendert. `?rules=ee2` verwendet UP; `?rules=ee2&profile=base` verwendet die Basisversion mit eigenen Spielstaenden. Multiplayer prueft NET10 und den exakten Simulationsfingerprint. Die lokale Datenbank fuer UP ist `openepoch-ee2-up15-v1`.

## Angriffe und Spielstaende

Die UP-Simulation verwendet native RPS-Zielklassen fuer Flugabwehr/Schiffe, Mindestreichweiten, direkten Schaden oder benannte Projektile mit Quellgeschwindigkeit sowie den belegten Radius-/Friendly-Fire-Binder. Der Angriff beginnt mit einer gespeicherten Runde und verursacht Schaden erst nach der strikt verglichenen `applyDamageTime`; er wird nicht schon beim Rundenstart ausgeloest. Ein normaler Bewegungsbefehl erlaubt Schiessen nur mit dem tatsaechlichen `attackOnTheMove`-Feld.

Saves beginnen weiterhin mit `OEU5` (0x3555454f). Nach dem unveraenderten vollstaendigen236/ECO1/DIP1/TER1/CGD1-Prefix folgt die versionierte optionale `ATK1`-Erweiterung: voller Generationhandle, naechster Angriffszeitpunkt und Rundenflags. Aeltere eigene UP-Saves ohne diesen Tail migrieren laufende Cooldowns. Fremde Profile, ungueltige Zahlen, doppelte/ungeordnete/veraltete Handles und beschaedigte Tails werden vor dem Ersetzen der Welt abgewiesen. Ent-/Player-Binaerlayouts bleiben unveraendert.

Projektile und Explosionen im Flug bleiben wie bisher fluechtig. Ein Speichern waehrend eines fliegenden Projektils ist deshalb noch nicht replaygleich. Pack-/Unpack-Capability, Turret-Baenke, native Schussbahn-/Militaerflags, Munition/Nachladen am Flugplatz und Animationsevents sind weiterhin offen. Latente Hausangriffe werden ohne belegte Aktivierungsfaktoren nicht pauschal freigeschaltet. 439 UpgradeRef-Feldvorkommen sowie 82 Forschungs-Spezialfaktoren sind als verbleibende Luecken inventarisiert.

## Reproduktion

Aus `outputs/OpenEpoch` ausfuehren. Die persoenlichen Originaldateien und der gepruefte Source-Resolver werden benoetigt, um Daten und Medien neu zu importieren; kompiliert werden kann der generierte Rust-Quellstand ohne Originalinstallation.

```powershell
python tools/extract_up_static_sources.py --install 'C:\Program Files (x86)\GOG.com\Empire Earth II' --out ../../work/up-resolved-sources --game ee2
python tools/import_ee2_up15_models.py
python tools/regenerate_ee2_up15.py
python tools/regenerate_ee2_up15.py --verify
cargo test --offline --features ee2-up15
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-up15 --target-dir ../../work/up15-target
```

Nur den neuen UP-Build als `docs/openepoch-ee2-up15.wasm` uebernehmen. Die beiden bestehenden WASMs nicht ersetzen. Der Modellimport erstellt ausschliesslich zusaetzliche private Derivate und die getrennten `local-original/models/manifest-up15.json` / `local-original/model-scales-up15.json`; die BASE-Manifeste bleiben unveraendert.

`regenerate_ee2_up15.py` fuehrt Units, belegte AA-Visualprogression, Combat, Research, Attack, Profile/Economy, Rohdaten-Orakel und den vollstaendigen UpgradeRef-Audit aus. `--verify` verlangt bytegleiche Wiederholung aller14 erzeugten Dateien. Das Original wendet laut0053C930 zuerst die UpgradeRefs und danach nonzeroCSV-Skalare an; 7.436 gemeinsame Feldwerte sind nach dieser Korrektur unveraendert.

```powershell
node tools/test_native_attack_wasm.mjs
node tools/test_ee2_up15_wasm.mjs docs/openepoch.wasm docs/openepoch-ee2.wasm ../../work/native-combat-abilities/builds
```

Der erste Pruefer nutzt die eingefrorenen Native4-WASMs aus `work/native-combat-abilities/baseline`; alternativ kann deren Verzeichnis als Argument angegeben werden. Der zweite verwendet einen isolierten Buildordner mit `openepoch-base.wasm`, `openepoch-ee2-reference.wasm`, `openepoch-up15.wasm`.

## Gepruefter Stand

UP-WASM: `c80906c19c7a329c868f5596e77b202d95262af87b48997bfeedc77636e05e51`, 460.364Bytes. Rust:90 UP,75 Referenz,32 BASE. 115.192 Rohquellenpruefungen,407.723 WASM-Pruefungen,5.429 Attack-/Save-/Malformed-Pruefungen und14 bytegleiche Regenerationsdateien sind gruen. 16 Simulationsfaelle mit je1.200Ticks bewahren die vollstaendigen BASE-/Referenz-Savebytes und Simulationshashes gegen die eingefrorenen Binaerdateien. Die echten Schadensereignisse folgen dem unabhaengigen nativen Timing-Orakel exakt an Ticks11/42/74/105.

Belege und Grenzen: `outputs/EE2-Research/analysis/native-combat5-evidence.json`, `native-combat5-regression-tests.json`, `native-combat5-attack-wasm-tests.json` und `docs/ee2-up15-unit-refs.json`. Originalmedien bleiben im privaten Spielordner; Archive, EXE/DLL und Ghidra-Datenbanken gehoeren nicht zum oeffentlichen Quellpaket.
