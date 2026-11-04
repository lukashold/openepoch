Der isolierte UP6-Kandidat verwendet weiterhin ruleset() = 2, aber schema_revision() = 6 und das neue OEU6-Spielstandsformat. Der61-Kandidat behält die bisherigen57 IDs und ergänzt Scout57, FightingSail58, AirDefense59 und CoastalDefense60. Phase1 mit57 Rollen bleibt separat eingefroren. Ein Frontend darf ihn nicht allein anhand ruleset() mit UP5 mischen; Multiplayer benötigt einen neuen Protokollstand oder den identischen Simulationsfingerprint.

Alle unten beschriebenen Kennungen sind u32 in der WASM-ABI. JavaScript muss Rückgabewerte mit `>>> 0` lesen: `0xffffffff` bedeutet ungültig. DefId ist intern u16, QueueItem u32; Team-, Ressourcen-, Epochen- und Waffentypen bleiben getrennte kleine Werte.

`definition_count()` liefert die gültige Zahl von Definitionen, derzeit61. `definition_flags(id)` liefert bit0=Gebäude, bit1=mobil, bit2=Arbeiter, bit3=Transport, bit4=bewaffnet. Diese statischen Definitionsmerkmale ersetzen die bisherigen UI-Tests auf8..46. Sie sind keine vollständige Aussage über aktuelle Epoche, Befehlsverfügbarkeit oder native Spezialfähigkeiten.

`queue_job_encode(kind, value)` erzeugt einen Job. `queue_item_kind(item)` und `queue_item_value(item)` dekodieren ihn ohne Zahlenbereichsheuristik. Kind0 ist eine Einheit/Definition; kind1 ist native Forschung0..179; kind2 ist unabhängige Technik; kind3 ist der Epochenwechsel mit value0. UP6 akzeptiert kind2 ausdrücklich nicht als Produktionsauftrag. Encodierbarkeit allein bedeutet keine Verfügbarkeit. `cmd_train(defId)`, `cmd_research(researchId)` und das neue `cmd_epoch()` sind die passenden UI-Befehle. Eine alte direkte255-Epochenaktion ist unter UP6 eine Einheit und muss entfernt werden. Auswahl-JSON `q` enthält kodierte Jobs. Für diese Actionbuttons ist `cmd_queue_job(rawJob)` vorgesehen: es wählt ausschließlich den gültigen ausgewählten Produzenten mit der kürzesten Warteschlange. Es verwendet keinen globalen Fallback; dieser bleibt nur dem vollständigen Forschungsbaum über `cmd_research` erhalten.

`render_range_len()` beziehungsweise `ranges_len()` liefert die u32-Anzahl hinter `ranges()`, derzeit634. Jedes Paar bleibt {start,count}; es gibt317 Renderkennungen. Für jede Kennung liefern `render_asset_kind(id)` und `render_asset_source_id(id)` die Zuordnung:

| Kind | Bedeutung | Source-ID |
|---|---|---|
|0|Einheit|DefId, derzeit0..60|
|1|Ressource|bestehende Manifest-ID100..110|
|2|Projektil|Waffentyp0..6|
|3|Quad|bestehende ID240..243|
|4|Dekoration|110=Flagge,111=Baustellengerüst|

Source110 ist je nach Kind Ressource oder Flagge. Der Renderer muss deshalb beide Angaben auswerten. Roh-Renderkennungen sind keine Modell- oder DefIds. Unbelegte Renderplätze melden ungültig. Das Einheitenmodellmanifest verwendet weiterhin Source-DefIds; Instanzattribute und Range-Offsets verwenden die neuen Renderkennungen. Der Browser muss den WASM-Speicher nach möglichem memory.grow neu lesen.

Das portable Save enthält176-Byte-Entitäten und explizite Feldkodierung statt unkontrollierter Rust-Paddingbytes. Nach CGD/ATK steht ein52-Byte-Descriptor mit Version, Kernschema und SHA256 der expliziten Registry. Original-OEU5 mit148-Byte-Entitäten wird vor dem transaktionalen Parser geprüft und normalisiert. Alte optionale ATK-Abwesenheit ist gültig; fremde Profile, ungültige Booleans/Floats, kaputte Referenzen und Registry-Fingerprints werden ohne Zustandsänderung abgewiesen. Vor der tatsächlichen Kadererweiterung bleibt diese57-Registry separat eingefroren.

Frontend-Handoff: Queue-Icons über die Decoder, Epochenaktion über cmd_epoch, dynamische Render-Ranges und Assetmapping, dynamische Labelvalidierung, Auswahlaktivität über Definitionsflags, neue Save-/IndexedDB-Grenze und eine explizite OEU5-Importaktion. BASE0/Referenz1 behalten ihre bisherigen Pfade. Die aktive Anwendung wurde durch diesen Kandidaten nicht verändert.

Der61-Kandidat akzeptiert außerdem den bekannten57-Descriptor mit dessen exakter57-Kardinalität. Die lebenden Definitionen, Garnisonen und Produktionsjobs werden gegen diese Eingangsgrenze geprüft, bevor die Headerzahl auf61 normalisiert wird. Alle vier neuen Sourcefamilien verwenden absolute native Damage-Werte; die ersten57 behalten ihre bisherige Darstellung und Multiplikationsreihenfolge. Der Katalog umfasst132 klassische Sourcefamilien, davon43 gebundene Familien und89 noch ohne eigene Laufzeitdefinition. Die vollständige native Segelsimulation, Küstenausrichtung und zahlreiche Spezialfähigkeiten bleiben offen.
