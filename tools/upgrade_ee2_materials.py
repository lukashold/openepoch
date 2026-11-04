"""Upgrade existing near/far exports from the same hash-verified native NIF.

Does not regenerate geometry, normalize models, resample animations, or change
texture bindings. Existing gameplay/media is never read from the installation
at runtime. Import-time dependencies and originals stay on the development PC.
"""
import argparse
import collections
import hashlib
import json
import pathlib
import zipfile
from extract_ee2_models import Reader, scene, ROOT, INSTALL
from native_materials import native_material_fields


def digest(value):
    return hashlib.sha256(json.dumps(value, separators=(',', ':'), sort_keys=True).encode()).hexdigest()


def upgrade(installation, report):
    docs = ROOT / 'docs'
    rows = json.loads((docs / 'local-original/models/manifest.json').read_text(encoding='utf8'))['models']
    files = sorted({row[key] for row in rows for key in ('url', 'lodFarUrl') if row.get(key)})
    counts = collections.Counter()
    state_counts = collections.defaultdict(collections.Counter)
    records = []
    cache = {}
    with zipfile.ZipFile(installation / 'zips/graphics.zip') as archive:
        for url in files:
            dest = docs / url
            model = json.loads(dest.read_text(encoding='utf8'))
            old_sha = hashlib.sha256(dest.read_bytes()).hexdigest()
            entry = model['source']['entry']
            if entry not in cache:
                raw = archive.read(entry)
                source_sha = hashlib.sha256(raw).hexdigest()
                reader = Reader(raw).parse()
                # Properties are inherited independently of animation transforms.
                # Collect both native LOD choices without altering the source.
                _, near_props, _, _, _ = scene(reader)
                _, far_props, _, _, _ = scene(reader, lod='far')
                cache[entry] = (source_sha, near_props | far_props)
            source_sha, properties = cache[entry]
            if source_sha != model['source']['sha256']:
                raise ValueError('Source hash mismatch: ' + entry)
            geometry_before = [{k: v for k, v in p.items() if k not in ('material', 'nativeProperties', 'materialStateVersion')} for p in model['parts']]
            data_sha = digest(geometry_before)
            for part in model['parts']:
                native = properties[part['sourceNode']]
                part.update(native_material_fields(native))
                counts['parts'] += 1
                for key in ('NiAlphaProperty', 'NiZBufferProperty', 'NiSpecularProperty', 'NiVertexColorProperty'):
                    prop = native.get(key)
                    state_counts[key][json.dumps({k: v for k, v in (prop or {}).items() if k in ('Flags', 'Function', 'Vertex Mode', 'Lighting Mode', 'Threshold')}, sort_keys=True)] += 1
            geometry_after = [{k: v for k, v in p.items() if k not in ('material', 'nativeProperties', 'materialStateVersion')} for p in model['parts']]
            if data_sha != digest(geometry_after):
                raise AssertionError('Nonmaterial data changed: ' + url)
            tmp = dest.with_suffix('.material-tmp')
            tmp.write_text(json.dumps(model, separators=(',', ':')), encoding='utf8')
            tmp.replace(dest)
            records.append({'url': url, 'sourceEntry': entry, 'sourceSha256': source_sha,
                            'beforeSha256': old_sha, 'afterSha256': hashlib.sha256(dest.read_bytes()).hexdigest(),
                            'nonMaterialDataSha256': data_sha, 'partCount': len(model['parts'])})
            counts['files'] += 1
            if counts['files'] % 25 == 0:
                print(f"{counts['files']}/{len(files)} models", flush=True)
    result = {'format': 'ee2-native-material-upgrade-v1', 'scope': 'Hash-verified native material/state fields only; every existing nonmaterial part field unchanged.',
              'nativeSources': len(cache), 'counts': dict(counts), 'stateCounts': {key: dict(value) for key, value in state_counts.items()},
              'records': records, 'limitations': ['Original runtime shader illumination, texture-stage combine and shadow algorithm are not reconstructed by this metadata export.']}
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(result, indent=2) + '\n', encoding='utf8')
    print(json.dumps({'counts': dict(counts), 'nativeSources': len(cache), 'report': str(report)}))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--install', type=pathlib.Path, default=INSTALL)
    parser.add_argument('--report', type=pathlib.Path, default=ROOT.parent / 'EE2-Research/analysis/visual-material-upgrade.json')
    args = parser.parse_args()
    upgrade(args.install, args.report)
