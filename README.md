# OpenEpoch · lokaler EE2-Ausbau

Arbeitsstand 05.11.2026 · native13. Das EE2-Profil verbindet die eigene Rust-/WASM-Simulation
mit importierten Originalmodellen, Animationen, Stimmen, Musik und belegten Spielwerten.
Die vollständige Übereinstimmung mit Empire Earth II wird weiter erarbeitet.

Für neue Scharmützel stehen die14 ursprünglichen Völker zur Wahl. Das UP-Profil
enthält80 Einheitendefinitionen und19 zusätzliche Spezialeinheiten mit ihren echten
Modellen, ihren Originalanimationen, Originalsymbolen und deutschen Namen.
Die Völkerwahl begrenzt die Produktion passend zur Epoche. Bereits ausgebildete
Einheiten bleiben nach dem Ende ihres Produktionszeitraums erhalten. Beim Anklicken
und Befehlen folgen ihre Stimmen auch den im Original belegten Elternklassen.

Die Bürgerproduktion wählt männliche und weibliche Bürger nach dem belegten
Originalschritt. Beide haben ihre eigenen Originalmodelle, Animationen, Symbole
und Stimmen aus vier Regionen und fünf Epochenstilen. Auswahl und Befehle nutzen
die tatsächliche Identität der Einheit. Einquartieren, Transport und Laden
erhalten sie. Auch die Todesgeräusche folgen der Identität. Stadtzentren zeigen
über alle 15 Epochen die sechs Originalstile der vier Kulturregionen. Die belegten
Jahreszeitentexturen, Farbvarianten und regionalen Symbole liegen im
Komplettpaket. Für Stadtzentren und Häuser der letzten Epoche werden die
registrierten Originaltexturen aus den vom Patch geladenen Saisontabellen
verwendet. Die aktuelle Scharmützelkarte verwendet gemäßigtes Klima.

Häuser verwenden ihre acht Originalstile mit den passenden Modellen,
Jahreszeiten und Symbolen über alle 15 Epochen. Bei Häusern mit gespeicherten
Detailstufen folgt die Auswahl der Stufe aus der Originalressource.
Häfen verwenden in allen 15 Epochen ihre Originalmodelle und Symbole,
einschließlich der eigenen nahöstlichen Variante der zehnten Epoche.
Kasernen zeigen die sechs Originalstile mit ihren Symbolen sowie Stadt-,
Land- und Winterböden. Das Scharmützelmenü verwendet Originalrahmen,
Pergament und Schaltflächen mit den Bereichen Spiel, Erweitert, Karte und
Umgebung. Die Anordnung passt sich der Fenstergröße an.
Das Komplettpaket enthält 70 Originalmusikdateien. Im Scharmützel bestimmen
Kulturregion und Epoche die Titelauswahl. Im Hauptmenü laufen die beiden
Originaltitel in der aus Ghidra ermittelten Reihenfolge, beginnend an einer
zufällig ausgewählten Stelle. Die Wiedergabe startet nach der ersten
Interaktion und blendet beim Wechsel zwischen Menü und Partie über.
Das animierte Menü nutzt die vollständige Originalschleife, im UP-Profil
mit 2560 × 1440 Pixeln. Die Videos sind für die Browserwiedergabe als MP4 kodiert.

Geschütze und weitere geeignete Einheiten packen beim Bewegen ein und vor dem
Angriff aus. Die Auswahl bietet außerdem die manuellen Befehle Einpacken und
Auspacken. Die originalen Packanimationen verwenden ihre tatsächlichen Anfangs-
und Endposen. Spielerfarben entstehen aus den originalen Textur-Patches; alle
elf Farbzuordnungen sind durch die native Farbtabelle und Namensbildung belegt.

Der Gastgeber überträgt Völker, Start- und Endepoche, Bevölkerungslimit, Startvorräte,
Kartenaufdeckung und Jahreszeiten für neue UP-Mehrspielergefechte. Zufällige Völker
werden vor dem Start aufgelöst und als konkrete Auswahl übertragen. Die älteren
Regelstände speichern keine Endepoche und laden mit der höchsten Epoche als Grenze.
Für die Zufallsauswahl werden erst die Völker und anschließend beide
Epochen aus demselben Präsentationszufallsstrom bestimmt, den auch die Musik
verwendet. Die Partie lässt sich auf eine einzige Epoche begrenzen.

Neue UP-Einzelspielerpartien aus der Steinzeit bieten die rekonstruierten
Aufträge im Stadtzentrum und Lager sowie Veteran und Elite für leichte
Infanterie in der Kaserne. Laufende und bezahlte wartende Aufträge werden
mitgespeichert. Ältere Spielstände behalten ihre bisherige Forschungslogik.

Produktion speichert den tatsächlich bezahlten Preis und die beim Arbeitsbeginn
festgelegte Dauer. Abbruch erstattet den gespeicherten Preis. Ein Klassenwechsel
berechnet wartende Aufträge neu. Aktuelle Spielstände sichern außerdem laufende
Packaufgaben, die für die Fortsetzung notwendige Wegfindung und fliegende
Geschosse in ihrer tatsächlichen Reihenfolge. Der gespeicherte Spielkern
enthält Bürgeridentitäten und den Zustand des zugehörigen Zufallsstroms.
Zusätzliche Datenfelder sichern die Endepoche und die manuelle Forschung.
Fehlende Historie
älterer Spielstände wird ausdrücklich als rekonstruiert gekennzeichnet. Die
bestehenden eigenen/Basis-WASM-Dateien und die bisherigen Speicherorte bleiben
bestehen. Netzwerkprotokoll 17 überträgt die gewählte Endepoche.

Von132 klassischen Quellenfamilien sind62 angebunden;70 sowie weitere Fähigkeiten,
weitere regionale Gebäudestile, Karteninitialisierung und native KI-Abläufe
sind weitere konkrete Arbeitsbereiche. Die aktuelle KI bleibt ein eigener Adapter.

Karavellen und Linienschiffe verwenden die originalen sechs beziehungsweise zwölf
Segelflächen mit getrennten Bewegungszuständen pro Schiff, lokalen Lichtfarben
und Schadensmasken. Alle Bilddurchläufe einschließlich Wasserreflexion verwenden
denselben vorbereiteten Zustand. Pause und Netzwerk-Wartezeit halten die Bewegung
an. Originale lokale Raster und Rechenfolgen sind belegt; Zeittakt, Sichtauswahl
und die Zuordnung zur eigenen Welt bleiben ausdrücklich Adapter.

Die gemeinsamen Prüfungen und der genaue Restumfang werden unter
`../EE2-Research/analysis/native8-*.json` dokumentiert. Die Dateien `native6-*.json`
und `native61-*.json` enthalten die davor geprüften Ausbaustufen.
Browserbedienung und ein physischer Mac wurden dabei nicht ausgeführt.

## Komplettpaket für den Mac

Das native13-Komplettpaket enthält den gemeinsam geprüften Laufzeitstand mit
Originalmedien. Das zugehörige Quellpaket enthält eigenen Code, WASM-Builds und
Prüfungen. Die vollständige Originalübereinstimmung wird weiter erarbeitet.

`OpenEpoch-Mac-Komplett.zip` enthält den fertigen Laufzeitordner einschließlich
aller importierten Originaldateien unter `docs/local-original`.
Den ganzen Ordner entpacken und `Start-OpenEpoch.command` öffnen. Der Starter
startet einen lokalen Server und öffnet den Browser. Nur Python3.8 oder neuer
ist zusätzlich erforderlich; Rust, Ghidra, zusätzliche Python-Pakete und die
Windows-Installation werden zum Spielen nicht benötigt.

Die Anleitung [MAC-START.txt](MAC-START.txt) erklärt Python, Browser und Ton,
den alternativen Terminalstart und lokale Spielstände. Der Server bindet nur
an 127.0.0.1, bevorzugt Port8125 und behandelt belegte Ports. Das Paket wird nach
dem Entpacken separat auf vollständige Medien und exakt geschriebene Pfade geprüft.
Eine tatsächliche macOS-Ausführung wurde hier nicht getestet.

