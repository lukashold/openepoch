"""Independent full-model comparison against the preceding portable release."""
import argparse
import hashlib
import json
import pathlib
import zipfile
from upgrade_ee2_materials import digest
from extract_ee2_models import ROOT


def without_material(model):
    return {**model,'parts':[{k:v for k,v in part.items() if k not in ('material','nativeProperties','materialStateVersion')} for part in model['parts']]}


def verify(baseline,report):
    upgrade=json.loads((ROOT.parent/'EE2-Research/analysis/visual-material-upgrade.json').read_text(encoding='utf8'))
    records=[]
    with zipfile.ZipFile(baseline) as archive:
        for row in upgrade['records']:
            entry='OpenEpoch/docs/'+row['url']
            before=archive.read(entry);after=(ROOT/'docs'/row['url']).read_bytes()
            if hashlib.sha256(before).hexdigest()!=row['beforeSha256']:
                raise AssertionError('Baseline differs from exact pre-upgrade input: '+entry)
            if hashlib.sha256(after).hexdigest()!=row['afterSha256']:
                raise AssertionError('Runtime model changed after metadata upgrade: '+entry)
            old=json.loads(before);new=json.loads(after)
            if without_material(old)!=without_material(new):
                raise AssertionError('Nonmaterial model data changed: '+entry)
            records.append({'url':row['url'],'baselineSha256':hashlib.sha256(before).hexdigest(),
                            'currentSha256':hashlib.sha256(after).hexdigest(),
                            'entireModelExceptNewMaterialFieldsSha256':digest(without_material(new))})
    result={'format':'ee2-native-material-baseline-verification-v1',
            'baselinePackage':str(baseline),'baselinePackageBytes':baseline.stat().st_size,
            'verifiedFiles':len(records),'unchanged':'Every complete JSON value except part.material, part.nativeProperties and part.materialStateVersion. Includes all geometry, animations, normalization, sources, LOD data and texture bindings.',
            'records':records}
    report.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    print(json.dumps({'verifiedFiles':len(records),'report':str(report)}))


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('baseline',type=pathlib.Path);parser.add_argument('--report',type=pathlib.Path,required=True)
    args=parser.parse_args();verify(args.baseline,args.report)
