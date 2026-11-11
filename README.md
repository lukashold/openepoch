# OpenEpoch

Eigene Rust-/WebAssembly-RTS mit WebGL2. Dieses Repository enthält eigenen
Quellcode, eigene Laufzeitmodule und technische Spielwertprofile. Originalgrafiken,
Modelle, Animationen, Sounds, Musik, Originalkarten und private Spielstände von
Empire Earth II bleiben ausschließlich lokal. Vollständige Spielparität ist noch
nicht erreicht.

## Verifizierter Stand

Der eigene Spielkern umfasst die Ausbaustufen bis `ee2-rect291-ai-scout`, darunter
rechteckige Welten als externes Eingabeformat, Bewegung und Kontakt, Bauaufträge,
Ausweichverhalten, wiederaufgenommene Sammelaufträge, Farmzustände, Rohstoffförderung,
Forschung und gespeicherte Bürger-Arbeitszustände. Die eigenen Präsentationsmodule
enthalten Ereignisuhren, Werkzeugdarstellung und Adapter für separat lokal geladene
Sounds und Effektgeometrie. Diese Adapter enthalten keine Originalmedien.

Die mitgelieferten eigenen WASMs wurden aus dem geprüften öffentlichen Quellstand
frisch gebaut. Der vollständige Host-Testlauf mit allen Features bestand mit
**518 erfolgreichen und 25 ausdrücklich ignorierten Tests**. Ein isolierter
öffentlicher Chrome-Origin ohne Originalmedien wurde mit echtem Spielstart,
laufender Darstellung, Pause, Speichern und Fortsetzen nach neuem Dokument geprüft.
Der gespeicherte Simulationszustand wurde dabei exakt wiederhergestellt. Dieser
Prüfumfang belegt keine vollständige Übereinstimmung mit dem Originalspiel.

Die normalen Regeln und der eigene prozedurale Renderer funktionieren ohne
Originalinstallation. Eigener Loadercode mit Dateinamen wie `*-private.js` und
`*-private.html` darf enthalten sein; er benötigt für Originalkarten und
Originaldarstellung separat lokal bereitgestellte Eingaben. `docs/local-original/`,
Originalkarten, private Spielstände und Original-Testdaten gehören nicht in Git.
Dateien eines privaten Komplettpakets dürfen nicht pauschal übernommen werden.

## Bauen und starten

`Build-All.ps1` baut die eigenen Standard- und Regelmodule sowie den rechteckigen
Laufzeitkern. Dafür müssen Rust und das Target `wasm32-unknown-unknown` lokal
vorhanden sein. Die geprüften WASMs liegen bereits in `docs/`.

Mit Python 3 aus dem Spielordner starten:

```sh
python tools/serve_game.py --root docs --port 8125
```

Auf macOS kann außerdem `Start-OpenEpoch.command` verwendet werden. Ohne lokale
Originaleingaben öffnet der Starter das normale Karten- und Modusmenü. Sind die
benötigten lokalen Karten- und Präsentationseingaben vorhanden, öffnet er das
lokale Spiel mit der Originalkarte. Die vollständige Integrität aller Medien
prüft diese Eingangswahl nicht.

Das normale Menü lässt sich ausdrücklich auswählen:

```sh
python tools/serve_game.py --root docs --port 8125 --page 'index.html?rules=ee2'
```

`--lan` behält ohne ausdrückliches `--page` das normale Karten- und
Mehrspielermenü bei. Der rechteckige Kern nimmt Kartendaten über seine Lade-ABI
entgegen; Originalkarten sind kein Build-Eingang. Ein Test auf echter Mac-Hardware
steht noch aus.

## Tests und private Integrationsdaten

Die öffentlichen Unit- und synthetischen Tests enthalten keine Originalkarte.
24 Integrationsprüfungen mit Originaleingaben sind ausdrücklich mit `#[ignore]`
markiert; ein weiterer ignorierter Test erzeugt eine isolierte Vergleichsdatei.
Diese 25 Tests zählen nicht zu den 518 erfolgreichen Prüfungen.

```sh
cargo test --offline --locked --all-features --lib -- --test-threads=1
```

Original-Testdaten werden ausschließlich aus dem ausdrücklich gesetzten lokalen
Verzeichnis `OPENEPOCH_PRIVATE_FIXTURES` gelesen. Erwartete Dateinamen sind
`world.w8in`, `world0.dvp1` und `animal-manager.anm`. Es gibt keine automatische
Suche nach diesen Testdaten und keine eingebetteten oder heruntergeladenen
Originaleingaben. Einzelne ignorierte Integrationsprüfungen können bei Bedarf mit
diesem Verzeichnis und `--ignored` bewusst separat ausgeführt werden.
