"""Pixel controls for the real production planar capture, no GUI/native launch."""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageChops
from replay_world_gles import replay


def difference(a, b):
    first, second = Image.open(a).convert('RGB'), Image.open(b).convert('RGB')
    if first.size != second.size:
        raise AssertionError('Pixel comparison dimensions differ')
    diff = ImageChops.difference(first, second)
    pixels = list(diff.getdata())
    return {'pixels': sum(any(channel > 1 for channel in p) for p in pixels),
            'maxChannel': max(max(p) for p in pixels),
            'sumChannels': sum(sum(p) for p in pixels)}


def run(capture, output_dir, report_path):
    capture, output_dir, report_path = Path(capture), Path(output_dir), Path(report_path)
    payload = json.loads(capture.read_text(encoding='utf-8'))
    checks = 0

    def require(condition, message):
        nonlocal checks
        if not condition:
            raise AssertionError(message)
        checks += 1

    require(payload.get('reflection', {}).get('enabled'), 'Production reflection capture must be active')
    reflection_calls = [(i, call) for i, call in enumerate(payload['draws']) if call.get('framebuffer') is not None]
    require(len(reflection_calls) > 1, 'Terrain and native objects must both draw into the FBO')
    framebuffer = reflection_calls[0][1]['framebuffer']
    terrain_program = reflection_calls[0][1]['program']
    texture = payload['reflection']['texture']
    for i, call in reflection_calls:
        require(call['framebuffer'] == framebuffer, 'One planar target in this world capture')
        require(call['uniforms']['uClipEnabled']['value'] == 1, 'World clip enabled on every reflected terrain/model draw')
        require(call['uniforms']['uClipPlane']['value'] == [0, 1, 0, 0], 'Keep source world geometry above water')
        require(texture not in call['textures'].values(), 'No FBO/sampler feedback in the reflection draws')
    main_calls = [call for call in payload['draws'] if call.get('framebuffer') is None]
    for call in main_calls:
        if 'uClipEnabled' in call['uniforms']:
            require(call['uniforms']['uClipEnabled']['value'] == 0, 'Main pass explicitly resets reflection clipping')
        require(call['state']['viewport'] == [0, 0, payload['width'], payload['height']], 'Main viewport restored')
    water_calls = [call for call in main_calls if 'uReflectionAlpha' in call['uniforms']]
    require(len(water_calls) == 1 and abs(water_calls[0]['uniforms']['uReflectionAlpha']['value'] - .32) < 1e-7, 'Production water uses the source reflection overlay alpha')
    require(water_calls[0]['textures']['7:3553'] == texture, 'Production water samples the exact captured color texture')
    output_dir.mkdir(parents=True, exist_ok=True)
    full = replay(capture, output_dir / 'native4-reflection-coast.png', report_path.with_name('native4-reflection-world-coast.json'))
    require(len(full['reflectionTargets']) == 1, 'Real GLES FBO has a readable native-size color attachment')
    full_target = full['reflectionTargets'][0]['output']
    controls = {}
    for mode in ('no-objects', 'no-clip', 'no-reflection'):
        data = copy.deepcopy(payload)
        if mode == 'no-objects':
            removed = {i for i, call in reflection_calls if call['program'] != terrain_program}
            data['commands'] = [command for command in data['commands'] if command.get('draw') not in removed]
            require(bool(removed), 'Object control removes real model draws only from the reflection target')
        elif mode == 'no-clip':
            for i, call in reflection_calls:
                data['draws'][i]['uniforms']['uClipEnabled']['value'] = 0
        else:
            removed = {i for i, call in enumerate(data['draws']) if 'uReflectionAlpha' in call['uniforms']}
            data['commands'] = [command for command in data['commands'] if command.get('draw') not in removed]
        path = capture.with_name(capture.stem + f'-control-{mode}.json')
        path.write_text(json.dumps(data), encoding='utf-8')
        controls[mode] = replay(path, output_dir / f'native4-reflection-control-{mode}.png')
    object_pixels = difference(full_target, controls['no-objects']['reflectionTargets'][0]['output'])
    clipping_pixels = difference(full_target, controls['no-clip']['reflectionTargets'][0]['output'])
    water_pixels = difference(full['output'], controls['no-reflection']['output'])
    require(object_pixels['pixels'] > 50, 'Native objects contribute actual reflected pixels beyond terrain')
    require(clipping_pixels['pixels'] > 1000, 'Above-water clipping removes submerged terrain/geometry from target')
    require(water_pixels['pixels'] > 1000, 'The actual water shader displays the captured scene')
    report = {'format': 'openepoch-native4-reflection-world-pixel-tests-v1', 'passed': True,
              'checks': checks, 'wasmSha256': payload['sourceHashes']['wasmSha256'],
              'sourceHashes': payload['sourceHashes'], 'context': full['context'],
              'reflectionDraws': len(reflection_calls), 'mainDraws': len(main_calls),
              'nativeObjectContribution': object_pixels, 'submergedClipContribution': clipping_pixels,
              'waterReflectionContribution': water_pixels,
              'productionWorld': str(report_path.with_name('native4-reflection-world-coast.json').resolve()),
              'output': full['output'], 'target': full_target,
              'scope': 'Actual WASM coast world; production helper, terrain/model/native source water shaders, native models/textures and GPU FBO. Diagnostic controls omit reflected objects, disable clipping, or omit only the source alpha0.32 reflection overlay while retaining base water, without changing source shader code.',
              'limitations': ['WebGL scene capture resolution, above-water fragment clipping and fallback sky color are explicit adapters unless separately source-proven.', 'No original-game same-scene visual comparison or browser/Mac execution.']}
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('capture', type=Path)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = run(args.capture, args.output_dir, args.report)
    print(json.dumps({key: result[key] for key in ('passed', 'checks', 'reflectionDraws', 'nativeObjectContribution', 'submergedClipContribution', 'waterReflectionContribution')}))
