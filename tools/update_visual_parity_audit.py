"""Refresh the parity memo after visual-seasons2; preserve combat-season1 history."""
from pathlib import Path
import hashlib
import json

APP = Path(__file__).resolve().parents[1]
RESEARCH = APP.parent / 'EE2-Research'
ANALYSIS = RESEARCH / 'analysis'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name):
    return json.loads((ANALYSIS / name).read_text(encoding='utf-8-sig'))


archive = ANALYSIS / 'original-parity-audit-combat-season1.json'
if not archive.exists():
    archive.write_bytes((ANALYSIS / 'original-parity-audit.json').read_bytes())
audit = json.loads(archive.read_text(encoding='utf-8'))
world, reflection, models, session = (read(name) for name in [
    'native-season-world-qa.json', 'native-reflection-implementation.json',
    'native-model-season-import.json', 'season-session-tests.json'])
for result in [world, session]:
    assert result['passed'] and result['wasmSha256'] == sha(APP / 'docs/openepoch-ee2.wasm')
for source, digest in world['sourceFiles'].items():
    assert sha(APP / 'docs' / source) == digest, f'World source changed: {source}'
for source, digest in session['sources'].items():
    assert sha(APP / source) == digest, f'Frontend source changed: {source}'
assert world['checks'] == 169 and len(world['scenes']) == 5
assert models['manifestSha256'] == sha(APP / 'docs/local-original/model-seasons.json')
audit.update({
    'date': '2026-11-04', 'currentBuild': 'visual-seasons2',
    'supersedes': {'file': archive.name, 'sha256': sha(archive),
                   'note': 'Preserved historical03.11 combat-season1 snapshot. Its dated report hashes do not describe files subsequently refreshed for visual-seasons2.'},
    'method': 'Independent file/source/report review plus current production-channel, real-WASM and real-GLES QA. New five-world proof uses createSeasonSession for terrain, ground and model skins, reflected scene capture and camera heading. No GUI, original executable or browser process launched.',
})
for entry in audit['completedSourceBackedSlices']:
    if entry['area'] == 'Camera and bound model scale':
        entry['done'] += ' Native heading/yaw, complete rotated basis, picking rays and terrain clearance are now implemented; north-up output remains bit-identical for288 tested cases. Native turn sign/wrap are source-verified. Browser input step/rate remains an adapter.'
        entry['evidence'] += ['native-camera-heading-evidence.json', 'native-camera-heading-tests.json', 'native-camera-heading-gles-tests.json']
    if entry['area'] == 'Calendar, terrain season and building-ground season':
        entry['area'] = 'Shared calendar, terrain, building ground and declared model skins'
        entry['done'] += f' Native explicit AnimatedTextureSetName direct/inherited NIF bindings now drive{models["seasonalModels"]} models/{models["seasonalParts"]} mesh bindings, including{models["inheritedParts"]} inherited bindings, using{models["setsPerClimate"]} sets per climate across three climates. Lists retain original order/duplicates/lengths1,8,11,12,13;232 physical textures and verified team variants are local. Current production session updates terrain, ground and models from the same saved tick.'
        entry['evidence'] += ['native-model-season-evidence.json', 'native-model-season-import.json', 'native-model-season-loader-tests.json', 'native-model-season-gpu-tests.json']
audit['completedSourceBackedSlices'].append({
    'area': 'Scene water reflection',
    'done': 'A real mirrored terrain/object framebuffer now feeds the production water shader. Source horizontal clip plane and camera height inversion, target bracketing and native normal distortion inputs are evidenced. Above-water clipping, framebuffer/viewport/state restoration and pixel contributions are tested. Complete original RTT/projection/culling policy remains partial.',
    'evidence': ['native-reflection-evidence.json', 'native-reflection-contract-tests.json', 'native-reflection-world-pixel-tests.json', 'native-reflection-implementation.json'],
})
for gap in audit['openSourceBackedParityGaps']:
    if gap['id'] == 'weather-and-season-appearance':
        gap['gap'] = 'Native declared/inherited model skin lists now switch and blend alongside terrain/building ground. Native weather event queue, probabilities, RNG, durations, precipitation/wind, gameplay effects and atmospheric lighting/tint remain incomplete. Two near/far stall-door bindings refer to a missing native CSV key and retain original base material; native terrain-derived fallback is not emulated.'
        gap['evidence'] += ['native-model-season-import.json', 'native-model-season-evidence.json']
    if gap['id'] == 'water-pipeline':
        gap['gap'] = 'Production scene reflection is now wired and pixel-tested. Remaining native gaps are separate reflected-view object culling, auxiliary plane0.02 projection/grid/UV construction, reflected near-clamp globals, exact RTT allocation/quality/refresh, complete native overlay/vertex-alpha/color/time, low/medium/palette paths and caustic linkage. Current reflection reuses main visible instances, so tall off-main-frustum objects can be absent.'
        gap['evidence'] += ['native-reflection-evidence.json', 'native-reflection-implementation.json']
    if gap['id'] == 'camera-controls':
        gap['gap'] = 'Native heading, basis, turn sign/wrap, picking rays and terrain clearance are implemented and GPU-tested. UI step/rate and pointer conversion remain adapters; full native picture-in-picture, remaining original controls and live launcher memory hooks are not established. Main keyboard/pan/save integration has its own independent root-owned QA.'
        gap['evidence'] += ['native-camera-heading-evidence.json']
