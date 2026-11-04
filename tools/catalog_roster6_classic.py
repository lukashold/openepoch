"""Classical132-family catalogue; native UP numbers stay separate from BASE membership."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT.parents[1]

CAPABILITIES = {
    'Attack': ('partial', 'Native RPS/target matrix, damage refs, projectile/direct, minimum range and source timing; packing, banks and ordnance remain open.'),
    'LOS': ('partial', 'Source LOS radius; existing visibility grid, no complete native occlusion/stealth implementation.'),
    'Move': ('adapter', 'Source speed/turning projected onto existing ground grid and scalar collision.'),
    'VehicleMove': ('adapter', 'Source speed/turning projected onto existing ground grid; no native acceleration/formation model.'),
    'NavalMove': ('adapter', 'Source speed/turning projected onto existing water grid.'),
    'AircraftMove': ('partial', 'Existing aircraft movement adapter; takeoff, rearm and per-aircraft ground orders incomplete.'),
    'Build': ('partial', 'Current citizen construction/repair; no claim all native builder permissions are implemented.'),
    'Produce': ('partial', 'Definition-based host queue; source production slot replacement and all native family jobs remain open.'),
    'Research': ('partial', '180 generic source nodes;294 per-family improvement and33 Main nodes are separate.'),
    'Garrison': ('partial', 'Existing capacity/popCount and supported host behavior; specialized hosts remain separate.'),
    'Garrisonable': ('partial', 'Source fortress eligibility/popCount; not every native garrison target.'),
    'ResourceDropoff': ('partial', 'Source citizens and current dropoff hosts/bonuses only.'),
    'GatherFarm': ('partial', 'Existing citizens/farms; fishing ships need their own native assignment loop.'),
    'resourceFarm': ('partial', 'Current agricultural farm loop; full native resource-farm family behavior remains separate.')
}


def run():
    inventory_path = ROOT/'docs/roster6-source-inventory.json'
    base_path = ROOT/'docs/roster6-base-membership.json'
    inventory = json.loads(inventory_path.read_text())
    classical = set(json.loads(base_path.read_text())['commonFamilies'])
    selected = {b['source_unit']:b for b in json.loads((ROOT/'docs/roster6-classic-selection.json').read_text())['bindings']}
    models = json.loads((ROOT/'docs/local-original/models/manifest-roster6.json').read_text())['models']
    records = []
    for family in sorted(inventory['families'],key=lambda f:f['sourceClass']):
        name = family['sourceClass']
        if name not in classical:
            continue
        old = family['currentExactRoleIds'] + family['currentWorkerChildAdapterIds']
        extra = selected.get(name)
        abilities = [{'name':ability,'status':CAPABILITIES.get(ability,('unsupported','No native runtime implementation in this slice'))[0],
                      'detail':CAPABILITIES.get(ability,('unsupported','No native runtime implementation in this slice'))[1]}
                     for ability in family['inheritedAbilityNames']]
        gates = []
        for ability in abilities:
            if ability['status']=='unsupported':
                gates.append('nativeAbility:'+ability['name'])
        if any(n.get('TTCIV')for n in family['production']):
            gates.append('civilizationEligibility')
        if any(n.get('PREREQUISITES')not in ('','[]')for n in family['production']):
            gates.append('nativePrerequisiteEvaluation')
        if any('ScriptUnlock'in n.get('SPECFLAGS','')for n in family['production']):
            gates.append('scriptUnlock')
        if not old and not extra:
            gates.append('notYetBoundToRuntimeDefinition')
        if name in ['AirDefense','CoastalDefense']:
            gates.append('activeTerritoryAllowedUnitsTemplateBinding')
        if name in ['Ram','HeavyMounted3']:
            gates.append('packUnpackStateGraph')
        graphics = []
        if name=='FightingSail':
            graphics.append('Native SailEffect children forCaravel/ShipOfTheLine are procedural and absent from a parent NIF mesh.')
        record = {'catalogueId':len(records),'sourceFamily':name,'source':family['source'],
                  'existingDefIds':old,'candidateDefId':extra['id']if extra else None,
                  'legacyChildAdapter':bool(family['currentWorkerChildAdapterIds']),
                  'category':family['category'],'rps':family['rps'],'abilities':abilities,
                  'productionNodes':family['production'],'upgrades':family['upgrades'],
                  'initialVisualSelection':family.get('initialVisualSelection'),
                  'candidateModelRows':[r for r in models if extra and r.get('assetKind')==0 and r['id']==extra['id']],
                  'remainingGates':sorted(set(gates)),'graphicLimits':graphics,
                  'completeNativeParity':False}
        records.append(record)
    assert len(records)==132
    bound = [r for r in records if r['existingDefIds']or r['candidateDefId']is not None]
    payload = {'format':'native-classic6-production-catalogue-v1',
               'scope':'BASE CSV membership only; every production row, ability, upgrade and numerical source comes from active UP CSV5/CFG5. CatalogueId is not DefId or RenderId.',
               'summary':{'families':132,'existingBoundFamilies':sum(bool(r['existingDefIds'])for r in records),
                          'newCandidateFamilies':len(selected),'boundFamiliesAfterCandidate':len(bound),
                          'unboundFamilies':132-len(bound),'candidateDefinitions':61,
                          'genericResearchNodes':180,'separateImprovementNodes':294,'separateMainNodes':33},
               'inventorySha256':hashlib.sha256(inventory_path.read_bytes()).hexdigest(),
               'baseMembershipAuditSha256':hashlib.sha256(base_path.read_bytes()).hexdigest(),
               'activeSourceManifestSha256':inventory['sourceManifestSha256'],
               'families':records,'full311Inventory':'roster6-source-inventory.json',
               'allRuntimeAbilitiesComplete':False,'originalExeOrDllExecuted':False,'activeRuntimeModified':False}
    target = ROOT/'docs/roster6-classic-catalogue.json'
    target.write_text(json.dumps(payload,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
    print(json.dumps(payload['summary']))


if __name__=='__main__':
    run()
