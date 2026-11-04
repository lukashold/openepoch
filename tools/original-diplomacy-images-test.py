"""Independent whole-RGBA crop/rotation and portable-path oracle."""
from pathlib import Path,PurePosixPath
import base64,hashlib,json,sys,zipfile
from PIL import Image
HERE=Path(__file__).resolve().parent;PROJECT=HERE.parent;WORKSPACE=PROJECT.parents[1];DOCS=PROJECT/'docs';ASSETS=DOCS/'local-original/diplomacy-native'
sys.path.insert(0,str(PROJECT/'tools'))
from extract_ee2_textures import decode_texcache
INSTALL=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
sha=lambda b:hashlib.sha256(b).hexdigest()
m=json.loads((ASSETS/'manifest.json').read_text(encoding='utf8'));checks=0
def check(ok,message):
 global checks
 assert ok,message
 checks+=1
archives={};decoded={};pixels=0;rotated=0
try:
 for name,record in m['sprites'].items():
  relative=PurePosixPath(record['file']);check(not relative.is_absolute() and '..' not in relative.parts and '\\' not in record['file'] and ':' not in record['file'],'relative asset '+name)
  path=ASSETS/record['file'];check(path.name in {p.name for p in ASSETS.iterdir()},'exact case '+name)
  check(sha(path.read_bytes())==record['sha256'],'PNG whole-file hash '+name)
  rgba=Image.open(path).convert('RGBA');check(list(rgba.size)==[record['width'],record['height']],'PNG dimensions '+name)
  origin=m['textures'][record['sourceTexture']];archive=origin['archive']
  if archive not in archives:archives[archive]=zipfile.ZipFile(INSTALL/archive)
  if record['sourceTexture'] not in decoded:
   raw=archives[archive].read(origin['member']);check(sha(raw)==origin['sha256'],'original atlas SHA '+record['sourceTexture']);decoded[record['sourceTexture']]=decode_texcache(raw)[0].convert('RGBA')
  source=decoded[record['sourceTexture']];u,v,w,h=record['sourceSprite']['rect'];check(u>=0 and v>=0 and u+w<=source.width and v+h<=source.height,'source crop containment '+name)
  # Independent coordinate mapping, not the importer's crop()/transpose().
  source_bytes=source.tobytes();actual=rgba.tobytes();expected=bytearray(len(actual))
  if record['restoredPackedRotation']:
   rotated+=1
   for y in range(rgba.height):
    for x in range(rgba.width):
     a=((v+x)*source.width+u+w-1-y)*4;b=(y*rgba.width+x)*4;expected[b:b+4]=source_bytes[a:a+4]
  else:
   for y in range(rgba.height):
    a=((v+y)*source.width+u)*4;b=y*rgba.width*4;expected[b:b+rgba.width*4]=source_bytes[a:a+rgba.width*4]
  check(actual==expected,'all original RGBA pixels and orientation '+name);pixels+=rgba.width*rgba.height
  scale=record['sourceResolutionScale'];ratios=[record['width']/record['logicalWidth'],record['height']/record['logicalHeight']];check(ratios==([scale,scale] if isinstance(scale,(int,float)) else scale),'declared physical/logical scale '+name)
 check({p.name for p in ASSETS.glob('*.png')}=={r['file'] for r in m['sprites'].values()},'No unbound original media in candidate subtree')
 for name,record in m['playerStripPixels'].items():
  rgba=base64.b64decode(record['rgba']);check(rgba==Image.open(ASSETS/record['pngFile']).convert('RGBA').tobytes(),'runtime color modulation input is exact original RGBA '+name);check(sha(rgba)==record['rgbaSha256'],'runtime strip byte hash '+name)
 for root in ['diplo_panel','prop_panel']:check(root in m['widgets'],'native root '+root)
 for row in m['widgets'].values():
  if row['SPRITE']:check(row['SPRITE'] in m['sprites'],'declared widget sprite binding '+row['NAME'])
 report={'format':'native5-diplomacy-independent-images-v1','passed':True,'checks':checks,'sprites':len(m['sprites']),'atlases':len(decoded),'comparedRgbaPixels':pixels,'rotatedPackedSprites':rotated,'manifestSha256':sha((ASSETS/'manifest.json').read_bytes()),'pngBytes':sum((ASSETS/r['file']).stat().st_size for r in m['sprites'].values()),'scope':'Direct installed atlas bytes and an independent full-pixel crop/rotation coordinate oracle; all151 original PNGs, exact case/relative paths and declared logical extents. No repaint, rescale, browser or original executable.'}
 (PROJECT.parent/'EE2-Research/analysis/native5-diplomacy-images-tests.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report))
finally:
 for archive in archives.values():archive.close()
