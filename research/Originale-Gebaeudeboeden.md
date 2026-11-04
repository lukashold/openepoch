# Originale Gebäudeböden

Stand: 31.10.2026. Die lokale Darstellung ergänzt echte Bodenbilder aus der installierten Empire-Earth-II-Basisversion. Der Import umfasst 125 zusätzliche Original-Texturcachedateien und dreizehn belegte Zuordnungen zu vorhandenen Gebäudedefinitionen. Er schließt eine sichtbare Lücke: Gebäude standen bisher direkt auf dem allgemeinen Gras, obwohl ihre originalen DDF-Modelle eigene `Under*`-Decal-Kinder besitzen.

## Quellenkette

Der Import liest ausschließlich lokale Basisarchive unter `C:\Program Files (x86)\GOG.com\Empire Earth II\zips`. Eine unbekannte VFS- oder Patchpriorität wird daraus nicht abgeleitet. Die Originalinstallation bleibt unverändert.

| Archiv | SHA256 |
|---|---|
| db.zip | `042291d39b8e5362124da5254c718966032533885638e8393d5900e91e9b492e` |
| hdrs.zip | `2776d86da7e8fa405e78dd6feee67aff2861dc4165637bfa8db9de84f66f37fc` |
| textures.zip | `e6b15e1b950ab6c3071ea3893a34e01c2ea5b5a998c69ecf03fbd6c63160ea1a` |

Die nachvollziehbare Kette lautet:

1. `db/Units/*.ddf`: `UnitType` liefert `SizeX/Y`, `UndertextureSizeX/Y` und ausdrücklich deklarierte Bodenversätze. Das zugehörige `UnitModel` enthält das passende `Under*`-Kind.
2. `db/Visuals/visual_decals.ddf`: Das `Decal` nennt je drei Rural-/Urban-Sprites für `Idle` und `Death`.
3. `db/Terrain/dbsprite_overlay.csv`: Sprite zu Bildname, Bildausschnitt und Flags.
4. `db/Terrain/DbSeasonalTextureSets_Temperate.csv`: Virtueller Bildname zu tatsächlich referenzierten Bilddateien; jede unterschiedliche referenzierte Variante wird extrahiert.
5. `textures.zip!cache/texcache/*.texcache`: Originales NiPixelData-Topmip wird ohne Größenänderung, Beschnitt, Spiegelung oder Übermalung als RGBA-PNG dekodiert.

Die vollständigen Einzelquellen mit Zeilennummern, SHA256, Spritearrays, Seasonal-Reihenfolge, ausgewähltem Bild und allen physischen Alternativen stehen unter `../OpenEpoch/docs/local-original/ground-decals-manifest.json`. Die kompakte Rendererzuordnung heißt `ground-decals.json`. `tools/extract_ee2_textures.py` erzeugt beide Dateien und die PNGs unter `docs/local-original/textures/ground/` reproduzierbar.

## Belegte Gebäudeflächen

Die Zahlen sind Original-DDF-Werte; sie werden nicht als unveränderter Weltmaßstab des Nachbaus ausgegeben.

| Aktuelle ID | Originalgrafik | Native SizeX × SizeY | Undertexture X × Y | Bodenversatz X / Y |
|---|---|---:|---:|---:|
| 0 | CityCenter / UnderCityCenter | 3,5 × 3 | 4 × 3 | −0,25 / 0* |
| 1 | House / UnderHouse | 1,5 × 1,5 | 1,5 × 1,5 | 0* / 0* |
| 3 | Barracks / UnderBarracks | 2 × 2 | 3 × 2 | −0,5 / 0 |
| 4, 48 | Mill / UnderMill | 2,5 × 2 | 2,5 × 2,5 | 0* / 0,25 |
| 6 | Airport / UnderAirport | 5 × 3 | 5 × 3 | 0* / 0* |
| 47 | University / UnderUniversity | 3 × 3 | 3 × 3 | 0* / 0* |
| 49 | Stable / UnderStable | 2 × 2 | 3 × 2 | −0,5 / 0* |
| 50 | Market / UnderMarket | 2 × 2 | 2 × 2 | 0* / 0* |
| 51 | Temple / UnderTemple | 2,5 × 2,5 | 2,5 × 2,5 | 0* / 0* |
| 52 | Warehouse / UnderWarehouse | 2 × 1 | 2 × 1 | 0* / 0* |
| 55 | HercFacility / UnderMill | 2,5 × 2 | 2,5 × 2,5 | 0* / 0,25 |
| 56 | Fortress / UnderFortress | 3 × 3 | 3 × 3 | 0* / 0* |

`*` Die betreffende Versatzachse ist im Original-UnitType nicht ausdrücklich deklariert. Der Nachbau verwendet den dokumentierten Adapterwert null; `offsetDefaults` kennzeichnet genau diese Achsen. ID4 verwendet bereits einen Mill-Rollenadapter. Das ist keine neue Behauptung einer ursprünglichen EE2-Klasse Workshop. HercFacility nennt UnderMill selbst ausdrücklich als Kind.

Die dreizehn Bindungen teilen elf physische Rural-Slot-0-PNGs mit zusammen 1.767.428 Bytes. Die Bildabmessungen reichen von 128 × 128 bis 512 × 256. Alle elf ausgewählten Bilder enthalten tatsächliche Alphawerte von 0 bis 255. Sämtliche 125 einschließlich Schnee-/Death-Alternativen belegen zusammen 20.649.273 PNG-Bytes; der Renderer muss diese Alternativen nicht vorab laden.

