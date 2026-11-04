"""Add the original variable-length seasonal sprite lists to compact bindings.

All physical PNGs were already decoded by extract_ee2_textures.py. This step
changes metadata only and verifies every compact sprite against that audit.
"""
import argparse,hashlib,json,pathlib
ROOT=pathlib.Path(__file__).resolve().parents[1]


def seasonal_urls(sprite):
    physical={row['originalPath']:row['url'] for row in sprite['physicalVariants']}
    return [physical[path] for path in sprite['seasonalPaths']]


def run():
    output=ROOT/'docs/local-original';compact_path=output/'ground-decals.json';audit_path=output/'ground-decals-manifest.json'
    compact=json.loads(compact_path.read_text(encoding='utf8'));audit=json.loads(audit_path.read_text(encoding='utf8'))
    by_id={row['id']:row for row in audit['entries']+audit['unresolvedBindings']};count=0;urls=set();lengths=set()
    for entry in compact['entries']+compact['unresolvedBindings']:
        source=by_id[entry['id']]
        for state,groups in [('Idle',entry),('Death',entry['death'])]:
            for category in ('rural','urban'):
                expected=source['states'][state][category]
                if len(groups[category])!=len(expected):raise AssertionError('Sprite count changed')
                for sprite,native in zip(groups[category],expected):
                    if any(sprite[key]!=native[key] for key in ('url','sprite','sourceSlot')):raise AssertionError('Source sprite mismatch')
                    sequence=seasonal_urls(native)
                    if not sequence:raise AssertionError('Empty native seasonal list')
                    for url in sequence:
                        if not (ROOT/'docs'/url).is_file():raise FileNotFoundError(url)
                    sprite['seasonalUrls']=sequence;count+=1;urls.update(sequence);lengths.add(len(sequence))
    compact['runtimeSeasonal']={'format':'ee2-ground-seasonal-bindings-v1','sourceAuditSha256':hashlib.sha256(audit_path.read_bytes()).hexdigest(),
        'arrayRule':'Source order and source array lengths preserved; duplicate entries are meaningful, not deduplicated.',
        'rendererPhase':'Shared year phase selects floor(phase*N), next modulo N, with linear interpolation.',
        'sourceNotes':'DbTerrainDefs.h documents uniform annual seasonal interpolation. GDbSpriteDefs.h explicitly marks m_bAnimated as a seasonal texture. Original sprite-manager render-state details remain outside this adapter.'}
    compact_path.write_text(json.dumps(compact,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    report={'format':'ee2-ground-seasonal-import-v1','spriteBindings':count,'uniquePhysicalPngs':len(urls),'sourceArrayLengths':sorted(lengths),
            'sourceAuditSha256':hashlib.sha256(audit_path.read_bytes()).hexdigest(),'runtimeManifestSha256':hashlib.sha256(compact_path.read_bytes()).hexdigest(),
            'mediaChanged':False,'scope':compact['runtimeSeasonal']}
    dest=ROOT.parent/'EE2-Research/analysis/ground-seasonal-import.json';dest.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report))


if __name__=='__main__':run()