`OpenEpoch-Scharmuetzel.zip` ist das separate Quellpaket
ohne Originalmedien. Für den Mac-Transfer das neue **Komplettpaket** verwenden.

## Enthaltener vorheriger Ausbau · native-ui5

Das Bedienfeld verwendet51 originale Widgetdefinitionen und129 hochauflösende
Originalgrafiken. Minikarte, Befehlsraster, Einheiteninfo, Rohstoffe und Epochenleiste
folgen den rekonstruierten Ankern. Stein, Pergament und Metall wechseln nach den
originalen Epochenbereichen1–5,6–10 und11–15. Die Schaltflächen für Forschung und
Diplomatie stehen links unten; das Menüsymbol rechts neben der Epochenleiste
öffnet Speichern, Ton, Vollbild und Hauptmenü. Die Browserdarstellung der Schrift,
das Befehlsangebot und Teile des Menüverhaltens bleiben angepasste Oberflächen.

Die Diplomatie verwendet jetzt die originalen Reiter, Rahmen, Spielerstreifen
und Vorschlagsfenster. Beziehungen, Ressourcen und Gebietsgeschenke/-verträge
sind an die vorhandenen Spielbefehle angeschlossen; Einheitenübergaben sind
weiter offen. Deutsche Originalnamen, Mehrzahlformen und Beschreibungen folgen
der tatsächlichen Quellenklasse und Epoche. Systemschriften bleiben ein Adapter.

Farmen verwenden vier originale Teilfelder. Alle zwölf Zustände der drei
originalen Feldfamilien sind importiert; aktiv ist der belegte Standardzustand
der Weizenfarm. Die Teilfelder folgen jeweils ihrer Geländehöhe. Gebäudeböden
verwenden nun die aus Ghidra belegten Größen, Achsen und Versätze einschließlich
des Faktors√2. Die frühere Behauptung, rohe DDF-Maße seien bereits endgültige
Weltmaße, ist damit korrigiert.

Wasser nutzt die originalen Skin1/2/4-Texturen, die nativen Farb-/Alphakanäle
und einen getrennten Reflexionspass. Die gespiegelte Szene wird mit dem
quellbelegten Anteil überlagert. UV-Maßstäbe, Zuordnung der eigenen Kartenklassen
und die separate native Sichtauswahl werden weiter rekonstruiert.

`?rules=ee2` startet jetzt die rekonstruierten numerischen Regeln der installierten
UP-Version. Alle57 bisherigen Rollen sind jetzt an Quellen gebunden; Paladin
und LightHerc verwenden ihre tatsächlichen UP-Modelle.279 Klassenintervalle
und313 RPS-Profile bilden Werte und Epochenfortschritte ab.
Das ist weiterhin ein kleineres Einheitenangebot mit offenen Fähigkeiten und
Forschungsfaktoren. Im Menü **Regelversion** bleibt **EE2 · Basisdaten** verfügbar;
die vorhandenen Spielstände dieses Profils bleiben dort erhalten. Die unabhängigen
OpenEpoch-Regeln und ihre Spielstände bleiben ebenfalls getrennt.

Die drei Profile haben eigene WASM-Dateien und Speicherbereiche. Multiplayer
weist verschiedene Regelversionen vor Spielbeginn ab. Die beiden bisherigen
WASM-Dateien sind unverändert; die UP-Datei verwendet jetzt gespeicherte native
Angriffsrunden und verzögerte Schadensereignisse. Netzwerkprotokoll10 verhindert
eine Mischung mit dem früheren Kampfstand. Alte UP-Spielstände werden beim Laden
migriert. Projektile im Flug sind weiterhin flüchtig und damit eine offene
Spielstandlücke. Alle neuen
Bilder, Farmzustände und Wassermedien liegen im Komplettpaket. Die Quellen und
Prüfungen sind unter [UP-Profil](docs/EE2-UP15-PROFIL.md),
[Profil-/Spielstandprüfung](../EE2-Research/analysis/native5-rules-profile-tests.json),
[Diplomatie](../EE2-Research/analysis/native5-diplomacy-integration.json),
[Modellbeleuchtung](../EE2-Research/analysis/native5-lighting-integration.json),
[Farm und Gebäudeböden](../EE2-Research/analysis/native3-farm-ground-integration.json)
und [Wasser](../EE2-Research/analysis/native3-water-implementation.json) dokumentiert.

Modelle nutzen die belegte Farb-/Lichtkette und die passenden statischen bzw.
skinned Materialpfade. Als Anwendungseinstellung ist gemäßigtes Klima am
Nachmittag bei normalem Wetter gewählt. Die vollständige native Tageszeit-,
Wetter- und Terrain-Lichtsteuerung ist noch offen.

Die folgenden Abschnitte dokumentieren frühere Ausbaustände; maßgeblich ist
der oben beschriebene Stand native13.

## Saisonale Modelle, Wasserreflexionen und Kameradrehung · visual-seasons2

Gebäude und Vegetation verwenden jetzt die originalen saisonalen Texturlisten.
106 Nah-/Fernmodelle mit248 Meshbindungen wechseln mit demselben Kalender wie
Gelände und Gebäudeböden. Die Zuordnung kommt aus den NIF-Mesh- und Elterndaten;
191 bestehende Geometrie-/Animationsdateien bleiben unverändert. Ein fehlender
Originalschlüssel einer Stalltür behält ausdrücklich das bisherige Basismaterial.

Die Wasserfläche erhält jetzt ein echtes Szenenbild aus einer gespiegelten
Kamera. Terrain und Modelle werden an der Wasserfläche abgeschnitten, anschließend
wird das Bild in der Wasseroberfläche abgetastet. Native Ziel-/Clip-/Kamerapfade
sind belegt. Wasserfarbe, Materialdetails und die separate native Sichtauswahl
werden weiter rekonstruiert; dieser Stand verwendet noch den gemeinsamen Instanzsatz.

Strg+Pfeil links/rechts dreht die Kamera; Umschalt beschleunigt die Eingabe.
Die drei Kameraknöpfe drehen um45° oder richten nach Norden aus. Auswahlstrahlen,
Schieben und Anzeigen folgen der gedrehten Ansicht. Spielstände erhalten die
Ausrichtung; ältere Spielstände und neue Partien beginnen nach Norden.

Die Simulation und beide WASM-Dateien sind gegenüber combat-season1 unverändert.
Die neuen Prüfungen umfassen echte Produktionsfunktionen für Eingaben und
Speicherstände sowie Szenen mit Sommer, Winter, Monatsübergang, Zukunft und90°-
Ansicht auf einem echten GLES-Framebuffer. Ein Start auf physischem macOS und
vollständige Bildgleichheit mit dem Original sind weiterhin nicht geprüft.

[Saisonale Modelle](../EE2-Research/analysis/native-model-season-integration.json),
[Reflexionspipeline](../EE2-Research/analysis/native-reflection-implementation.json),
[Kameradrehung](../EE2-Research/analysis/native-camera-heading-evidence.json),
[Eingaben und Spielstände](../EE2-Research/analysis/visual-controls-tests.json).

## Kampfregeln und Jahreszeiten · combat-season1

Die Schadensberechnung verwendet für zugeordnete Einheiten die belegte BASE-RPS-
Matrix mit 35 Gegenklassen, 269 Klassen-/Epochenprofilen und 221 spezifischen
Upgradefaktoren. Einheiten mit einem Faktor null werden nicht mehr als gültiges
Angriffsziel angenommen. Geschosse behalten ihr Angreiferprofil vom Abschuss;
Flächenschaden wird für jedes Opfer neu berechnet und fällt für Nebenziele nach
der belegten quadratischen Distanzkurve ab. Die eigene Verteidigungsforschung
und nicht zugeordnete Profile bleiben begrenzte Adapter.

