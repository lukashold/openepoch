# EE2-UP15-Datenprofil in native7

Das ausgelieferte UP-Profil verwendet die eigenen Cargo-Features
`ee2-paidqueue7,ee2-render-meta7`. Beide bauen auf `ee2-roster6`, `ee2-up15`
und `ee2-reference` auf. Ruleset bleibt2, Speicherrevision ist7. Die originale
CSV5-/CFG5-Priorität und die ursprünglichen Kampfquellen bleiben belegt.
Das ist weiterhin ein kleinerer Kader als das vollständige Empire Earth II.

`?rules=ee2` verwendet UP. `?rules=ee2&profile=base` behält die Basisversion
und ihre eigenen Spielstände. Die vorhandenen WASMs der unabhängigen Regeln0
und der Basisregeln1 bleiben bytegleich. UP behält die Datenbank
`openepoch-ee2-up15-v1`; Netzwerkprotokoll14 trennt ältere Simulationsstände.

61 Rollen sind an Quellen gebunden. Die zusätzlichen Familien sind Scout,
FightingSail, AirDefense und CoastalDefense. Neue Segelkriegsschiffe können nur
in den belegten Epochen6–10 produziert werden; spätere Originalmodelle bereits
bestehender Schiffe bleiben erhalten. Insgesamt sind43 der132 inventarisierten
klassischen Quellenfamilien angebunden. Die übrigen89, Zivilisationsregeln und
zahlreiche Fähigkeiten sind weitere Arbeit. Die311 installierten Produktions-
klassen sind keine311 gleichzeitig verfügbaren Scharmützeleinheiten.

## Produktion und Spielstände

Neue UP-Spielstände beginnen mit OEU7 (`0x3755454f`). Die176-Byte-Einheiten und
96-Byte-Spieler behalten ihr Layout. Die zusätzliche PQP1-Erweiterung, Version2,
enthält für jeden echten Generationhandle und jeden Queueplatz den bezahlten
Preis in elf nativen Ressourcenkomponenten sowie die festgelegte Produktionsdauer.
Der Descriptor verwendet Version7/Schema2. Der genaue Vertrag steht in
`paidqueue7-format.json` und `PAIDQUEUE7.md`.

Die Dauer wird beim Arbeitsbeginn festgelegt. Wartende Aufträge haben bis dahin
den Wert−1. Neue Einheitaufträge verwenden den belegten ersten Zeitinkrement0.01.
Spätere Klassen-/Forschungsänderungen ändern eine bereits laufende Dauer nicht.
Bei Klassenwechseln werden nur wartende bezahlte Aufträge neu berechnet. Der
laufende Auftrag behält seine Rechnung; Abbruch verwendet diese gespeicherten
Preiskomponenten. Marktumrechnung gilt für die belegten Standardkurse110/90.
Marktaktionen, Kronen und Produktionszyklus fehlen noch im eigenen Spiel.

Bekannte OEU5-/OEU6-Spielstände werden geprüft übernommen. Ihre nicht enthaltene
Preis-/Zeitgeschichte wird ausdrücklich als rekonstruiert markiert. Beschädigte
Ledger, Dauerfelder, Handles oder Deskriptoren werden vor dem Weltwechsel
abgewiesen. Flüchtige Projektile bleiben eine separate Spielstandlücke.
Forschung/Epochen, Hostzuordnung,20Hz-Takt und dynamische Gebietseffekte sind
weiterhin begrenzte Adapter; das Original wird damit nicht vollständig simuliert.

## Grafik und Reproduktion

Die optionale64-Byte-Renderliste trägt echte Einheitenhandles parallel zum
unveränderten Instanzstrom. Sie verändert weder Saves noch Simulationshashes.
Segel verwenden getrennte eigene visuelle Zustände und den separaten eigenen
WASM-Kern. Originale lokale Raster, Taganbindungen, Masken und Materialstufen
bleiben erhalten. Umfang und Rechengenauigkeitsgrenzen stehen in `SOURCE-SAILS7.md`.
Originalmedien befinden sich ausschließlich im privaten `local-original`-Ordner.

Aus dem Projektordner `Build-All.ps1` beziehungsweise `sh Build-All.sh` ausführen.
Das baut alle drei numerischen Profile und den visuellen Kern. Ein bloßer Build
mit `--features ee2-up15` erzeugt weiterhin das ältere Datenprofil und darf nicht
an die Stelle des aktuellen UP-Builds im7er-Frontend kopiert werden.

Die gespeicherten eigenen historischen WASM-Dateien unter `tools/fixtures`
ermöglichen echte OEU5-/OEU6-Migrationsprüfungen ohne Originalinstallation:

```
node tools/test_paidqueue7_wasm.mjs
node tools/test_paidqueue7_gameplay_wasm.mjs
node tools/test_paidqueue7_duration_wasm.mjs
node tools/test_paidqueue7_network_wasm.mjs
node tools/source-sail-frame-test.mjs
```

Die ursprünglichen Archive, EXE/DLL und Ghidra-Projekte werden nicht
veröffentlicht und niemals als Spielruntime ausgeführt. Die vollständige
Originalübereinstimmung, natives KI-/Kartenverhalten, vollständige Fähigkeiten
und ein tatsächlicher Mac-/Browserstart bleiben offen.
