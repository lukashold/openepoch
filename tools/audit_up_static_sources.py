"""Compact auditable account of the native helper and resolved DB namespaces."""
from pathlib import Path
import datetime,hashlib,json

ROOT=Path(__file__).resolve().parents[1]
A=ROOT.parent/'EE2-Research/analysis'
W=ROOT.parents[1]/'work/up-resolved-sources'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=json.loads((W/'effective-db-sources.json').read_text())
tests=json.loads((A/'native-up-source-reader-tests.json').read_text())
assert tests['status']=='passed' and tests['manifestSha256']==sha(W/'effective-db-sources.json')
assert (W/'manifest.json').read_bytes()==(W/'effective-db-sources.json').read_bytes()
native=(A/'native-uphelper-protected-callback.c').read_text()
hook=(A/'native-vfs-protected-hook.c').read_text()
assert 'MOV EDX,dword ptr [0x00faadb4]' in hook and 'JMP 0x00a29af1' in hook
assert '1006c555 MOV ECX,0x101b2fec' in native and '1006c656 MOV EAX,ESI' in native
assert 'cRam00c163c3' in native
sources=['native-vfs-priority.c','native-vfs-resolution.c','native-vfs-order.c','native-vfs-enumeration.c','native-vfs-protected-hook.c',
         'native-uphelper-protected-callback.c','native-uphelper-vfs-load.c','native-uphelper-temp-gates.c','native-uphelper-temp-path.c','native-uphelper-temp-api.c']
core={name:manifest['effectiveByBasename'][name] for name in ['dbcomba5.csv','dbcombat_unittypeadjus5.csv','upgrade_unittype5.csv','dbunitfile5.cfg','dbunittypeattributes.ddf','territory.ddf','turret.ddf']}
payload=manifest['optionalSourceInventory']
report={
 'format':'native-uphelper-source-evidence-v1','status':'static-profile-resolved-and-verified','createdUtc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'engineSha256':manifest['engineSha256'],'helperSha256':manifest['helperSha256'],
 'helperExport':{'name':'UP15_Load_Protected_Zip','va':'1006bf10','ordinal':355},
 'sourceRules':[
  {'source':'engine00FA7E9B -> DLL1006BF10','fact':'Archive hook calls helper with type1 and input filename; nonzero char* return replaces EDX before original00A29AF1/UseZipArchive.'},
  {'source':'DLL1006C555–1006C656','fact':'Replacement path is stored in persistent std::string101B2FEC; pointer (SSO or heap) is returned through EAX=ESI.'},
  {'source':'DLL1006C222–1006C58F','fact':'db_160_protected.zip selects EE2ExeSourcePart10 or EE2XExeSourcePart10, protected2 selects sharedPart12. Selection requires UP16 flag101B8320 and a nonempty existing payload.'},
  {'source':'DLL1006C2A2–1006C473','fact':'textures_terrain_160.zip selects Part13 only under enabled terrain flag101B8317 and raw mode/game gating. Mode2 loads for both game types; mode1/3 for EE2; mode4 for EE2X; mode0 skips.'},
  {'source':'DLL1006C6A6 onward','fact':'db_158_protected.zip uses Part8/Part9:protected or generated158 temp fallback. Installed Part9:protected is already a valid readable ZIP; offline extraction needs no key/decryption.'},
  {'source':'DLL1006D149 onward','fact':'Ultrawide marker and explicit sourcePart14 paths are conditional on UP16,1006BE90 predicate and101B8316. Both sourceParts14/ADS and specifically named161 temp files are absent in audited locations; no format/content assertion is made for nonexistent Part14.'},
  {'source':'DLL1002FAB0 -> 100EFCF0','fact':'Temporary directory uses an optional GetProcAddress API and fallback GetTempPathW. Original-process environment is unobserved; default user-local temp was probed without enumerating unrelated files.'},
  {'source':'DLL1006CF8F branch; engine00C163AC string','fact':'Optional db_158_disable_new_ambient_sounds.zip changes character00C163C3 in db/audio/dbsound_ambien5.csv from5 to6. This is an audio-table filename switch, not a hidden numeric unit/RPS table.'}],
 'payloadInventory':payload,
 'payloadNamespaces':[
  {'part':'EE2ExeSourcePart10','format':'ordinary unencrypted ZIP','members':10,'namespace':'db/UI/PackedSprite7/*.csv','crcAllMembers':'passed'},
  {'part':'EE2EE2XExeSourcePart12','format':'ordinary unencrypted ZIP','members':8,'namespace':'cache/texcache nuclear-effects caches and directories','crcAllMembers':'passed'},
  {'part':'EE2EE2XExeSourcePart13','format':'ordinary unencrypted ZIP','members':759,'namespace':'cache/texcache terrain/building/road textures','crcAllMembers':'passed'},
  {'part':'EE2EE2XExeSourcePart9:protected','format':'ordinary unencrypted ZIP in an already existing NTFS named stream','members':707,'namespace':'Sounds/* only; no DB/HDR source members','crcAllMembers':'passed'},
  {'part':'EE2EE2XExeSourcePart8','format':'opaque non-ZIP in this audit; no decryption attempted','bytes':121338841}],
 'effectiveManifest':{'path':str(W/'effective-db-sources.json'),'sha256':sha(W/'effective-db-sources.json'),'counts':manifest['counts'],
  'reproducibility':'Two fresh complete extractions produced byte-identical JSON; all staged text bytes match independent original ZIP/member or loose-file reads.',
  'insertionOrdinalScope':'Ordinal is within the selected DB/HDR projection. Relative order of matching native basename records is preserved; unrelated media records are omitted.',
  'coreSources':core,'profileConditions':manifest['conditions']},
 'version5Comparison':tests['version5SourceComparison'],
 'reader':{'module':'tools/resolved_archive_reader.py','class':'ResolvedArchiveReader','methods':['read(name)','namelist()','close()','origin(name)','origin_ref(name)'],
  'semantics':'No slash is first-basename lookup; slash is full-path lookup; ASCII names are case-insensitive. Reader validates size and SHA before returning bytes. Complete namespace enumeration is distinct from active dbunitfile5.cfg declaration order.'},
 'tests':{'report':'native-up-source-reader-tests.json','sha256':sha(A/'native-up-source-reader-tests.json'),'checks':tests['checks'],'status':'passed','protectedZipCRCValidatedMembers':1484},
 'sourceFiles':{name:sha(A/name) for name in sources},
 'limits':['Profile assumes EE2, UP16 enabled and installation cwd as explicitly declared; original game execution/state is not observed.',
  'Part14 is absent from audited source/default-temp locations; no general claim about another installation or a custom original-process TEMP is made.',
  'The helper also applies memory patches during initialization; this audit does not treat unspecified memory patch tables as invented file aliases or prove every runtime gameplay change.',
  'Media content is not extracted into the source package. Raw source data are under ignored work/up-resolved-sources; new Ghidra helper project and DLL workspace copy must stay out of GitHub/source publication.'],
 'originalsReadOnly':True,'gameCodeExecuted':False,'licenseCdKeyNetworkPathsUsed':False}
(A/'native-uphelper-source-evidence.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps({'status':report['status'],'manifestSha256':report['effectiveManifest']['sha256'],'counts':manifest['counts'],'checks':tests['checks'],'sourcePairs':len(tests['version5SourceComparison'])}))