Im Menü sind Startmonat, fester Monat oder Jahreszeitenwechsel und eine Dauer
von 1/5/10/20/30 Minuten pro Jahreszeit verfügbar. Gelände und Gebäudeböden
blenden die originalen Bildfolgen kontinuierlich über. Beide verwenden denselben
gespeicherten Simulationstick; Pause und Netzwerkwarten halten den Kalender an.
Spielstände speichern die Einstellung, ältere Spielstände behalten festen Sommer.
Im Mehrspieler bestimmt der Gastgeber die Jahreszeiten. Protokoll9 trennt diesen
Kampfstand von älteren Spielern.

Gebäudeböden berücksichtigen jetzt native Epochenbereiche und Rural/Urban-
Varianten samt originalen Größen und Versätzen. Ein einmal gewachsener Stadtradius
bleibt auch nach einer Übergabe an einen Spieler mit niedrigerer Epoche erhalten.
Abriss und Neubau beginnen einen neuen Radius; Spielstände erhalten das Maximum.
Alte Spielstände bleiben ladbar. Der Stadtbereich verwendet noch fertige
Stadtzentren statt der vollständigen originalen CityManager-Registrierung.
Die BASE-Daten bleiben absichtlich konsistent mit den bisherigen Einheitenwerten;
die untersuchte UP-EXE lädt für Kämpfe abweichende *5-Tabellen. Eine vollständige
Gleichheit mit diesem installierten Patch ist deshalb nicht belegt.

Die Kampfprüfungen umfassen290.363 WASM-Abfragen. Die Stadtradius-Erweiterung
ergänzt elf gezielte Tests für Lebensdauer, Übergabe, Grenzen und Spielstände.
75 EE2-/32 Basis-Rusttests und425 integrierte Kalender-/Netzwerkprüfungen bestehen.
Vier echte WASM-Weltszenen wurden mit den Produktionsshadern auf der GPU geprüft.
Wetterereignisse, volle Original-KI, alle Zivilisationen, Spezialfähigkeiten,
der gesamte Einheitenkatalog und native Beleuchtung sind weiterhin unvollständig.
Es handelt sich weiterhin nicht um ein fertiges1:1-Empire-Earth-II.

[Kampfintegration](../EE2-Research/analysis/native-combat-integration.json),
[Stadtradius](../EE2-Research/analysis/native-city-ground-integration.json),
[Kalenderbelege](../EE2-Research/analysis/native-calendar-evidence.json),
[GPU-Weltszenen](../EE2-Research/analysis/native-season-world-qa.json).

Die folgenden Abschnitte dokumentieren die vorherigen Ausbauschritte; deren
Hashes und damalige Grenzen gehören jeweils zum genannten Stand.

## Kamera, Maßstab und Gelände · native-render1

Die Darstellung verwendet jetzt den mit Ghidra belegten Kamera-Updatepfad:
Entfernung, Pitchkurve, horizontaler Bildwinkel und Frustum folgen der untersuchten
EXE und der vorhandenen Kamerakonfiguration. Der ungewöhnliche interne Zugriff auf
MINPITCH als Knickschwelle wird beibehalten. Zoomen, Schieben und Auswahlstrahlen
verwenden dieselbe Projektion. Alte gespeicherte Pixelzooms werden umgerechnet.

Belegte Klassenmaßstäbe und Modellursprünge ersetzen bei zugeordneten Modellen die
bisherige individuelle Würfelnormalisierung. Nicht belegte Zuordnungen bleiben
ausdrücklich beim bisherigen Adapter. Kollisions- und Gebäuderaster sind weiterhin
die der eigenen Simulation; die Kameraausrichtung bleibt an deren Norden gebunden.

191 vorhandene Nah-/Fernmodell-Dateien behalten ihre Geometrie und Animationen.
Zusätzlich sind originale Ambient-, Diffuse-, Specular-, Emissive- und Glanzwerte
sowie relevante Materialzustände enthalten. Der Renderer berücksichtigt native
Alpha-/Tiefenzustände und aktivierte Glanzlichter. Die vollständige native
Beleuchtungs- und Shaderauswahl ist noch nicht rekonstruiert. Schatten folgen nun
dem Höhenfeld; ihre Projektion ist weiterhin ein eigener Darstellungsadapter.

216 weitere Bilder enthalten Gelände-/Jahreszeitentexturen, die originale
Alphamaske und Wasserbilder in Originalauflösung. Die Darstellung liest echte
Geländeklassen der eigenen Karte statt Farben der Minimap. Terrainübergänge,
UV-Maßstäbe und Teile des Wassers bleiben eigene Adapter; native Wasserbewegungs-
und Verzerrungskonstanten sind aus den vorhandenen Shaderdateien übernommen.
Der Spielstart verwendet Sommer. Die importierten Jahreszeitensätze bedeuten
noch keinen vollständigen originalen Wetter- oder Jahreszeitenzyklus.

Diese Fassung wurde ohne Computersteuerung oder gestartetes Originalspiel geprüft:
Produktion-Shader und aufgezeichnete Weltpassagen liefen auf einem nativen
GLES-/ANGLE-Framebuffer. Das ersetzt keine vollständige Browser-Bedienprüfung
oder einen Start auf einem echten Mac. Zwölf Szenarien mit je 600 Ticks haben
gegenüber fortress1 identische Simulationshashes, Rohstoffe und Speicherstände.
Die neue WASM-API ergänzt ausschließlich lesende Geländeklassen-Exporte.

[Kamera-/Skalierungsbelege](../EE2-Research/analysis/native-render-evidence.json),
[vollständige Medienprüfung](../EE2-Research/analysis/native-render-portable-assets.json),
[unveränderte Simulation](../EE2-Research/analysis/graphics-build-equivalence.json).

## Festungen und Gebietserwerb · fortress1

Die Festung ist jetzt als eigenes Gebäude verfügbar: sieben originale Modell- und
Symbolstufen decken die Epochen1–15 ab. Drei echte reduzierte Fernmodelle,
Originaltexturen, Spielerfarben, Gebäudeboden und Auswahlklang sind enthalten.
Das EE2-Profil hat damit57 Definitionen; das Basisprofil bleibt bei53.

Für den Erwerb eines angrenzenden freien Gebiets reicht ein fertiges Stadtzentrum.
Ein abgelegenes Gebiet braucht zusätzlich eine fertige eigene Festung. Ein erstes
Stadtzentrum kann auch ohne vorhandenen Nachbarn ein Reich gründen. Baustellen
zählen für diese Mindestanforderungen noch nicht. Diese Zählung und die
Nachbarschaft über gemeinsame Rasterkanten sind im Originalcode nachgewiesen.
Bereits erworbener Besitz geht beim Verlust der Festung nicht wieder verloren.
Die KI bevorzugt angrenzende Expansion und ergänzt eine Festung bei isolierten
Stadtzentren. Alte56er-Spielstände behalten ihre vorherigen Gebietsrechte.

Die Diplomatiekarte erlaubt nun auch die Auswahl bekannter freier Gebiete. Der
Bereich „Gebiet ansehen“ zeigt den eigenen Anspruch und fehlende Voraussetzungen;
„Auf der Karte zeigen“ springt dorthin. Bauvorschauen für Stadtzentrum und Festung
zeigen die jeweilige Anforderung. Bereits vorhandenes Verschenken und Tauschen
bleibt verfügbar und löst keine neue Nachbarschaftspflicht aus.

Rechtsklick mit zulässigen eigenen Einheiten auf die Festung quartiert sie ein;
„Ausquartieren“ entlässt sie. Zehn Garnisonsplätze, drei Prozent Angriffsbonus je
belegtem Platz und die epochenabhängige Heilung kommen aus den Originaldaten.
Die Platzbelegung ist durch den PopCount-Getter und seine Datentypregistrierung
belegt. Die Anzeige unterscheidet Einheiten und belegte Plätze. Festungsforschung
wirkt nun auch auf ID56. Drehen, Baukosten, Nachladezeit, Geschossart, Garnison,
Heiluhr und Gebietsanspruch bleiben in Spielständen und Lockstep erhalten.

