"""Import the nineteen explicitly audited Civ8 families into a private stage.

The original installation is read only. No executable, DLL or native game code
is run. All outputs derived from original media stay in local-original/.
"""
from pathlib import Path, PureWindowsPath
import argparse, hashlib, json, math, struct, time
import import_roster6_models as media
import extract_ee2_models as meshes
from extract_ee2_textures import decode_texcache
from nif_reader import Reader
from up15_source_database import ActiveDatabase, default_manifest

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'
OUT = DOCS / 'local-original'
sha = lambda b: hashlib.sha256(b).hexdigest()
f32 = lambda x: struct.unpack('<f', struct.pack('<f', x))[0]


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(value, ensure_ascii=False, separators=(',', ':')) + '\n', encoding='utf8')
    temp.replace(path)


def run(plan_path, report_path):
    plan_bytes = plan_path.read_bytes()
    plan = json.loads(plan_bytes)
    families = plan['families']
    if [f['id'] for f in families] != list(range(61, 80)):
        raise ValueError('Expected exact typed family IDs61..79')
    manifest_path = default_manifest(ROOT)
    sources = media.MediaSources()
    db = ActiveDatabase(manifest_path)
    old_out, old_texture = meshes.OUT, meshes.extract_texture
    texture_records = {}
    readers = {}
    model_rows, scale_rows, checks = [], {}, 0
    original_manifest_bytes = (OUT / 'models/manifest-roster6.json').read_bytes()
    original_scales_bytes = (OUT / 'model-scales-roster6.json').read_bytes()
    model_manifest = json.loads(original_manifest_bytes)
    scales = json.loads(original_scales_bytes)
    defaults = json.loads((OUT / 'model-scales.json').read_bytes())
    if any(r.get('assetKind') == 0 and r['id'] >= 61 for r in model_manifest['models']):
        raise ValueError('Base manifest already contains candidate family IDs')

    def checked(record):
        nonlocal checks
        raw = sources.read(record['lookup'])
        if sha(raw) != record['sha256'] or len(raw) != record['bytes']:
            raise ValueError('Planned source differs: ' + record['lookup'])
        actual = sources.origin(record['lookup'])
        if actual != record['origin']:
            raise ValueError('Planned namespace origin differs: ' + record['lookup'])
        checks += 3
        return raw, actual

    def texture(source, output_dir, _archive=None):
        stem = PureWindowsPath(source.replace('/', '\\')).stem
        lookup = stem + '.texcache'
        origin = sources.origin(lookup)
        raw = sources.read(lookup)
        key = stem.casefold()
        if key in texture_records:
            prior = texture_records[key]
            if prior['source_sha256'] != sha(raw):
                raise ValueError('Ambiguous texture stem: ' + stem)
            return dict(prior, original_texture_path=source)
        image, metadata = decode_texcache(raw)
        dest = OUT / 'model-textures/civ8' / (key + '.png')
        dest.parent.mkdir(parents=True, exist_ok=True)
        image.save(dest, optimize=True)
        info = {'url': dest.relative_to(DOCS).as_posix(), 'lookup': lookup,
                'original_texture_path': source, 'archive_member': origin['member'],
                'origin': origin, 'source_sha256': sha(raw),
                'png_sha256': sha(dest.read_bytes()), **metadata}
        texture_records[key] = info
        return info

    meshes.OUT, meshes.extract_texture = OUT, texture
    started = time.monotonic()
    try:
        for family in families:
            inventory = family['modelInventory']
            raw, origin = checked(inventory['model'])
            reader = Reader(raw).parse()
            # This affects normalization only; source scale cancels it again.
            kind = 'human' if any(o.get('Skin Instance', -1) >= 0 for o in reader.objects) else 'vehicle'
            animations, native_clips = {}, {}
            for state, record in inventory['states'].items():
                kf, _ = checked(record)
                digest = sha(kf)
                if digest not in readers:
                    readers[digest] = Reader(kf).parse()
                key = {'Idle': 'idle', 'Walk': 'walk', 'Attack1': 'combat'}.get(state, state)
                if key in animations:
                    raise ValueError('Duplicate explicitly bound animation key')
                endpoint = state in ('Pack', 'Unpack')
                animations[key] = {'reader': readers[digest], 'entry': record['origin']['member'],
                                   'sha256': digest, 'samples': 25 if endpoint else 24,
                                   'includeEndpoint': endpoint}
                native_clips[state] = key
            for texture_source in inventory['textures']:
                checked(texture_source)
            data = meshes.convert(raw, origin['member'], Path(origin['physical']), sources,
                                  kind, animations=animations)
            data['source']['origin'] = origin
            data['nativeStateClips'] = native_clips
            data['nativeSourceProfile'] = 'ee2-civ8'
            data['databaseVisualBinding'] = {
                'visualDefinition': family['sourceVisual'], 'sourceUnit': family['sourceFamily'],
                'sourceCivilization': family['sourceCiv'], 'declaredModel': family['declaredModel'],
                'source': inventory['visualChain'][0], 'inheritance': inventory['visualChain'],
                'scope': 'Exact inherited active UP default model and declared KF states. Class survives until15; production window is separate.'}
            data['sourceLookupScope'] = 'Proven archiveOrder/fullpath/first-basename timestamp namespace; no filename-derived animation states.'
            data['animationSampling'] = {
                'loopPolicy': '24 original-duration samples at t=duration*i/24',
                'oncePolicy': 'Pack/Unpack25 samples at t=duration*i/24 including both endpoints',
                'sampleCounts': {a['name']: a['frames'] for a in data['animationSources']},
                'interpolationLimit': 'Existing linear/slerp source-key adapter; native quadratic tangents, text-key events, turret/child effect handlers remain separate.'}
            dest = OUT / 'models' / ('civ8_' + PureWindowsPath(family['declaredModel']).stem.casefold() + '.json')
            write_json(dest, data)
            row = {'assetKind': 0, 'id': family['id'], 'epoch': family['epoch'], 'until': 15,
                   'sourceFamily': family['sourceFamily'], 'sourceCivilization': family['sourceCiv'],
                   'sourceEpoch': family['sourceEpoch'], 'sourceVisual': family['sourceVisual'],
                   'verified': True, 'yawOffset': -math.pi / 2, 'url': dest.relative_to(DOCS).as_posix(),
                   'source': data['source'], 'triangles': sum(len(p['indices']) // 3 for p in data['parts']),
                   'nativeStateClips': native_clips, 'databaseVisualBinding': data['databaseVisualBinding']}
            model_rows.append(row)
            scale_type, force = inventory['scaleType'], inventory['forceScale']
            selected = defaults['classDefaults'][scale_type]['selectedValue']
            native_scale = f32(f32(float(force) if force is not None and float(force) > 0 else selected) * defaults['unitConversion']['value'])
            scale_rows[family['scaleVariantKey']] = {
                'url': row['url'], 'sourceSha256': sha(raw), 'sourceEntry': origin['member'],
                'verified': True, 'visual': family['sourceVisual'], 'scaleType': scale_type,
                'forceScale': float(force) if force is not None else None, 'sourceScale': native_scale,
                'inheritance': inventory['visualChain'], 'sourceBounds': data['sourceBounds'],
                'normalization': data['normalization'], 'sourceOrigin': origin}
            print(json.dumps({'id': family['id'], 'family': family['sourceFamily'],
                              'parts': len(data['parts']), 'triangles': row['triangles'],
                              'states': list(native_clips), 'bytes': dest.stat().st_size}), flush=True)
        base_model_count = len(model_manifest['models'])
        model_manifest['models'].extend(model_rows)
        model_manifest['profile'] = 'ee2-up15'
        model_manifest['nativeSourceManifestSha256'] = sha(manifest_path.read_bytes())
        model_manifest['civ8MediaPlanSha256'] = sha(plan_bytes)
        scales['variants'].update(scale_rows)
        scales['scope'] += ' Nineteen explicitly typed civilization families use exact inherited ScaleType/ForceScale with separate production/class lifetimes.'
        write_json(OUT / 'models/manifest-civ8.json', model_manifest)
        write_json(OUT / 'model-scales-civ8.json', scales)
        # Prove that all existing0..60 records and their ordering were retained.
        assert json.loads(original_manifest_bytes)['models'] == model_manifest['models'][:base_model_count]
        assert all(scales['variants'][k] == v for k, v in json.loads(original_scales_bytes)['variants'].items())
        assert (OUT / 'models/manifest-roster6.json').read_bytes() == original_manifest_bytes
        assert (OUT / 'model-scales-roster6.json').read_bytes() == original_scales_bytes
        report = {'format': 'native-civ8-private-model-import-v1', 'originalInstallReadonly': True,
                  'originalExecuted': False, 'activeRuntimeModified': False, 'planSha256': sha(plan_bytes),
                  'sourceManifestSha256': sha(manifest_path.read_bytes()), 'sourceChecks': checks,
                  'models': model_rows, 'scales': scale_rows, 'textures': list(texture_records.values()),
                  'baseModelRecordsPreserved': base_model_count,
                  'baseScaleRecordsPreserved': len(json.loads(original_scales_bytes)['variants']),
                  'uniqueKFs': len(readers),
                  'uniqueKFLookups': len({r['lookup'].casefold() for f in families for r in f['modelInventory']['states'].values()}),
                  'seconds': time.monotonic() - started,
                  'limits': ['Private original derivatives; excluded from public source.',
                             'No complete native interpolation, animation events, child attachments, turret or corpse lifecycle parity implied.',
                             'Initial empty voices and regional civilian/building bindings are separate source work.']}
        write_json(report_path, report)
        print(json.dumps({'models': len(model_rows), 'textures': len(texture_records), 'uniqueKFs': len(readers), 'checks': checks, 'report': str(report_path)}), flush=True)
    finally:
        meshes.OUT, meshes.extract_texture = old_out, old_texture
        sources.close()
        db.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    run(args.plan.resolve(), args.report.resolve())
