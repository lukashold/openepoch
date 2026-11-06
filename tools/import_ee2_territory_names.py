"""Build the numeric catalog and a private localized Fullmap name dictionary."""
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


def territory_pools(data):
    source = re.sub(r'//[^\n]*', '', data.decode('cp1252'))
    pools = {}
    for match in re.finditer(r'\bCivilizationTerritories\s*=\s*\[', source):
        depth = 1
        for end in range(match.end(), len(source)):
            depth += (source[end] == '[') - (source[end] == ']')
            if depth == 0:
                break
        else:
            raise ValueError('Unterminated territory name list')
        for block in re.findall(r'\bCivilizationNames\s*\{([^{}]*)\}', source[match.end():end]):
            civilization = re.search(r'\bCivilization\s*=\s*(\w+)', block)[1]
            items = re.search(r'\bNames\s*=\s*\[([^]]*)\]', block)[1]
            assert civilization not in pools
            pools[civilization] = [token.casefold() for token in re.findall(r'\btx_\w+', items, re.I)]
    assert set(pools) == set(CIVILIZATIONS) and all(pools.values())
    return [pools[civ] for civ in CIVILIZATIONS]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('install', type=Path, help='Local Empire Earth II installation directory')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    with ZipFile(args.install / 'zips' / 'db.zip') as archive:
        regional = archive.read('db/Simulation/RegionalSettings.ddf')
        text = archive.read('db/Text/dbtext_territory_names.utf8')
    if sha256(regional).hexdigest() != REGIONAL_SHA or sha256(text).hexdigest() != TEXT_SHA:
        raise ValueError('This name catalog expects the verified German regional data')
    pools = territory_pools(regional)
    tokens = list(dict.fromkeys(['tx_terr_unclaimed', 'tx_terr_default'] + sum(pools, [])))
    index = {token: number for number, token in enumerate(tokens)}
    rows = [[index[token] for token in pool] for pool in pools]
    localized = {row[0].strip().casefold(): row[1].strip().strip('"')
                 for row in csv.reader(StringIO(text.decode('utf-8-sig')))
                 if len(row) >= 2 and row[0].strip().casefold().startswith('tx_')}
    names = [localized[token] for token in tokens]
    assert len(names) < 65536 and all(names)
    fingerprint = int.from_bytes(sha256(json.dumps(pools, separators=(',', ':')).encode()).digest()[:4], 'little')
    numeric = ('//! Numeric catalog generated from the installed data. Localized strings are separate.\n'
               f'pub const VERSION: u32 = 0x{fingerprint:08x};\n'
               f'pub const NAME_LIMIT: u16 = {len(names)};\n'
               'pub const POOLS: [&[u16]; 14] = [\n' +
               ''.join(f'    &{row},\n' for row in rows) + '];\n')
    (root / 'src/territory_name_catalog21.rs').write_text(numeric, encoding='utf-8')
    output = root / 'docs/local-original/fullmap-native/territory-names.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({'format': 'ee2-fullmap-territory-names-v1', 'catalog': fingerprint,
                                  'names': names}, ensure_ascii=False) + '\n', encoding='utf-8')
    print(f'catalog {fingerprint:08x}, {len(names)} labels, 14 ordered pools')


if __name__ == '__main__':
    main()
