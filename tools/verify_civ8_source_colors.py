"""Apply a separate native filename/source-byte gate to weak colour measurements.

Classifier thresholds and image bytes are preserved. The input proof binds the
native11-object colour table, pointer-identity slot lookup and ASCII_A+slot.
"""
from pathlib import Path
import argparse, hashlib, json
from up15_source_database import default_manifest

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'
sha = lambda b: hashlib.sha256(b).hexdigest()


def run(proof_path, report_path):
    proof_raw = proof_path.read_bytes()
    proof = json.loads(proof_raw)
    if proof.get('format') != 'native8-player-color-suffix-source-proof-v1' or proof.get('status') != 'PASS':
        raise ValueError('Completed native colour source proof required')
    if proof['sourceManifest']['manifest_sha256'] != sha(default_manifest(ROOT).read_bytes()):
        raise ValueError('Colour proof source profile differs')
    path = DOCS / 'local-original/team-textures/manifest-civ8.json'
    manifest = json.loads(path.read_bytes())
    mapping = proof['mapping']
    if len(mapping) != 11:
        raise ValueError('Expected all11 original colour slots')
    for index, row in enumerate(mapping):
        assert row['index'] == index and row['letter'] == chr(65 + index)
        assert manifest['suffix_by_colour'][row['color']] == row['letter']
        assert manifest['palette'][row['color']] == row['rgb']
    verified = []
    for row in proof['verifiedWeakMeasurementVariants']:
        entry = manifest['textures']['civ8:' + row['texture']]
        variant = entry['variants'][row['letter']]
        assert row['sourceSuffixVerified'] is True and row['pixelClassifierVerified'] is False
        assert variant['colour'] == row['color']
        assert variant['png_sha256'] == row['pngSha256'] == sha((DOCS / variant['url']).read_bytes())
        assert variant['patch_source']['sha256'] == row['patchSha256']
        # Repeatable application retains the original classifier outcome.
        prior_classifier = variant.get('pixelClassifierVerified', variant['verified'])
        assert prior_classifier is False
        variant['pixelClassifierVerified'] = False
        variant['sourceSuffixVerified'] = True
        variant['sourcePatchBytesVerified'] = True
        variant['verificationMethod'] = 'Native11-color-object slot→ASCII_A+slot filename proof and exact original base+patch RGBA/PNG reconstruction.'
        variant['verified'] = True
        entry['verified'] = any(v['verified'] for v in entry['variants'].values())
        entry['all_variants_verified'] = all(v['verified'] for v in entry['variants'].values())
        verified.append({'texture':row['texture'],'letter':row['letter'],'color':row['color'],
                         'pngSha256':row['pngSha256'],'pixelClassifierVerified':False,'sourceSuffixVerified':True})
    if len(verified) != 3:
        raise ValueError('Expected exactly the three independently reconstructed weak measurements')
    compact = {k:proof[k] for k in ['format','status','checks','engineSha256','sourceEvidence','nativeHeaderSha256','paletteCsv','mapping','verifiedWeakMeasurementVariants','runtimeChain','limits']}
    compact['sourceProofSha256'] = sha(proof_raw)
    target = DOCS / 'local-original/team-textures/civ8-source-color-proof.json'
    target.write_text(json.dumps(compact, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    manifest['civ8SourceColorProof'] = {'url':target.relative_to(DOCS).as_posix(),
                                       'sha256':sha(target.read_bytes()),'sourceProofSha256':sha(proof_raw),
                                       'scope':'Named source colors→native pointer-table slots→filename suffixes. Player/team color assignment stays separate.'}
    manifest['verified_colour_maps'] = sum(x['verified'] for x in manifest['textures'].values())
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    variants = [v for key, texture in manifest['textures'].items() if key.startswith('civ8:') for v in texture['variants'].values()]
    assert len(variants) == 286 and all(v['verified'] for v in variants)
    report = {'format':'native-civ8-source-colour-gate-v1','passed':True,'proofSha256':sha(proof_raw),
              'manifestSha256':sha(path.read_bytes()),'verifiedSourceVariants':verified,
              'civ8VerifiedVariants':len(variants),'pngFilesModified':False,'classifierThresholdsModified':False,
              'activeRuntimeModified':False}
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print(json.dumps(report))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--proof',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    args = parser.parse_args()
    run(args.proof.resolve(),args.report.resolve())
