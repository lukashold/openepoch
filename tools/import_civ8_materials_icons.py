"""Resolve Civ8 private player patches and exact CSV sprite crops.

Uses the audited active media namespace, never an executable or DLL. Existing
neutral textures, team variants and model records remain unchanged.
"""
from pathlib import Path, PureWindowsPath
import argparse, copy, hashlib, json
from PIL import Image
from extract_ee2_textures import decode_texcache
from extract_ee2_team_textures import LETTERS, apply_patch, classify, source_palette
from import_roster6_models import MediaSources
from up15_source_database import ActiveDatabase, default_manifest

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'
OUT = DOCS / 'local-original'
sha = lambda b: hashlib.sha256(b).hexdigest()


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf8')


def run(plan_path, report_path):
    plan_raw = plan_path.read_bytes()
    plan = json.loads(plan_raw)
    sources = MediaSources(extensions={'.texcache', '.pcpatch'})
    db = ActiveDatabase(default_manifest(ROOT))
    before = (OUT / 'team-textures/manifest.json').read_bytes()
    manifest = json.loads(before)
    base = copy.deepcopy(manifest)
    if not manifest.get('colour_mapping_verified'):
        raise ValueError('Prior proven native colour mapping required')
    expected = {letter: colour for colour, letter in manifest['suffix_by_colour'].items()}
    if set(expected) != set(LETTERS):
        raise ValueError('Incomplete proven colour suffix mapping')
    palette, palette_source = source_palette(db.zip)
    palette_source['origin'] = db.reader.origin('db/UI/dbcolor.csv')
    textures = {}
    records, without_patch, icon_rows = [], [], []
    checks = 0
    try:
        for family in plan['families']:
            for row in family['modelInventory']['textures']:
                stem = PureWindowsPath(row['lookup']).stem.casefold()
                if stem in textures and textures[stem]['sha256'] != row['sha256']:
                    raise ValueError('Conflicting source texture')
                textures[stem] = row
        for stem, source in sorted(textures.items()):
            raw = sources.read(source['lookup'])
            assert sha(raw) == source['sha256']
            assert sources.origin(source['lookup']) == source['origin']
            checks += 2
            try:
                sources.origin(stem + 'a.pcpatch')
            except ValueError:
                without_patch.append(stem)
                continue
            images, variants = {}, {}
            for letter in LETTERS:
                lookup = stem + letter.lower() + '.pcpatch'
                patch = sources.read(lookup)
                origin = sources.origin(lookup)
                patched, metadata = apply_patch(raw, patch)
                image, decoded = decode_texcache(patched)
                images[letter] = image
                dest = OUT / 'team-textures/civ8' / (stem + '-' + letter + '.png')
                dest.parent.mkdir(parents=True, exist_ok=True)
                image.save(dest, optimize=True)
                variants[letter] = {'url': dest.relative_to(DOCS).as_posix(),
                                    'png_sha256': sha(dest.read_bytes()),
                                    'patch_source': {'lookup': lookup, 'origin': origin, 'sha256': sha(patch)},
                                    **metadata}
                checks += 1
            identification = classify(images, palette)
            for letter, variant in variants.items():
                measurement = identification['letters'].get(letter, {})
                variant['measured_colour'] = measurement.get('colour')
                variant['colour'] = expected[letter]
                variant['verified'] = bool(measurement.get('colour') == expected[letter]
                                            and measurement.get('distance', 1) < .22
                                            and measurement.get('margin', 0) > .015)
            url = 'local-original/model-textures/civ8/' + stem + '.png'
            key = 'civ8:' + stem
            entry = {'base_texture_urls': [url],
                     'base_source': {'lookup': source['lookup'], 'origin': source['origin'], 'sha256': sha(raw)},
                     'width': decoded['width'], 'height': decoded['height'], 'format': decoded['format'],
                     'colour_identification': identification, 'variants': variants,
                     'verified': any(v['verified'] for v in variants.values()),
                     'all_variants_verified': all(v['verified'] for v in variants.values())}
            assert key not in manifest['textures'] and url not in manifest['texture_bindings']
            manifest['textures'][key] = entry
            manifest['texture_bindings'][url] = key
            records.append({'texture': stem, 'verifiedVariants': sum(v['verified'] for v in variants.values()),
                            'maskPixels': identification.get('mask_pixels'), 'key': key})
            print(json.dumps(records[-1]), flush=True)
        manifest['civ8PaletteSource'] = palette_source
        manifest['civ8MediaPlanSha256'] = sha(plan_raw)
        manifest['verified_colour_maps'] = sum(x['verified'] for x in manifest['textures'].values())
        manifest['scope'] += ' Civ8 variants use active namespace origins and the same measured colour gates.'
        assert all(manifest['textures'][k] == v for k, v in base['textures'].items())
        assert all(manifest['texture_bindings'][k] == v for k, v in base['texture_bindings'].items())
        assert (OUT / 'team-textures/manifest.json').read_bytes() == before
        write_json(OUT / 'team-textures/manifest-civ8.json', manifest)

        path = 'db/UI/UnpackedSprites/dbsprite_unitIcons.csv'
        sprite_rows = db.table(path)
        sprites = {}
        for row in sprite_rows:
            key = row['NAME'].casefold()
            if key in sprites:
                raise ValueError('Ambiguous native sprite: ' + key)
            sprites[key] = row
        upgrade_rows = db.table('db/TechTree/upgrade_unittypes.csv')
        for family in plan['families']:
            matches = [r for r in upgrade_rows if r['UNIT'] == family['sourceFamily']
                       and r['CIV'] == family['sourceCiv'] and int(r['EPOCH']) == family['sourceEpoch']
                       and r['UPGRADE'] == family['sourceUpgrade']]
            if len(matches) != 1:
                raise ValueError('Expected one exact source upgrade row')
            upgrade = matches[0]
            if upgrade['VISUAL'] != family['sourceVisual'] or upgrade['_source'] != family['upgradeOrigin']:
                raise ValueError('Planned class/icon source differs')
            sprite_name = upgrade['ICON'].strip()
            sprite = sprites[sprite_name.casefold()]
            stem = PureWindowsPath(sprite['FILENAME']).stem
            lookup = stem + '.texcache'
            raw = sources.read(lookup)
            image, metadata = decode_texcache(raw)
            values = [float(sprite[k]) for k in ('U', 'V', 'WIDTH', 'HEIGHT')]
            if not all(v.is_integer() for v in values):
                raise ValueError('Fractional native icon rectangle')
            u, v, w, h = map(int, values)
            if not (u >= 0 and v >= 0 and w > 0 and h > 0 and u + w <= image.width and v + h <= image.height):
                raise ValueError('Native icon rectangle outside top mip')
            dest = OUT / 'ui/civ8' / (str(family['id']) + '-' + sprite_name.casefold() + '.png')
            dest.parent.mkdir(parents=True, exist_ok=True)
            image.crop((u, v, u + w, v + h)).save(dest, optimize=True)
            icon_rows.append({'assetKind': 0, 'id': family['id'], 'epoch': family['epoch'], 'until': 15,
                              'sourceFamily': family['sourceFamily'], 'sourceUnit': family['sourceFamily'],
                              'sourceCivilization': family['sourceCiv'], 'sourceVisual': family['sourceVisual'],
                              'sourceEpoch': family['sourceEpoch'], 'sprite': sprite_name,
                              'resolvedSprite': sprite['NAME'], 'spriteRect': [u, v, w, h],
                              'spriteSource': sprite['_source'], 'spriteTableOrigin': db.reader.origin(path),
                              'sourceUpgrade': family['upgradeOrigin'], 'sourceFlags': sprite['FLAGS'],
                              'textureOrigin': sources.origin(lookup), 'sourceTextureSha256': sha(raw),
                              'sourceTextureWidth': image.width, 'sourceTextureHeight': image.height,
                              'url': dest.relative_to(DOCS).as_posix(), 'pngSha256': sha(dest.read_bytes()),
                              'width': w, 'height': h,
                              'bindingType': 'exact-native-sprite',
                              'transformation': 'Original top mip RGBA with exact source sprite crop; no rescale/repaint.'})
            checks += 3
        write_json(OUT / 'ui/icons-civ8.json', {'format': 'native-civ8-icons-v1', 'rows': icon_rows})
        report = {'format': 'native-civ8-private-materials-icons-v1', 'originalInstallReadonly': True,
                  'originalExecuted': False, 'activeRuntimeModified': False, 'planSha256': sha(plan_raw),
                  'checks': checks, 'uniqueModelTextures': len(textures), 'patchedTextures': records,
                  'withoutPatch': without_patch, 'icons': icon_rows,
                  'baseTeamTextureRecordsPreserved': len(base['textures']),
                  'limits': ['Only colour variants passing existing measured native palette gates are renderable.',
                             'Exact source art/sprite identity does not assert complete gameplay or animation event parity.']}
        write_json(report_path, report)
        print(json.dumps({'patchedTextures': len(records), 'withoutPatch': len(without_patch),
                          'icons': len(icon_rows), 'checks': checks}), flush=True)
    finally:
        sources.close()
        db.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    run(args.plan.resolve(), args.report.resolve())