Die Gebietsbaugrenze zählt für Festungen fertige Gebäude aller Spieler sowie
eigene Baustellen, entsprechend dem Originalprüfer. Fremde Baustellen zählen
beim Setzen noch nicht mit. Der Originalzustand „umstritten“ nach Verlust eines
Stadtzentrums ist weiterhin offen; der Nachbau gibt das Gebiet dabei frei.
Eigene Maßstabs-/Boundingboxregeln, blockierende Fundamente, deterministische
Anspruchsreihenfolge und gespeicherte Garnisons-HP in Ein-Prozent-Schritten
bleiben ausdrücklich begrenzte Adapter. Die vollständige native RPS-Matrix,
alle Spezialfähigkeiten, Kultursets und das gesamte Einheitenangebot fehlen noch.

[Quellen und Ghidra-Adressen](../EE2-Research/Festung-und-Gebietserwerb.txt),
[belegter PopCount-Slotgetter](../EE2-Research/analysis/fortress-population-binding.json),
[Fortress-Prüfungen](../EE2-Research/analysis/fortress-territory-tests.json),
[Regressionen](../EE2-Research/analysis/fortress-regression-tests.json).

## Bauen und Vollbild · building2

Der Vollbildbutton steht im Menü und im Spiel bereit; F11 schaltet um.
Das ist echtes Browservollbild. Eine vom Browser blockierte Anfrage wird
verständlich gemeldet. Die Bauvorschau zeigt Ausrichtung, belegte Kachelfläche
und den konkreten Grund, wenn ein Bauplatz nicht benutzt werden kann.

Strg, R sowie Komma/Punkt drehen die Vorschau standardmäßig um 90°. Das Original
benutzt eine neue Strg-Tastenflanke für gewöhnliche Gebäude. Umschalt ermöglicht
zusätzliche 45°-Schritte; Umschalt+Mausrad dreht ebenfalls um 45°. Die 45°-Option und
ihre konservative quadratische Kachel-Boundingbox sind eigene Adapter.
Diese Ausrichtung bleibt beim Bauen, Speichern/Laden und im Mehrspieler erhalten.
Fertige Gebäude werden dadurch nicht nachträglich versetzt oder umgebaut.

Der Hafen richtet sich zunächst an einer erkannten begehbaren Küste aus.
Die manuelle Korrektur ist eine zusätzliche Bedienoption; „Küste: automatisch“
stellt den Automatikmodus wieder her. Das Original überschreibt die normale
Strg-Drehung beim Dock und leitet seinen Winkel aus der Küstennormale ab.
Unser Rasteradapter prüft mindestens 10% Land, 25% Wasser und drei Wasserzellen,
begehbares Ufer und einen freien Seeausgang. Die genaue native Wasserklassifizierung,
Terrainhöhenprüfung und kontinuierliche Normalenberechnung sind nicht gleichgesetzt.

„Erkundet“ und „vollständig aufgedeckt“ gelten jetzt auch bei der Bauprüfung als
bekanntes Gelände. Sie geben der KI weiterhin keine zusätzliche Kampfsicht.
Ein Stadtzentrum darf auf freiem, erreichbarem Land gebaut werden; maximal eines
pro Gebiet, einschließlich vorhandener Baustellen. Eigene ausgewählte Bürger,
erreichbare Außenkante, Epoche, Kosten und das Einheitenlimit werden geprüft.
Der bisherige stille Erfolg ohne Bauarbeiter ist beseitigt. Gewöhnliche Gebäude
dürfen auf neutralem Land gebaut werden und benötigen dort die doppelte Bauzeit;
Stadtzentren sind vom territorialen Bauzeitfaktor ausgenommen.

Die frühere Baustellen-Claimregel ist im fortress1-Stand durch den oben
beschriebenen belegten Mindestanforderungscheck für fertige Gebäude ersetzt. Fremdes Land bleibt ohne das
vollständige Grenzrechtesystem für Neubauten gesperrt; die originale Regel mit
dreifacher Bauzeit ist damit nicht vollständig umgesetzt. Alte Spielstände mit
mehreren Stadtzentren bleiben ladbar. Das Speicherlayout ist unverändert;
Dieser vorherige Ausbauschritt verwendete Protokoll8 für Festungen und die neuen Gebietsregeln.

[Originalbelege und Ghidra-Adressen](../EE2-Research/Stadtzentren-und-Gebaeuderotation.txt).

Aktueller Enginebuild: EE2 `8B5C1A021F2642DBEFE9F6FA8BBFB7B0BD273E366B07B63312C6581FECCD69FD`,
Basis `FEE97BEC031288416FDAFBBAD83CE838D3C00E9971EFC399ED908F57F12BED4D`.
Die Rustprüfungen bestehen48/48 im EE2-Profil und32/32 im Basisprofil.
Die späteren 6E-/Grafikberichte dokumentieren den vorherigen Stand.

## Grafikausbau · graphics2

113 weitere Originaltexturen ergänzen die Gebäudeböden. Zwölf Gebäudezuordnungen
sind durch ihre Under*-Kinder und DDF-Maße belegt; die Darstellung folgt dem
Terrain und verwendet das unveränderte Originalalpha. Die native Auswahl der
drei Texturslots nach Epoche sowie Rural/Urban ist noch ungeklärt. Die aktuelle
Darstellung verwendet ausdrücklich Rural-Slot 0 als eigenen Adapter.
[Quellen und Grenzen](../EE2-Research/Originale-Gebaeudeboeden.md).

Doppeldecker, Bomber und Hubschrauber verwenden nun die originalen Idle-,
Bewegungs- und Angriffsclips mit 48 Samples für bewegte Propeller/Rotorteile.
Konstante Teilposen benötigen nur einen Frame. Alle 39 vorhandenen mobilen
Definitionen haben quellengeprüfte Auswahl-, Bewegungs- und Angriffsantworten.
26 spätere Epochenbindungen verwenden vorhandene, identische Originalmodelle
und passende Originalsymbole. Die Stimmen wechseln in denselben belegten
Intervallen mit. Der Mech-Gang verwendet jetzt 24 statt acht Samples; beim
Myrmidon bewegen sich die originalen Radgruppen durch Idle-/Walk-KFs.
Der ausdrückliche A-Klick-Befehl verwendet das Angriffsereignis; alle drei
Sprachaktionen werden beim Audiostart vorgewärmt. 194 aktive Audiodateien wurden
im echten Browser erfolgreich dekodiert. Die vollständige Ereignisregie des
Originalspiels ist damit noch nicht umgesetzt.

## Start und lokaler Import

Im Projektordner zuerst die Originalmedien aus der vorhandenen Installation importieren:

```powershell
powershell -ExecutionPolicy Bypass -File .\Import-EE2-Assets.ps1
```

Danach `Start-OpenEpoch.cmd` starten:
<http://127.0.0.1:8125/?rules=ee2&build=visual-seasons2>.
Im Menü sind alle 15 Startepochen auswählbar; Standard bleibt die Steinzeit.
Eine Endepoche ist noch nicht konfigurierbar. Die Startepochen-API erlaubt
Änderungen nur vor dem ersten Tick im Einzelspieler.

Der Import liest standardmäßig `C:\Program Files (x86)\GOG.com\Empire Earth II`.
`-Installation 'D:\Spiele\Empire Earth II'` wählt einen anderen Pfad;
`-Python` wählt den lokalen Interpreter. Für den Medienimport sind Pillow und
NumPy erforderlich. Texturen, Modelle, native Fern-LODs, Audio und Spielerfarben
werden unter `docs/local-original/` erzeugt. Originalarchive, EXE-Dateien und
Original-Spielstände bleiben unverändert.

