"""Auditable base EE2 statistics for the existing roster, without editing the game.

Values come from db.zip CSV/DDF declarations. Visual adapters remain explicit.
No patch precedence, full roster, combat engine or native world-unit conversion
is inferred. Run after the local model import when model evidence is available.
"""
import argparse
import csv
import hashlib
import json
import math
import pathlib
import re
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_INSTALL = pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
RESOURCES = ['FOOD', 'WOOD', 'IRON', 'STONE', 'GOLD', 'TIN', 'SALTPETER', 'OIL', 'URANIUM']
NUM = r'[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?'
COMMENT = re.compile(r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*.*?\*/', re.S)
BUILDINGS = {
    0: ('CityCenter', True, 'Town Center is the CityCenter name alias'),
    1: ('House', True, ''), 2: ('Farm', True, ''),
    3: ('Barracks', True, ''),
    4: ('Mill', False, 'Workshop is a separate existing gameplay role using Mill graphics'),
    5: ('Dock', True, ''), 6: ('Airport', True, 'Airfield is the Airport name alias'),
    7: ('Outpost', False, 'Watchtower uses Outpost graphics; Wall_Tower is a different original type'),
    47: ('University', True, ''), 48: ('Mill', True, ''),
    49: ('Stable', True, ''), 50: ('Market', True, ''),
    51: ('Temple', True, ''), 52: ('Warehouse', True, ''),
    53: ('OilRig', True, 'OilDerrick is the OilRig name alias'),
    54: ('UraniumMine', True, ''),
    55: ('HercFacility', True, ''),
    56: ('Fortress', True, ''),
}
# Identity is a semantic roster decision, separate from verified source numbers.
ADAPTERS = {
    9: 'Clubman uses the original Maceman visual',
    10: 'Slinger uses Bowman graphics; this is not a sling unit',
    12: 'Archer uses the original Composite Bowman visual',
    17: 'Marine is an independent gameplay archetype using AssaultRifleman graphics',
    18: 'Horseman uses Mounted Infantry graphics',
    19: 'Knight uses Armored Cavalry graphics',
    24: 'Howitzer uses Siege Gun graphics',
    28: 'SAM Launcher uses Paladin Defense graphics',
    43: 'Sentinel Mech is an independent gameplay archetype',
    44: 'Aurora Interceptor is an independent gameplay archetype',
    45: 'Leviathan Sub is an independent gameplay archetype',
    46: 'Aegis Hover Tank is an independent gameplay archetype',
}


def clean_comments(text):
    """Keep positions/newlines so every field can retain its original line."""
    return COMMENT.sub(lambda m: m[0] if m[0].startswith('"') else
                       ''.join('\n' if c == '\n' else ' ' for c in m[0]), text)


def brace_end(text, start):
    depth = 1
    quoted = False
    escape = False
    for i in range(start, len(text)):
        c = text[i]
        if quoted:
            if escape:
                escape = False
            elif c == '\\':
                escape = True
            elif c == '"':
                quoted = False
        elif c == '"':
            quoted = True
        elif c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                return i
    raise ValueError('Unclosed DDF declaration')


def declarations(text, kind):
    for m in re.finditer(r'\b' + re.escape(kind) + r'\s+(\w+)\s*\{', text, re.I):
        end = brace_end(text, m.end())
        yield m[1], text[m.end():end], m.end(), m.start()


def sections(text, name):
    for m in re.finditer(r'\b' + re.escape(name) + r'\s*\{', text, re.I):
        end = brace_end(text, m.end())
        yield text[m.end():end], m.end()


def numeric_assignments(text):
    return [(m[1], float(m[2]), m.start()) for m in
            re.finditer(r'\b(\w+)\s*=\s*(' + NUM + r')(?=\s|;|$)', text)]


def scalar(text, name):
    m = re.search(r'\b' + re.escape(name) + r'\s*=\s*(".*?"|[\w.\\/-]+)', text, re.I)
    return m[1].strip('"') if m else None


def model_stem(name):
    stem = pathlib.PurePosixPath(name.replace('\\', '/')).name.rsplit('.', 1)[0].lower()
    return re.sub(r'_(?:we|fe|me|mi|af|ar|sa)$', '', stem)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class Database:
    def __init__(self, archive):
        self.archive = pathlib.Path(archive)
        self.zip = zipfile.ZipFile(self.archive)
        self.paths = {name.lower(): name for name in self.zip.namelist()}
        self.files = {}
        self.blocks = {}
        self.resolved = {}
        for path in self.zip.namelist():
            if not (path.lower().startswith('db/units/') and path.lower().endswith('.ddf')):
                continue
            text = clean_comments(self.read(path))
            for kind in ['UnitType', 'UnitModel', 'UpgradeSet', 'UpgradeAbilities', 'UpgradeUnitTypeUpgrade']:
                for name, body, body_start, declaration_start in declarations(text, kind):
                    key = (kind.lower(), name.lower())
                    if key in self.blocks:
                        raise ValueError('Duplicate DDF declaration: ' + str(key))
                    self.blocks[key] = {
                        'kind': kind, 'name': name, 'body': body, 'path': path,
                        'body_start': body_start,
                        'line': text.count('\n', 0, declaration_start) + 1,
                        'text': text,
                    }

    def close(self):
        self.zip.close()

    def read(self, name):
        path = self.paths[name.lower()]
        if path not in self.files:
            raw = self.zip.read(path)
            try:
                text = raw.decode('utf-8-sig')
            except UnicodeDecodeError:
                text = raw.decode('cp1252')
            self.files[path] = {'raw': raw, 'text': text.replace('\r', ''), 'sha256': sha(raw)}
        return self.files[path]['text']

    def ref(self, path, line, declaration=None):
        self.read(path)
        result = {'archive': 'db.zip', 'entry': path, 'line': line,
                  'sha256': self.files[path]['sha256']}
        if declaration:
            result['declaration'] = declaration
        return result

    def block(self, kind, name):
        return self.blocks.get((kind.lower(), name.lower()))

    def block_ref(self, data, offset=0):
        line = data['text'].count('\n', 0, data['body_start'] + offset) + 1
        return self.ref(data['path'], line, data['kind'] + ' ' + data['name'])

    def table(self, name):
        text = self.read(name)
        path = self.paths[name.lower()]
        lines = text.splitlines()
        header = next(csv.reader([lines[0].removeprefix('//').strip()]))
        seen = {}
        for i, column in enumerate(header):
            if column in seen:
                header[i] = column + '_PLURAL'
            seen[column] = True
        result = []
        for line_number, line in enumerate(lines[1:], 2):
            if not line.strip() or line.lstrip().startswith('//'):
                continue
            values = next(csv.reader([line]))
            if len(values) != len(header):
                raise ValueError(f'{path}:{line_number}: table width mismatch')
            row = dict(zip(header, values))
            row['_source'] = self.ref(path, line_number)
            result.append(row)
        return result

    def resolve(self, name, stack=()):
        key = name.lower()
        if key in self.resolved:
            return self.resolved[key]
        if key in stack:
            raise ValueError('Cyclic UnitType inheritance: ' + name)
        data = self.block('UnitType', name)
        if not data:
            raise ValueError('Missing UnitType parent: ' + name)
        parent = scalar(data['body'], 'parent')
        base = self.resolve(parent, stack + (key,)) if parent else {'properties': {}, 'abilities': {}, 'chain': [], 'attribute_declarations': []}
        properties = dict(base['properties'])
        abilities = {kind: dict(values) for kind, values in base['abilities'].items()}
        attribute_declarations=list(base.get('attribute_declarations',[]))
        for attributes in re.finditer(r'\battributes\s*=\s*\[([^]]*)\]',data['body'],re.I):
            attribute_declarations.append((re.findall(r'\w+',attributes[1]),self.block_ref(data,attributes.start())))
        for body, offset in sections(data['body'], 'properties'):
            for field, value, position in numeric_assignments(body):
                properties[field.lower()] = (value, self.block_ref(data, offset + position))
        for ability in ['Move', 'NavalMove', 'AircraftMove', 'LOS', 'GatherFarm', 'Build',
                        'Garrison', 'Garrisonable', 'Attack', 'resourceFarm']:
            for body, offset in sections(data['body'], ability):
                values = abilities.setdefault(ability.lower(), {})
                for field, value, position in numeric_assignments(body):
                    values[field.lower()] = (value, self.block_ref(data, offset + position))
                for field in ['gatherType', 'rps', 'WeaponType', 'missileName', 'garrisonType']:
                    value = scalar(body, field)
                    if value is not None:
                        values[field.lower()] = (value, self.block_ref(data, offset))
                types=re.search(r'\bgarrisonTypes\s*=\s*\[([^]]*)\]',body,re.I)
                if types:values['garrisontypes']=(re.findall(r'\w+',types[1]),self.block_ref(data,offset+types.start()))
        result = {'properties': properties, 'abilities': abilities, 'attribute_declarations': attribute_declarations,
                  'chain': base['chain'] + [self.block_ref(data)],
                  'rps': scalar(data['body'], 'rps') or base.get('rps'),
                  'placement_type': scalar(data['body'], 'placementType') or base.get('placement_type')}
        self.resolved[key] = result
        return result


def field(value=None, sources=(), note=None):
    result = {'value': value, 'verified': value is not None, 'sources': list(sources)}
    if note:
        result['note'] = note
    return result


def value_from(values, key):
    item = values.get(key.lower())
    return field(item[0], [item[1]]) if item else field()


def row_number(row, column):
    raw = row.get(column, '').strip()
    if not raw:
        return None
    value = float(raw)
    if not math.isfinite(value):
        raise ValueError('Nonfinite original number: ' + column)
    return value


def csv_field(row, column):
    return field(row_number(row, column), [row['_source']])


def cost_field(row):
    values = [row_number(row, name) for name in RESOURCES]
    if any(value is None for value in values):
        return field(note='Incomplete resource columns are not replaced by zero')
    if any(value != int(value) or not 0 <= value <= 65535 for value in values):
        raise ValueError('Original cost does not fit exact u16 resource slots')
    return field([int(value) for value in values], [row['_source']])


def upgrade_abilities(db, original, epoch, abilities):
    result = {key: dict(values) for key, values in abilities.items()}
    # Only the explicit standard per-epoch UpgradeAbilities declarations are used.
    for ability in ['Garrison', 'Attack', 'Move', 'NavalMove', 'AircraftMove']:
        candidates = []
        prefix = original + 'Epoch'
        for data in db.blocks.values():
            match = re.fullmatch(re.escape(prefix) + r'(\d+)' + ability, data['name'], re.I)
            if data['kind'] == 'UpgradeAbilities' and match and int(match[1]) <= epoch:
                candidates.append((int(match[1]), data))
        for _, data in sorted(candidates, key=lambda item: item[0]):
            for body, offset in sections(data['body'], ability):
                values = result.setdefault(ability.lower(), {})
                for key, number, position in numeric_assignments(body):
                    values[key.lower()] = (number, db.block_ref(data, offset + position))
                for key in ['gatherType', 'WeaponType', 'missileName', 'garrisonType']:
                    value = scalar(body, key)
                    if value:
                        values[key.lower()] = (value, db.block_ref(data, offset))
    return result


def make_fields(db, original, epoch, upgrade=None, production=None):
    resolved = db.resolve(original)
    props = resolved['properties']
    abilities = upgrade_abilities(db, original, epoch, resolved['abilities'])
    result = {
        'hp': csv_field(upgrade, 'HP') if upgrade else value_from(props, 'hitpoints'),
        'buildtime': csv_field(upgrade, 'BUILDTIME') if upgrade else
                     csv_field(production, 'TIME') if production else field(),
        'cost9': cost_field(upgrade or production) if upgrade or production else field(),
        'pop': value_from(props, 'popCount'),
        'pop_capacity': value_from(props, 'popCapacity'),
        'sight': csv_field(upgrade, 'LOS') if upgrade else value_from(abilities.get('los', {}), 'range'),
        'raw_size': field(),
        'raw_footprint': field(),
        'raw_speed': field(),
        'gather': field(),
        'farm_rate': value_from(abilities.get('gatherfarm', {}), 'farmRate'),
        'carry': value_from(abilities.get('gatherfarm', {}), 'carryLimit'),
        'farm_carry': value_from(abilities.get('gatherfarm', {}), 'farmCarryLimit'),
        'repair_rate': value_from(abilities.get('build', {}), 'repairRate'),
        'damage': csv_field(upgrade, 'DAMAGE') if upgrade else value_from(abilities.get('attack', {}), 'damage'),
        'range': csv_field(upgrade, 'RANGE') if upgrade else value_from(abilities.get('attack', {}), 'range'),
        'reload': csv_field(upgrade, 'RELOAD') if upgrade else value_from(abilities.get('attack', {}), 'reloadTime'),
        'garrison_slots': value_from(abilities.get('garrison', {}), 'numOfSlots'),
        'garrison_heal': value_from(abilities.get('garrison', {}), 'healPoints'),
        'garrison_attack_bonus': value_from(abilities.get('garrison', {}), 'attackBonus'),
        'garrison_size': field(note='No inherited Fortress eligibility declaration: no slot use assigned.'),
        'fortress_garrison': field(False, resolved['chain'], 'No inherited Fortress eligibility declaration found'),
        'attack_missile': value_from(abilities.get('attack', {}), 'missileName'),
        'generation': value_from(abilities.get('garrison', {}), 'gatherRate'),
        'generation_resource': field(),
    }
    if result['buildtime']['value'] == 0 and not upgrade:
        result['buildtime'] = field(note='Production placeholder TIME=0 has no verified build-time override')
    for output, keys in [('raw_size', ('sizex', 'sizey')), ('raw_footprint', ('footprintsizex', 'footprintsizey'))]:
        if all(key in props for key in keys):
            result[output] = field([props[key][0] for key in keys], [props[key][1] for key in keys],
                                   'Native values only; conversion into simulation tile size remains separate')
    movement = [abilities[name] for name in ['move', 'navalmove', 'aircraftmove']
                if 'speed' in abilities.get(name, {})]
    if len(movement) == 1:
        result['raw_speed'] = value_from(movement[0], 'speed')
        result['raw_speed']['note'] = 'Native units; no unverified world-unit scaling applied'
    gather = abilities.get('gatherfarm', {})
    gather_keys = [name.lower() + 'rate' for name in RESOURCES]
    if all(key in gather for key in gather_keys):
        result['gather'] = field([gather[key][0] for key in gather_keys],
                                 [gather[key][1] for key in gather_keys])
    garrison = abilities.get('garrison', {})
    allowed=abilities.get('garrisonable',{}).get('garrisontypes')
    if allowed:
        result['fortress_garrison']=field('Fortress' in allowed[0],[allowed[1]],'Exact inherited Garrisonable.garrisonTypes membership')
    if result['fortress_garrison']['value']:
        leaders=[source for names,source in resolved['attribute_declarations'] if 'Leaders' in names]
        result['garrison_size']=(field(1,leaders,'Native Leaders exception: required-slots getter returns exactly one.')
            if leaders else value_from(props,'popCount'))
        result['garrison_size']['note']=('Native 0x007E63B0: Leaders use one slot; other eligible units use inherited popCount. '
            'Property binder 0x0053A0F0 and loader 0x0053ACFE identify UnitType+0xA0 as popCount; '
            'see native_garrison_slots evidence. No CSV population-capacity or geometric-size substitution.')
        result['garrison_size']['semanticEvidence']='native_garrison_slots'
    resource = garrison.get('gathertype')
    if resource:
        name = str(resource[0]).upper()
        index = RESOURCES.index(name) if name in RESOURCES else 9 if name == 'TECH' else None
        result['generation_resource'] = field(index, [resource[1]],
            'Slots 0..8 use internal resource order; 9 is technology, not a natural resource')
    # Citizen HP is an explicit per-epoch UpgradeSet factor, not an upgrade CSV row.
    if original == 'CitizenM':
        candidates = []
        for data in db.blocks.values():
            match = re.fullmatch(r'IntCitizenUpgradeEpoch(\d+)', data['name'])
            if data['kind'] == 'UpgradeSet' and match and int(match[1]) <= epoch:
                candidates.append((int(match[1]), data))
        for _, data in sorted(candidates, key=lambda item: item[0]):
            for body, offset in sections(data['body'], 'UpgradeUnitTypeFactor'):
                if (scalar(body, 'UnitType') == 'Citizen' and scalar(body, 'factor') == 'HitPoints'
                        and scalar(body, 'method') == 'Set'):
                    number = next((value for key, value, _ in numeric_assignments(body) if key.lower() == 'value'), None)
                    if number is not None:
                        result['hp'] = field(number, [db.block_ref(data, offset)],
                            'Citizen ancestor factor with upgradeChildren=1 also targets CitizenM')
    return result, resolved


def rust_option(value, shape):
    if value is None:
        return 'None'
    if shape == 'float':
        return f'Some({float(value)!r}_f32)'
    if shape == 'int':
        if int(value) != value:
            raise ValueError('Noninteger typed field')
        return f'Some({int(value)})'
    if shape == 'float_array':
        return 'Some([' + ','.join(f'{float(x)!r}_f32' for x in value) + '])'
    if shape == 'int_array':
        return 'Some([' + ','.join(str(int(x)) for x in value) + '])'
    if shape == 'str':
        return 'Some('+json.dumps(value)+')'
    if shape == 'bool':
        return 'true' if value else 'false'
    raise ValueError(shape)


def write_rust(rows, class_rows, production_from, path):
    fields = [
        ('hp', 'f32', 'float'), ('buildtime', 'f32', 'float'),
        ('cost9', '[u16;9]', 'int_array'), ('pop', 'u8', 'int'),
        ('pop_capacity', 'u16', 'int'), ('sight', 'f32', 'float'),
        ('raw_size', '[f32;2]', 'float_array'), ('raw_footprint', '[f32;2]', 'float_array'),
        ('raw_speed', 'f32', 'float'), ('gather', '[f32;9]', 'float_array'),
        ('farm_rate', 'f32', 'float'), ('carry', 'f32', 'float'),
        ('farm_carry', 'f32', 'float'), ('repair_rate', 'f32', 'float'),
        ('damage', 'f32', 'float'), ('range', 'f32', 'float'), ('reload', 'f32', 'float'),
        ('garrison_slots', 'u8', 'int'), ('generation', 'f32', 'float'),
        ('generation_resource', 'u8', 'int'),
        ('garrison_heal','f32','float'), ('garrison_attack_bonus','f32','float'),
        ('garrison_size','u8','int'), ('fortress_garrison','bool','bool'),
        ('attack_missile',"&'static str",'str'),
    ]
    output = [
        '// Generated by tools/import_ee2_units.py from the explicitly selected base db.zip.',
        '// None means unverified/missing. Native size/speed/range must not be blindly rescaled.',
        '// Internal cost slots: FOOD WOOD IRON STONE GOLD TIN SALTPETER OIL URANIUM.',
        '#[derive(Clone, Copy, Debug)]',
        'pub struct UnitRow {',
        ' pub id:u8, pub epoch:u8, pub until:u8, pub source_epoch:u8,',
        ' pub source_unit:&\'static str, pub source_visual:&\'static str, pub source_host:&\'static str,',
        ' pub identity_verified:bool, pub model_verified:bool,',
        *[f' pub {name}:'+ (typ if shape=='bool' else f'Option<{typ}>')+',' for name,typ,shape in fields],
        '}',
        'pub const ROWS:&[UnitRow]=&[',
    ]
    def emit(items):
        for row in items:
            members = [f'id:{row["id"]}', f'epoch:{row["epoch"]}', f'until:{row["until"]}',
                       f'source_epoch:{row["source_epoch"]}',
                       'source_unit:' + json.dumps(row['source_unit']),
                       'source_visual:' + json.dumps(row['source_visual']),
                       'source_host:' + json.dumps(row['source_host']),
                       'identity_verified:' + str(row['identity_verified']).lower(),
                       'model_verified:' + str(row['model_verified']).lower()]
            for name, _, shape in fields:
                data = row['fields'][name]
                members.append(name + ':' + rust_option(data['value'] if data['verified'] else None, shape))
            output.append(' UnitRow{' + ','.join(members) + '},')
    emit(rows)
    output += ['];', '// Numeric source-class progression, including variants absent from the smaller visual roster.',
               'pub const CLASS_ROWS:&[UnitRow]=&[']
    emit(class_rows)
    output += [
        '];',
        '/// Original epoch indices are 0..14; until is exclusive, at most 15.',
        'pub fn lookup(id:u8,epoch:u8)->Option<&\'static UnitRow>{',
        ' ROWS.iter().find(|row|row.id==id && epoch>=row.epoch && epoch<row.until)',
        '}',
        '/// Complete declared progression of the bound source class, not a claim of visual equivalence.',
        'pub fn lookup_class(id:u8,epoch:u8)->Option<&\'static UnitRow>{',
        ' CLASS_ROWS.iter().find(|row|row.id==id && epoch>=row.epoch && epoch<row.until)',
        '}',
        '/// Zero-based native production-node epoch. This is separate from the bound visual variant epoch.',
        'pub fn production_class_from(id:u8)->Option<u8>{match id{',
        *[f' {identifier}=>Some({epoch}),' for identifier, epoch in sorted(production_from.items())],
        ' _=>None,}}',
        'pub fn available_from(id:u8)->Option<u8>{ROWS.iter().filter(|row|row.id==id).map(|row|row.epoch).min()}',
        'pub fn available_until(id:u8)->Option<u8>{ROWS.iter().filter(|row|row.id==id).map(|row|row.until).max()}',
    ]
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text('\n'.join(output) + '\n', encoding='utf-8')
    temporary.replace(path)


def run(installation=DEFAULT_INSTALL, extra_bindings=(), unit_civs=None):
    db = Database(pathlib.Path(installation) / 'zips/db.zip')
    try:
        upgrades = db.table('db/TechTree/upgrade_unittypes.csv')
        production = db.table('db/TechTree/dbtechtreenode.csv')
        by_unit = {}
        for row in upgrades:
            # CIV-specific source families are opt-in per exact binding. The
            # original general-class/legacy visual alias path remains All-only.
            if row.get('CIV', 'All') == 'All' or (unit_civs or {}).get(row['UNIT']) == row.get('CIV'):
                by_unit.setdefault(row['UNIT'], []).append(row)
        manifest_path = ROOT/'docs/local-original/models/manifest.json'
        if not manifest_path.exists():
            raise ValueError('Import local original models first: docs/local-original/models/manifest.json is required to verify combat identities')
        manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
        model_rows = manifest['models']
        definition_names = re.findall(r'^\s+(?:unit|bld)\("([^"]+)"', (ROOT/'src/defs.rs').read_text(encoding='utf-8'), re.M)
        if len(definition_names) < 57:
            definition_names += ['OilDerrick', 'UraniumMine', 'HercFacility','Fortress'][max(0, len(definition_names)-53):]
        if len(definition_names) != 57:
            raise ValueError('Expected the explicitly mapped 57-definition EE2 roster')
        extras = {b['id']: b for b in extra_bindings}
        assert sorted(extras) == list(range(57,57+len(extras)))
        definition_names += [extras[i]['definition_name'] for i in sorted(extras)]
        visual_models = {}
        for data in db.blocks.values():
            if data['kind'] == 'UnitModel':
                filename = scalar(data['body'], 'DefaultModel')
                if filename:
                    visual_models[data['name'].lower()] = (filename, db.block_ref(data))
        by_stem = {}
        for row in upgrades:
            visual = visual_models.get(row['VISUAL'].lower())
            if visual and row.get('CIV', 'All') == 'All':
                by_stem.setdefault(model_stem(visual[0]), []).append(row)
        rows, class_rows, audit, production_from = [], [], [], {}
        for identifier, name in enumerate(definition_names):
            models = [item for item in model_rows if item['id'] == identifier]
            base_model = min(models, key=lambda item: item['epoch']) if models else None
            mapped = []
            candidate_sources = []
            identity = identifier not in ADAPTERS
            reason = ADAPTERS.get(identifier, '')
            model_verified = True
            if identifier in extras:
                original = extras[identifier]['source_unit']
                identity, reason = True, 'Explicit source family, not an inferred legacy role alias'
                model_verified = bool(base_model and base_model.get('verified'))
                mapped = sorted(by_unit.get(original, []), key=lambda item: int(item['EPOCH'])) or [None]
            elif identifier in BUILDINGS:
                original, identity, reason = BUILDINGS[identifier]
                model_verified = bool(base_model and base_model.get('verified'))
                mapped = sorted(by_unit.get(original, []), key=lambda item: int(item['EPOCH']))
                if original in ['Farm', 'OilRig', 'UraniumMine']:
                    mapped = [None]
            elif identifier == 8:
                original = 'CitizenM'
                mapped = [None] * 5
            else:
                candidates = by_stem.get(model_stem(base_model['source']['entry']), []) if base_model else []
                # Some initial CSV rows reuse a later VISUAL/EPOCH but carry
                # obsolete numbers. Follow the production node's exact UPGRADE
                # reference rather than silently selecting the first image match.
                candidate_sources = [candidate['_source'] for candidate in candidates]
                candidate_pairs = {(candidate['UNIT'], int(candidate['EPOCH'])) for candidate in candidates}
                requested = {item['UPGRADE'] for item in production
                             if (item.get('PRODUCE'), int(item['EPOCH'])) in candidate_pairs}
                selected = [candidate for candidate in candidates if candidate['UPGRADE'] in requested]
                if selected:
                    candidates = selected
                keys = {(candidate['UNIT'], int(candidate['EPOCH']), candidate['VISUAL']) for candidate in candidates}
                compared = ['HP','LOS','DAMAGE','RANGE','RELOAD','BUILDTIME'] + RESOURCES
                numeric_keys = {tuple(row_number(candidate, column) for column in compared) for candidate in candidates}
                if len(keys) == 1 and len(numeric_keys) == 1:
                    original = candidates[0]['UNIT']
                    mapped = [candidates[0]]
                else:
                    original = 'Tank' if identifier == 23 else ''
                    model_verified = False
                    reason = (reason + '; ' if reason else '') + 'No unique CSV VISUAL to actual source-NIF match'
            node_name = 'Citizen' if original == 'CitizenM' else original
            nodes = [item for item in production if item.get('PRODUCE') == node_name]
            node = min(nodes, key=lambda item: int(item['EPOCH'])) if nodes else None
            source_min = int(node['EPOCH']) if node else None
            if source_min is not None:
                production_from[identifier] = source_min-1
            binding_audit = {
                'id': identifier, 'definition_name': name, 'source_unit': original,
                'identity_verified': identity, 'role_adapter': not identity, 'reason': reason,
                'model_verified': model_verified, 'model_binding': base_model,
                'model_matching_upgrade_rows': candidate_sources,
                'production_node': node, 'records': [], 'class_records': [],
                'excluded_preproduction_class_rows': [],
            }
            if not mapped:
                binding_audit['status'] = 'unresolved; runtime fields absent rather than guessed'
                audit.append(binding_audit)
                continue
            source_rows = by_unit.get(original, [])
            for index, upgrade in enumerate(mapped):
                if original == 'CitizenM':
                    epoch = [1, 4, 7, 10, 13][index]
                    until = [4, 7, 10, 13, 16][index]
                elif upgrade:
                    epoch = int(upgrade['EPOCH'])
                    until = min([int(item['EPOCH']) for item in source_rows if int(item['EPOCH']) > epoch] or [16])
                else:
                    epoch, until = source_min, 16
                if epoch is None or not 1 <= epoch <= 15:
                    raise ValueError('Missing original availability epoch for ' + original)
                data, inherited = make_fields(db, original, epoch, upgrade, node)
                source_visual = upgrade['VISUAL'] if upgrade else 'CitizenM' if original == 'CitizenM' else original
                row = {
                    'id': identifier, 'epoch': epoch-1, 'until': min(15, until-1),
                    'source_epoch': epoch, 'source_unit': original, 'source_visual': source_visual,
                    'source_host': node.get('HOST', '') if node else '',
                    'identity_verified': identity, 'model_verified': model_verified,
                    'fields': data, 'inheritance': inherited['chain'], 'rps': inherited.get('rps'),
                    'placement_type': inherited.get('placement_type'),
                    'availability_source': node['_source'] if node else None,
                    'upgrade_source': upgrade['_source'] if upgrade else None,
                    'source_upgrade': upgrade,
                }
                rows.append(row)
                binding_audit['records'].append(len(rows)-1)
            # Retain the strict model rows above, and expose the complete declared
            # source-class progression separately for native current-epoch costs.
            if original == 'CitizenM' or original in ['Farm', 'OilRig', 'UraniumMine']:
                complete = [dict(rows[number]) for number in binding_audit['records']]
            else:
                complete = []
                grouped = {}
                for upgrade in source_rows:
                    grouped.setdefault(int(upgrade['EPOCH']), []).append(upgrade)
                strict_visuals = {rows[number]['source_visual'] for number in binding_audit['records']}
                for epoch, candidates in sorted(grouped.items()):
                    requested = node.get('UPGRADE') if node and int(node['EPOCH']) == epoch else None
                    selected = [candidate for candidate in candidates if candidate['UPGRADE'] == requested]
                    candidates = selected or candidates
                    compared = ['HP','LOS','DAMAGE','RANGE','RELOAD','BUILDTIME'] + RESOURCES
                    signatures = {tuple(row_number(candidate, column) for column in compared) for candidate in candidates}
                    if len(signatures) != 1:
                        raise ValueError(f'Conflicting native class rows require explicit resolution: {original} epoch{epoch}')
                    upgrade = candidates[0]
                    next_epoch = min([number for number in grouped if number > epoch] or [16])
                    start_runtime = max(epoch-1, (source_min or 1)-1)
                    end_runtime = min(15, next_epoch-1)
                    if start_runtime >= end_runtime:
                        binding_audit['excluded_preproduction_class_rows'].append({
                            'source': upgrade['_source'],
                            'reason': 'Initialization variant ends before native production-node availability',
                        })
                        continue
                    data, inherited = make_fields(db, original, epoch, upgrade, node)
                    same_visual = upgrade['VISUAL'] in strict_visuals
                    complete.append({
                        'id': identifier, 'epoch': start_runtime, 'until': end_runtime,
                        'source_epoch': epoch, 'source_unit': original,
                        'source_visual': upgrade['VISUAL'], 'source_host': node.get('HOST', '') if node else '',
                        'identity_verified': identity and same_visual,
                        'model_verified': model_verified and same_visual,
                        'fields': data, 'inheritance': inherited['chain'], 'rps': inherited.get('rps'),
                        'placement_type': inherited.get('placement_type'),
                        'availability_source': node['_source'] if node else None,
                        'upgrade_source': upgrade['_source'], 'source_upgrade': upgrade,
                        'visual_adapter': not same_visual,
                        'note': 'Numeric values are original source-class data; differing VISUAL does not claim the bound rendered model changed.',
                    })
            for row in complete:
                class_rows.append(row)
                binding_audit['class_records'].append(len(class_rows)-1)
            binding_audit['status'] = 'verified original numeric declarations; identity flag remains separate'
            audit.append(binding_audit)
        slot_report=ROOT.parent/'EE2-Research/analysis/fortress-population-binding.json'
        slot_evidence=json.loads(slot_report.read_text(encoding='utf8'))
        if slot_evidence['propertyTable']['name']!='popCount' or slot_evidence['format']!='ee2-fortress-population-binding-v1':
            raise ValueError('Missing verified native garrison-slot property binding')
        result = {
            'format': 'ee2-unit-runtime-audit-v1',
            'scope': 'Explicit base db.zip only; existing 57 EE2 definition roles, not all declared UnitTypes. No civilization, research or veteran/elite effects applied.',
            'resources_internal': RESOURCES,
            'epochs': 'source_epoch=1..15; runtime epoch=0..14; until exclusive',
            'availability_scope': 'Intervals cover the bound original stat/visual variant until the next CSV upgrade of that source class. They do not prove production disable/replacement rules; gaps in the smaller roster are explicit. No old eight-epoch remapping is used.',
            'source_archive': str(db.archive), 'source_sha256': sha(db.archive.read_bytes()),
            'model_manifest_sha256': sha(manifest_path.read_bytes()) if manifest_path.exists() else None,
            'movement_scope': 'raw_size/raw_footprint/raw_speed/range remain native; no world-unit conversion is asserted',
            'population_scope': 'pop comes from inherited DDF popCount; CSV POP CAPACITY is preserved in source_upgrade, not mistaken for population usage. DDF popCapacity is separate; zero table placeholders do not establish a population usage value.',
            'native_garrison_slots': {'report':'../EE2-Research/analysis/fortress-population-binding.json',
                'report_sha256':sha(slot_report.read_bytes()),'evidence':slot_evidence,
                'scope':'Eligible bound native source classes use inherited DDF popCount, except Leaders use one. The semantic binding is verified on the explicitly identified EnabledUP15Units executable; numeric DDF values remain from the selected base db.zip. No general patch/VFS precedence is inferred.'},
            'class_progression_scope': 'class_rows includes every declared upgrade epoch of each bound source class. Latest declaration carries forward until the next declaration or epoch15. Differing native visuals explicitly clear model/identity equivalence flags; numeric source verification remains independent.',
            'production_class_from': production_from,
            'bindings': audit, 'rows': rows, 'class_rows': class_rows,
            'source_files': {name: {'sha256': item['sha256']} for name, item in sorted(db.files.items())},
        }
        output_path = ROOT/'docs/ee2-units.json'
        temporary = output_path.with_name(output_path.name + '.tmp')
        temporary.write_text(json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False), encoding='utf-8')
        temporary.replace(output_path)
        write_rust(rows, class_rows, production_from, ROOT/'src/ee2_units.rs')
        print(f'{len(audit)} existing definition roles, {len(rows)} visual-epoch records, {len(class_rows)} class-epoch records; {sum(bool(item["records"]) for item in audit)} resolved sources, {sum(item["role_adapter"] for item in audit)} explicit role adapters')
        unresolved = [item['id'] for item in audit if not item['records']]
        if unresolved:
            print('Unresolved IDs retain runtime fallback:', unresolved)
        return result
    finally:
        db.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('installation', nargs='?', type=pathlib.Path)
    parser.add_argument('--install', type=pathlib.Path)
    arguments = parser.parse_args()
    run(arguments.install or arguments.installation or DEFAULT_INSTALL)
