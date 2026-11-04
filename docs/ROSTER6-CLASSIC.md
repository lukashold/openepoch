Der61-Kandidat ergänzt vier echte Sourcefamilien: Scout57, FightingSail58, AirDefense59 und CoastalDefense60. Die bisherigen57 Kennungen bleiben erhalten. Der getrennte klassische Katalog umfasst132 Familien;43 davon sind jetzt gebunden,89 noch nicht. Alle Zahlen und Produktionszeilen stammen aus dem konfigurierten aktiven UP-CSV5/CFG5-Profil. Die alte BASE-Tabelle bestimmt nur die Mitgliedschaft im klassischen Katalog.

Neue Daten:295 Klassenintervalle,145 visuelle Zahlenbindungen und332 RPS-Profile. Die vier zusätzlichen Familien besitzen16 explizite Originalmodellintervalle mit14 unterschiedlichen Meshdateien und19 Texturen. Die Modelle und Skalen stehen in separaten lokalen Manifesten `local-original/models/manifest-roster6.json` und `local-original/model-scales-roster6.json`. Neue Einträge tragen assetKind0 und Skalenkeys `0:DefId:epoch`.

OEU5 und der bekannte57-Registry-Stand von OEU6 werden transaktional importiert. Die Headerzahl und der Registry-Fingerprint müssen als bekanntes Paar zusammenpassen; Definitionen, Garnisonen und Trainingsjobs dürfen die alte Eingangsgrenze nicht überschreiten. Das aktuelle Save bleibt OEU6 mit176-Byte-Entitäten. Die Datenbank braucht keinen Reset. NET11 darf sich nicht mit dem61-Kern verbinden; die Anwendung bindet einen neuen Protokollstand und den genauen WASM-Fingerprint.

Neue Sourcefamilien speichern absoluten nativen Schaden. Dadurch bleibt AirDefense59 statt59.0000038. Die Kombination Military3_1 plus Military8_1 verwendet den belegten additiven Prozentzustand. Unsere aktuelle Arithmetik liefert dabei 180 aus Basis150 und nach dem Epochenupgrade204 aus Basis170 einschließlich Projektilschaden und Save/Load. Das sind eigene Regressionswerte, keine unbedingten Originalbits: Der Originalkontext mit x87 CHOP53 versus nearest am konkreten Setter ist noch offen (siehe native-percent-rounding-context-addendum.json). Die ersten57 Rollen behalten ihre bisherige Darstellung und Multiplikationsreihenfolge. Profile0/1/5 wurden gegen ihre eingefrorenen WASMs mit vollständigen Savebytes und Simhash geprüft.

Küstenverteidigung verlangt im Kandidaten Wasser in der belegten nativen Suchweite4. Der native Distanznäherungsfaktor ist0.4569999873638153; der Adapter verwendet das vorhandene tiefe Wasser als Ersatz für die nativen erlaubten Wasserklassen. Die ursprüngliche Küstenausrichtung, Terrainrasterung und vollständige Kollision sind damit nicht nachgebaut.

Weitere konkrete Grenzen: Bei frühen Kampfseglern fehlen die separaten prozeduralen SailEffect-Kinder. Vollständige Produktionsslot-Ersetzung über diese vier Familien hinaus, Territory-AllowedUnits-Anbindung,294 Familienverbesserungen,33 Main-Upgrades, Konversion, Heilung, Handel, Flugzeugbewaffnung und zahlreiche Spezialfähigkeiten bleiben offen. Die Quellvorlage territory.ddf nennt AirDefense7 und CoastalDefense6; ohne belegte aktive Vorlagenauswahl wurden diese Zahlen nicht als allgemeine Limits eingebaut. Die61 Definitionen sind kein vollständiger Originalkader.

Reproduktion mit vorhandenem geprüftem lokalem Source-Resolver:

```powershell
python tools/import_roster6_models.py
python tools/regenerate_roster6_classic.py --verify
cargo test --offline --features ee2-roster6
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-roster6
node tools/test_roster6_classic_wasm.mjs
```

Der WASM-Test verwendet die mitgelieferten alten eigenen Builds als Orakel. Ein frisch kompilierter aktueller Build kann unter dem `builds`-Ordner neben dem Projekt liegen; ansonsten verwendet der Test die fertige Datei in docs. Originalprogramme werden für keinen dieser Schritte gestartet.

`tools/ordinary-classic-fixture.mjs` baut eine Aufnahmewelt ausschließlich über öffentliche Befehle. `buildOrdinaryClassicFixture(w,{instantBuild:false})` bildet Späher und Schiff aus und lässt Bürger Dock, Luftabwehr und Küstenverteidigung errichten. Der geprüfte Seed7/native Epoche10 (Runtime9) braucht4120Ticks; die explizite InstantBuild-Option braucht5Ticks. Die Testhilfe liest Savebytes zur Beobachtung, verändert aber keine Speicherfelder oder Spielstände.


Die Produktionsverfügbarkeit ist von den fortbestehenden Klassen- und Modellintervallen getrennt. Scout57 gilt Runtime[0,15), FightingSail58[5,10), AirDefense59[9,15), CoastalDefense60[1,15). MainEpoch11 sperrt FightingSail im Original. Vorhandene Schiffe behalten ihre späteren Werte, Modelle, Stimmen und Namen. Die gemeinsame normale Aufnahme muss Runtime9 verwenden. Details und Einschränkungen stehen in ROSTER6-AVAILABILITY.md.