## Darstellung und offene Auswahlregeln

Der Renderer `docs/original-ground.js` wählt ausdrücklich **Rural-Slot 0 als eigenen Adapter**. Die drei Arrayplätze sind im Import lediglich `sourceSlot: 0..2`. Ihre originalen Epochenschwellen sowie die genaue Bedingung für Rural gegenüber Urban sind noch nicht belegt. Es werden deshalb keine Epochengruppen oder Urban-Grenzen erfunden. Auch die Reihenfolge der saisonalen Bildliste wird erhalten, ohne ungeklärte Indizes als konkrete Monate oder Jahreszeiten auszugeben. Für die kompakte Vorschau wird der am häufigsten genannte physische Bildpfad verwendet, bei Gleichstand der erste.

Die originalen Seitenverhältnisse und Vorzeichen der Bodenversätze bleiben erhalten. Die Anpassung an die bereits normalisierten Gebäudemodelle erfolgt isotrop mit `instance.size / max(nativeSizeX, nativeSizeY)`. Beim aktuellen Stadtzentrum mit Instanzgröße 4 (`src/defs.rs`, unverändert an den Renderer übergeben) ergibt das etwa 4,571 × 3,429 Bodenfläche und −0,286 X-Versatz. Das ist ein dokumentierter Größenadapter, kein rekonstruierter nativer Weltmaßstab.

Die Flächen verwenden ein eigenes GRID32-Gitter. Höheninterpolation und Fragmenttiefe folgen den tatsächlichen beiden Terraindreiecken mit der b–c-Diagonale; damit wird kein flaches schwebendes Bodenrechteck eingesetzt. Ein kleiner eigener Tiefenversatz vermeidet Flimmern. Der Renderer verwendet Originalalpha, Nebel und eigene Beleuchtung; die PNG-Alphaebene selbst bleibt unverändert. Seine Shader-Ausgabe wird für die Alphaüberblendung vormultipliziert, ohne das gespeicherte Bild zu bearbeiten.

Die zugehörigen Spritezeilen enthalten nur `MipMap`. Die in `GDbHdrs/GDbSpriteDefs.h` definierten FlipU-, FlipV- und Rotate-Flags sind dort nicht gesetzt. Der Import exportiert daher keine Bildspiegelung oder Drehung. Er verwendet den vollständigen Bildrahmen: U, V, WIDTH und HEIGHT sind in diesen Spritezeilen jeweils null. Das ist kein willkürlicher Beschnitt auf den sichtbaren Alphabereich.

Bodenflächen erscheinen zunächst nur bei fertigen Gebäuden. Bauvorschau-/Ghost-Flächen sowie die separat gesicherten Death-/Urban-/weiteren Slot-Varianten sind nicht als vollständig integrierte Originalzustände ausgegeben. Der Import verändert weder Terrain, Kollisionen, Baurechte noch Simulationszustand.

FarmWheat besitzt zwar ein echtes UnderFarm-Kind und extrahierte Bilder, aber keine ausdrücklich deklarierte UndertextureSizeX/Y. Dieser Fall steht unter `unresolvedBindings` und erzeugt keine geratene Bodenfläche. Für Dock, Outpost, OilRig und UraniumMine wird kein unbelegtes Under*-Kind ergänzt und kein generisches Pflaster als Originalbindung behauptet.

## Prüfung

`python -m unittest test_extract_ee2_textures test_ee2_ground_decals -v` im Projektverzeichnis `tools` besteht elf Tests: fünf Container-/Decoderprüfungen und sechs neue lokale Integrationsprüfungen.

Die neuen Prüfungen bestätigen alle 125 PNGs gegen die vollständigen dekodierten Original-Topmip-Pixel, PNG-/Texcache-/Archivhashes, die durchgehende DDF-/Sprite-/Seasonal-Kette, Größen und Versätze, vollständige Spriteframes und Alpha sowie den Ausschluss unbelegter Flächengrößen. Die bisherigen 146 Texturcachedateien, elf Gelände-/Oberflächenbilder, 51 Iconbindungen, 66 Iconvarianten und 31 HUD-Bilder bleiben erhalten. Nach zusätzlich sechs nativen Epochen-Symbolquellen umfasst der Texturimport insgesamt 284 unterschiedliche Texturcachedateien und 55 Basis-Iconbindungen; er meldet null Extraktionsfehler. Die sechs gesonderten Prüfungen `test_ee2_epoch_icons` bestätigen diese sechs Spritequellen für acht zuvor fehlende Modellübergänge sowie alle sieben originalen Fortress-Symbole pixelgenau gegen CSV und Texcache. Zusammen bestehen 17 Decoder-/Boden-/Epochen-Iconprüfungen.

Diese Datenprüfungen belegen die importierten Quellen und ihre unveränderten Bilder. Sie ersetzen keine vollständige Sichtprüfung aller Epochen, Gebäudezustände und Kamerapositionen im Original und im Nachbau. Originalmedien und ihre PNG-Ableitungen bleiben ausschließlich lokal und werden nicht in das verteilbare Quell-ZIP aufgenommen.