for adapter in audit['explicitAdaptersAndUnverifiedScope']:
    if adapter['area'] == 'Lighting, shadow and water appearance':
        adapter['adapter'] += ' The approximate sky-only production reflection has been replaced by real scene capture. Its WebGL target resolution policy, geometric mirrored VP, fragment clipping and shared main/reflected instance list remain explicit adapters.'
audit['currentVerification']['frontendProtocol9'].update({'checks': session['checks'], 'currentMainSha256': session['sources']['docs/main.js']})
audit['currentVerification']['gpuWorld'].update({
    'checks': world['checks'], 'cases': [case['name'] for case in world['scenes']],
    'method': 'Five real GLES scenes from actual WASM and production createSeasonSession; terrain/ground/model skin source pairs are independently checked, including reflected/main passes and90-degree heading.',
})
audit['currentVerification']['reflection'] = {
    'sourceChecks': reflection['validation']['sourceChecks'],
    'matrixAndStateChecks': reflection['validation']['stateAndMatrixChecks'],
    'worldPixelChecks': reflection['validation']['reflectionPixelChecks'],
    'source': 'native-reflection-implementation.json',
}
audit['currentVerification']['modelSkinLoader'] = {'checks': read('native-model-season-loader-tests.json')['checks'], 'source': 'native-model-season-loader-tests.json'}
audit['currentVerification']['cameraHeading'] = {'mathChecks': read('native-camera-heading-tests.json')['checks'], 'glesChecks': read('native-camera-heading-gles-tests.json')['checks'], 'source': 'native-camera-heading-evidence.json'}
audit['currentVerification']['mainVisualControls'] = {'source': 'visual-controls-tests.json', 'coverage': 'Production VM input/start/save/load integration; root-owned73 checks.'}
audit['historicalReportPolicy'] += ' The combat-season1 parity audit is archived separately. Missing production reflections, fixed-north camera and entirely static model skins are superseded findings; native weather and the explicitly retained gaps remain open.'
extra_reports = [
    'native-season-world-heading90.json', 'native-reflection-evidence.json',
    'native-reflection-contract-tests.json', 'native-reflection-world-pixel-tests.json',
    'native-reflection-world-coast.json', 'native-reflection-implementation.json',
    'native-model-season-evidence.json', 'native-model-season-integration.json',
    'native-model-season-import.json', 'native-model-season-loader-tests.json',
    'native-model-season-gpu-tests.json', 'native-model-season-material-gpu-tests.json',
    'native-model-season-portable-assets.json', 'native-camera-heading-evidence.json',
    'native-camera-heading-tests.json', 'native-camera-heading-gles-tests.json',
    'visual-controls-tests.json',
]
for record in audit['reviewedReports']:
    record['sha256'] = sha(ANALYSIS / record['file'])
for name in extra_reports:
    audit['reviewedReports'].append({'file': name, 'sha256': sha(ANALYSIS / name), 'scope': 'visual-seasons2-source-or-current-component-execution'})