Das Quell-ZIP enthält keine Originalmedien oder konvertierten Medienableitungen.
Es enthält die Importer, Tests, Zahlentabellen und Attributionshinweise.
Beim Regenerieren der Einheitenwerte muss nach dem Modell-/Assetimport
`python tools/import_ee2_units.py` ausgeführt werden: Der VISUAL-/NIF-Abgleich
benötigt das lokale Modellmanifest und bricht bei dessen Fehlen ausdrücklich ab.
Für einen anderen Quellpfad gilt hier `--install "D:\Spiele\Empire Earth II"`.
`Build-All.ps1` baut anschließend beide WASM-Profile aus dem Quellstand.
Der vorhandene Darstellungs-/Audiofallback bleibt bei fehlenden Medien verfügbar.

## Kartensicht und Diplomatie

Vor dem Scharmützel stehen in beiden Profilen drei Kartenmodi zur Wahl:
unerforscht mit Kriegsnebel, erkundet mit Kriegsnebel und vollständig aufgedeckt
ohne Kriegsnebel. „Erkundet“ zeigt das Terrain, gibt aber keine dauerhafte Sicht
auf gegnerische Einheiten. Der gewählte Modus bleibt im Spielstand erhalten;
im Mehrspieler übermittelt der Host ihn mit Protokoll 6 an den Gast.
Nach Ende gemeinsamer Sicht verschwinden bewegliche Partnereinheiten außerhalb
der eigenen Sicht; gelerntes Terrain bleibt erkundet. Bereits bekannte Gebäude werden unter Nebel
noch aus dem aktuellen Einheitenzustand dargestellt; eingefrorene Geometrie-/HP-
Momentaufnahmen der letzten Beobachtung sind noch nicht implementiert.

`F4` oder der Diplomatiebutton öffnet die Beziehungen. Krieg und Neutralität
werden je Richtung geführt; die Haltung des anderen Spielers ist sichtbar.
Friedensangebote und symmetrische Allianzen müssen angenommen werden und können
abgelehnt oder zurückgezogen werden. Ein Bündnisangebot wählt volle gemeinsame
Sicht oder keine gemeinsame Sicht; vorausgewählt ist die quellbelegte volle
Sicht. Beide Partner erhalten die vereinbarte Sicht erst nach Annahme. Auch
spätere Änderungen der gemeinsamen Sicht benötigen ein angenommenes Angebot.
Freie Allianzen ändern das Ziel des Einzelsiegs nicht.

Rohstoffgeschenke sind im EE2-Profil über die neun Konten möglich, im Dialog
stehen die zur Epoche aktiven Ressourcen zur Wahl. Das Basisprofil verwendet
seine drei bisherigen Ressourcen. Die Antwortlogik der eigenen KI ist eine
eigene Umsetzung und keine Rekonstruktion der Originalverhandlung.
Die KI antwortet nach drei bis vier Simulationssekunden. Vor Ablauf der ersten
60 Sekunden verlangt sie für Frieden mindestens 100 geschenkte Rohstoffeinheiten;
danach ist Frieden ohne Geschenk möglich. Für eine Allianz benötigt sie 30 Sekunden
gegenseitigen Frieden und insgesamt 200 geschenkte Einheiten mit gemeinsamem
Gegner oder 400 ohne gemeinsamen Gegner. Sie akzeptiert nur, wenn beide noch
keinen anderen Bündnispartner haben. Bei Pause läuft diese Antwortzeit nicht weiter.
Gebäude-/Mobil-Sichtstufen, Zeitverträge, Grenzrechte, Zölle und separate
Geschenke ausgewählter Einheiten gehören noch nicht zum implementierten Teilumfang.
Originalbelege: `../EE2-Research/Diplomatie-und-Sicht.txt` und
`../EE2-Research/analysis/diplomacy-visibility-evidence.json`.
Die bestätigten Diplomatie-/Kartensichtbelege stehen unten als vorheriger
Stand. Die neuen WASM-Prüfungen und die gezielte Browserabnahme sind bestanden.

## Territorien verschenken und tauschen

Im Diplomatiefenster gibt es den Reiter „Territorien“ mit Gebietslisten,
Kartenübersicht und Kamerafokus. Ein Geschenk benötigt eine zusätzliche
Bestätigung mit aktueller Vorschau. Die Abgabe des letzten Territoriums warnt
vor aufgehobenen Bündnissen und möglicher Niederlage, falls keine Gebäude
bleiben. Ein direktes Geschenk ist auch an einen feindlichen Spieler möglich.

Beim Tausch werden ein eigenes und ein gewünschtes Gebiet angeboten. Erst die
Zustimmung des anderen Spielers überträgt beide Richtungen gemeinsam aus einem
Snapshot. Für einen Tausch müssen beide Haltungen friedlich oder verbündet sein.
Besitz und Teilnehmer werden bei Angebot und Annahme erneut geprüft; Angebote
können angenommen, abgelehnt oder zurückgezogen werden und laufen nach einer
Minute Simulationszeit ab. Unvollständige gegnerische Zahlen werden als sichtbare
Mindestwerte kenntlich gemacht; ein Angebot legt die betroffenen Flächen offen.

Die eigenen Gebäude sowie Feld-, Luft- und Seeeinheiten im Gebiet wechseln
den Besitzer; Drittspielerobjekte bleiben bei ihrem Besitzer. Besatzung/Cargo
wird ebenfalls mitgegeben. Alle Bau-/Einheiten-/Forschungs-/Epochenaufträge
werden vorher normal abgebrochen und dem alten Besitzer erstattet.
Technologien, Epoche und Rohstoffbanken wechseln nicht mit. Baustellenfortschritt
und Gesundheitsanteil bleiben erhalten; es gelten die Werte des Empfängers.
Bewegungs-/Angriffs-/Arbeitsaufträge, Ziele und Sammelpunkte werden zurückgesetzt.
Besatzung oberhalb des aktiven Empfängerlimits bleibt ausquartierbar erhalten.

Geschenkte Fördergebäude umgehen keine Epochenfreischaltung: Ölbohrturm und
Uranmine behalten ihre Besatzung, fördern aber nur einen beim Empfänger aktiven
Rohstoff. Universität, Ablieferbonus und Förderung nutzen höchstens das aktuelle
Besatzungslimit des Empfängers; überschüssige Insassen bleiben ausquartierbar.
Auch Produktion und Forschung folgen weiterhin seinen Voraussetzungen. Eine
getragene Bürgerladung bleibt nur erhalten, wenn ihr Rohstoff beim Empfänger
nutzbar ist. Inaktive oder abgelaufene Ladung wird wie bei der normalen
Rückgabe verworfen, ohne Gutschrift auf ein Rohstoffkonto. Das sind die eigenen
Abwicklungsregeln des Nachbaus, keine zusätzlich behaupteten Originalregeln.

Die Engine benutzt bestehende Objekte und verlangt ein gültiges Stadtzentrum
als Gebietsanker. Diese konkrete Abwicklung, die vollständige Cargo-Mitgabe,
Auftragsrückzahlungen und der atomare Snapshot sind eigene Adapter. Die
Originalquellen belegen den allgemeinen Gebiets-/Objekttransfer, aber nicht
jeden dieser Sonderfälle. Die KI antwortet nach vier Simulationssekunden und
bewertet Gebiet plus Objektkosten: unter Verbündeten mindestens gleichwertig,
sonst mindestens zehn Prozent Mehrwert. Das ist eine eigene Preisregel,
keine rekonstruierte Original-KI. Mehrspieler verwendet jetzt Protokoll 6;
TER1 ergänzt das Speicherformat um 524 Bytes hinter dem unveränderten DIP1.

