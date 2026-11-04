"""Reproducible source hashes/constants for the targeted native calendar audit."""
import hashlib
import json
from pathlib import Path
import struct
import zipfile
from extract_ee2_textures import ROOT, INSTALL

RESEARCH=ROOT.parent/'EE2-Research/analysis'
ENGINE=ROOT.parents[1]/'work/EE2-engine.exe'
raw=ENGINE.read_bytes()
sha=lambda value:hashlib.sha256(value).hexdigest()
pe=struct.unpack_from('<I',raw,60)[0]
imagebase=struct.unpack_from('<I',raw,pe+52)[0]
sections=struct.unpack_from('<H',raw,pe+6)[0]
optional=struct.unpack_from('<H',raw,pe+20)[0]
ranges=[struct.unpack_from('<8sIIII',raw,pe+24+optional+i*40) for i in range(sections)]
def offset(va):
    return next((start+va-imagebase-rva for _,virtual,rva,size,start in ranges if rva<=va-imagebase<rva+size),None)
constant_offset=offset(0x00b474d8)
bits=struct.unpack_from('<I',raw,constant_offset)[0]
inverse_months=struct.unpack_from('<f',raw,constant_offset)[0]
assert abs(inverse_months-1/12)<1e-8
constructor=raw[offset(0x006c8b60):offset(0x006c8b60)+44]
assert struct.pack('<I',0x44960000) in constructor
sources=[]
for archive_name,member in [('hdrs.zip','DbHdrs/DbTerrainDefs.h'),('hdrs.zip','DbHdrs/DbFrontEndDefs.h'),('hdrs.zip','DbHdrs/DbClimateDefs.h'),('db.zip','db/Terrain/dbclimate.ddf'),('db.zip','db/Terrain/DbSeasonalTextureSets_Temperate.csv')]:
    with zipfile.ZipFile(INSTALL/'zips'/archive_name) as archive:
        data=archive.read(member)
    sources.append({'archive':archive_name,'entry':member,'sha256':sha(data),'bytes':len(data)})
exports=[]
for name in ['native-calendar-runtime.c','native-calendar-phase.c']:
    data=(RESEARCH/name).read_bytes()
    assert b'd41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f' in data
    exports.append({'file':name,'sha256':sha(data),'bytes':len(data)})
report={
 'format':'ee2-native-calendar-evidence-v1','date':'2026-11-03',
 'scope':'Two bounded read-only/noanalysis Ghidra exports from the installed EnabledUP15Units engine; native configuration/header/table checks. No original files, gameplay rules or weather RNG changed.',
 'engineSha256':sha(raw),'sources':sources,'exports':exports,
 'constants':{'constructorSecondsPerYear':1200.0,'constructorSecondsBits':'44960000','constructorCycleMonth':True,'constructorStartMonth':0,
     'monthsPerYear':12,'inverseMonthConstantVA':'00b474d8','inverseMonthBits':f'{bits:08x}','inverseMonthFloat':inverse_months,
     'frontEndMinutesPerSeasonOptions':[1,5,10,20,30]},
 'verifiedFunctions':[
     {'address':'006c8b60','finding':'Calendar constructor writes vtable, currentMonth0, cycleMonthtrue, secondsPerYear1200, yearTimer0, yearsPlayed0, stopMonth-1.'},
     {'address':'006c8b90','finding':'Copies year length and cycle flag from supplied settings; initializes yearTimer=(secondsPerYear/12)*requestedMonth for a valid month0..11.'},
     {'address':'006c8c10','finding':'Adds supplied elapsed time when cycling; wraps at secondsPerYear and increments yearsPlayed. Assembly computes currentMonth=truncate(yearTimer/(secondsPerYear*1/12)). Optional stopMonth may stop cycling.'},
     {'address':'006c8d30','finding':'Changes secondsPerYear and rescales yearTimer by new/old to preserve annual phase.'},
     {'address':'006c8d50','finding':'Serializer identifies offsets +4CurrentMonth,+8CycleMonth,+CSecondsPerYear,+10YearTimer,+14YearsPlayed,+18PauseCalendar,+19CurrentMonthChangedManually.'},
     {'address':'006c8ec0','finding':'Returns elapsed seconds within current month by subtracting currentMonth*monthlyDuration from yearTimer.'},
     {'address':'006cb350','finding':'EE2Season update forwards the same supplied delta to Calendar, then invokes season/weather/lighting-related updates.'},
     {'address':'00fa8119','finding':'Installed patch wrapper calls an optional function pointer before native Calendar initialization; untracked live patch settings are not inferred.'},
 ],
 'sourceInterpolation':{'header':'DbHdrs/DbTerrainDefs.h:76-82','rule':'Ordered texture list is interpolated evenly across the year; twelve textures give one per month.',
     'textureLists':32,'sourceImagesPerList':12,'gpuImplementation':'Eight native-size RGBA8 texture arrays, twelve monthly layers each; adjacent monthly texture samples mixed by the normalized month fraction.'},
 'ownAdapter':{'simulationTicksPerSecond':20,'phaseSource':'Current deterministic WASM tick, reduced modulo integral year duration, plus explicit startMonth.',
     'applicationStartMonth':6,'defaultMinutesPerSeason':5,'rateStatus':'Matches native1200-second constructor year; summer starting month is an application choice.',
     'saveLoadAndNetwork':'Calendar settings must accompany a save/host setup. Phase is recomputed from restored/current tick without a wall-clock timer or accumulated render deltas.',
     'floatDifference':'Original accumulates floating elapsed-time steps; the render-only adapter derives phase from integer ticks and does not reproduce native float drift or all live patch hooks.'},
 'weatherSourceOnly':{'climate':'ClimateWeatherTemperate','rainySeasonStart':3,'rainySeasonStop':6,'snowySeasonStart':11,'snowySeasonStop':3,
     'snowRequiredLeadTime':5,'snowRequiredMonths':3,'dustySeasonStart':-1,'leafySeasonStart':9,'leafySeasonStop':11,
     'status':'Availability/source fields only. No rain/snow scheduling, boundary inclusivity, random event probability/duration, precipitation geometry or gameplay weather penalties are reconstructed by these fields alone.'},
 'open':['Live front-end/random starting-season selection and patch callbacks','Calendar stop-at-month editor control','Complete seasonal tint/weather/lighting and vegetation changes','Weather queue RNG, event probabilities and durations'],
}
(RESEARCH/'native-calendar-evidence.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps({'engineSha256':report['engineSha256'],'nativeSecondsPerYear':1200,'nativeInverseMonths':inverse_months,'sourceFiles':len(sources),'exports':len(exports),'weatherScheduleImplemented':False},indent=2))