audit['runtimeSourceHashes'] = {f'docs/{source}': digest for source, digest in world['sourceFiles'].items()}
audit['runtimeSourceHashes'].update(session['sources'])
(ANALYSIS / 'original-parity-audit.json').write_text(json.dumps(audit, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

# Human-readable current header is written last. Preserve the previous complete
# handover, including its own pre03.11 archive, as an explicitly historical block.
stand = RESEARCH / 'Originaltreue-Stand.txt'
stand_archive = RESEARCH / 'Originaltreue-Stand-combat-season1.txt'
if not stand_archive.exists():
    stand_archive.write_bytes(stand.read_bytes())
header = '''Aktueller Originaltreue-Abgleich — 04.11.2026 — visual-seasons2

Das vollständige, identische EE2 ist weiterhin nicht erreicht. Die aktuelle
quellbelegte Umsetzung und ihre Grenzen stehen in analysis/original-parity-audit.json.
Die Übergabe combat-season1 vom03.11 bleibt darunter vollständig historisch erhalten.

NEU: Das Wasser spiegelt jetzt tatsächlich Terrain und native Modelle aus einem
separaten Szenenbild. Die belegte horizontale Clipplane verhindert Unterwasser-
geometrie im Bild; Kamera und Zielbindungen werden für den Hauptpass zurückgesetzt.
Original-Waterbump und die belegte Zwei-Normalen-Verzerrung lesen dieses Bild.
Die native separate View-Cullingpolitik ist noch offen: aktuell teilen Reflexion
und Hauptpass denselben sichtbaren Instanzsatz. Hohe Objekte außerhalb dieses
Ausschnitts können fehlen. Native Zusatzprojektion/Near-Clamp und vollständige
RTT-Qualitäts-/Zeit-/Wasser-Materialpolitik bleiben teilweise Adapter.

NEU: Explizite AnimatedTextureSetName-Einträge in nativen Modellen und ihren Eltern
steuern jetzt106 Modelle mit248 Meshbindungen, davon83 geerbte Bindungen.115 Sets
je Klima in drei Klimata behalten ihre originalen Listen, Wiederholungen und
Längen1/8/11/12/13.232 Originaltexturen und geprüfte Spielerfarben liegen lokal.
Terrain, Gebäudeböden und deklarierte Modellskins lesen gemeinsam denselben
gespeicherten Simulationstick. Zwei Stalltür-Near/Far-Bindungen mit fehlendem
Original-CSV-Key behalten das Original-Basismaterial. Wind/Wetter-RNG, Regen/Schnee-
Ereignisse, Wetter-Spielregeln und komplette Atmosphäre sind weiterhin offen.

NEU: Die belegte Kamerarichtung dreht jetzt Auge, Basis, Picking und Geländehöhen-
korrektur gemeinsam. Nordausgabe bleibt in288 Fällen bitgleich; mathematische und
reale GLES-Proben decken weitere Richtungen ab. Eingaberate/-schritt, vollständiges
Picture-in-Picture und Live-Launcher-Hooks sind dadurch nicht originalgleich belegt.

Aktuelle Prüfungen: fünf kombinierte echte GLES-Welten mit169 Checks für Sommer,
Winter, Halbblend bei echtem Tick1000, Zukunft und90° Kameradrehung;105 Reflexions-
Pixelchecks und423 Matrix-/Zustandschecks. Die Kontrollen weisen29430 echte native
Objektpixel im Ziel,542000 vom Clip betroffene Pixel und14987 sichtbar veränderte
Wasserpixel nach.33687 Modell-Saison-Laderchecks bestehen. Der Frontend-Protokoll9-
Test mit zwei echten WASMs/Locksteps wurde auf aktueller main.js mit425 Checks
erneuert. Diese Kopfzeile ersetzt keine separate GUI-/Mac-/Internet-Abnahme.
Der unabhängige Produktions-VM-Test prüft Start, Kameraeingaben und Speichern/Laden
zusätzlich mit73 Checks; sourcegebundener Bericht: analysis/visual-controls-tests.json.

Die Engine bleibt unverändert zum finalen combat-season1-Kern:
EE2 SHA256 d69f6d38f61b74ffcd23673870933e9feed37dbd05fac6b8541896dc3b12ff18
BASE SHA256230cc5a3c708aca73d2afd72683d3750563da89da39def5605ec89f4843d0a7a
75 EE2-/32 BASE-Rusttests und21692 Kalenderchecks gehören zu diesem unveränderten
Kern. Aktuelle Runtime-/Shader-/Assethashes stehen in den Berichten, nicht in alten
Freigabezahlen. Neue Belege: Wasserreflexion.txt und analysis/native-reflection-*.json,
native-model-season-*.json, native-camera-heading-*.json, native-season-world-qa.json.

Weiter offen bleiben insbesondere konfigurierte UP5-/VFS-Profildaten gegenüber
unseren expliziten BASE-Regeln, vollständiger Inhalt/Kampfsonderregeln/KI, genaue
native Terrain-/Beleuchtungs-/Schattenpipeline und die zuvor belegten Teilgrenzen.
Die neue Grafik beweist keine1:1-Bildgleichheit: kein aktueller kontrollierter
Originalvergleich, kein physischer MacBook- und kein Browser-/Internettest in dieser
Headless-QA. Weitere Rekonstruktion bleibt erforderlich.

=========== HISTORISCHE ÜBERGABE combat-season1, 03.11.2026 ===========
'''
stand.write_text(header + '\n' + stand_archive.read_text(encoding='utf-8-sig'), encoding='utf-8')
print(json.dumps({'passed': True, 'build': 'visual-seasons2', 'reports': len(audit['reviewedReports']),
                  'auditSha256': sha(ANALYSIS / 'original-parity-audit.json'), 'mainSha256': session['sources']['docs/main.js']}))