Originalbefunde: `../EE2-Research/Territorien-und-Diplomatie.txt` und
`../EE2-Research/analysis/territory-evidence.json`. Build-/WASM- und Browserbelege stehen unten; die nachfolgenden 991E-Belege
beschreiben
den vorherigen Diplomatie-/Kartensichtstand.

## Scharmützel und Wirtschaft

Das EE2-Profil bietet 15 Epochen bis zum Synthetikzeitalter und neun Konten für
Nahrung, Holz, Stein, Gold, Zinn, Eisen, Salpeter, Öl und Uran. Die Oberfläche
zeigt die jeweils fünf oder sechs aktiven Rohstoffe. Nahrung, Holz, Stein und
Gold bleiben aktiv; die Spezialrohstoffe haben belegte Epochenfenster:

| Rohstoff | Originalepochen |
|---|---|
| Zinn | 1–6 |
| Eisen | 4–9 |
| Salpeter | 7–12 |
| Öl | 10–15 |
| Uran | 13–15 |

Bürger sammeln regulär 0,85 Einheiten pro Sekunde, an Farmen 4,37, tragen
höchstens 15 Einheiten und liefern zum Stadtzentrum oder Lagerhaus.
Neue Karten verwenden die Originalvorratsmengen: 1.277 Holz pro Baum und
1.000 Einheiten an den übrigen natürlichen Knoten. Drei Bürger je Baum und
sechs je anderem direkt sammelbaren Knoten sind als Belegungsgrenzen geprüft.
Farmen beschäftigen im EE2-Profil einen Bürger. Tests decken Belegung vor und
nach dem Bau ab; die Farmgrenze gilt ausschließlich für das EE2-Profil.

Ölbohrturm/OilRig (ID 53, Originalepoche 10) und Uranmine (54, Epoche 13)
benötigen die passende Lagerstätte. Je sechs einquartierte Bürger erzeugen
0,75 Einheiten pro Bürger und Sekunde. Ganze Einheiten werden gutgeschrieben,
Bruchteile bleiben erhalten. Stadtzentrum und Lagerhaus haben fünf Plätze und
geben bei der tatsächlichen Abgabe je Bürger 2 % Rohstoffbonus.
Die neun Vorräte werden auf 0 bis 999.999 begrenzt.

Die fünf Startvorratsprofile Low, Defensive, Standard, High und DeathMatch
stammen aus `db/Simulation/dbresourcelevels.csv`; der aktuelle Start verwendet
die belegten Epoch-1-Zeilen. Diese Presets ersetzen keine vollständige
Rekonstruktion aller Original-Scharmützeloptionen und Kartenverteilungen.

## Originalwerte, Produktion und Forschung

`docs/ee2-units.json` dokumentiert für57 vorhandene Spieldefinitionen
111 strikte Modellvarianten und 262 Klassen-/Epochenrecords.
Kosten in neun Slots, Lebenspunkte, Bauzeiten, Sicht, Bevölkerungskapazität
und Garrisonwerte folgen der aktuellen Originalklasse. Klassenwerte schließen
Lücken des kleineren Grafik-/Einheitenangebots; abweichende Identität und
Grafikvariante bleiben ausdrücklich als Rollenadapter markiert.
Native Weltgrößen und Bewegungsraten wurden nicht ohne bestätigte Skalierung übernommen.

Die Produktionsorte folgen den Quellzeilen: Stadtzentrum, Kaserne, Mühle (48),
Stall (49), Werft, Flugplatz und HERC-Anlage (55, Originalepoche 14).
Tanks bleiben der originalen Klasse Stable zugeordnet, Artillerie der Mill.
Der ältere Workshop-Typ 4 bleibt als Rollenadapter für bestehende Spielstände.
Diese Zuordnung ist kein Nachweis eines vollständigen Original-Produktionskatalogs
mit allen Bauvoraussetzungen, Fähigkeiten und Spezialfällen.

180 Haupttechnologien aus der ausdrücklich gewählten Basis `zips/db.zip`
behalten ihre IDs. Universitäten nehmen zunächst drei Bürger mit je 0,03
Technologiepunkten pro Sekunde auf. Sechs Technologien der aktuellen Epoche
und die nötigen Technologiepunkte erlauben den 15 Sekunden dauernden Aufstieg.
Forschungsbaum und KI-Forschung sind vorhanden.

187 Runtimeeffekte sind zugeordnet, 82 referenzierte Faktoren bleiben offen.
Sechs reine Tooltipfaktoren erzeugen keinen Zusatzbonus. Stein, Gold und Eisen
besitzen getrennte Forschungsfelder; drei Originaltechnologien erhöhen die
Kapazität jedes Hauses um jeweils zwei. `docs/research-audit.json` weist
Quellen, Rollenadapter und offene Wirkungen aus.

Im Einzelspieler öffnet Enter den Cheatdialog. Zuerst `icheat` aktivieren:

- `loot`: +10.000 je Rohstoffkonto, begrenzt durch die Vorratsobergrenze.
- `sea monkeys`: Sofortbau und Einheitenproduktion an/aus.
- `give tech`: +50 reguläre Technologiepunkte im EE2-Profil.
- `convert` (UP): Die zuletzt angeklickte übernehmbare Einheit oder das Gebäude wechselt den Besitzer.
- `idontcheat`: Cheats deaktivieren.

## Originalgrafik und Originalton

149 Modellbindungen auf 118 Nahmodelle decken 56 Spieldefinitionen und
11 Natur-/Requisitentypen ab. 63 echte Fernmodelle bedienen 85 Bindungen.
Bürger besitzen sieben aus Original-KFs abgetastete Aktionen mit passenden
Werkzeugen; insgesamt sind 72 KF-Quellen referenziert. Weitere Original-KFs
animieren Infanterie, Reiterei, Fahrzeuge und Artillerie.

265 Original-Texturcachedateien liefern unter anderem elf Gelände-/Oberflächenbilder,
54 Basis-Icons, 66 Epochenvarianten sowie 31 HUD-Bilder und 31 Ausschnitte.
Hinzu kommen 26 genaue Symbolbindungen für die neuen Epochenübergänge und
113 Original-Bodentexturen; diese Kategorien teilen teilweise dieselben Bilder.
Modelle nutzen 172 Materialtextur-URLs. 146 Spielerfarbfamilien enthalten
1.606 Originalpatchvarianten; nur bestätigte Einzelvarianten sind freigegeben.
Blau/Rot/Grün/Gelb entsprechen A/B/C/G.

194 bytegetreue Original-Audiodateien plus eine dokumentierte PCM-Ableitung
ergeben 195 Audioassets. Bürgerantworten, Arbeit, Gebäudeselektion, Umgebung und
Musik sind eingebunden. Im echten Browser wurden alle 194 aktiven Wiedergabedateien
dekodiert; Angriffsantworten und die neuen Epochenquellen wurden abgespielt.
Für Ölbohrturm und Uranmine ist kein Auswahlklang belegt; ihre Arbeitsloops
werden nicht als Klickklänge verwendet. Stimmen, Musik, Stummschaltung und
Lautstärke bleiben bedienbar.

Der Grafikabgleich bestätigt 137 von 262 betrachteten nativen Klassen-/Epochen-
Visualzeilen; 125 bleiben ohne vollständigen Originalwechsel. Das ist keine
Fertigstellungsquote des ganzen Spiels. Weitere Kulturen, Modelle, Animationen
und Effekte fehlen weiterhin. Der vollständige Einheiten-/Produktionskatalog
und Spezialfähigkeiten sind ebenfalls offen.
[Grafikabdeckung](../EE2-Research/analysis/graphics-coverage.txt).

Die korrigierten Bürger-/Skinning-Exporte bestehen die unabhängige Datenprüfung.
Industrie-Bürger mit 50/50 HP sind im echten Browser sowohl als Near- als auch
Far-Modell geprüft; die Nahansicht zeigt normale Körperproportionen, passende
Füße und Auswahlring. Das belegt diese Ansichten und ersetzt keinen systematischen
Vergleich aller Einheitentypen und Animationsposen mit dem Original.

