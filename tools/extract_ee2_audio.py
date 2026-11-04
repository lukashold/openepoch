"""Extract selected original audio from the user's local EE2 installation.

Files are copied byte-for-byte, never uploaded. The generated manifest retains
archive, database mapping and hashes. local-original must stay outside public
or portable source packages; rerun this script against a local installation.
"""
import argparse
import csv
import hashlib
import io
import json
import pathlib
import re
import shutil
import zipfile
from wav_ima import decode as decode_ima_wav, info as wave_info
from import_ee2 import without_comments, block

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_INSTALL = pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')

# Current rendered UnitModel sources, not guessed voice filename stems. The
# original upgrade's SND* columns may deliberately name another visual family
# (LightMounted3 uses HeavyInfantry1_15; HeavyMounted1_13 uses _15 recordings).
# A verified sound-to-visual binding does not remove a simulation role adapter.
VOICE_MODELS = {
    9: 'HeavyInfantry1_01', 10: 'LightInfantry_01', 11: 'HeavyInfantry2_02',
    12: 'LightInfantry_03', 13: 'HeavyInfantry1_03', 14: 'LightInfantry_05',
    15: 'HeavyInfantry1_07', 16: 'HeavyInfantry1_11', 17: 'HeavyInfantry1_13',
    18: 'HeavyMounted1_02', 19: 'HeavyMounted1_04', 20: 'HeavyArtillery_06',
    21: 'LightMounted1_10', 22: 'LightArtillery1_09', 23: 'LightMounted1_11',
    24: 'HeavyArtillery_10', 25: 'LightArtillery2_11', 26: 'HeavyMounted1_13',
    27: 'LightArtillery1_13', 28: 'LightArtillery2_15', 29: 'WarGalley_01',
    30: 'Galleon_08', 31: 'Frigate_10', 32: 'Destroyer_11',
    33: 'Battleship_11', 34: 'Submarine_11', 35: 'Battleship_13',
    36: 'Submarine_13', 37: 'Fighter_11', 38: 'Bomber_11',
    39: 'Fighter_13', 40: 'Bomber_14', 41: 'HeavyMounted2_13', 42: 'CargoShip',
    43: 'LightMounted3', 44: 'Fighter_15', 45: 'Submarine_15', 46: 'HeavyMounted1_15',
}
EFFECTS = {
    '1': 'fx_battle_arrow.mp3', '2': 'fx_battle_rifle.mp3',
    '3': 'fx_cannonshot_1.mp3', '4': 'fx_battle_torp_launch.mp3',
    '5': 'fx_battle_rocket_launch.mp3', '6': 'fx_battle_bomb_drop.mp3',
    '7': 'fx_battle_swing.mp3', '10': 'fx_exp_01.mp3',
    '11': 'fx_building_collapse_1.mp3', '12': 'fx_feedback_citizen_created.mp3',
    '13': 'fx_research_complete.mp3',
    '15': 'fx_feedback_under_attack.mp3', '20': 'fx_ui_button_press.mp3',
    '22': 'fx_feedback_rally_point.mp3', '23': 'fx_resource_wood.mp3',
    '24': 'fx_resource_stone.mp3', '25': 'fx_contruction_01.mp3',
}
BUILDING_EVENTS = {0:'CityCenter_select',1:'House_select',2:'Farm_select',3:'Barracks_select',4:'Mill_select',
                   5:'Dock_select',6:'Airport_select',7:'WallTower_select',47:'University_select',48:'Mill_select',
                   49:'Stable_select',50:'Market_select',51:'Temple_select',52:'Warehouse_select',
                   55:'Herc_plant_select',56:'Fortress_select'}

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def run(install=DEFAULT_INSTALL, output=None):
    install=pathlib.Path(install)
    out=pathlib.Path(output or ROOT/'docs/local-original/audio')
    out.mkdir(parents=True,exist_ok=True)
    archive=install/'zips/sounds.zip'
    manifest={'version':1,'scope':'Locally extracted original EE2 audio; no redistribution package.',
              'source':str(install),'archive':str(archive),'archive_sha256':sha(archive.read_bytes()),
              'region':'West (current single-region preset)',
              'language':'Untouched installed recordings; language not inferred from region suffix.',
              'voices':{},'voiceVariants':[],'unresolvedVoiceEvents':[],'effects':{},'buildingSelections':{},'unresolvedBuildingSelections':{},
              'music':{},'ambient':{},'assets':{}}
    with zipfile.ZipFile(install/'zips/db.zip') as db, zipfile.ZipFile(archive) as sounds:
        upgrade_path='db/TechTree/upgrade_unittypes.csv'
        upgrade_raw=db.read(upgrade_path)
        upgrade_lines=upgrade_raw.decode('cp1252').replace('\r','').splitlines()
        upgrade_header=next(csv.reader([upgrade_lines[0].removeprefix('//').strip()]))
        upgrade_rows=[]
        for line_number,line in enumerate(upgrade_lines[1:],2):
            if not line.strip() or line.lstrip().startswith('//'):continue
            row=dict(zip(upgrade_header,next(csv.reader([line]))))
            row['source_line']=line_number;upgrade_rows.append(row)
        herc=next(row for row in upgrade_rows if row['UNIT']=='HercFacility'
                  and row['UPGRADE']=='HercFacilityUpgradeEpoch14')
        if re.findall(r'\b\w+\b',herc['SNDSEL'])!=['Herc_plant_select']:
            raise ValueError('HercFacility original SNDSEL changed; refusing an assumed selection mapping')
        fortress=[row for row in upgrade_rows if row['UNIT']=='Fortress']
        if [int(row['EPOCH']) for row in fortress]!=[1,4,6,8,10,12,14] or any([name.casefold() for name in re.findall(r'\b\w+\b',row['SNDSEL'])]!=['fortress_select'] for row in fortress):
            raise ValueError('Fortress epoch selection source changed; no guessed sound mapping allowed')
        # Work-loop events are not selection events. Preserve the two explicit
        # evidence gaps instead of assigning oil/uranium machinery loops to clicks.
        resources_path='db/Units/resources.ddf';building_path='db/Units/building.ddf'
        resources_raw=db.read(resources_path);building_raw=db.read(building_path)
        resource_types=without_comments(resources_raw.decode('cp1252').replace('\r',''))
        building_types=without_comments(building_raw.decode('cp1252').replace('\r',''))
        for unit,original,work_event in [(53,'OilRig','Resource_oil'),(54,'UraniumMine','Resource_uranium')]:
            declarations=[block(resource_types,'UnitType',original),
                          block(building_types,'UnitType','NoProduceBuilding'),
                          block(building_types,'UnitType','BareBoneBuilding')]
            if not all(declarations):raise ValueError('Missing original building inheritance evidence for '+original)
            if any(re.search(r'\bsndSelection\s*=',content,re.I) for content in declarations):
                raise ValueError(original+' now has a direct/inherited selection event; audit it before mapping')
            if any(row['UNIT']==original and re.findall(r'\b\w+\b',row['SNDSEL']) for row in upgrade_rows):
                raise ValueError(original+' now has an upgrade selection event; audit it before mapping')
            manifest['unresolvedBuildingSelections'][str(unit)]={
                'unit_type':original,'status':'No verified selection binding; not a claim of original silence.',
                'reason':'No sndSelection in UnitType or checked building parents, and no SNDSEL upgrade row.',
                'checked_declarations':[original,'NoProduceBuilding','BareBoneBuilding'],
                'sources':[{'archive':'db.zip','entry':resources_path,'sha256':sha(resources_raw)},
                           {'archive':'db.zip','entry':building_path,'sha256':sha(building_raw)},
                           {'archive':'db.zip','entry':upgrade_path,'sha256':sha(upgrade_raw)}],
                'work_loop_not_repurposed':work_event}
        tablepath='db/Audio/dbsound.csv'
        records=[]
        for source_table in [tablepath,'db/Audio/dbsound_ambient.csv']:
            table_raw=db.read(source_table)
            text=table_raw.decode('cp1252').replace('\r','')
            for source_line,line in enumerate(text.splitlines(),1):
                if line.strip() and not line.lstrip().startswith('//'):
                    row=next(csv.reader([line]))
                    if len(row)>=6:
                        records.append({'name':row[0],'filename':row[1], 'volume':float(row[2] or 1),'loop':row[3]=='1',
                                        'table':'db.zip!'+source_table,'line':source_line,'table_sha256':sha(table_raw)})
        by_file={}
        for record in records:by_file.setdefault(record['filename'].lower(),[]).append(record)
        paths={pathlib.PurePosixPath(n).name.lower():n for n in sounds.namelist() if not n.endswith('/')}
        def extract(filename, purpose):
            key=filename.lower()
            if key in manifest['assets']:return manifest['assets'][key].get('playback_file',manifest['assets'][key]['file'])
            path=paths.get(key)
            if not path:return None
            raw=sounds.read(path);name=pathlib.PurePosixPath(path).name
            (out/name).write_bytes(raw)
            manifest['assets'][key]={'file':name,'source_archive':'zips/sounds.zip','source_entry':path,
                'sha256':sha(raw),'bytes':len(raw),'format':pathlib.PurePosixPath(name).suffix[1:],
                'transform':'byte-for-byte copy','mapping_source':sorted({r['table'] for r in by_file.get(key,[])}),
                'sound_records':by_file.get(key,[]),'purpose':purpose}
            if key.endswith('.wav'):
                format=wave_info(raw);manifest['assets'][key]['wave_format']=format
                if format['codec']==0x11:
                    pcm,metadata=decode_ima_wav(raw);playback=pathlib.PurePosixPath(name).stem+'.pcm.wav'
                    (out/playback).write_bytes(pcm);manifest['assets'][key]['playback_file']=playback
                    manifest['assets'][playback.lower()]={'file':playback,'derived_from':name,
                        'source_archive':'zips/sounds.zip','source_entry':path,'source_sha256':sha(raw),
                        'sha256':sha(pcm),'bytes':len(pcm),'format':'wav','purpose':purpose,
                        **metadata}
                    return playback
                if format['codec'] not in (1,3,0xfffe):
                    raise ValueError(f'{name}: unsupported WAVE codec {format["codec"]:#x}; original preserved, no invented conversion')
            return name
        by_name={record['name'].lower():record for record in records}
        model_sources={}
        for entry in db.namelist():
            if not entry.startswith('db/Units/') or not entry.lower().endswith('.ddf'):continue
            raw=db.read(entry);text=raw.decode('cp1252').replace('\r','')
            clean=re.sub(r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*.*?\*/',
                         lambda m:m[0] if m[0].startswith('"') else re.sub(r'[^\n]',' ',m[0]),text,flags=re.S)
            for match in re.finditer(r'(?m)^[ \t]*UnitModel\s+(\w+)\s*\{',clean):
                name=match.group(1)
                # Index the declaration independently of the upgrade sound row.
                model_sources[name]={'archive':'db.zip','entry':entry,'sha256':sha(raw),
                                     'declaration':'UnitModel '+name,
                                     'line':clean.count('\n',0,match.start())+1}
        def event_files(unit,action,names):
            result=[]
            for event in names:
                record=by_name.get(event.lower())
                file=extract(record['filename'],f'Original voice {unit} {action}, event {event}') if record else None
                if file:result.append(file)
                else:manifest['unresolvedVoiceEvents'].append({'id':unit,'action':action,'event':event,'status':'Missing sound record or archive file; not substituted.'})
            return list(dict.fromkeys(result))
        # CitizenM's declared events use the explicitly selected West records.
        # No campaign/tutorial speech or guessed combat clip is reused.
        citizen_path='db/Units/citizen.ddf';citizen_raw=db.read(citizen_path)
        citizen_body=block(without_comments(citizen_raw.decode('cp1252').replace('\r','')),'UnitType','CitizenM')
        def citizen_events(field):
            match=re.search(r'\b'+field+r'\s*=\s*\[([^]]*)\]',citizen_body)
            if not match:raise ValueError('Missing CitizenM '+field+' evidence')
            return [name+'_we' for name in re.findall(r'"([^"]+)"',match.group(1))]
        male_select=event_files(8,'select',citizen_events('sndSelection'))
        male_order=event_files(8,'order',citizen_events('sndCmdMove'))
        male_attack=event_files(8,'attack',citizen_events('sndCmdAttack'))
        manifest['voices']['8']={'select':male_select,'order':male_order,'build':male_order,
            'attack':male_attack,'mapping_verified':True,'source_unit':'CitizenM','source_visual':'CitizenM',
            'source':{'archive':'db.zip','entry':citizen_path,'sha256':sha(citizen_raw),'declaration':'UnitType CitizenM'},
            'events':{'select':citizen_events('sndSelection'),'order':citizen_events('sndCmdMove'),'attack':citizen_events('sndCmdAttack')},
            'mapping':'CitizenM declared selection/move/attack events, explicit West preset; build retains the move acknowledgement adapter because there is no build-response event in this interface.'}
        def make_voice(unit,visual,source):
            if visual not in model_sources:raise ValueError('No declared original UnitModel '+visual)
            events={action:re.findall(r'\b\w+\b',source.get(column,'')) for action,column in [('select','SNDSEL'),('order','SNDMOVE'),('attack','SNDATTACK')]}
            voice={action:event_files(unit,action,names) for action,names in events.items()}
            if not voice['select'] or not voice['order']:raise ValueError('Incomplete verified selection/move set for '+visual)
            return {**voice,'mapping_verified':True,'source_visual':visual,'source_unit':source['UNIT'],
                'source_epoch':int(source['EPOCH']),'source_upgrade':source['UPGRADE'],'events':events,
                'upgrade_source':{'archive':'db.zip','entry':upgrade_path,'line':source['source_line'],'sha256':sha(upgrade_raw),
                                  'columns':{'select':'SNDSEL','order':'SNDMOVE','attack':'SNDATTACK'}},
                'visual_source':model_sources[visual],
                'mapping':'Original SNDSEL/SNDMOVE/SNDATTACK for the current UnitModel source; current game identity may remain a role adapter. Fixed visual binding, not automatic original epoch/civilization sound upgrades.'}
        for unit,visual in VOICE_MODELS.items():
            candidates=[row for row in upgrade_rows if row.get('VISUAL')==visual]
            if not candidates:raise ValueError('No original upgrade row for UnitModel '+visual)
            source=min(candidates,key=lambda row:(int(row['EPOCH'] or 99),row['source_line']))
            manifest['voices'][str(unit)]=make_voice(unit,visual,source)
        # Only the renderer's explicitly source-proven progression intervals
        # receive later voices. Missing intermediate visuals keep the base
        # fallback; do not project the native class over unexported models.
        model_manifest_path=ROOT/'docs/local-original/models/manifest.json'
        if model_manifest_path.is_file():
            model_raw=model_manifest_path.read_bytes();model_manifest=json.loads(model_raw)
            for model in model_manifest.get('models',[]):
                p=model.get('progressionBinding');unit=model['id']
                if not p or not 8<=unit<=46:continue
                src=p['upgradeSource'];visual=p['sourceVisual'];epoch=p['epoch'];until=p['until']
                if src['entry']!=upgrade_path or src['sha256']!=sha(upgrade_raw):
                    raise ValueError('Progression sound source differs from explicitly selected base db.zip')
                source=next(row for row in upgrade_rows if row['source_line']==src['line'])
                if source['UNIT']!=p['sourceUnit'] or source['VISUAL']!=visual or int(source['EPOCH'])-1!=epoch:
                    raise ValueError('Progression class/visual/epoch is not the declared native sound row')
                if epoch!=model['epoch'] or not 0<=epoch<until<=15:
                    raise ValueError('Invalid native sound progression interval')
                voice=make_voice(unit,visual,source)
                if p['visualSource']['sha256']!=voice['visual_source']['sha256']:
                    raise ValueError('Progression UnitModel declaration hash mismatch')
                manifest['voiceVariants'].append({'id':unit,'epoch':epoch,'until':until,**voice,
                    'progression_source':{'type':p['type'],'sourceUnit':p['sourceUnit'],'sourceVisual':visual,
                                          'reusedFrom':p.get('reusedFrom'), 'upgradeSource':src},
                    'mapping':'Exact native sourceUnit/sourceVisual upgrade sounds for this verified rendered progression interval. Epoch zero-based, until exclusive; no interpolation through missing models.'})
            manifest['voiceVariants'].sort(key=lambda row:(row['id'],row['epoch']))
            keys=[(row['id'],row['epoch']) for row in manifest['voiceVariants']]
            if len(keys)!=len(set(keys)):raise ValueError('Duplicate voice progression intervals')
            manifest['voiceVariantSource']={'path':'local-original/models/manifest.json','sha256':sha(model_raw),
                                             'bindings':len(manifest['voiceVariants'])}
        for kind,filename in EFFECTS.items():
            name=extract(filename,'Current renderer sound-event '+kind)
            if name:
                rec=by_file.get(filename.lower(),[{'volume':1}])[0]
                manifest['effects'][kind]={'file':name,'volume':rec['volume']}
        for unit,event in BUILDING_EVENTS.items():
            record=by_name[event.lower()]
            file=extract(record['filename'],'Original building selection event '+event)
            if file:
                manifest['buildingSelections'][str(unit)]={'file':file,'volume':record['volume'],'event':event}
                if unit==55:
                    manifest['buildingSelections']['55'].update({
                        'unit_type':'HercFacility','source_epoch':14,
                        'selection_source':{'archive':'db.zip','entry':upgrade_path,
                                            'line':herc['source_line'],'sha256':sha(upgrade_raw),
                                            'column':'SNDSEL','value':herc['SNDSEL']},
                        'event_source':{'archive':'db.zip','entry':tablepath,
                                        'line':next(i for i,line in enumerate(db.read(tablepath).decode('cp1252').replace('\r','').splitlines(),1)
                                                    if line.startswith(event+',')),
                                        'sha256':sha(db.read(tablepath))}})
                if unit==56:
                    manifest['buildingSelections']['56'].update({
                        'unit_type':'Fortress','source_epochs':[int(row['EPOCH']) for row in fortress],
                        'selection_sources':[{'archive':'db.zip','entry':upgrade_path,'line':row['source_line'],
                            'sha256':sha(upgrade_raw),'column':'SNDSEL','value':row['SNDSEL']} for row in fortress],
                        'event_source':{'archive':'db.zip','entry':tablepath,
                            'line':next(i for i,line in enumerate(db.read(tablepath).decode('cp1252').replace('\r','').splitlines(),1)
                                        if line.casefold().startswith(event.casefold()+',')),
                            'sha256':sha(db.read(tablepath))},
                        'scope':'Exact common selection event across all seven original Fortress stages. No empty SNDATTACK/SNDMOVE or unrelated work-loop mapped as a fortress response.'})
        manifest['ambient']['loop']=extract('ambloop_land_01_temp.wav','dbambientaudio.ddf ambset_land_temperate loopingSounds')
        manifest['ambient']['birds']=[extract(f'amb_temp_bird_call_{i:02}.mp3','dbambientaudio.ddf ambset_forest_temperate sounds') for i in range(1,4)]
        manifest['mappingEvidence']={
            'citizen':'db.zip!db/Units/citizen.ddf: sndSelection, sndCmdMove and PLAY_SOUND Resource_wood / Resource_stone / Citizen_build',
            'music':'db.zip!db/Audio/dbmusic.ddf: region West; epoch Modern..Synthetic adds amb_mid_late_*',
            'ambient':'db.zip!db/Audio/dbambientaudio.ddf: ambset_land_temperate / ambset_forest_temperate',
            'effects':'db.zip!db/Audio/dbsound.csv: original filename and volume mappings',
            'unitVoices':'Current rendered UnitModel declaration -> exact VISUAL upgrade row -> SNDSEL/SNDMOVE/SNDATTACK event -> dbsound.csv recording; source lines and hashes retained. Repeated identical recording entries are de-duplicated in playback rotation; original event arrays remain in metadata.'}
        manifest['coverage']={'mobile_ids':list(range(8,47)),'mobile_bindings':len(manifest['voices']),
            'voice_progression_bindings':len(manifest['voiceVariants']),
            'voice_scope':'All current 39 mobile definitions, bound to fixed source visuals. This is not the complete native roster, all civilizations, regional voice variants, or native audio timing.',
            'attack_api':"acknowledge(def, 'attack'); source events are separate from move acknowledgements.",
            'building_selection_bindings':len(manifest['buildingSelections']),
            'unresolved_building_selection_ids':sorted(map(int,manifest['unresolvedBuildingSelections']))}
    for group,names in [('west',['amb_we_1.mp3','amb_we_2.mp3','amb_we_3.mp3']),
                        ('late',['amb_mid_late_1.mp3','amb_mid_late_2.mp3'])]:
        manifest['music'][group]=[]
        for name in names:
            source=next((install/'music').rglob(name));raw=source.read_bytes();(out/name).write_bytes(raw)
            manifest['assets'][name.lower()]={'file':name,'source_file':str(source.relative_to(install)),
                'sha256':sha(raw),'bytes':len(raw),'format':'mp3','transform':'byte-for-byte copy',
                'purpose':'Original in-game '+group+' music','mapping_source':'db.zip!db/Audio/dbmusic.ddf'}
            manifest['music'][group].append(name)
    (out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf8')
    originals=sum('derived_from' not in asset for asset in manifest['assets'].values());derived=len(manifest['assets'])-originals
    print(f'Extracted {originals} original audio files + {derived} PCM derivatives, {sum(a["bytes"] for a in manifest["assets"].values()):,} bytes; {len(manifest["voices"])} unit voice mappings')
    return manifest

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('installation',nargs='?',default=str(DEFAULT_INSTALL));parser.add_argument('--output')
    args=parser.parse_args();run(args.installation,args.output)
