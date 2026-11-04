"""Import source-proven native high-water media without changing the installation."""
from pathlib import Path
import argparse
import hashlib
import io
import json
import sys
import zipfile
from PIL import Image

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parent
from extract_ee2_textures import decode_texcache

INSTALL = Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
OUT = PROJECT / 'docs/local-original/water-native'
def import_native_water(installation=INSTALL, output=OUT, evidence_path=None):
    installation=Path(installation).resolve();output=Path(output).resolve()
    if output==installation or installation in output.parents:
        raise ValueError('Output must stay outside the original installation')
    if evidence_path is not None:
        evidence=Path(evidence_path)
        if not evidence.is_file():raise FileNotFoundError(evidence)
    else:
        evidence=next((path for path in [PROJECT.parent/'EE2-Research/analysis/native-water-appearance-evidence.json',PROJECT/'research/analysis/native-water-appearance-evidence.json'] if path.is_file()),None)
    output.mkdir(parents=True,exist_ok=True)
    sha = lambda raw: hashlib.sha256(raw).hexdigest()
    images = {}
    with zipfile.ZipFile(installation / 'zips/textures.zip') as archive:
        for key, member in {**{f'skin{i}': f'cache/texcache/Skin_{i}.texcache' for i in range(1, 7)},
                            'normal': 'cache/texcache/Waterbump.texcache',
                            'palette': 'Graphics/Fx/NoConvert/WaterLayer1Palette.tga',
                            'lookup': 'Graphics/Fx/NoConvert/WaterColor_OceanRiverCoast.tga'}.items():
            raw = archive.read(member)
            if member.endswith('.texcache'):
                image, metadata = decode_texcache(raw)
            else:
                image = Image.open(io.BytesIO(raw)).convert('RGBA');image.load()
                metadata = {'format': 'original-TGA', 'width': image.width, 'height': image.height,
                            'transformation': 'Original pixels decoded; no rescaling/recoloring'}
            dest = output / f'{key}.png';image.save(dest, optimize=True)
            images[key] = {'url': dest.name, 'archive': 'textures.zip', 'member': member,
                           'sourceSha256': sha(raw), 'pngSha256': sha(dest.read_bytes()), **metadata}
            if key == 'palette':
                palette = [pixel[3] for pixel in image.getdata()]
            if key == 'lookup':
                lookup = [list(pixel[:3]) for pixel in image.getdata()]
    manifest = {'format': 'ee2-native-water-v1',
                'images': images, 'paletteAlphaBytes': palette, 'lookupRGBBytes': lookup,
                'profiles': {'high': {'dark': 'skin1', 'detail': 'skin2', 'gloss': 'skin4'},
                             'fallback': {'dark': 'skin1', 'detail': 'skin5', 'gloss': 'skin6'}},
                'native': {'depthRange': 2, 'reflectionAlpha': .3199999928474426,
                           'baseFlow': [[.0155, .02], [-.015, .01]],
                           'reflectionFlow': [[.01125, .02], [-.01125, .01]],
                           'reflectionDistortion': .1, 'uvWarpFrequency': .25, 'uvWarpAmplitude': .029999999329447746},
                'adapter': {'uvScale': [1/12, 1/12], 'normalUvScale': [1/8, 1/8],
                            'ownClassToMovement': [7, 5, 0, 0, 4, 3],
                            'reason': 'Own archipelago shallow class maps to coast; native river class requires explicit movement input. UV scales and map-class mapping are adapters, not recovered defaults.'},
                'evidence': {'file': 'native-water-appearance-evidence.json',
                             'sha256': sha(evidence.read_bytes()) if evidence is not None else None}}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    return manifest

if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--install',type=Path,default=INSTALL)
    parser.add_argument('--out',type=Path,default=OUT)
    parser.add_argument('--evidence',type=Path)
    args=parser.parse_args()
    manifest=import_native_water(args.install,args.out,args.evidence)
    print(json.dumps({'output':str(args.out),'images':len(manifest['images']),
        'pngBytes':sum((args.out/row['url']).stat().st_size for row in manifest['images'].values()),
        'manifestSha256':hashlib.sha256((args.out/'manifest.json').read_bytes()).hexdigest()}))