Quellarchive, Einträge und Hashes bleiben prüfbar. Die Originalmedien machen
einen visuellen Rollenadapter nicht automatisch zu einer identischen Einheit.
Beispielsweise verwendet „Marine“ die echte AssaultRifleman-Grafik, bleibt
aber ein eigener Regel-Archetyp.

## Steuerung, Speicherstände und Mehrspieler

Linksklick/Ziehen wählt aus; Rechtsklick befiehlt Bewegung, Angriff, Arbeit oder
Bau. `A` plus Klick erteilt Angriffsmarsch, `F` wählt untätige Bürger, `H` springt
zum Stadtzentrum. Mausrad zoomt; Pfeiltasten, Bildschirmrand und mittlere
Maustaste bewegen die Kamera. `F4` öffnet Diplomatie, `M` schaltet Ton, `P` pausiert, `Ctrl+S` speichert.

Das versionierte Speicherformat erhält die neun Konten, Forschungsbits,
Garrisonbelegung und Bruchteilfortschritte. Frühere 47-/53-/55-Definitionen-
Spielstände werden migriert. Die jüngste Ergänzung überträgt alte Lebenspunkte
prozentual auf die neuen nativen Maximalwerte. Die Migration für 47-/53-/55-
Definitionen-Stände ist geprüft, einschließlich Forschungs- und Vorratserhalt.
Inkompatible alte Forschungsaufträge werden erstattet. Vor dem ersten
Überschreiben einer neuen Sitzung wird der vorige Stand gesichert.
`?rules=ee2&qa=1` verwendet einen getrennten Browser-Speicherbereich.

Mehrspieler bleibt WebRTC 1 gegen 1 mit manuellem Codeaustausch, jetzt Protokoll 6.
Die neun Konten, Kartensicht sowie Diplomatie-/Territorienbefehle gehören zum synchronisierten Zustand. Es gibt keinen Relayserver.
Eine Abnahme zwischen physischen Rechnern und verschiedenen NATs steht aus.

## Prüfstand und verbleibende Grenzen

Historischer Territorienbuild vor fortress1, SHA256:

- EE2: `6E133AE7DC456FF196D0A6EA98EEB0848688DC4CD9BEDB1126899BA37D860724`
- Basis: `73383515FBF8D83277F1B1CE69EB90AC1C18C76F2D5C4F7240E64185A0A106FF`

Die finalen Quellen bestehen 29 Rust-Tests im EE2-Profil und 25 im Basisprofil.
Der finale 6E-Build besteht 3.682 Territorienchecks mit 618 Lockstep-Runden und
3.177 Diplomatiechecks mit 632 Runden. Geprüft sind Gebietsbesitz und Baugrenzen,
Geschenk/Tausch, Drittobjekte, eigene Cargo-/Queue-Adapter, Empfängerwerte und
Freischaltungen, letzte Gebiete, Angebots-/Eigentumsrennen, Sichtinformationen,
ungültige Netzbefehle, Speicherstände und alte 991E/DIP1-Spielstände.

Die vorherige 5C-Fassung bestand alle bisherigen Kernregressionen und die
20-/10-Minuten-KI-Stressfixtures ohne Warnungen/Progressionsfehler. Danach änderte
sich die Reihenfolge der UI-Loganzeige. Zehn repräsentative echte WASM-Fälle
mit 877 Prüfungen bestätigen für 5C→6E gleiche Simulation und bytegleiche
Speicherstände trotz unterschiedlicher UI-Leserate; die korrigierte Chronologie
wurde separat geprüft. Der lange Soak wurde nicht als neuer 6E-Lauf ausgegeben.
Belege: `../EE2-Research/analysis/territory-tests.json`,
`../EE2-Research/analysis/diplomacy-tests.json`,
`../EE2-Research/analysis/territory-logsort-equivalence.json` und
`../EE2-Research/analysis/economy-ai-soak.json`.

Die gezielte echte Browserabnahme ist bestanden. Zwei lokale Browserclients
verbanden sich über die echte WebRTC-Einladungs-/Antwortoberfläche mit Protokoll 6
auf dem 5C-Vorstand: Frieden wurde bilateral angenommen, Territorium 1 gegen 2
getauscht; beide hatten danach ein Gebiet und 5/40 Bevölkerung. Nach gerichteter
Kriegserklärung gab der Host sein letztes Gebiet mit Bestätigung ab: 0 Gebiete/
0 Bevölkerung beim Schenker, 2 Gebiete und 10/80 beim Empfänger. Kein Desyncstatus;
Kamerafokus und H zeigten nach dem Tausch das neue eigene südöstliche Gebiet.

Der finale 6E-Build mit UI territory2 wurde zusätzlich ohne Cheats im Einzelspieler
bedient: fünf Bürger erkundeten und bauten ein zweites Stadtzentrum für 300 Holz
und 300 Stein. Danach 2 Gebiete und 5/80 Bevölkerung. Ein Geschenk von Territorium 13
zeigte ein Stadtzentrum/fünf Bürger; Abbrechen bewirkte keine Übergabe. Erst
Bestätigen gab das Paket an die KI: ein eigenes Gebiet und 0/40 Bevölkerung.
Speichern, vollständiges Neuladen und Fortsetzen erhielten Besitz, Paket und
Geschenklog sowie 800 Nahrung, 500 Holz, 500 Stein, 800 Gold und 400 Zinn.
Die Verlustwarnung erschien nur beim letzten Gebiet; Auswahl, Objektzahlen,
Kriegssperre für Tausch, Karten-/Kamerafokus und kompakte Oberfläche wurden
geprüft. Die Browserwarnungs-/Fehlerprotokolle blieben leer. WebRTC wurde auf
demselben Rechner geprüft; verschiedene Rechner/NATs bleiben offen. Eine
vollständige Originalpartie oder Bildratenabnahme wird damit nicht behauptet.
Der frische normale Start unter ?rules=ee2&build=territory2 ist fertig geladen:
Startschaltfläche aktiv, volle Karte ausgewählt, alte normale Spielstände
unverändert sichtbar und keine Browserwarnungen/-fehler.

Belegbilder: [Territorien und Geschenk](territory-diplomacy-preview.jpg),
[lokaler WebRTC-Tausch](territory-swap-preview.jpg).
[Browserbericht](../EE2-Research/analysis/territory-browser-check.json).

Vorheriger Diplomatie-/Kartensicht-Build vor Gebietsübergaben, SHA256:

- EE2: `991E4736D420A14BC853D81B99C6D5EE6F9B0884809E4314051AA294739A3D75`
- Basis: `E03C2F14E1F4DE06295CA014142416968977049E1CEC4FDBFB9A13FF71AF4516`

Die damalige gezielte Diplomatieprüfung bestand 3.174 Assertions und 632 tatsächliche
Lockstep-Runden. Sie umfasst gerichtete Haltung, angenommene Sichtverträge,
alle neun Geschenkkonten, ungültige Netzbefehle, laufende Geschosse bei
Beziehungswechsel, drei Kartenmodi, KI-Antworten und Speicherstände. Beide
Profile bestehen je zehn Rust-Tests. Auch die bisherigen Kernregressionen sind
bestanden, einschließlich echter älterer 47-/53-/56-Definitionen-Spielstände,
43 Forschungswirkungsszenarien und des Forschungsdurchlaufs180/15.
Beleg: `../EE2-Research/analysis/diplomacy-tests.json`.

Die Vier-KI-CPU-Stressfixtures wurden mit genau diesem vorherigen EE2-Build erneut
20 Simulationsminuten ab Steinzeit und zehn ab Epoche 10 ausgeführt: keine
Warnungen oder Progressionsfehler. Kampfwellen-Fristen sind in diesen
Stressfixtures verschoben; das ist keine vollständige Partie-/Bildratenabnahme.
Beleg: `../EE2-Research/analysis/economy-ai-soak.json`. Der historische Vergleich BAE9→2A12 ist
mit portablen eigenen WASM-Fixtures erneut bestätigt; er ist kein Nachweis,
dass der neue Diplomatiestand dieselben Zustände wie jener Altbuild erzeugt.

