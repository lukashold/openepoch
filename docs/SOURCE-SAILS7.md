# Segeldarstellung in native7

Die Originalmedien bleiben unter `local-original/sails-native`: sechs lokale
Raster der Karavelle, zwölf des Linienschiffs, ihre NIF-Anbindungen, Texturen und
neun Masken. Der spätere Ironclad verwendet kein erfundenes Segel. Die bisherigen
statischen Originalflächen bleiben der Rückfallpfad bei fehlenden Modulen.

Die eigene Simulation liefert echte generierte Einheitenhandles in einer
separaten Renderliste. Darauf baut ein eigener Controller die visuellen
Bewegungszustände auf. Weder Zufallsstartwert noch Position oder Renderindex
dienen als Identität. Ein Modell- oder Weltwechsel baut die passenden Flächen
neu auf. Ein noch lebendes, ausgeblendetes Schiff behält seinen Zustand ohne
weiteren Physikschritt. Einheiten-Lebendigkeit und niedrige Trefferpunkte werden
nicht als originale Death-Nachricht ausgelegt.

Der eigene WebAssembly-Kern setzt die statisch untersuchten Rechenfolgen für
Federn, Dämpfung, Schwerkraft, Zufallsfolge und lokale Segelfarben um. Er wird
beim Laden auf Größe und SHA-256 geprüft. Die Genauigkeit gilt für das explizite
Modell PC53/RC_CHOP innerhalb der dokumentierten Rechendomäne; die vollständige
zeitliche FPU-Konfiguration des Originalspiels ist weiter offen. Nachbarwahl,
Speicherstellen und wiederverwendete Zufallswerte bleiben erhalten.

Die eigenen Schiffsdrehungen und Bewegungsdifferenzen werden in die belegten
lokalen Koordinaten umgerechnet. Der Controller läuft einmal vor Reflexion,
Schatten und Hauptbild. Pause und Netzwerk-Wartezeit führen keinen Schritt aus;
der Einzelspielertakt skaliert die übergebene Renderzeit mit der Spielgeschwindigkeit.
Diese Zuordnung und die Sichtpolitik sind eigene Adapter. Das bestehende
Simulations-Culling bleibt vorgeschaltet. Neue native Leichen- oder Segelschatten-
Lebenszyklen sind damit nicht implementiert.

Der Flag-Materialpfad begrenzt `Base * VertexRGB * 4` vor der Multiplikation mit
der dunklen Maske. Alpha verwendet `Base.a * Dark.a > 0`; Tiefenprüfung und
Tiefenschreiben bleiben aktiv. Die Texturen sind linear, geklemmt und ohne
Mipmaps. Die originale GPU-Konvertierung gepackter Vertexfarben ist noch nicht
belegt; der aktuelle Float-RGBA-Upload bleibt ein Adapter.

Die eigene Rust-Quelle steht unter `visual-cloth7`. `Build-All.ps1` beziehungsweise
`Build-All.sh` bauen den Kern mit und aktualisieren die Integritätskennung.
`python tools/build_source_cloth.py` baut ihn separat. Zum Spielen sind Rust und
Ghidra nicht erforderlich. Quellalgorithmen, tatsächliche Spielstandsidentität,
Weltbilder und GPU-Materialproben werden getrennt geprüft; daraus folgt keine
vollständige Bild- oder Spielgleichheit mit Empire Earth II.
