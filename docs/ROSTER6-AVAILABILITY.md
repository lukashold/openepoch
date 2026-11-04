Die isolierte 61v2-Korrektur berücksichtigt nur die Produktionsgrenzen der vier ergänzten Familien. Alte 57 Rollen und alle bestehenden Zahlen-, Modell-, Audio- und Namensintervalle bleiben erhalten. Runtime-Epochen sind native Epoche minus 1.

| DefId | Familie | Produktion Runtime | Native Epochen |
|---|---|---|---|
|57|Scout|[0,15)|1–15|
|58|FightingSail|[5,10)|6–10|
|59|AirDefense|[9,15)|10–15|
|60|CoastalDefense|[1,15)|2–15|

Die aktive CSV5 dbtechtreenod5.csv bindet Main11 an MainEpoch11. Die aktive epoch11_upgrades_15.ddf enthält dessen ObsoleteFightingSail mit tech=FightingSail. Der vollständige Graph enthält 33 Main-Knoten und 90 Epoch/Obsolete/Undo-Aktionen aus 158 aktiven CFG5-DDFs. Keine Undo-Aktion stellt diese vier Produktionsknoten wieder her. Die anderen offenen Main-Referenzen SSBN_Improve_Reset, Thaad_Improve_Reset und fix_plane_deathh1 sind dokumentiert und werden nicht aus inaktiven Quellen ergänzt.

Ghidra-Belege zur unveränderlichen Engine-SHA d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f:

* 005EB830 CreatePlayer ruft 006E5760 mit der aktuellen Epoche; dieser Wrapper ruft 006E5780.
* 0053CF90 ExecuteUpgradeEpoch ruft 006E17A0, dieser ebenfalls 006E5780.
* 006E5780 behandelt Main als Branch1 außerhalb der Forschungslimits von Branch2..4, prüft node.epoch<=epoch und ruft 006E50F0 und 0053C540 zur Upgradeausführung.
* 0053CFE0 ExecuteUpgradeObsoleteTech setzt node.flags=(flags&~3)|0x10.
* Das Original storniert betroffene Produktionsaufträge über 006E38E0. Dieser erfragt 008167F0, das elf gespeicherte Preisfloats aus Job+0x64 kopiert; Refund läuft nur über die neun Naturressourcen.

Diese 61v2-Sperre löscht keine Einheiten und spielt Main beim Laden nicht erneut ab. Gespeicherte Warteschlangen bleiben erhalten. Die genaue Originalstornierung wird im separaten PaidQueue7-Kandidaten ergänzt: Der bisherige Kern hat keinen gespeicherten Jobpreis. Aktuelle Kosten zu erstatten wäre bei einem Epochen- oder Forschungswechsel falsch. Diese offene Stornierung ist ausdrücklich keine Behauptung vollständiger Originalparität.

Die Klassenwerte sind Quellenwerte. Prozentresultate 180/204 in bisherigen Tests beschreiben unsere Implementierung; der tatsächliche native x87-Rundungskontext am Setter ist noch offen. AA59 ist der CSV-/Set-Datenwert, kein vollständiger Original-Firetrace.

Reproduktion: tools/roster6_availability.py, tools/regenerate_roster6_classic.py --verify, tools/test_roster6_availability_sources.py und tools/test_classic_availability_wasm.mjs. Der normale öffentliche Befehlsfixture baut alle vier in Runtime9 in 4120 Ticks ohne Speicher- oder Save-Manipulation. Original-EXE/DLL und GUI werden nie gestartet.