Der Browser dieses vorherigen Builds bestätigte F4, Geschenk von 200 Nahrung mit Vorrat 800→600,
angenommenen Frieden und anschließend eine Allianz mit gegenseitiger voller
Sicht. Die übrigen Gegner blieben feindlich; es gab keinen voreiligen Teamsieg.
Full→None→Full wurde jeweils angeboten und erst nach KI-Annahme wirksam;
während des Angebots blieb die vorige Sicht bestehen. Pausehinweis und
„Spiel fortsetzen“ funktionieren im Dialog. Nach Speichern, Neuladen und
Fortsetzen blieben Vertrag, Sicht, Vorrat 600 und Ereignislog erhalten, auch
wenn der neue Spielstart im Menü eine andere Kartenoption ausgewählt hatte.

Modus 2 zeigt die volle Karte ohne Nebel. Eine neue Partie in Modus 1 zeigte das
gesamte Terrain, entfernte Gebiete blieben aber abgedunkelt und ohne entfernte
Gegneranzeigen. Beziehungen und Log waren für die neue Partie zurückgesetzt.
Die abschließende Dialogdarstellung ist opak/lesbar, ohne gekachelte Rahmen;
Bedienung und Kartenoptionen passen bei 1280×720. Keine Browserwarnungen/-fehler
wurden nach diesen Prüfungen gemeldet. Das belegt diese UI-Fälle, keine
vollständige Originalpartie, Bildrate oder physische Mehrspieler-/NAT-Abnahme.
Ein frisch geladener normaler Start zeigt die aktive Startschaltfläche und die
volle Kartenoption. Vorhandene normale Spielstände bleiben sichtbar und
unverändert; der frühere Spieltab wurde erhalten.

Belegbilder: [Diplomatie](diplomacy-preview.jpg),
[aufgedeckte Karte](revealed-map-preview.jpg),
[erkundete Karte](explored-map-preview.jpg),
[frischer Start mit Kartenoption](map-settings-preview.jpg).
[Maschinenlesbarer Browserbericht](../EE2-Research/analysis/diplomacy-browser-check.json).

Der vorherige Wirtschafts-/Assetabschluss bestand seine Engineprüfungen.
Nach dessen Metadatenregeneration blieben 30 Laufzeit-Vergleichsfälle gegenüber
BAE9 ohne Abweichung. Historische EE2-Prüfsumme vor Diplomatie/Kartensicht:
`2A127D17ED05E27DDAFA54BF78F0DFA61E202BBFC8F099644843D073DDA59B47`.

Geprüft:

- Neun Konten, alle Rohstoffspalten, Zahlungen/Rückzahlungen, Transport mit 15,
  Ablieferbonus und Sechser-Garrison für Spezialförderung.
- Belegungsgrenzen: drei am Baum, sechs an anderen direkt sammelbaren Knoten,
  sechs im Fördergebäude und einer auf der Farm; auch vor/nach dem Bau.
- Alte 47-/53-/55-Definitionen-Saves: Gesundheitsanteile, Vorräte und Forschung
  bleiben erhalten. Beispiele für volle Maxima: Bürger 40→30, mit Weberei
  48→36 und Stadtzentrum 2.400→8.000; beschädigte Einheiten behalten ihre Quote.
- 43 tatsächliche WASM-Szenarien zu Forschungswirkungen, alle 180 Forschungen
  über 15 Epochen und 2.650 Forschungs-Lockstep-Runden.
- Alle 15 Startepochen symmetrisch für vier Spieler sowie Tick-/Mehrspieler-
  und Bau-Gates. Beide Profile bestehen Personen-/Werkzeug-, Scharmützel-
  und Cheatprüfungen; die Referenzprüfung mit 2.000 Einheiten ist bestanden.

CPU-Stressfixtures mit vier KI-Spielern liefen 20 Minuten ab Steinzeit und
zehn Minuten ab Originalepoche 10, ohne externe Kampffrist. Es gab keine
Progressionsfehler oder Warnungen. Im Steinzeitlauf erreichten alle Epoche 2
mit 24 Armeeeinheiten, Universität und 19–20 freien Bürgern. Der späte Lauf
erreichte Ölbohrturm/Universität mit 16–19 freien Bürgern. Reserven und
Besatzungsverteilung sind für die neue Wirtschaft angepasst; das ist eine
eigene KI, keine Rekonstruktion der Original-KI oder vollständige Balanceabnahme.

Datentests belegen Einheitenquellen, Klassenfortschreibung und Forschungsfelder.
Assettests belegen Formate, Puffer, PNGs, Quellhashes, Spriteausschnitte,
Spielerfarben und Audio. Die regenerierten Skinning-Dateien bestehen erneut
den unabhängigen Modelltest und vier NIF/KF-Posenregressionen; Industrie-Bürger
sind zusätzlich in Near/Far im echten Browser geprüft. FakeWebGL prüft die
Renderer-API, führt aber keine GPU-Shader aus.

Der Browserlauf des vorherigen Wirtschaftsabschlusses bestätigte eine volle Holzladung von 15 mit anschließender
Vorratsgutschrift. Ein originaler Ölbohrturm wurde für 100 Holz gebaut; ein Bürger
auf sechs Plätzen erzeugte 0,75 Öl/s, der Ölstand stieg von 400 auf über 640.
Nach dem Ausquartieren: 0/6 Besatzung, Förderung +0,00 und unverändert 671 Öl
über mehrere fortschreitende Messungen. Steinzeit zeigt fünf aktive Rohstoffe,
Industriezeit sechs; 30/50 Bürger-HP und Transport sind sichtbar geprüft.
Belegbilder: [Industriesicht](economy-industry-preview.jpg) und
[Bürger-Nahansicht](economy-citizen-preview.jpg).
Ein frischer normaler Start mit allen 15 Startepochen und Originalgrafiken
wurde abschließend ohne Browserwarnungen oder -fehler geladen.
Die maschinenlesbaren UI-Belege stehen in
`../EE2-Research/analysis/economy-browser-check.json`.
Aktuelle FPS oder eine vollständige visuelle Spielabnahme werden nicht behauptet.

Offen bleiben unter anderem der volle produktive Katalog, Kampfmatrix und
Gegenklassen, zahlreiche Spezialfähigkeiten, Zivilisationen, Veteran/Elite,
Handel, die vollständigen Diplomatieverträge und Siegmodi, Kronen, Wetter
und weitere Scharmützeloptionen.
Die wirksame Archiv-/VFS-Reihenfolge des installierten Unofficial Patch ist
nicht vollständig bestätigt. Die Simulation und der Renderer sind eigene
Implementierungen; dekompilierte Original-Codepassagen wurden nicht übernommen.

Quellnachweise und Grenzen:
`../EE2-Research/Einheiten-und-Kosten.txt`,
`../EE2-Research/Forschung-und-Universitaet.txt`,
`../EE2-Research/Originalgrafiken-und-Sound.txt`,
`../EE2-Research/Originaltreue-Stand.txt`,
`../EE2-Research/Diplomatie-und-Sicht.txt`.

## Attribution

Die MIT-Lizenz und Attribution des ursprünglichen OpenEpoch-Codes bleiben in
[LICENSE](LICENSE) erhalten. Die reduzierte niftools/nifxml-Parsingmetadatei
ist in `tools/NIFXML-NOTICE.txt` und `tools/NIFXML-LICENSE.txt` als GPL-3.0
dokumentiert. Dieser Parser läuft beim lokalen Build; der Browser liest
exportierte Daten. Originalmedien behalten ihre Herkunft und werden nicht
pauschal unter die MIT- oder GPL-Lizenz des Quellpakets gestellt.
