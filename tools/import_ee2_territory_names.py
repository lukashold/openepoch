"""Build numeric catalogs and private localized Fullmap name dictionaries."""
import argparse
import csv
from hashlib import sha256
from io import StringIO
import json
from pathlib import Path
import re
from zipfile import ZipFile


CIVILIZATIONS = ('German', 'English', 'Roman', 'American', 'Greek', 'Chinese', 'Korean',
                 'Japanese', 'Egyptian', 'Turkish', 'Babylonian', 'Inca', 'Aztec', 'Mayan')
REGIONAL_SHA = 'e0727a8316d8ae3398b88c3e6d8cb75a9d482a2bc18d28f008633831ee0d81df'
TEXT_SHA = 'f4e66146f8edb97810e4b5bc26018e1c7f63b3ffa432782cad08f3dc8da30c54'
CITY_TEXT_SHA = '83339556dd5f7cf94d4c32d8485e9d8415970f6f73fbff53af57969a363bf4e4'
UNIT_TEXT_SHA = '85134b6179bbb35f8a39de295676fc5ec55466693a9849026912c75192050900'


def civilization_name_pools(data, field):
    source = re.sub(r'//[^\n]*', '', data.decode('cp1252'))
    pools = {}
    for match in re.finditer(rf'\b{field}\s*=\s*\[', source):
        depth = 1
        for end in range(match.end(), len(source)):
            depth += (source[end] == '[') - (source[end] == ']')
            if depth == 0:
                break
        else:
            raise ValueError(f'Unterminated {field} name list')
        for block in re.findall(r'\bCivilizationNames\s*\{([^{}]*)\}', source[match.end():end]):
            civilization = re.search(r'\bCivilization\s*=\s*(\w+)', block)[1]
            items = re.search(r'\bNames\s*=\s*\[([^]]*)\]', block)[1]
            assert civilization not in pools
            pools[civilization] = [token.casefold() for token in re.findall(r'\btx_\w+', items, re.I)]
    assert set(pools) == set(CIVILIZATIONS) and all(pools.values())
    return [pools[civ] for civ in CIVILIZATIONS]


def localized_text(text):
    return {row[0].strip().casefold(): row[1].strip().strip('"')
            for row in csv.reader(StringIO(text.decode('utf-8-sig')))
            if len(row) >= 2 and row[0].strip().casefold().startswith('tx_')}


def write_catalog(root, pools, defaults, localized, rust_file, dictionary):
    tokens = list(dict.fromkeys(defaults + sum(pools, [])))
    index = {token: number for number, token in enumerate(tokens)}
    rows = [[index[token] for token in pool] for pool in pools]
    names = [localized[token] for token in tokens]
    assert len(names) < 65536 and all(names)
    fingerprint = int.from_bytes(sha256(json.dumps(pools, separators=(',', ':')).encode()).digest()[:4], 'little')
    numeric = ('//! Numeric catalog generated from the installed data. Localized strings are separate.\n'
               f'pub const VERSION: u32 = 0x{fingerprint:08x};\n'
               f'pub const NAME_LIMIT: u16 = {len(names)};\n'
               'pub const POOLS: [&[u16]; 14] = [\n' +
               ''.join(f'    &{row},\n' for row in rows) + '];\n')
    (root / 'src' / rust_file).write_text(numeric, encoding='utf-8')
    output = root / 'docs/local-original/fullmap-native' / f'{dictionary}-names.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({'format': f'ee2-fullmap-{dictionary}-names-v1', 'catalog': fingerprint,
                                  'names': names}, ensure_ascii=False) + '\n', encoding='utf-8')
    print(f'{dictionary} catalog {fingerprint:08x}, {len(names)} labels, 14 ordered pools')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('install', type=Path, help='Local Empire Earth II installation directory')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    with ZipFile(args.install / 'zips' / 'db.zip') as archive:
        inputs = {name: archive.read(name) for name in ('db/Simulation/RegionalSettings.ddf',
            'db/Text/dbtext_territory_names.utf8', 'db/Text/dbtext_miscsim.utf8', 'db/Text/dbtext_unittypenames.utf8')}
    regional, territory, city, units = inputs.values()
    hashes = tuple(sha256(data).hexdigest() for data in inputs.values())
    if hashes != (REGIONAL_SHA, TEXT_SHA, CITY_TEXT_SHA, UNIT_TEXT_SHA):
        raise ValueError('These name catalogs expect the verified German regional data')
    write_catalog(root, civilization_name_pools(regional, 'CivilizationTerritories'),
                  ['tx_terr_unclaimed', 'tx_terr_default'], localized_text(territory),
                  'territory_name_catalog21.rs', 'territory')
    write_catalog(root, civilization_name_pools(regional, 'CivilizationCities'), ['tx_utn_citycenter_name'],
                  localized_text(city) | localized_text(units), 'city_name_catalog22.rs', 'city')


if __name__ == '__main__':
    main()
