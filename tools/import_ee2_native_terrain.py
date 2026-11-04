"""Import the installed temperate terrain tables and original full-size images.

The own-map material adapter is deliberately separate from the source tables.
No native terrain generation, shader timing or unresolved UV scale is asserted.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import zipfile
from PIL import Image

from extract_ee2_textures import ROOT, INSTALL, extract_texture

OUT = ROOT / 'docs/local-original/terrain-native'
TABLES = {
    'types': 'db/Terrain/dbterrain_temperate.csv',
    'definitions': 'db/Terrain/dbterraintexture_temperate.csv',
    'seasons': 'db/Terrain/DbSeasonalTextureSets_Temperate.csv',
}
ROLES = {
    'grass': 'Grass', 'patch': 'GrassPatchy', 'forest': 'GrassDark',
    'rock': 'Cliff01', 'beach': 'Beach', 'coral': 'OceanFloor',
    'rocky': 'GrassRocky', 'roots': 'ForestRoots',
}

def rows(raw):
    for line, row in enumerate(csv.reader(io.StringIO(raw.decode('cp1252'))), 1):
        if row and row[0].strip() and not row[0].lstrip().startswith(('#', '//')):
            yield line, [value.strip() for value in row]

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def import_terrain(installation=INSTALL):
    installation = Path(installation)
    OUT.mkdir(parents=True, exist_ok=True)
    sources, types, definitions, seasons = [], [], {}, {}
    with zipfile.ZipFile(installation / 'zips/db.zip') as db:
        data = {}
        for key, member in TABLES.items():
            raw = db.read(member)
            sources.append({'archive': 'db.zip', 'entry': member, 'sha256': sha(raw), 'bytes': len(raw)})
            data[key] = raw
        for line, row in rows(data['types']):
            types.append({'name': row[0], 'high': row[1], 'medium': row[2], 'low': row[3], 'sourceLine': line})
        for line, row in rows(data['definitions']):
            definitions[row[0]] = {'seasonalSet': row[1], 'mask': row[2], 'tintFractions': [float(value) for value in row[3].split()], 'sourceLine': line}
        required_sets = {row['seasonalSet'] for row in definitions.values()}
        for line, row in rows(data['seasons']):
            if row[0] in required_sets:
                images = row[1].split()
                if len(images) != 12:
                    raise ValueError(f'{row[0]}: expected twelve seasonal source textures')
                seasons[row[0]] = {'sourceTextures': images, 'sourceLine': line}
    if set(seasons) != required_sets:
        raise ValueError('Unresolved terrain seasonal set')
    by_name = {row['name']: row for row in types}
    for row in types:
        if any(row[detail] not in definitions for detail in ('high', 'medium', 'low')):
            raise ValueError('Unresolved terrain detail definition')
    textures = {}
    with zipfile.ZipFile(installation / 'zips/textures.zip') as archive:
        def image(source):
            key = source.casefold()
            if key not in textures:
                if key in ('waterlayer1palette.tga', 'watercolor_oceanrivercoast.tga'):
                    members = [name for name in archive.namelist() if name.casefold().endswith('/' + key)]
                    if len(members) != 1:
                        raise ValueError('Expected one native unconverted water lookup image')
                    raw = archive.read(members[0])
                    original = Image.open(io.BytesIO(raw)); original.load()
                    dest = OUT / key.replace('.tga', '.png')
                    original.convert('RGBA').save(dest, optimize=True)
                    textures[key] = {'url': dest.relative_to(ROOT / 'docs').as_posix(), 'original_texture_path': source,
                        'archive_member': members[0], 'source_sha256': sha(raw), 'png_sha256': sha(dest.read_bytes()),
                        'format': 'native-TGA-' + original.mode, 'width': original.width, 'height': original.height,
                        'transformation': 'Original TGA pixels decoded to RGBA PNG without rescaling or repainting'}
                else:
                    textures[key] = extract_texture(source, OUT, archive)
            return textures[key]['url']
        for row in seasons.values():
            row['urls'] = [image(source) for source in row['sourceTextures']]
        mask = image('Terr_Mask.tga')
        water = {name: image(source) for name, source in {
            'normal': 'Waterbump.tga', 'layer1': 'waterLayer1.tga',
            'layer2': 'waterLayer2.tga', 'palette': 'WaterLayer1Palette.tga',
            'colorLut': 'WaterColor_OceanRiverCoast.tga',
        }.items()}
        caustic_sources = sorted({PurePosixPath(name).stem for name in archive.namelist()
            if name.lower().endswith('.texcache') and PurePosixPath(name).stem.lower().startswith('caust')})
        water['unboundCaustics'] = [image(name + '.tga') for name in caustic_sources]
    manifest = {
        'format': 'ee2-native-terrain-v1', 'sources': sources,
        'shaderEvidence': [{'file': name, 'sha256': sha((installation / name).read_bytes())} for name in (
            'shaders/Water_LOD_HIGH.NSF', 'shaders/WaterReflection_LOD_HIGH.NSF',
            'shaders/vsh_psh/Water_LOD_HIGH.vsh', 'shaders/vsh_psh/Water_LOD_HIGH.psh',
            'shaders/vsh_psh/WaterReflection_LOD_HIGH.vsh', 'shaders/vsh_psh/WaterReflection_LOD_HIGH.psh')],
        'terrainTypes': types, 'textureDefinitions': definitions,
        'seasonalSets': seasons, 'textures': textures, 'maskUrl': mask, 'water': water,
        'runtimeRoles': {role: {'terrainType': name, 'textureDefinition': by_name[name]['high']} for role, name in ROLES.items()},
        'adapter': {
            'scope': 'Original full-size images and source bindings rendered on OpenEpoch generated maps; not original map generation or a reconstructed complete native shader.',
            'classes': {'0': 'deep-water', '1': 'shallow-water', '2': 'beach', '3': 'grass', '4': 'forest', '5': 'rock'},
            'classSource': 'Map.ter, not minimap RGB', 'initialSeasonSlot': 6,
            'seasonSelection': 'All twelve original monthly images are resident in native-size texture arrays. setSeasonPhase supplies adjacent monthly indices and a uniform linear blend, derived from the deterministic simulation tick by original-seasons.js. Initial summer is an application starting choice; native Calendar constructor year length1200seconds is preserved as the default cycle rate. Native runtime float accumulation, seasonal tints and full weather scheduler are separate.',
            'uvTilesPerRepeat': 4, 'transitionRadiusTiles': 2,
            'uvAndTransitions': 'Own four-tile texture repeat and smooth class weights perturbed with original Terr_Mask alpha. Native UV, mask rotations and material order remain unresolved.',
            'water': 'Original 512px normal map plus native water surface layers and grayscale color LUT. NSF UV rates and reflection distortion0.1 are retained. Source native time phase, material input bindings and water vertex-color construction are not completely recovered. Texture repeat, base-color blend, depth opacity, Fresnel and specular are explicit own rendering choices. Optional reflection texture is sampled using the source two-normal average/distortion formula; without it a sky color approximation is used.',
            'unbound': ['caustic frame timing', 'palette-indexed compatibility water path', 'native terrain detail-distance thresholds', 'complete seasonal tint and weather'],
        },
    }
    dest = OUT / 'manifest.json'
    dest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf8')
    return manifest

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--install', type=Path, default=INSTALL)
    result = import_terrain(parser.parse_args().install)
    print(json.dumps({'types': len(result['terrainTypes']), 'definitions': len(result['textureDefinitions']),
        'seasonalSets': len(result['seasonalSets']), 'images': len(result['textures']),
        'nativeBytes': sum((ROOT / 'docs' / row['url']).stat().st_size for row in result['textures'].values())}, indent=2))
