// OpenEpoch front end: input, camera, WebGL2 drawing, HUD, sound, saves, networking.
// The simulation lives in openepoch.wasm (Rust); this file never touches game rules.
import { buildAtlas, tintAtlas, CELL } from './atlas.js?v=fortress1';
import { buildModels } from './models.js?v=native77';
import { parseCheat } from './cheats.js?v=native45';
import { researchPanel } from './research.js?v=rank-families';
import {loadCrownArt,mountCrownProgress,mountCrownVictoryRows} from './crown-progress.js?v=native65';
import {loadCrownStatusArt,readCrownActorVisuals,crownStatusSlots} from './crown-visuals.js?v=native58';
import { originalResearchActions, researchCost, researchState } from './original-research-actions.js?v=rank-families';
import { diplomacyPanel } from './diplomacy.js?v=territory-batch154';
import {enhanceOriginalDiplomacy} from './original-diplomacy.js?v=border107';
import {loadNativeFontUI} from './original-font-ui.js?v=native21names';
import {territoryClaimMessage} from './territory-help.js?v=native64-koth';
import * as sfx from './audio.js?v=native112voice';
import {liveGeneratedAmbientCoverage,loadOriginalAmbient,originalAmbientPriorities} from './original-ambient.js?v=native106';
import {loadOriginalAssets,loadOriginalEpochIcons,applyOriginalInterface,originalIcon} from './original-assets.js?v=citizen9';
import {mountOriginalShellVideo} from './original-shell-video.js?v=native12';
import {mountSkirmishMenu} from './skirmish-menu.js?v=native13';
import {mountSkirmishTeams} from './skirmish-teams.js?v=native74-teams';
import {checkStartingTeams, checkpointStartingTeams, mountNetworkTeamPicker, packStartingTeams} from './network-starting-teams.js?v=native75-teams';
import {loadCitizenPresentation} from './original-citizen-presentation.js?v=regional10';
import {selectedCitizenContext,decodeCitizenDeathCue} from './source-citizen-context.js?v=regional10';
import {loadOriginalLabels,escapeLabelHtml} from './original-labels.js?v=native6';
import {loadNativeHud} from './original-hud.js?v=native66';
import {loadOriginalModels} from './original-renderer.js?v=valley-city155';
import {loadOriginalBoatWake,readNavalWakeActors} from './original-boat-wake.js?v=native78-wake83';
import {loadKingOfTheHillPresentation} from './original-koth.js?v=native64-koth';
import {loadWonderPresentation} from './original-wonders.js?v=native63';
import {loadWonderScaffold} from './original-wonder-scaffold.js?v=native63';
import {loadWonderFence} from './original-wonder-fence.js?v=native63';
import {loadDiversionPresentation} from './original-diversion-presentation.js?v=native62';
import {createDiversionShimmerFrames} from './original-diversion-shimmer.js?v=native62';
import {loadHeavyMounted3Presentation} from './original-heavy-mounted3.js?v=native62';
import {loadArtemisPresentation} from './original-artemis.js?v=native62';
import {createGuidedHercAttack} from './guided-herc-attack.js?v=native66';
import {loadCrownLeaderPresentation} from './original-crown-leaders.js?v=native60';
import {loadUngarrisonArt,ungarrisonIcon} from './original-ungarrison.js?v=native60';
import {loadLeaderCursors,loadImperialismBadge} from './original-leader-cursors.js?v=native63';
import {createRallyHealGate,loadOriginalRallyHeal} from './original-rally-heal.js?v=native60';
import {ownMapTerrainOrigin} from './original-citycenter-terrain.js?v=regional10';
import {loadCityCenterIcons,loadHouseIcons,loadDockIcons,loadBarracksIcons,loadMillIcons,loadTempleIcons,loadLightArtilleryIcons,loadWarehouseIcons,loadStableIcons,loadKingIcons} from './original-citycenter-icons.js?v=native17';
import {loadSourceSailRuntime} from './source-sail-runtime.js?v=sails7';
import {createRenderAssetMapping} from './original-render-namespace.js?v=native6';
import {createFrontendNamespace} from './render-namespace.js?v=native77';
import {ORIGINAL_SPEEDS, changeOriginalSpeed, checkOriginalTempo, loadOriginalTempo, originalStartingSpeed} from './original-game-tempo.js?v=native37';
import {addF64, advanceAdaptiveSourceFrame, advanceSourceSoloFrame, createAdaptiveSourceClock, sampleSourceRenderFps, SOURCE_SOLO_INTERVAL, sourceRenderSeconds, sourceStepDelta, sourceWallInterval, storeF32} from './source-solo-clock.js?v=native44';
import {mountOriginalGameOptions} from './original-game-options.js?v=native37';
import {mountOriginalGameSettings} from './original-game-settings.js?v=native79-wake';
import {createRenderActorMetadata} from './render-actor-metadata.js?v=pack8';
import {createPackRenderMetadata} from './original-pack-states.js?v=pack-frame123';
import {createNativePackSceneClock} from './original-pack-scene.js?v=pack-offscreen132';
import {createMrlsPackAudio,mrlsPackAudioCatalog} from './mrls-pack-audio.js?v=mrls-pack133';
import {createFullmapPipBridge} from './original-fullmap-pip-bridge.js?v=native66';
import {loadOriginalGround,sampleGroundHeight} from './original-ground.js?v=native67';
import {createCaptureGlow} from './capture-glow.js?v=native64-koth';
import {loadOriginalTerrain} from './original-terrain.js?v=skirmish-climates';
import {loadOriginalWater} from './original-water.js?v=water-farm3';
import {createOriginalReflection} from './original-reflection.js?v=visual-seasons2';
import {NATIVE_SEASON_CHOICE,normalizeSeasonChoice,resolveSeasonChoice} from './original-seasons.js?v=native108';
import {createSeasonSession,peerSeasonSettings,savedSeasonSettings} from './season-session.js?v=native108';
import {checkedClimate,loadSkirmishClimates,peerClimates,savedClimate,supportedSkirmishClimates} from './skirmish-climates.js';
import {UP15_PROFILE_ENABLED,resolveRulesProfile,rulesProfileURL,loadRulesWasm,savedRulesProfileMatches} from './rules-profile.js?v=native116';
import {createEconomyBar,RESOURCE_NAMES} from './economy-bar.js?v=fortress1';
import { Lockstep, PROTOCOL_VERSION as BASE_PROTOCOL_VERSION, host as rtcHost, join as rtcJoin } from './net.js?v=native55';
import {SourceRoundPeer, SOURCE_ROUND_VERSION, groupedOwnerLayout} from './source-network-round.js?v=native54';
import {createSourceRoundCarrier, selectedRoundTripMilliseconds} from './source-network-wire.js?v=native54';
import {mountSourceRosterLobby,requiresCapitolRoster} from './source-network-lobby.js?v=fish109';
import {applyNetworkCheckpoint} from './source-network-resume.js?v=native75-teams';
import {createFullscreenController} from './fullscreen.js?v=building2';
import {createCivilizationSetup,civilizationChoice,resolveCivilizations,checkedCivilizations,applyMatchSettings,checkedEpochRange,resolveEpochRange} from './civilization-setup.js?v=native67';
import {bindCrownVictoryMenu,checkCrownVictorySettings,readCrownVictoryHud,CROWN_VICTORY_COUNT} from './crown-victory-ui.js?v=network-crowns1';
import {bindCapitolsMenu,readCapitolHud} from './capitols-menu.js?v=native66';
import {capitolFeedback,loadCapitolFeedback} from './capitol-feedback.js?v=native67';
import {bindKothMenu,formatKothStatus} from './koth-menu.js?v=native64-koth';
import {createBuildingControls,nextBuildingTurn,placementMessage} from './building-controls.js?v=farm-cursor147';
import {createFarmAngleMemory,FARM_INITIAL_YAW} from './farm-angle-memory.js?v=farm-angle148';
import {updateOriginalCamera,originalCameraRay,originalFractionForZoom,normalizeOriginalHeading,rotateOriginalCameraHeading} from './original-camera.js?v=heading2';
import {updateAuthoredCamera} from './authored-camera-axis.js';
import {createFullmap} from './fullmap.js?v=native77';
import {createPlayerFlare} from './player-flare.js?v=native24';
import {createBattleFlare} from './battle-flare.js?v=native25';
import {createCaptureOrder,captureOrderText,decorateCommandButton} from './capture-order.js?v=native-spy';
import {nativeMetaEpoch} from './original-hud-layout.js';
import {loadPriestCommandArt} from './original-priest-command-art.js?v=native45';
import {loadSpyCommandArt,loadSpyAdvancedCommandArt} from './original-spy-command-art.js?v=native-spy-powers';
import {createSpyShimmerFrames} from './original-spy-shimmer.js?v=native-spy';
import {createSpyExtractionTimer} from './spy-extraction-renderer.js?v=native-spy';
import {loadPriestEffects} from './original-priest-fx.js?v=native45';
import {guidedImpactInside,loadGuidedImpactEffects} from './guided-herc-impact.js?v=native66';
import {loadGuidedHercBeam} from './guided-herc-beam.js?v=native73';
import {loadOriginalPriestParticles} from './original-priest-particles.js?v=native47';
import {loadBlessGroundGlow} from './priest-ground-glow.js?v=native58';
import {loadAirCrownGlow} from './crown-air-glow.js?v=native58';
import {createTradeRoute,supportsTradeRoute,tradeRouteText} from './trade-route.js?v=native30';
import {loadMarketArt,marketOrders,marketOrderButton,marketOrderHotkey} from './market-orders.js?v=native58';
import {loadCarrierPresentation,loadFighterBomberPresentation,loadPriestPresentation,loadSpyPresentation,loadTradePresentation,loadTradeRouteArt} from './original-trade-presentation.js?v=native-spy-powers';
import {loadFishingPresentation} from './original-fishing.js?v=native77';
import {createTradeWaitStatus,loadTradeWaitArt,selectedTradeWait} from './trade-wait-status.js?v=native31';
import {airActions,airMissionActions,airMissionSelection,airMissionTrainingKey,createAirMissionOrder,decorateAirMissionButton,loadAirMissionArt,supportsAirMission} from './air-mission-order.js?v=native32';
import {loadAirMissionFlag,loadCarrierGarrisonFlag,projectAirMissionFlags,readAirMissionVisualRoutes} from './air-mission-flag.js?v=native76-carrier';
import {createTakeoffAudioClock,loadAirTakeoff,takeoffElapsed} from './air-takeoff.js?v=native40clock';
import {addCarrierFlightAudio,loadAirFlightAudio,readAirFlightAudio,submarineMoveAudio} from './air-flight-audio.js?v=submarine-launch122';
import {loadFullmapCityNameCatalog,loadFullmapTerritoryNameCatalog} from './fullmap-territory-names.js?v=native22';
import {createWarplanPeer,WARPLAN_PEER_VERSION} from './original-warplan-peer.js';
import {activeValleyGeometry,initialValleyViews,pickValleyGround,valleyMinimapPoint,valleyMinimapSnapshot} from './authored-view.js';
import {installedValleyManifest,loadValleyPreview,valleySaveLocator} from './valley-preview.js?v=valley-city155';

const $ = id => document.getElementById(id);
const canvas = $('gl');
const gl = canvas.getContext('webgl2', { antialias: true, alpha: false, stencil: true, powerPreference: 'high-performance' });
if (!gl) { document.body.innerHTML = '<p style="padding:40px">OpenEpoch needs WebGL2. Please use a current browser.</p>'; throw new Error('no webgl2'); }

const MAP = 256, TICK = 0.05, STRIDE = 12;
const FOV = 35 * Math.PI / 180, PITCH = 55 * Math.PI / 180, TANF = Math.tan(FOV / 2);
const TEAM = [[0.27, 0.59, 1.0], [0.94, 0.27, 0.24], [0.31, 0.86, 0.39], [0.96, 0.82, 0.27], [1, 1, 1], [1, 1, 1], [1, 1, 1], [0.3, 0.75, 1]];

// ---------------------------------------------------------------- wasm
const profileSelect=$('rules-profile');
for(const option of profileSelect.options)if(option.value==='up15')option.disabled=!UP15_PROFILE_ENABLED;
profileSelect.onchange=()=>location.assign(rulesProfileURL(location.href,profileSelect.value).href);
function rulesLoadFailure(error){
 $('rules-note').textContent=error.message;
 for(const id of ['play','mpopen','continue','previous-save'])$(id).disabled=true;
 $('play').textContent='Regelversion konnte nicht geladen werden';
}
let rulesProfile;
try{rulesProfile=resolveRulesProfile(location.href);}catch(error){rulesLoadFailure(error);throw error;}
profileSelect.value=rulesProfile.id;
const referenceMode=rulesProfile.reference,qaMode=rulesProfile.qa;
const privateMapLinks=$('local-private-maps');
if(privateMapLinks&&referenceMode&&['127.0.0.1','localhost','[::1]'].includes(location.hostname)&&
  !location.pathname.endsWith('-private.html')&&new URL(location.href).searchParams.get('assets')!=='procedural'){
  for(const link of privateMapLinks.querySelectorAll('a[data-carrier]')){
    Promise.all([link.href,new URL(link.dataset.carrier,location.href)].map(url=>fetch(url,{method:'HEAD'})))
      .then(([page,carrier])=>{
        if(page.ok&&page.headers.get('content-type')?.includes('text/html')&&
          carrier.ok&&carrier.headers.get('content-type')?.includes('application/wasm')){
          link.hidden=false;
          privateMapLinks.hidden=false;
        }
      }).catch(()=>{});
  }
}
const immigrationCue={url:new URL('local-original/audio/fx_feedback_citizen_created.mp3',import.meta.url),
  sha256:'e123f7873b3ac1481a62519ff0d6c2f138212c4ce61c83dca9183797ebaf821d',gain:1};
try { const preference=localStorage.getItem('openepoch-map-visibility');if(['0','1','2'].includes(preference))$('map-visibility').value=preference; } catch { /* preferences are optional */ }
$('map-visibility').onchange=()=>{try{localStorage.setItem('openepoch-map-visibility',$('map-visibility').value);}catch{/* private storage */}};
function menuSeasonSettings(){
  return resolveSeasonChoice(menuSeasonChoice(),sfx.presentationWord);
}
function menuSeasonChoice(){
  return normalizeSeasonChoice({cycle:$('season-cycle').checked,startMonth:+$('season-start').value,minutesPerSeason:+$('season-length').value});
}
function setSeasonControls(settings){
  $('season-cycle').checked=settings.cycle;
  $('season-start').value=settings.startMonth;
  $('season-length').value=settings.minutesPerSeason;
  $('season-length').disabled=!settings.cycle;
}
$('season-options').hidden=!referenceMode;
if(rulesProfile.id==='up15')setSeasonControls(NATIVE_SEASON_CHOICE);
try{
  const saved=localStorage.getItem('openepoch-seasons');
  if(saved){
    const choice=normalizeSeasonChoice(JSON.parse(saved));
    if(referenceMode||choice.startMonth>=0)setSeasonControls(choice);
  }
}catch{/* optional preferences */}
for(const id of ['season-cycle','season-start','season-length'])$(id).onchange=()=>{
  const settings=menuSeasonChoice();setSeasonControls(settings);
  try{localStorage.setItem('openepoch-seasons',JSON.stringify(settings));}catch{/* private storage */}
};
if (referenceMode) {
  $('play').disabled=true;$('play').textContent='Originalgrafiken laden …';
  $('supplies').innerHTML='<option value="3">Wenig</option><option value="4">Defensiv</option><option value="0">Standard</option><option value="1" selected>Viel</option><option value="2">Deathmatch</option>';
  document.title = 'OpenEpoch · EE2 Rekonstruktion';
  $('terr').parentElement.title='Deine Gebiete. Angrenzend genügt ein Stadtzentrum; abgelegene Gebiete benötigen zusätzlich eine Festung.';
  $('cheat-rule').textContent='give tech erhöht das reguläre Forschungskonto um 50 Technologiepunkte. Technologien und Epochenwechsel werden mit diesen Punkten bezahlt.';
  $('rules-note').textContent = '15 Epochen, neun Rohstoffe, Originalkosten und Forschung durch Bürger in Universitäten. Erweitere dein Reich mit Stadtzentren und Festungen. Originalgrafiken, Stimmen und Musik liegen im Spielordner. Einheitenangebot und Spezialfähigkeiten werden weiter ergänzt.';
  // Both multiplayer peers must explicitly match this ruleset.
}
let wasm;
const valleyParams=new URL(location.href).searchParams;
const valleyMode=valleyParams.get('valley-preview');
const valleyNormal=rulesProfile.id==='up15'&&valleyMode===null&&valleyParams.get('assets')!=='procedural';
const valleyManifest=valleyNormal?await installedValleyManifest():null;
const valleyNeutralRequested=valleyMode==='neutral-qa'||valleyNormal;
const valleyRequested=rulesProfile.id==='up15'&&(!!valleyManifest||['1','archive-qa','neutral-qa'].includes(valleyMode));
try{
  if(valleyRequested){
    const requiredExports=valleyNormal?[...rulesProfile.requiredExports,'valley76_commit','valley71_force_attack_at','valley71_neutral_opening','configure_late_population']:rulesProfile.requiredExports;
    const previewRules={...rulesProfile,requiredExports,wasm:'openepoch-ee2-valley-preview.wasm?v=native116-valley'};
    try{wasm=await loadRulesWasm(previewRules,import.meta.url);}catch{/* The ordinary ruleset remains available. */}
  }
  wasm??=await loadRulesWasm(rulesProfile,import.meta.url);
}catch(error){rulesLoadFailure(error);throw error;}
// Upstream placement admission is part of the loaded UP engine's wire contract.
const foreignConstructionVersion=wasm.foreign_construction_version?.()??0;
const territoryGiftBatchVersion=typeof wasm.cmd_territory_gift_batch==='function'?1:0;
const PROTOCOL_VERSION=rulesProfile.id==='up15'?BASE_PROTOCOL_VERSION+3+foreignConstructionVersion+territoryGiftBatchVersion:
  wasm.early_artillery_pack_version?.()===1?BASE_PROTOCOL_VERSION+1:BASE_PROTOCOL_VERSION;
// Always take the pointer BEFORE touching memory.buffer: the call may grow (and detach) the memory.
const u8 = (ptr, len) => new Uint8Array(wasm.memory.buffer, ptr, len);
const f32 = (ptr, len) => new Float32Array(wasm.memory.buffer, ptr, len);
const dec = new TextDecoder();
const assetMapping=createRenderAssetMapping(wasm);
const renderNamespace=createFrontendNamespace(wasm,{referenceMode,assetMapping});
const packMetadata=createPackRenderMetadata(wasm),packActorReader=packMetadata?createRenderActorMetadata(wasm):null;
const civilizationSetup=createCivilizationSetup({wasm,document,anchor:$('sk-player-options')});
if(referenceMode){
  const row=document.createElement('div');row.className='row';
  const label=document.createElement('label');label.htmlFor='start-epoch';label.textContent='Startepoche';
  const select=document.createElement('select');select.id='start-epoch';
  for(let i=0;i<wasm.epoch_count();i++){const option=document.createElement('option');option.value=i;option.textContent=`${i+1} · ${dec.decode(u8(wasm.epoch_name_ptr(i),wasm.epoch_name_len(i)))}`;select.append(option);}
  const requested=+(new URLSearchParams(location.search).get('epoch')||0);if(requested>=0&&requested<wasm.epoch_count())select.value=requested;
  row.append(label,select);
  if (typeof wasm.configure_match_epoch_range === 'function') {
    const endLabel = document.createElement('label');
    endLabel.htmlFor = 'end-epoch';
    endLabel.textContent = 'Endepoche';
    const end = select.cloneNode(true);
    end.id = 'end-epoch';
    end.value = wasm.epoch_count() - 1;
    const random = document.createElement('option');
    random.value = '-1';
    random.textContent = 'Zufall';
    select.append(random);
    end.append(random.cloneNode(true));
    const updateEpochOptions = () => {
      for (const option of select.options) option.disabled = +end.value >= 0 && +option.value > +end.value;
      for (const option of end.options) option.disabled = +option.value >= 0 && +option.value < +select.value;
    };
    select.onchange = end.onchange = updateEpochOptions;
    updateEpochOptions();
    row.append(endLabel, end);
  }
  $('pop-limit').closest('.row').before(row);
}

// ---------------------------------------------------------------- GL setup
function shader(type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src); gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) + src);
  return s;
}
function program(vs, fs) {
  const p = gl.createProgram();
  gl.attachShader(p, shader(gl.VERTEX_SHADER, vs)); gl.attachShader(p, shader(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {};
  const n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) { const name = gl.getActiveUniform(p, i).name.replace('[0]', ''); u[name] = gl.getUniformLocation(p, name); }
  return { p, u };
}

const NOISE = `
float hash(vec2 p){ p = fract(p*vec2(123.34, 456.21)); p += dot(p, p+45.32); return fract(p.x*p.y); }
float noise(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1,0)),f.x), mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y); }`;
const FOG = `float fogAt(vec2 w){ return texture(uFog, w/${MAP}.).r; }
vec3 fogged(vec3 c, float f){ return c*smoothstep(.02,.45,f)*mix(.5,1.,smoothstep(.55,.95,f)); }`;
const SUN = 'const vec3 SUN = vec3(0.42, 0.82, 0.39);';

const terrain = program(`#version 300 es
layout(location=0) in vec3 aP; layout(location=1) in vec3 aN; uniform mat4 uVP; out vec3 vP; out vec3 vN;
void main(){ vP = aP; vN = aN; gl_Position = uVP*vec4(aP,1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog, uTerr; uniform highp sampler2DArray uOriginalTerrain;uniform bool uOriginal;
uniform vec3 uTeam[8]; uniform float uPx; in vec3 vP; in vec3 vN; out vec4 o;
${NOISE}${FOG}${SUN}
void main(){
  vec2 w = vP.xz, uv = w/${MAP}.;
  float visibility=fogAt(w);if(visibility<.015){o=vec4(0.,0.,0.,1.);return;}
  vec4 m = texture(uMap, uv);
  vec3 c = m.rgb;
  float grass = smoothstep(1.02,1.26,c.g/(max(c.r,c.b)+.001));
  float groundPatch=noise(w*.32)*.6+noise(w*1.1)*.4;
  if(uOriginal){
    vec2 tiled=w*.22;
    float beach=smoothstep(.54,.7,m.r)*(1.-grass);
    float stone=(1.-smoothstep(.035,.1,abs(m.r-m.g)))*(1.-beach)*(1.-grass);
    float forest=grass*(1.-smoothstep(.22,.33,m.r));
    if(m.a<.7)c=texture(uOriginalTerrain,vec3(tiled,6.)).rgb;
    else if(beach>.99)c=texture(uOriginalTerrain,vec3(tiled,3.)).rgb;
    else if(stone>.99)c=texture(uOriginalTerrain,vec3(tiled,4.)).rgb;
    else{
      vec3 base=texture(uOriginalTerrain,vec3(tiled,0.)).rgb;
      vec3 patchGrass=texture(uOriginalTerrain,vec3(tiled+vec2(.31,.23),1.)).rgb;
      vec3 soil=texture(uOriginalTerrain,vec3(tiled,2.)).rgb;
      c=mix(base,patchGrass,smoothstep(.38,.72,groundPatch)*.55);
      c=mix(c,soil,forest*.28+smoothstep(.68,.85,groundPatch)*.18);
      if(beach>.01)c=mix(c,texture(uOriginalTerrain,vec3(tiled,3.)).rgb,beach);
      if(stone>.01)c=mix(c,texture(uOriginalTerrain,vec3(tiled,4.)).rgb,stone);
    }
  }else{
    float n=noise(w*.7)*.35+noise(w*5.)*.25+noise(w*23.)*.4;
    vec3 turf=mix(c*vec3(.83,.92,.77),vec3(.39,.36,.20),smoothstep(.53,.76,groundPatch)*.4);
    float blades=pow(noise(w*16.),2.);turf*=.94+blades*.18*clamp(1.-uPx*18.,0.,1.);
    c=mix(c,turf,grass)*(.88+.3*n);
  }
  float lit = .55 + .6*max(dot(normalize(vN), SUN), 0.);
  c *= lit;
  c *= mix(1., .5, clamp(-vP.y*.9, 0., 1.));                       // sea bed fades with depth
  // territory borders
  vec4 t = texture(uTerr, uv);
  float d = max(.09, uPx*1.3)/${MAP}.;
  float e = 0.;
  e = max(e, step(.001, abs(texture(uTerr, uv+vec2(d,0.)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv-vec2(d,0.)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv+vec2(0.,d)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv-vec2(0.,d)).r - t.r)));
  int owner = int(t.g*255.+.5);
  vec3 tc = owner > 0 ? uTeam[owner-1] : vec3(.9);
  c = mix(c, tc, owner > 0 ? .06 + e*.7 : e*.2);
  o = vec4(fogged(c, visibility), 1.);
}`);

const water = program(`#version 300 es
layout(location=0) in vec2 aP; uniform mat4 uVP; out vec2 vW;
void main(){ vW = aP; gl_Position = uVP*vec4(aP.x, 0., aP.y, 1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog;uniform highp sampler2DArray uOriginalTerrain;uniform bool uOriginal;
uniform float uTime; in vec2 vW; out vec4 o;
${NOISE}${FOG}
void main(){
  vec2 uv = vW/${MAP}.;
  if (uv.x<0.||uv.y<0.||uv.x>1.||uv.y>1.) { o = vec4(.012,.03,.05,1.); return; }
  vec4 m = texture(uMap, uv);
  if(m.a>=.72)discard;
  float visibility=fogAt(vW);if(visibility<.015)discard;
  float waves=sin(vW.x*2.1+uTime*1.3)+sin(vW.y*1.8-uTime*.9);
  float wv = .5 + waves*.12 + noise(vW*.7+uTime*.1)*.15;
  vec3 c = mix(vec3(.05,.2,.36), vec3(.1,.42,.58), smoothstep(0.,.3,m.a))*(.75+.5*wv) + vec3(.9,.95,1.)*smoothstep(.7,.9,wv)*.14;
  c += vec3(.55,.75,.85)*pow(max(0.,sin(vW.x*.7+vW.y*1.2+uTime*.6)),24.)*.12;
  if(uOriginal){
    vec3 original=texture(uOriginalTerrain,vec3(vW*.16+vec2(uTime*.009,-uTime*.005),5.)).rgb;
    c=mix(c,original*(.85+.15*wv),.52);
  }
  float foam = smoothstep(.1,.36,m.a)*(1.-smoothstep(.36,.6,m.a));
  c += foam*(.2 + .14*sin(uTime*1.7 + wv*9.));
  float a = (1.-smoothstep(.4,.72,m.a))*.74;
  o = vec4(fogged(c, visibility)*a, a);
}`);

const INST = `layout(location=3) in vec4 iA; layout(location=4) in vec4 iB; layout(location=5) in vec4 iC;`;
const models = program(`#version 300 es
layout(location=0) in vec3 aP; layout(location=1) in vec3 aN; layout(location=2) in vec4 aCol; layout(location=6) in float aJoint; ${INST}
uniform mat4 uVP; uniform highp int uShadow; out vec3 vN; out vec4 vCol; out vec2 vW; out vec3 vLocal; out float vLy; flat out float vJoint; flat out vec4 vB;
void main(){
  float c=cos(iA.w), s=sin(iA.w);
  vec3 local=aP, normal=aN;
  float phase=iC.z*8.+iA.x*.7+iA.y*.3, moving=min(iC.y,1.);
  bool human=iB.x>=8. && iB.x<=17.;
  if(human){
    int bone=int(aJoint+.5), activity=int(iC.w+.5);
    float side=bone==1||bone==3?-1.:1.;
    float angle=0.; vec3 pivot=vec3(0.);
    if(bone==1||bone==2){
      pivot=vec3(0.,.31,side*.075); angle=sin(phase)*side*.65*moving;
    }else if(bone==3||bone==4||bone>=6){
      pivot=vec3(0.,.52,side*.18); angle=-sin(phase)*side*.45*moving;
      if(activity>=2 && activity<=5){
        float stroke=sin(iC.z*(activity==4?4.:7.)+iA.x);
        angle=activity==4?.35+stroke*.2:.65+stroke*.75;
        if(bone==3)angle*=.6;
      }else if(activity==6){ angle=.8+sin(iC.z*9.)*.22; }
    }else if(bone==5){ pivot=vec3(0.,.59,0.); angle=sin(iC.z*1.2+iA.x)*.035; }
    if(bone>0){
      float ca=cos(angle),sa=sin(angle);vec3 q=local-pivot;
      local=vec3(q.x*ca-q.y*sa,q.x*sa+q.y*ca,q.z)+pivot;
      normal=vec3(aN.x*ca-aN.y*sa,aN.x*sa+aN.y*ca,aN.z);
    }
    local.y+=abs(sin(phase))*.018*moving;
    if(activity==4)local.x+=sin(iC.z*4.)*.025;
  }else if(iB.x==43.){
    if(aP.y<.25)local.x+=sin(phase+sign(aP.z)*1.57)*.12*moving;
    local.y+=abs(sin(phase))*.025*moving;
  }
  vec3 p=local*iC.x;
  if(iB.x==46.)p.y+=.12+sin(iC.z*2.+iA.x)*.025;
  if(iB.x>=29. && iB.x<=36.)p.y+=sin(iC.z*1.4+iA.x)*.015;
  vec3 w = vec3(iA.x + p.x*c - p.z*s, iA.z + p.y, iA.y + p.x*s + p.z*c);
  if(uShadow==1){
    w.xz -= vec2(.62,.44)*max(p.y,0.);
    w.y = iA.z+.035;
  }
  vLocal=aP;
  vN = vec3(normal.x*c-normal.z*s,normal.y,normal.x*s+normal.z*c);
  vCol = aCol; vW = w.xz; vLy = aP.y; vB = iB; if(human)vB.z=iC.w; vJoint=aJoint;
  gl_Position = uVP*vec4(w,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uFog; uniform vec3 uTeam[8]; uniform highp int uShadow;
in vec3 vN; in vec4 vCol; in vec2 vW; in vec3 vLocal; in float vLy; flat in float vJoint; flat in vec4 vB; out vec4 o;
${FOG}${SUN}${NOISE}
void main(){
  int fl = int(vB.w+.5);
  if(vB.x==8. && vJoint>=6.){
    int activity=int(vB.z+.5);
    if((vJoint==6. && activity!=2)||(vJoint==7. && activity!=3)||(vJoint==8. && activity!=5)||(vJoint==9. && activity!=4))discard;
  }
  bool tree = vB.x==100. || vB.x==101.;
  if(tree && vLocal.y>.38 && noise(vLocal.xz*65.+vLocal.y*17.)<.16) discard;
  if(uShadow==1){
    if((fl&24)!=0 || fogAt(vW)<.15) discard;
    o=vec4(vec3(.018,.025,.014)*.18,.18); return;
  }
  vec3 c = vCol.a > .5 ? uTeam[int(vB.y+.5)]*vCol.r : vCol.rgb;
  float a = 1.;
  vec3 n = normalize(vN);
  c *= vec3(.36,.43,.52) + vec3(.8,.72,.57)*max(dot(n, SUN), 0.) + .13*max(n.y, 0.);
  c *= mix(.72,1.,smoothstep(0.,.35,vLy));
  if(tree){
    if(vLocal.y>.38) c*=.68+.5*noise(vLocal.xz*63.+vLocal.y*30.);
    else c*=.72+.35*noise(vLocal.xz*80.);
  }
  if((fl&64)!=0 && vCol.a<.5){
    float grain=noise(vLocal.xz*70.+vLocal.y*50.);
    c*=.9+.17*grain;
    if(abs(n.y)<.5){
      vec2 wall=vec2(abs(n.x)>.5?vLocal.z:vLocal.x,vLocal.y);
      vec2 bricks=vec2(wall.x*14.+floor(wall.y*18.)*.5,wall.y*18.);
      vec2 edge=abs(fract(bricks)-.5);
      float mortar=smoothstep(.43,.49,max(edge.x,edge.y));
      c*=1.-mortar*.22;
    }
  }
  if ((fl&64)!=0 && vB.z < 1.) {                                   // under construction: the building rises
    if (vLy > .03 + vB.z*.95) discard;
    c = mix(vec3(dot(c, vec3(.33)))*vec3(1.,.9,.7), c, vB.z);
  }
  if ((fl&4)!=0) c = mix(c, vec3(1.), .5);
  if ((fl&8)!=0) { c = mix(c, vec3(.3,1.,.4), .4); a = .6; }
  if ((fl&16)!=0) { c = mix(c, vec3(1.,.15,.15), .65); a = .6; }
  if ((fl&32)==0) { float f = fogAt(vW); c = fogged(c, f); if (f < .05) discard; }
  o = vec4(c*a, a);
}`);

const quads = program(`#version 300 es
layout(location=0) in vec2 aC; ${INST}
uniform mat4 uVP; uniform vec3 uRight, uUp;uniform float uPx;out vec2 vL; out vec2 vW; flat out vec4 vB;
void main(){
  vec3 base = vec3(iA.x, iA.z, iA.y); float s = iC.x;
  bool flat_ = iB.x < 240.5 || iB.x > 242.5;                       // rings and shadows lie on the ground
  float sy=iB.x>240.5&&iB.x<241.5?min(s,32.*uPx):s;
  vec3 w = flat_ ? base + vec3(aC.x, 0., aC.y)*s : base + uRight*aC.x*s + uUp*aC.y*sy;
  vL = aC; vW = base.xz; vB = iB;
  gl_Position = uVP*vec4(w,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uFog; uniform vec3 uTeam[8]; uniform float uPx;
in vec2 vL; in vec2 vW; flat in vec4 vB; out vec4 o;
${FOG}
void main(){
  int team = int(vB.y+.5); float r = length(vL)*2.; vec4 col;
  if (vB.x < 240.5) {                                              // selection ring
    float w = clamp(uPx*1.6, .03, .2);
    float a = smoothstep(.86-w,.86-w*.5,r)*(1.-smoothstep(.86+w*.5,.86+w,r));
    col = vec4(mix(vec3(.6,1.,.6), uTeam[team], .35),1.)*a*.95;
  } else if (vB.x < 241.5) {                                       // health / progress bar
    float y = abs(vL.y), x = vL.x+.5;
    if (team==6) {
      if (y > 1./24.) discard;
      vec3 low=vec3(150.,16.,16.), mid=vec3(200.,32.,32.), high=vec3(255.,32.,32.);
      vec3 c = vB.z<.5 ? mix(low,mid,clamp((vB.z-.1)*2.5,0.,1.))
        : mix(mid,high,clamp((vB.z-.5)*2.5000002384,0.,1.));
      col = x>vB.z ? vec4(vec3(.2)*.95,.95) : vec4(c/255.,1.);
    } else {
      if (y > .075) discard;
      vec3 c = team==7 ? vec3(.3,.75,1.) : mix(vec3(.95,.2,.15), vec3(.25,.9,.3), smoothstep(.2,.7,vB.z));
      if (x > vB.z) c = vec3(.1);
      if (y > .045) c = vec3(0.);
      col = vec4(c,1.);
    }
  } else if (vB.x < 242.5) {                                       // explosion
    float t = vB.z, q = r/(.3+.7*t);
    float a = (1.-smoothstep(.55,1.,q))*(1.-t*t);
    vec3 c = mix(vec3(1.,.95,.7), vec3(1.,.4,.05), clamp(q*.7+t*.6,0.,1.));
    col = vec4(mix(c, vec3(.15), smoothstep(.5,1.,t))*a, a*.7);
  } else {                                                         // aircraft shadow
    float a = (1.-smoothstep(.5,1.,r))*.34;
    col = vec4(0.,0.,0.,a)*smoothstep(.3,.9,fogAt(vW));
  }
  o = col;
}`);

function tex(unit, filter) {
  const t = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}
gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
const mapTexture=tex(1, gl.LINEAR); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, MAP, MAP, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
const fogTexture=tex(2, gl.LINEAR); gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, MAP, MAP, 0, gl.RED, gl.UNSIGNED_BYTE, null);
const territoryTexture=tex(3, gl.NEAREST); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, MAP, MAP, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
function bindTexture(unit,texture,target=gl.TEXTURE_2D){gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(target,texture);}

// 2D icons for the HUD come from the sprite atlas; the world itself is 3D
const iconAtlas = tintAtlas(buildAtlas(), TEAM[0].map(v => v * 255));

// terrain mesh: (MAP+1)^2 vertices, heights filled in per game
const HN = MAP + 1;
const vaoT = gl.createVertexArray(); gl.bindVertexArray(vaoT);
const terrBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, terrBuf);
gl.bufferData(gl.ARRAY_BUFFER, HN * HN * 24, gl.DYNAMIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
{
  const idx = new Uint32Array(MAP * MAP * 6);
  for (let y = 0, k = 0; y < MAP; y++) for (let x = 0; x < MAP; x++) {
    const a = y * HN + x, b = a + 1, c = a + HN, d = c + 1;
    idx[k++] = a; idx[k++] = c; idx[k++] = b; idx[k++] = b; idx[k++] = c; idx[k++] = d;
  }
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);
}
let heights = new Float32Array(HN * HN);
function uploadTerrain() {
  heights = f32(wasm.hmap_ptr(), HN * HN).slice();
  originalGround.updateTerrain(heights);
  originalModels.updateTerrain?.(heights);
  if(valleyWorld)return;
  uploadOriginalTerrainClasses();
  const v = new Float32Array(HN * HN * 6), h = (x, y) => heights[Math.min(MAP, Math.max(0, y)) * HN + Math.min(MAP, Math.max(0, x))];
  for (let y = 0, k = 0; y < HN; y++) for (let x = 0; x < HN; x++) {
    const nx = h(x - 1, y) - h(x + 1, y), nz = h(x, y - 1) - h(x, y + 1), l = Math.hypot(nx, 2, nz);
    v[k++] = x; v[k++] = h(x, y); v[k++] = y; v[k++] = nx / l; v[k++] = 2 / l; v[k++] = nz / l;
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, terrBuf); gl.bufferSubData(gl.ARRAY_BUFFER, 0, v);
}
function uploadOriginalTerrainClasses(){
  const types=wasm.terrain_types_ptr&&wasm.terrain_types_len?.()===MAP*MAP?u8(wasm.terrain_types_ptr(),MAP*MAP).slice():undefined;
  originalTerrain.updateMap({heights,types});
  originalWater.updateMap({heights,types});
}
const groundAt = (x, y) => {
  if(valleyWorld)return wasm.valley37_view_ground(x,y);
  x = Math.min(MAP - 0.001, Math.max(0, x)); y = Math.min(MAP - 0.001, Math.max(0, y));
  const xi = x | 0, yi = y | 0, fx = x - xi, fy = y - yi, i = yi * HN + xi;
  const a = heights[i] + (heights[i + 1] - heights[i]) * fx, b = heights[i + HN] + (heights[i + HN + 1] - heights[i + HN]) * fx;
  return Math.max(0, a + (b - a) * fy);
};

const vaoW = gl.createVertexArray(); gl.bindVertexArray(vaoW);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-400, -400, 656, -400, -400, 656, 656, 656]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

const instBuf = gl.createBuffer();
function instanced(vao) {
  gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
  for (const loc of [3, 4, 5]) { gl.enableVertexAttribArray(loc); gl.vertexAttribDivisor(loc, 1); }
}
/** Point the instance attributes at instance `first` (WebGL2 has no base-instance draw). */
function instOffset(first) {
  for (const loc of [3, 4, 5]) gl.vertexAttribPointer(loc, 4, gl.FLOAT, false, STRIDE * 4, first * STRIDE * 4 + (loc - 3) * 16);
}
const mesh = buildModels();
const vaoM = gl.createVertexArray(); gl.bindVertexArray(vaoM);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer()); gl.bufferData(gl.ARRAY_BUFFER, mesh.data, gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, mesh.stride, 0);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, mesh.stride, 12);
gl.enableVertexAttribArray(2); gl.vertexAttribPointer(2, 4, gl.FLOAT, false, mesh.stride, 24);
gl.enableVertexAttribArray(6); gl.vertexAttribPointer(6,1,gl.FLOAT,false,mesh.stride,40);
instanced(vaoM);
const vaoQ = gl.createVertexArray(); gl.bindVertexArray(vaoQ);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-.5, -.5, .5, -.5, -.5, .5, .5, .5]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
instanced(vaoQ);

gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
const teamFlat = TEAM.flat();
for (const pr of [terrain, water, models, quads]) {
  gl.useProgram(pr.p);
  if (pr.u.uMap) gl.uniform1i(pr.u.uMap, 1);
  if (pr.u.uFog) gl.uniform1i(pr.u.uFog, 2);
  if (pr.u.uTerr) gl.uniform1i(pr.u.uTerr, 3);
  if (pr.u.uTeam) gl.uniform3fv(pr.u.uTeam, teamFlat);
}
const originalEnabled=referenceMode&&new URLSearchParams(location.search).get('assets')!=='procedural';
const packScene=originalEnabled&&rulesProfile.ruleset===2&&typeof wasm.pack_visual_revision==='function'
  ? createNativePackSceneClock(wasm.pack_visual_revision,wasm.entity_alive,()=>ui?.local) : null;
const marketArt=await loadMarketArt(originalEnabled&&typeof wasm.market_quote==='function');
const dockArt=await loadUngarrisonArt(originalEnabled&&typeof wasm.configure_dock_garrison==='function');
const ambientPresentation=await loadOriginalAmbient({enabled:originalEnabled&&rulesProfile.id==='up15'});
sfx.configureOriginalAmbience(ambientPresentation);
let ambientCoverage=null;
const shellMovie=mountOriginalShellVideo($('shell-movie'),originalEnabled?rulesProfile.id:null);
mountSkirmishMenu($('menu'), {enabled:rulesProfile.ruleset===2, originalArt:originalEnabled});
const victoryOptions=$('sk-victory-options');
victoryOptions.hidden=rulesProfile.id!=='up15';
const victoryCondition=$('sk-victory-condition');
const skirmishTeams=mountSkirmishTeams({wasm:rulesProfile.id==='up15'?wasm:{},document,
  opponents:$('nai'),victory:victoryCondition,soleSurvivor:$('sk-sole-survivor')});
const hotspotAvailable=rulesProfile.id==='up15'&&typeof wasm.configure_match_hotspots==='function'&&typeof wasm.hotspot_point==='function';
const hotspotOption=victoryCondition.querySelector('option[value="hotspots"]');
hotspotOption.hidden=hotspotOption.disabled=!hotspotAvailable;
const hotspotTotal=()=>wasm.hotspot_status?.(0,0)??0;
const hotspotRevealed=()=>hotspotTotal()>0?wasm.hotspot_status(1,0):0;
const hotspotPoints=()=>Array.from({length:hotspotRevealed()},(_,index)=>[wasm.hotspot_point(index,0),wasm.hotspot_point(index,1)]);
let knownHotspotCount=0;
const kothDefinition=rulesProfile.id==='up15'?wasm.koth_definition?.():undefined;
const kothMenu=bindKothMenu({victoryCondition,durationRow:$('sk-koth-victory'),duration:$('sk-koth-duration'),
  enabled:kothDefinition!==undefined&&typeof wasm.configure_match_koth==='function'&&typeof wasm.init_hill_world==='function'});
const crownVictoryMenu=bindCrownVictoryMenu({victoryCondition,countRow:$('sk-crown-count-victory'),
  count:$('sk-crown-count'),durationRow:$('sk-crown-time-victory'),duration:$('sk-crown-time'),
  enabled:rulesProfile.id==='up15'&&typeof wasm.configure_match_crown_victory==='function'&&typeof wasm.crown_victory_status==='function'});
const capitolsMenu=bindCapitolsMenu({victory:victoryCondition,durationRow:$('sk-capitols-victory'),
  duration:$('sk-capitols-duration'),enabled:rulesProfile.id==='up15'&&typeof wasm.configure_match_capitols==='function'
    &&typeof wasm.capitol_status==='function'});
const capitolArt=await loadCapitolFeedback(originalEnabled&&typeof wasm.capitol_status==='function');
function updateVictoryOptions(){
  const koth=kothMenu.update(), crowns=crownVictoryMenu.update({locked:victoryCondition.disabled});
  const hotspots=hotspotAvailable&&!victoryCondition.disabled&&victoryCondition.value==='hotspots';
  $('sk-territory-victory').hidden=victoryOptions.hidden||victoryCondition.value!=='territory';
  const hotspotRow=$('sk-hotspot-victory');
  if(hotspots&&hotspotRow.hidden)$('sk-hotspot-count').value='1';
  hotspotRow.hidden=!hotspots;
  const capitols=capitolsMenu.update({locked:victoryCondition.disabled});
  $('sk-sole-victory').hidden=victoryOptions.hidden||victoryCondition.value==='territory'||koth||crowns||hotspots||capitols===6;
  skirmishTeams.update();
}
victoryCondition.addEventListener('change',updateVictoryOptions);
updateVictoryOptions();
const kothPresentation=await loadKingOfTheHillPresentation({enabled:originalEnabled,definition:kothDefinition});
const originalAssets=await loadOriginalAssets(gl,originalEnabled);
const citizenPresentation=await loadCitizenPresentation({enabled:originalEnabled&&rulesProfile.sourceIdentityVersion===1});
const tradeAvailable=rulesProfile.ruleset===2&&supportsTradeRoute(wasm);
const airMissionAvailable=rulesProfile.ruleset===2&&supportsAirMission(wasm);
const hasTakeoffState=typeof wasm.air_takeoff_start_tick==='function';
const diversionSupported=rulesProfile.ruleset===2&&typeof wasm.configure_leader_diversion==='function';
const heavyMounted3Id=rulesProfile.ruleset===2?wasm.heavy_mounted3_definition?.():undefined;
const fishingId=rulesProfile.ruleset===2?wasm.fishing_definition?.():undefined;
const leaderPresentation=await loadCrownLeaderPresentation({
  enabled:originalEnabled&&rulesProfile.ruleset===2&&typeof wasm.configure_crown_leaders==='function',
});
const [tradePresentation,tradeRouteArt,tradeWaitArt,fighterBomberPresentation,carrierPresentation,priestPresentation,spyPresentation,airMissionArt,airMissionFlag,airTakeoff,aircraftFlightAudio,wonderPresentation,diversionPresentation,heavyMounted3Presentation,artemisPresentation,fishingPresentation]=await Promise.all([
  loadTradePresentation({enabled:originalEnabled&&rulesProfile.ruleset===2&&renderNamespace.definitionCount>82}),
  loadTradeRouteArt({enabled:originalEnabled&&tradeAvailable}),
  loadTradeWaitArt({enabled:originalEnabled&&tradeAvailable&&typeof wasm.trade_wait_active==='function'}),
  loadFighterBomberPresentation({enabled:originalEnabled&&rulesProfile.ruleset===2&&renderNamespace.definitionCount>83}),
  loadCarrierPresentation({enabled:originalEnabled&&rulesProfile.ruleset===2&&renderNamespace.definitionCount>84}),
  loadPriestPresentation({enabled:originalEnabled&&rulesProfile.ruleset===2&&renderNamespace.definitionCount>85}),
  loadSpyPresentation({enabled:originalEnabled&&rulesProfile.ruleset===2&&renderNamespace.definitionCount>86}),
  loadAirMissionArt({enabled:originalEnabled&&airMissionAvailable}),
  loadAirMissionFlag(gl,{enabled:originalEnabled&&airMissionAvailable&&typeof wasm.air_mission_visual_routes==='function'}),
  loadAirTakeoff({enabled:originalEnabled&&airMissionAvailable&&hasTakeoffState}),
  loadAirFlightAudio({enabled:originalEnabled&&airMissionAvailable&&wasm.air_audio_version?.()===1}),
  loadWonderPresentation({enabled:originalEnabled&&rulesProfile.ruleset===2&&renderNamespace.definitionCount>=102}),
  loadDiversionPresentation({enabled:originalEnabled&&diversionSupported,firstDefinition:102}),
  loadHeavyMounted3Presentation({enabled:originalEnabled,definition:heavyMounted3Id}),
  loadArtemisPresentation({enabled:originalEnabled&&rulesProfile.ruleset===2}),
  loadFishingPresentation({enabled:originalEnabled,definition:fishingId}),
]);
const hercAttack=originalEnabled&&Number.isInteger(heavyMounted3Id)&&
  ['guided_herc_attack_prepare','guided_herc_attack_ptr','guided_herc_attack_shot_at'].every(name=>typeof wasm[name]==='function')
  ?createGuidedHercAttack(wasm,heavyMounted3Id,{soundReady:sfx.airFlightEffectReady,
    play:(event,x,y,z)=>sfx.playAirFlightEffect(event,x,y,z,view)}):null;
const airFlightAudio=addCarrierFlightAudio(addCarrierFlightAudio(
  addCarrierFlightAudio(aircraftFlightAudio,wasm.air_audio_version?.()===1?carrierPresentation?.movementAudio:null),hercAttack?.audio),
  submarineMoveAudio(originalEnabled&&wasm.air_audio_version?.()===1&&typeof wasm.configure_submarine_depth==='function',
    wasm.submarine_attack_audio_version?.()===1));
const takeoffSample={tick:0,alpha:0,now:null};
const takeoffStartTick=handle=>wasm.air_takeoff_start_tick(handle)>>>0;
const takeoffStartSeconds=handle=>wasm.air_takeoff_start_seconds?.(handle)??-1;
const takeoffAudio=airTakeoff?createTakeoffAudioClock({presentation:airTakeoff,startTick:takeoffStartTick,
  startSeconds:takeoffStartSeconds,play:sfx.playTakeoff,stop:sfx.stopTakeoff}):null;
sfx.bindTakeoffEvents(airTakeoff?.events);
const mrlsPack=packScene&&packMetadata?createMrlsPackAudio({
  revisionFor:wasm.pack_visual_revision,isAlive:wasm.entity_alive,groundAt,play:sfx.playAirFlightEffect,
}):null;
const animationAudio=addCarrierFlightAudio(airFlightAudio,mrlsPack?mrlsPackAudioCatalog:null);
sfx.configureAirFlightAudio(animationAudio);
const hasGuidedImpacts=typeof wasm.guided_herc_impact_prepare==='function'&&typeof wasm.guided_herc_impact_ptr==='function';
sfx.configureGuidedImpact(originalEnabled&&hasGuidedImpacts);
const guidedImpactFX=await loadGuidedImpactEffects(gl,originalEnabled&&hasGuidedImpacts&&
  typeof wasm.guided_herc_impact_visible==='function');
const hercBeam=await loadGuidedHercBeam(gl,originalEnabled&&Number.isInteger(heavyMounted3Id)&&
  typeof wasm.guided_herc_beam_count==='function'&&typeof wasm.guided_herc_beam_ptr==='function',heavyMounted3Id);
const guidedImpactVisible=(x,y)=>wasm.guided_herc_impact_visible(x,y)===1;
const tradeWaitStatus=createTradeWaitStatus($('trade-wait-status'),tradeWaitArt);
const regionalIconOptions={enabled:originalEnabled&&rulesProfile.ruleset===2,profile:'packed7',neutralFallback:'west'};
const [cityCenterIcons,houseIcons,dockIcons,barracksIcons,millIcons,templeIcons,lightArtilleryIcons,warehouseIcons,stableIcons,kingIcons]=await Promise.all([
  loadCityCenterIcons(regionalIconOptions),loadHouseIcons(regionalIconOptions),
  loadDockIcons(regionalIconOptions),loadBarracksIcons(regionalIconOptions),
  loadMillIcons(regionalIconOptions),loadTempleIcons(regionalIconOptions),loadLightArtilleryIcons(regionalIconOptions),loadWarehouseIcons(regionalIconOptions),loadStableIcons(regionalIconOptions),loadKingIcons(regionalIconOptions),
]);
const regionalIcons = new Map([[0, cityCenterIcons], [1, houseIcons], [3, barracksIcons], [5, dockIcons], [22, lightArtilleryIcons], [48, millIcons], [49, stableIcons], [51, templeIcons], [52, warehouseIcons], [80, kingIcons]]);
const regionalPortraits = new Set([...regionalIcons].filter(([, icons]) => icons.status().enabled).map(([id]) => id));
originalAssets.citizenPresentation=citizenPresentation;
const originalLabels=await loadOriginalLabels(rulesProfile,{definitionCount:renderNamespace.definitionCount,...(rulesProfile.ruleset===2?{manifestURL:'./local-original/labels-native-civ8-regicide.json'}:{})});
const climateProfiles=await loadSkirmishClimates(originalEnabled&&rulesProfile.ruleset===2);
const capturePoseForActor = typeof wasm.capture_animation_request === 'function' ? actor => {
  const packed = wasm.capture_animation_request(actor.handle) >>> 0;
  if (!packed) return null;
  const elapsed = wasm.capture_animation_elapsed(actor.handle, actor.renderTime);
  if (!(elapsed >= 0)) return null;
  return {state:packed&3, variation:Math.fround((packed>>>2)*Math.fround(1/32767)), elapsed};
} : null;
const priestPoseForActor=priestPresentation&&typeof wasm.priest_visual_state==='function'&&
  typeof wasm.priest_visual_elapsed==='function'?actor=>{
    const state=wasm.priest_visual_state(actor.handle)>>>0;
    if(state!==1&&state!==2)return null;
    return {state,elapsed:wasm.priest_visual_elapsed(actor.handle,actor.renderTime),once:state===2,duration:state===2?4:0};
  }:null;
const priestEffectOptions={enabled:originalEnabled&&!!priestPresentation&&typeof wasm.priest_effect_count==='function'};
const [priestEffects,priestParticles,rallyHeal]=await Promise.all([
  loadPriestEffects(gl,priestEffectOptions),loadOriginalPriestParticles(gl,priestEffectOptions),
  loadOriginalRallyHeal(gl,{enabled:originalEnabled&&typeof wasm.cmd_leader_rally==='function'}),
]);
const rallyHealGate=rallyHeal?createRallyHealGate():null;
const blessGround=await loadBlessGroundGlow({enabled:originalEnabled&&typeof wasm.bless_glow_active==='function',
  crowns:typeof wasm.crown_actor_glow==='function'});
const airCrownGlow=await loadAirCrownGlow(gl,{enabled:originalEnabled&&!!blessGround&&typeof wasm.crown_actor_glow==='function'});
let previousPriestViews=[],currentPriestViews=[],priestFrameEffects=null;
const priestAttachments=new Map();
let blessPreview=null;
const spyArtEnabled=originalEnabled&&rulesProfile.ruleset===2&&typeof wasm.cmd_spy_intel_at==='function';
const [spyArt,spyAdvancedArt]=await Promise.all([
  loadSpyCommandArt({enabled:spyArtEnabled}),
  loadSpyAdvancedCommandArt({enabled:spyArtEnabled&&typeof wasm.cmd_spy_operation_at==='function'}),
]);
const spyTimer=rulesProfile.ruleset===2&&typeof wasm.spy_extraction_count==='function'?createSpyExtractionTimer(gl):null;
const spyShimmer=spyPresentation&&typeof wasm.spy_disguised==='function'?createSpyShimmerFrames():null;
const diversionShimmer=diversionPresentation?createDiversionShimmerFrames():null;
let worldRenderFrame=0;
const spyPoseForActor=spyPresentation&&typeof wasm.spy_visual_state==='function'&&
  typeof wasm.spy_visual_elapsed==='function'?actor=>{
    const state=wasm.spy_visual_state(actor.handle);
    if(!state)return null;
    return {state,elapsed:wasm.spy_visual_elapsed(actor.handle,actor.renderTime),once:true,duration:5};
  }:null;
const [wonderScaffold,wonderFence]=await Promise.all([
  loadWonderScaffold(wonderPresentation,{enabled:valleyParams.get('wonderScaffold')!=='0'}),
  loadWonderFence(wonderPresentation,{enabled:valleyParams.get('wonderFence')!=='0'}),
]);
const navalBasisForActor=typeof wasm.render_naval_basis==='function'?actor=>{
  const at=wasm.render_naval_basis(actor.handle,actor.renderYaw);
  return at?new Float32Array(wasm.memory.buffer,at,9).slice():null;
}:null;
const boatWake=await loadOriginalBoatWake(gl,{enabled:originalEnabled&&rulesProfile.ruleset===2&&
  typeof wasm.render_naval_wakes==='function'&&typeof wasm.render_naval_wakes_ptr==='function',
  fleet:typeof wasm.render_naval_wake_fleet==='function'&&typeof wasm.render_naval_wake_fleet_ptr==='function',
  additionalHulls:fishingPresentation?.wakeProfiles});
const originalModels=await loadOriginalModels(gl,originalEnabled,TEAM,(loaded,total)=>{
  $('play').textContent=`Originalgrafiken laden · ${Math.round(loaded/total*100)} %`;
},{assetMapping,climateProfiles,capturePoseForActor,tradePresentation,fighterBomberPresentation,carrierPresentation,priestPresentation,priestPoseForActor,spyPresentation,spyPoseForActor,leaderPresentation,wonderPresentation,wonderScaffold,wonderFence,diversionPresentation,heavyMounted3Presentation,kothPresentation,fishingPresentation,
  spyShimmerForActor:spyShimmer?actor=>spyShimmer.sample(actor,wasm.spy_disguised(actor.handle)===1):null,
  diversionShimmerForActor:diversionShimmer?(actor,visual)=>diversionShimmer.sample(actor,visual,wasm.leader_diversion_visual(actor.handle)===1):null,
  sourceVisualForModel:blessGround?.modelVisual,priestParticleScales:!!priestParticles,rallyTargetTags:airMissionFlag?.targetTags,
  hercAttack,hercBeam,
  navalBasisForActor,
  boatWakeBodySha:boatWake?.bodySha256,
  scoutIdle:rulesProfile.id==='up15',
  airTakeoff,takeoffElapsedForActor:airTakeoff?actor=>{
    if(takeoffSample.now===null)return takeoffElapsed(takeoffSample.tick,takeoffSample.alpha,takeoffStartTick(actor.handle));
    const start=takeoffStartSeconds(actor.handle);
    return start<0 ? -1 : Math.fround(Math.max(0,takeoffSample.now-start));
  }:null,
  packAnimations:rulesProfile.ruleset===2,dynamicSails:rulesProfile.ruleset===2&&typeof wasm.render_meta_version==='function',lightingSettings:{profile:rulesProfile.ruleset===1?'base':'native155'},...(rulesProfile.ruleset===2?{manifestURL:'./local-original/models/manifest-civ8.json',scaleManifestURL:'./local-original/model-scales-civ8.json',teamManifestURL:'./local-original/team-textures/manifest-civ8.json',regionalManifestURL:'./local-original/regional-stable/regional-models-native17.json',additionalRegionalManifestURL:'./local-original/king-regicide/manifest.json',productionRegionalManifestURL:'./local-original/production-visuals/manifest.json',stoneAttackElapsedForActor:actor=>wasm.render_stone_thrower_attack_elapsed(actor.handle,actor.renderTime),terrainFallbackManifestURL:'./local-original/regional-e15-seasons/terrain-fallback.json',houseLodManifestURL:'./local-original/regional-house-lod/manifest.json',houseLodProfile:{mode:'resource',modelDetail:2,rendererCap0x200:false},citizenManifestURL:'./local-original/citizens9/manifest.json',citizenNeutralFallback:'west',sourceIdentityForActor:actor=>wasm.actor_source_identity(actor.handle)>>>0}:{})});
const carrierGarrisonFlag=airMissionAvailable&&originalModels.available.has(renderNamespace.unitRenderId(84))
  ?await loadCarrierGarrisonFlag(gl,{source:carrierPresentation?.garrisonFlag,navalBasisForActor:originalModels.carrierHullBasis}):null;
const wakeEnabled=boatWake&&(boatWake.fleet?originalModels.hasBoatWakeSources:originalModels.boatWakeHull);
const carrierOccupied=handle=>wasm.air_mission_status(handle,1)>0;
const sourceSails=await loadSourceSailRuntime({wasm,namespace:renderNamespace,models:originalModels,enabled:originalEnabled&&rulesProfile.ruleset===2,decorateMetadata:metadata=>packMetadata?packMetadata.snapshot(metadata):metadata});
if(sourceSails?.metrics.error)console.warn('Source sails:',sourceSails.metrics.error);
const unitPresentations=[tradePresentation,fighterBomberPresentation,carrierPresentation,priestPresentation,spyPresentation,leaderPresentation,diversionPresentation,heavyMounted3Presentation,fishingPresentation].filter(Boolean);
sfx.configureOriginalDefinitions({wasm,bindings:[...(originalModels.manifest?.models||[]),...(originalModels.additionalRegionalAudio||[]),...unitPresentations.flatMap(source=>source.audioModelRows)],additionalFamilyManifest:unitPresentations.map(source=>source.audioManifest),citizenPresentation,regionalMusic:rulesProfile.ruleset===2,...(rulesProfile.ruleset===2?{familyManifestURL:'native-families-civ8-regicide.json',playbackManifestURL:'manifest-stone-thrower-up.json'}:{})});
const captureGlow = originalEnabled && typeof wasm.active_capture_glow === 'function'
  ? createCaptureGlow({kothDefinition:kothPresentation?kothDefinition:undefined}) : null;
const originalGround=await loadOriginalGround(gl,originalEnabled,{assetMapping,climateProfiles,captureGlow:!!captureGlow,blessGlow:!!blessGround,wonderPresentation,
  extractionDecal:spyArt?.extractionVisuals.decal.url,mixedGlow:spyAdvancedArt?.mixedGlow,hotspotMarker:hotspotAvailable,...(rulesProfile.ruleset===2
  ?{manifestURL:'./local-original/regional-temple/ground-decals.json'}:{})});
const originalTerrain=await loadOriginalTerrain(gl,originalEnabled,TEAM,{climateProfiles});
originalTerrain.setLightingContext?.(originalModels.lighting);
const originalWater=await loadOriginalWater(gl,originalEnabled);
const climates=supportedSkirmishClimates(climateProfiles,originalTerrain,originalGround,originalModels);
const seasonSession=createSeasonSession(wasm,originalTerrain,originalGround,originalModels,climates);
const climateControl=$('map-climate');
for(const option of climateControl.options)option.hidden=option.disabled=!climates.includes(+option.value);
$('map-climate-row').hidden=climates.length<2;
try{const previous=localStorage.getItem('openepoch-map-climate');if(previous!==null&&climates.includes(+previous))climateControl.value=previous;}catch{/* optional preference */}
climateControl.onchange=()=>{try{localStorage.setItem('openepoch-map-climate',climateControl.value);}catch{/* optional preference */}};
const menuClimate=()=>checkedClimate(+climateControl.value,climates);
const originalReflection=createOriginalReflection(gl);
let reflectionError=null;
await loadOriginalEpochIcons(originalAssets,originalModels.manifest);
if(qaMode){
  canvas.dataset.originalGroundBindings=String(originalGround.entries.size);
  canvas.dataset.originalGroundErrors=JSON.stringify(originalGround.errors);
  canvas.dataset.originalGroundPolicy=originalGround.policy;
  canvas.dataset.originalModelBindings=String(originalModels.manifest?.models?.length||0);
  canvas.dataset.wonderScaffoldBindings=String(originalModels.wonderScaffoldModels??0);
  canvas.dataset.wonderFenceBindings=String(originalModels.wonderFenceModels??0);
  canvas.dataset.originalEpochIcons=String([...originalAssets.epochIcons.values()].reduce((n,list)=>n+list.filter(row=>row.nativeExact).length,0));
  canvas.dataset.originalTerrainPolicy=originalTerrain.policy;
  canvas.dataset.originalTerrainErrors=JSON.stringify(originalTerrain.errors);
  canvas.dataset.originalWaterErrors=JSON.stringify(originalWater.errors);
}
applyOriginalInterface(originalAssets);
const economyBar=createEconomyBar(originalAssets,referenceMode);
const nativeHud=await loadNativeHud(originalAssets,referenceMode);
for(const pr of [terrain,water]){
  gl.useProgram(pr.p);gl.uniform1i(pr.u.uOriginalTerrain,4);gl.uniform1i(pr.u.uOriginal,originalAssets.enabled?1:0);
}
if(originalAssets.enabled){
  $('rules-note').textContent='Lokaler Nachbau mit mitgelieferten Originalgrafiken und Originalklängen. 15 Epochen, neun Rohstoffe, Forschung, Diplomatie und Festungen. Wähle deine Kartenaufdeckung für das neue Scharmützel.';
}
if(originalEnabled&&(!originalAssets.enabled||originalModels.available.size<67)){
  $('rules-note').textContent='Einige Originalmedien fehlen. Entpacke das vollständige Spielpaket und lasse den Spielordner zusammen. Bis dahin verwendet das Spiel die vorhandenen Ersatzgrafiken.';
}
const valleyPreview=await loadValleyPreview({requested:valleyRequested&&originalEnabled,
  authoredRequested:valleyMode==='archive-qa'||valleyNeutralRequested,neutralRequested:valleyNeutralRequested,
  requireAuthored:valleyNormal,manifest:valleyManifest,wasm,gl,teams:TEAM,lighting:originalModels.lighting,
  onUnavailable:error=>console.warn('Valley-Vorschau:',error.message)});
$('valley-preview-menu').hidden=!valleyPreview;
const valleyMapChoice=$('valley-map-choice');
const valleyArchiveOption=$('valley-archive-qa-option');
valleyArchiveOption.disabled=valleyArchiveOption.hidden=!valleyPreview?.authoredAvailable;
if(valleyNormal&&valleyPreview?.authoredAvailable){
  const earlier=$('valley-preview-map-option');
  earlier.disabled=earlier.hidden=true;
  valleyArchiveOption.textContent='Tal des Todes';
  $('valley-archive-qa-note').textContent='Originalkarte mit festem Start in der Steinzeit und normalen Startvorräten. Wähle mit Umschalt+A den Angriff und klicke mit Alt+Linksklick auf einen sichtbaren gegnerischen Bürger.';
}else if(valleyNeutralRequested&&valleyPreview?.authoredAvailable){
  valleyArchiveOption.textContent='Valley · Neutraler Archivstart (Test)';
  $('valley-archive-qa-note').textContent += ' Wähle im Test mit Umschalt+A den Angriff. Klicke mit Alt+Linksklick auf den sichtbaren gegnerischen Bürger.';
}
const valleyFixedControls=new Map();
let valleyPreviousKothDuration=null,valleyPreviousCrown,valleyPreviousCapitols;
const valleyPresetValues={nai:'1','pop-limit':'250',supplies:'0','start-epoch':'0','end-epoch':String(wasm.epoch_count()-1),
  'map-climate':'0','season-start':'6','season-length':'5','initial-ceasefire':'0','sk-victory-condition':'conquest',
  'civilization-0':'4','civilization-1':'8'};
const valleyPresetUnchecked=new Set(['season-cycle','disable-crowns','source-solo-clock','source-adaptive-clock','sk-sole-survivor']);
valleyMapChoice.onchange=()=>{
  const selected=valleyMapChoice.value!=='own';
  $('valley-fixed-settings').hidden=!selected;
  $('valley-archive-qa-note').hidden=valleyMapChoice.value!=='valley-archive-qa';
  if(selected&&!valleyFixedControls.size){
    valleyPreviousKothDuration=kothMenu.readMinutes();
    valleyPreviousCrown=crownVictoryMenu.read();
    valleyPreviousCapitols=capitolsMenu.read();
    const ids=['seed','nai','pop-limit','supplies','start-epoch','end-epoch','map-climate','season-cycle','season-start',
      'season-length','sk-victory-condition','sk-sole-survivor','sk-territory-percent',
      'disable-crowns','initial-ceasefire','source-solo-clock','source-adaptive-clock','mpopen'];
    for(const control of [...ids.map($),...document.querySelectorAll('#civilization-options select')].filter(Boolean)){
      valleyFixedControls.set(control,{disabled:control.disabled,value:control.value,checked:control.checked});
      if(valleyNormal){
        if(Object.hasOwn(valleyPresetValues,control.id))control.value=valleyPresetValues[control.id];
        if(valleyPresetUnchecked.has(control.id))control.checked=false;
      }
      control.disabled=true;
    }
  }else if(!selected){
    for(const [control,previous] of valleyFixedControls){
      control.disabled=previous.disabled;
      if(valleyNormal){
        if(Object.hasOwn(valleyPresetValues,control.id))control.value=previous.value;
        if(valleyPresetUnchecked.has(control.id))control.checked=previous.checked;
      }
    }
    valleyFixedControls.clear();
  }
  updateVictoryOptions();
  if(!selected&&valleyPreviousKothDuration!==null){
    $('sk-koth-duration').value=String(valleyPreviousKothDuration);
    valleyPreviousKothDuration=null;
  }
  if(!selected&&valleyPreviousCrown){
    const field=valleyPreviousCrown.mode===CROWN_VICTORY_COUNT?'sk-crown-count':'sk-crown-time';
    $(field).value=String(valleyPreviousCrown.target);
    valleyPreviousCrown=undefined;
  }
  if(!selected&&valleyPreviousCapitols){
    $('sk-capitols-duration').value=String(valleyPreviousCapitols.minutes);
    valleyPreviousCapitols=undefined;
  }
  if(valleyNormal&&window.openepoch)void refreshContinue();
};
$('play').disabled=false;$('play').textContent='Scharmützel starten';

// ---------------------------------------------------------------- state
const cam = { x: 128, y: 128, zoom: 28, heading:0 };
const mouse = { x: 0, y: 0, down: false, sx: 0, sy: 0, drag: false, pan: false, inside: false };
const keys = new Set();
let running = false, paused = false, speed = 1, acc = 0, last = 0, bench = 0;
const pauseReasons = new Set();
let gameOptions = null;
let worldMaterialPending = null;
let initialMusicEpoch = 0, musicCivilization = 0;
let net = null;          // { ls: Lockstep, ch: RTCDataChannel } in multiplayer
let valleyWorld = null;
let ghost = null;        // { id, size, name, turn } while placing a building
let placementKey='',lastWheelRotation=0;
let attackMode = false;  // A pressed: next left click is attack-move
let lastWorkSound = 0;
let ui = null, actSig = '', lastGroup = { n: -1, t: 0 }, lastClick = 0;
let simMs = 0, frameMs = 16.7, worstMs = 0, fogTick = -1, lastUi = 0, lastMini = 0, lastAuto = 0, waitSince = 0;
const SPEEDS = [0.5, 1, 2, 4];
let originalTempo = null;
let pendingTempo = 0;
let restartPoint = null;
const hasOriginalTempo = rulesProfile.id==='up15';
if(hasOriginalTempo)sfx.initializeOriginalAudioSettings();
const supportsSourceSoloClock = hasOriginalTempo && typeof wasm.tick_with_source_delta === 'function';
const supportsAdaptiveSoloClock = supportsSourceSoloClock && typeof wasm.tick_with_adaptive_source_slot === 'function';
let sourceSoloClock = false;
let adaptiveSoloClock = null;
const adaptiveRenderSample = {remainder:0,count:0,fps:0};
$('original-tempo-options').hidden = !hasOriginalTempo;
const hasNetworkResignation=hasOriginalTempo&&typeof wasm.configure_network_resignation==='function';
$('mp-resignation-options').hidden=!hasNetworkResignation;
const supportsSourceNetworkClock=hasNetworkResignation&&typeof wasm.tick_with_network_source_slot==='function';
$('mp-source-rounds-option').hidden=!supportsSourceNetworkClock;
const hasInitialCeasefire=hasOriginalTempo&&typeof wasm.configure_initial_ceasefire==='function';
$('initial-ceasefire-option').hidden=!hasInitialCeasefire;
const ceasefireStatus=$('initial-ceasefire-status');
function updateInitialCeasefire() {
  if(!hasInitialCeasefire)return;
  const seconds=Math.round(wasm.initial_ceasefire_seconds());
  ceasefireStatus.hidden=seconds<=0;
  if(seconds<=0)return;
  const hours=Math.floor(seconds/3600);
  const minutes=Math.floor(seconds/60)%60;
  const clock=`${hours?hours+':':''}${minutes}:${String(seconds%60).padStart(2,'0')}`;
  const text=`Anf. Waffenst. - ${clock}`;
  if(ceasefireStatus.textContent!==text)ceasefireStatus.textContent=text;
}
$('source-solo-clock-option').hidden = !supportsSourceSoloClock;
$('source-adaptive-clock-option').hidden = !supportsAdaptiveSoloClock;
$('source-solo-clock').onchange = () => { $('source-adaptive-clock').disabled = !$('source-solo-clock').checked; };
function menuTempo() {
  return checkOriginalTempo({preset: +$('game-speed-preset').value, locked: $('game-speed-locked').checked});
}

function startingTempo(tempo) {
  return {...tempo, requested: originalStartingSpeed(tempo)};
}

function showSpeed() {
  $('speed').textContent = originalTempo ? `${speed.toLocaleString('de-DE',{maximumFractionDigits:2})}×` : `${speed}×`;
  $('speed').disabled = originalTempo?.locked === true || (net!==null&&(net.ls.local!==0||(net.sourceRounds&&net.ls.alone)));
}
const fullmapPip = createFullmapPipBridge({gl,canvas,available:()=>referenceMode&&originalTerrain.enabled&&running&&!bench,
  mainPose:()=>cam, setMainFocus(x,y){cam.x=x;cam.y=y;updateCamera();}, heightAt:groundAt,
  resolveUnit:handle=>wasm.entity_alive?.(handle)?{x:wasm.ent_x(handle),y:wasm.ent_y(handle)}:null,
  onOpen(){keys.clear();cancelMapGesture();cancelBuilding();attackMode=false;},
  onClose(){keys.clear();cancelMapGesture();}});

// ---------------------------------------------------------------- camera
// At heading0 the eye sits south of and above the look-at point.
const view = { eye: [0, 0, 0], f: [0, -Math.sin(PITCH), -Math.cos(PITCH)], u: [0, Math.cos(PITCH), -Math.sin(PITCH)], vp: new Float32Array(16) };
function updateCamera() {
  cam.x = Math.min(valleyWorld?.width??MAP, Math.max(0, cam.x));
  cam.y = Math.min(valleyWorld?.height??MAP, Math.max(0, cam.y));
  if(referenceMode){
    if(valleyWorld)updateAuthoredCamera(updateOriginalCamera,cam,view,innerWidth,innerHeight,groundAt);
    else updateOriginalCamera(cam,view,innerWidth,innerHeight,groundAt);
    if(running)wasm.set_eye(...view.eye);
    return;
  }
  cam.zoom = Math.min(80, Math.max(6, cam.zoom));
  const aspect = innerWidth / innerHeight, dist = innerHeight / 2 / (cam.zoom * TANF);
  const e = view.eye = [cam.x, dist * Math.sin(PITCH), cam.y + dist * Math.cos(PITCH)];
  const f = view.f, u = view.u, near = Math.max(0.5, dist * 0.08), far = dist * 3.5 + 120, t = 1 / TANF;
  const tx = -e[0], ty = -(u[1] * e[1] + u[2] * e[2]), tz = f[1] * e[1] + f[2] * e[2];
  // projection * view, written out (right = +x, so the view matrix is sparse)
  const A = (far + near) / (near - far), B = 2 * far * near / (near - far), m = view.vp;
  m[0] = t / aspect; m[1] = 0; m[2] = 0; m[3] = 0;
  m[4] = 0; m[5] = t * u[1]; m[6] = -A * f[1]; m[7] = f[1];
  m[8] = 0; m[9] = t * u[2]; m[10] = -A * f[2]; m[11] = f[2];
  m[12] = t / aspect * tx; m[13] = t * ty; m[14] = A * tz + B; m[15] = -tz;
  if (running) wasm.set_eye(e[0], e[1], e[2]);   // so clicks land on what is drawn, even on aircraft
}
function panCameraScreen(right,down){
  const r=referenceMode?(view.groundRight||[1,0]):[1,0],f=referenceMode?(view.groundForward||[0,-1]):[0,-1];
  cam.x+=r[0]*right-f[0]*down;cam.y+=r[1]*right-f[1]*down;
}
function rotateView(delta){
  if(!referenceMode||!running)return;
  rotateOriginalCameraHeading(cam,delta);updateCamera();mouse.inside=false;
}
function ray(px, py) {
  if(referenceMode)return originalCameraRay(view,px,py,innerWidth,innerHeight);
  const nx = (px / innerWidth * 2 - 1) * TANF * innerWidth / innerHeight, ny = (1 - py / innerHeight * 2) * TANF;
  return [nx, view.f[1] + view.u[1] * ny, view.f[2] + view.u[2] * ny];
}
/** Screen pixel -> point on the y = 0 plane. */
function toPlane(px, py) {
  const d = ray(px, py), e = view.eye, t = -e[1] / Math.min(d[1], -1e-4);
  return [e[0] + d[0] * t, e[2] + d[2] * t];
}
/** Screen pixel -> point on the actual terrain (ray-marched against the height map). */
function toWorld(px, py) {
  if(valleyWorld){
    const point=pickValleyGround(valleyWorld,view,px,py,innerWidth,innerHeight);
    return point?[point.x,point.y]:null;
  }
  const d = ray(px, py), e = view.eye, dy = Math.min(d[1], -1e-4);
  let t = Math.max(0,(3.2 - e[1]) / dy), prev = t;
  const end = -e[1] / dy;
  for (; t < end; prev = t, t += 0.2) if (e[1] + dy * t <= groundAt(e[0] + d[0] * t, e[2] + d[2] * t)) break;
  t = Math.min(t, end);
  for (let i = 0; i < 6; i++) { const m = (prev + t) / 2; if (e[1] + dy * m <= groundAt(e[0] + d[0] * m, e[2] + d[2] * m)) t = m; else prev = m; }
  return [e[0] + d[0] * t, e[2] + d[2] * t];
}
const corners = () => [toPlane(0, 0), toPlane(innerWidth, 0), toPlane(innerWidth, innerHeight), toPlane(0, innerHeight)];
function viewRect() {
  const c = corners();
  return [Math.min(...c.map(p => p[0])) - 2, Math.min(...c.map(p => p[1])) - 2, Math.max(...c.map(p => p[0])) + 2, Math.max(...c.map(p => p[1])) + 6];
}

function toast(msg) { const t = $('toast'); t.textContent = msg; t.style.opacity = 1; clearTimeout(toast.h); toast.h = setTimeout(() => t.style.opacity = 0, 2200); }
const buildingControls=createBuildingControls({rotate:rotateBuilding,cancel:cancelBuilding,automatic(){if(ghost){ghost.manual=false;placementKey='';updatePlacement();}}});
const farmAngleMemory=createFarmAngleMemory();
function cancelBuilding(){farmAngleMemory.close();ghost=null;placementKey='';buildingControls.hide();delete canvas.dataset.placement;canvas.style.cursor='default';}

function farmCursorPose(overMap=mouse.inside&&document.elementFromPoint(mouse.x,mouse.y)===canvas,now=performance.now()){
  if(!overMap)return null;
  const raw=toWorld(mouse.x,mouse.y);
  if(!raw)return null;
  const x=Math.fround(raw[0]),y=Math.fround(raw[1]);
  if(x!==ghost.pick?.[0]||y!==ghost.pick?.[1]){
    ghost.pick=[x,y];
    wasm.valley_farm_cursor_pointer(x,y);
  }
  const pose=[0,1,2,3].map(field=>wasm.valley_farm_cursor_component(field));
  if(wasm.valley_farm_cursor_validation_revision)
    farmAngleMemory.observe(pose[3],wasm.valley_farm_cursor_validation_revision(),wasm.sim_elapsed_seconds(),now);
  return pose;
}

function rotateBuilding(direction,fine=false){
  if(!ghost)return;
  if(ghost.nativeCursor){
    farmCursorPose();
    wasm.valley_farm_cursor_rotate(direction,fine?3:1);
    placementKey='';updatePlacement();return;
  }
  if(!wasm.cmd_place_rotated)return toast('Lade die aktuelle Spielfassung neu, um Gebäude zu drehen.');
  const isWonder=Boolean(wasm.wonder_footprint?.(ghost.id,0));
  ghost.turn=nextBuildingTurn(ghost.turn,direction,fine&&!isWonder);ghost.manual=true;placementKey='';updatePlacement();
}
function footprintSize(){return ghost?(wasm.footprint_size?.(ghost.id,ghost.turn)||ghost.size):0;}
function footprintAxes(){
  const packed=ghost&&wasm.wonder_footprint?.(ghost.id,ghost.turn);
  if(packed)return [packed&0xffff,packed>>>16];
  const size=footprintSize();
  return [size,size];
}
function ghostTile() {
  const point=toWorld(mouse.x,mouse.y);
  if(!point)return null;
  const [wx,wy]=point;
  if(ghost.id===5&&!ghost.manual&&wasm.dock_orientation){
    const size=wasm.footprint_size(ghost.id,0)||ghost.size;ghost.turn=0;
    const tx=Math.round(wx-size/2),ty=Math.round(wy-size/2),turn=wasm.dock_orientation(tx,ty);
    if(turn<8)ghost.turn=turn;
  }
  const [width,height]=footprintAxes();
  if(referenceMode&&(ghost.id===53||ghost.id===54)){
    const direction=ray(mouse.x,mouse.y),distance=(groundAt(wx,wy)-view.eye[1])/direction[1];
    const resource=originalModels.pickResource(view.eye,direction,distance+.02);
    if(resource)return [Math.floor(resource[0])-Math.floor(width/2),Math.floor(resource[1])-Math.floor(height/2)];
  }
  return [Math.round(wx - width / 2), Math.round(wy - height / 2)];
}
function updatePlacement(now=performance.now(),farmPose){
  if(!ghost){buildingControls.hide();placementKey='';delete canvas.dataset.placement;return;}
  if(ghost.nativeCursor){
    const pose=farmPose===undefined?farmCursorPose():farmPose;
    const size=footprintSize();
    if(!pose){
      placementKey='';delete canvas.dataset.placement;
      const yaw=wasm.valley_farm_cursor_component(3);
      buildingControls.update({name:ghost.name,id:ghost.id,yaw:Number.isFinite(yaw)?yaw:FARM_INITIAL_YAW,error:1,size});
      return;
    }
    const [x,y,z,yaw]=pose,key=['native',ghost.id,x,y,z,yaw,ui?.sel?.n,ui?.sel?.def,Math.floor(now/250)].join(':');
    if(key===placementKey)return;placementKey=key;
    const error=wasm.valley_farm_cursor_error();
    const time=wasm.valley_farm_cursor_time_multiplier?.();
    const note=!error&&time===2?'Neutrales Land: doppelte Bauzeit':!error&&time===3?'Fremdes Gebiet: dreifache Bauzeit':'';
    buildingControls.update({name:ghost.name,id:ghost.id,yaw,error,size,note});
    if(qaMode)canvas.dataset.placement=JSON.stringify({id:ghost.id,nativeFarm:ghost.id===2,nativeHouse:ghost.id===1,pick:ghost.pick,x,y,z,yaw,error,size});
    return;
  }
  const tile=ghostTile();
  if(!tile){
    placementKey='';delete canvas.dataset.placement;
    buildingControls.update({name:ghost.name,id:ghost.id,turn:ghost.turn,error:1,size:footprintSize(),manual:ghost.manual,note:''});
    return;
  }
  const [tx,ty]=tile,key=[ghost.id,ghost.turn,tx,ty,ui?.sel?.n,ui?.sel?.def,Math.floor(now/250)].join(':');
  if(key===placementKey)return;placementKey=key;
  const error=wasm.place_error_rotated?wasm.place_error_rotated(ghost.id,tx,ty,ghost.turn):wasm.place_error(ghost.id,tx,ty);
  const [width,height]=footprintAxes();
  const area=wasm.territory_at(tx+width/2,ty+height/2),owner=wasm.territory_owner(area);
  const notes=[];
  if(error===0&&ghost.id!==0&&owner===255)notes.push('Neutrales Land: doppelte Bauzeit');
  if(error===0&&foreignConstructionVersion>0&&
    wasm.placement_time_multiplier(ghost.id,tx,ty,ghost.turn)===3)notes.push('Fremdes Gebiet: dreifache Bauzeit');
  if(referenceMode&&error===0&&(ghost.id===0||ghost.id===56)&&owner===255){
    const info=ui?.diplomacy?.territories?.find(territory=>territory.id===area);
    if(info)notes.push(territoryClaimMessage(info,{building:true}));
  }
  const note=notes.join(' · ');
  buildingControls.update({name:ghost.name,id:ghost.id,turn:ghost.turn,error,width,height,manual:ghost.manual,note});
  if(qaMode)canvas.dataset.placement=JSON.stringify({id:ghost.id,turn:ghost.turn,tx,ty,error,size:footprintSize(),width,height});
}

const sourceResearch=originalResearchActions(wasm);
const sciencePanel=researchPanel({wasm,sourceResearch,
 onOrder(){sfx.play(20);lastUi=0;},
 onOpen(){setPauseHold('research',true);keys.clear();mouse.inside=false;},
 onClose(){setPauseHold('research',false);lastUi=0;}
});
$('research-button').style.display=referenceMode?'':'none';
$('research-button').onclick=()=>{if(running&&ui)sciencePanel.open(ui);};
const diplomacy=diplomacyPanel({wasm,referenceMode,
 onOrder(){sfx.play(20);lastUi=0;},
 onOpen(){keys.clear();cancelMapGesture();ghost=null;attackMode=false;$('tip').style.display='none';},
 onClose(){keys.clear();mouse.inside=false;lastUi=0;},
 isPaused:()=>paused&&!net,
 resume(){if(paused&&!net)togglePause();},
 onTerritoryFocus(territory){cam.x=territory.x;cam.y=territory.y;updateCamera();toast(territory.name);}
});
const nativeDiplomacy=await enhanceOriginalDiplomacy(diplomacy,{wasm,enabled:referenceMode&&originalAssets.enabled});
document.body.dataset.originalDiplomacy=String(nativeDiplomacy.enabled);
const nativeBitmapFonts=await loadNativeFontUI({enabled:referenceMode&&originalAssets.enabled,hud:nativeHud,diplomacy:nativeDiplomacy});
document.body.dataset.originalBitmapFonts=String(nativeBitmapFonts.enabled);
const [territoryNameCatalog,cityNameCatalog] = rulesProfile.id === 'up15'
  ? await Promise.all([loadFullmapTerritoryNameCatalog(wasm),loadFullmapCityNameCatalog(wasm)]) : [];
const nameSettings = new URLSearchParams(location.search);
$('diplomacy-button').onclick=()=>{if(running&&!bench)diplomacy.open();};

const playerFlare = createPlayerFlare({mapSize:MAP,tickSeconds:TICK,originalArt:originalEnabled,local:()=>ui?.local,teams:TEAM,
  onMode(active){
    $('player-flare-button').setAttribute('aria-pressed',String(active));
    canvas.style.cursor=active?'crosshair':'default';
  }});
const battleFlare = typeof wasm.battle_signals_enable === 'function' ? createBattleFlare({wasm,originalArt:originalEnabled}) : null;
const canTargetOrder=()=>running&&!bench&&!ui?.win;
const militaryCaptureCount=()=>wasm.capture_selection_count_for?.(1)??wasm.capture_selection_count?.()??0;
const captureOrder = typeof wasm.cmd_capture_at === 'function' ? createCaptureOrder({canvas,originalArt:originalEnabled,
  canOrder:()=>canTargetOrder()&&militaryCaptureCount()>0,
  pickTarget:(x,y)=>wasm.capture_target_at_for?.(x,y,1)??wasm.capture_target_at(x,y),issue:wasm.cmd_capture_at}) : null;
const priestEnabled=rulesProfile.ruleset===2&&typeof wasm.configure_priest_skirmish==='function';
const crownSupported=rulesProfile.ruleset===2&&typeof wasm.configure_crowns==='function';
const supportsPeerCrowns=crownSupported&&typeof wasm.configure_crowns_peer==='function';
const supportsNetworkCrownVictory=supportsPeerCrowns&&typeof wasm.configure_network_crown_victory==='function';
const supportsNetworkCapitols=rulesProfile.id==='up15'&&typeof wasm.configure_network_capitols==='function';
const supportsNetworkCapitolRoster=supportsNetworkCapitols&&typeof wasm.configure_network_capitols_roster==='function';
const supportsCaptureCheckpoint=rulesProfile.id==='up15'&&typeof wasm.configure_network_capture_checkpoint==='function';
const crownArt=await loadCrownArt(crownSupported&&originalEnabled);
const crownVictoryRows=mountCrownVictoryRows($('crown-victory-rows'),crownArt);
const crownStatusArt=await loadCrownStatusArt({enabled:crownSupported&&originalEnabled});
const crownProgress=crownSupported?mountCrownProgress({button:$('crown-progress-toggle'),panel:$('crown-progress'),
  ledger:$('crown-ledger'),timers:$('crown-timers'),chooser:$('crown-choice'),wasm,art:crownArt}):null;
$('disable-crowns-option').hidden=!crownSupported;
const spyEnabled=priestEnabled&&typeof wasm.cmd_spy_intel_at==='function';
sfx.configurePriestCues(originalEnabled&&priestEnabled);
const priestArt=await loadPriestCommandArt({enabled:originalEnabled&&priestEnabled});
function targetedPower(definition, command, art, cssClass) {
  return {...command,definition,art,cssClass,order:createCaptureOrder({canvas,originalArt:false,fallback:command.fallback,
    shortcut:command.shortcut??`Alt+${command.key}`,canOrder:()=>canTargetOrder()&&command.count()>0,pickTarget:command.pick,issue:command.issue,
    appearance:art?{button:(theme,state)=>art.button(command.action,theme,state),cursor:valid=>art.cursor(command.action,valid)}:null,
  })};
}
const priestOrders=priestEnabled?[
  {action:'convert',key:'C',fallback:'↪',label:'Bekehren',description:'Bekehrt eine gegnerische Einheit.',voice:'attack',
    count:()=>wasm.capture_selection_count_for(2),pick:(x,y)=>wasm.capture_target_at_for(x,y,2),issue:wasm.cmd_capture_at},
  {action:'bless',key:'L',fallback:'✧',label:'Segnen',description:'Segnet verbündete Landeinheiten im Umkreis.',voice:'order',
    count:wasm.bless_selection_count,pick:wasm.bless_target_at,issue:wasm.cmd_bless_at},
].map(command=>targetedPower(85,command,priestArt,'priest-action')):[];
const spyOrders=spyEnabled?[targetedPower(86,{action:'gatherIntel',key:'N',fallback:'◈',label:'Informationen sammeln',
  description:'Deckt für kurze Zeit gleichartige Gebäude beim Zielspieler auf.',
  count:wasm.spy_selection_count,pick:wasm.spy_intel_target_at,issue:wasm.cmd_spy_intel_at,
},spyArt,'spy-action')]:[];
if(spyEnabled&&typeof wasm.spy_operation_unlocked==='function'){
  for(const [operation,action,key,fallback,label,description] of [
    [1,'sabotage','I','✖','Sabotage','Legt das gewählte gegnerische Gebäude vorübergehend still.'],
    [2,'restore','R','✚','Wiederherstellen','Behebt die Sabotage an einem eigenen oder verbündeten Gebäude.'],
    [3,'contaminate','J','☣','Kontaminieren','Vergiftet ein gegnerisches Stadtzentrum oder Lagerhaus.'],
    [4,'cleanup','P','✚','Dekontaminieren','Reinigt ein eigenes oder verbündetes Stadtzentrum oder Lagerhaus.'],
  ]){
    spyOrders.push(targetedPower(86,{action,key,fallback,label,description,
      available:()=>wasm.spy_operation_unlocked(operation)>0,
      count:()=>wasm.spy_operation_selection_count(operation),
      pick:(x,y)=>wasm.spy_operation_target_at(operation,x,y),
      issue:(x,y)=>wasm.cmd_spy_operation_at(operation,x,y),
    },spyAdvancedArt,'spy-action'));
  }
}
const imperialSupported=typeof wasm.cmd_imperial_construction_at==='function';
const mobilizationSupported=typeof wasm.cmd_leader_mobilization_at==='function';
const [imperialCursors,mobilizationCursors]=await Promise.all([
  loadLeaderCursors(originalEnabled&&imperialSupported,'imperial-construction'),
  loadLeaderCursors(originalEnabled&&mobilizationSupported,'mobilization'),
]);
const imperialSourceAction=theme=>leaderPresentation?.power(89,ui?.epoch??0,theme);
const imperialActionName=()=>ui?.epoch<5?'Endeavor':'FastTracking';
const imperialArt={
  label:()=>imperialSourceAction(1)?.label??(imperialActionName()==='Endeavor'?'Bestrebungen':'Schnelles Aufspüren'),
  description:()=>imperialSourceAction(1)?.help??(imperialActionName()==='Endeavor'
    ?'Verringert die Bauzeit für das Fundament jedes angewählten Stadtzentrums oder Wunders.'
    :'Diese Fertigkeit vollendet sofort den Bau aller Gebäude mit Ausnahme von Wundern und Stadtzentren.'),
  button:(_action,theme,state)=>imperialSourceAction(theme)?.art?.[state],
  cursor:(_action,valid)=>imperialCursors?.(imperialActionName(),valid),
};
const imperialOrder=imperialSupported?targetedPower(89,{
  action:'imperial-construction',shortcut:';',fallback:'⌂',
  available:()=>ui?.epoch<10&&wasm.leader_power_reserve(wasm.selected_actor_handle()>>>0)>=0,
  count:wasm.imperial_construction_selection_count,pick:wasm.imperial_construction_target_at,issue:wasm.cmd_imperial_construction_at,
},imperialArt,'imperial-action'):null;
const mobilizationSource=theme=>leaderPresentation?.power(88,ui?.epoch??0,theme);
const mobilizationArt={
  label:()=>mobilizationSource(1)?.label??'Mobilisierung',
  description:()=>mobilizationSource(1)?.help??'Beschleunigt die Ausbildung in einem eigenen Produktionsgebäude.',
  button:(_action,theme,state)=>mobilizationSource(theme)?.art?.[state],
  cursor:(_action,valid)=>mobilizationCursors?.('Mobilization',valid),
};
const mobilizationOrder=mobilizationSupported?targetedPower(88,{
  action:'mobilization',shortcut:';',fallback:'⚙',
  available:()=>ui?.epoch<5&&wasm.leader_power_reserve(wasm.selected_actor_handle()>>>0)>=0,
  count:wasm.leader_mobilization_selection_count,pick:wasm.leader_mobilization_target_at,issue:wasm.cmd_leader_mobilization_at,
},mobilizationArt,'economic-action'):null;
const purchaseSource=theme=>leaderPresentation?.power(88,ui?.epoch??5,theme);
const purchaseSupported=typeof wasm.cmd_leader_purchase_at==='function';
const imperialismSupported=typeof wasm.cmd_imperialism_at==='function';
const [lateLeaderCursors,imperialismBadge]=await Promise.all([
  loadLeaderCursors(originalEnabled&&(purchaseSupported||imperialismSupported),'late-leader-powers'),
  loadImperialismBadge(originalEnabled&&imperialismSupported),
]);
const purchaseArt={
  label:()=>purchaseSource(1)?.label??(ui?.epoch<10?'Auslandsinvestition':'Kapitalismus'),
  description:()=>purchaseSource(1)?.help??(ui?.epoch<10
    ?'Kauft ein zulässiges fremdes Gebäude für den doppelten Preis.'
    :'Kauft eine fremde Landeinheit für den doppelten Preis in Gold.'),
  button:(_action,theme,state)=>{
    const art=purchaseSource(theme)?.art;
    return art?.[state]??art?.idle;
  },
  cursor:(_action,valid)=>lateLeaderCursors?.(ui?.epoch<10?'ForeignInvestment':'Capitalism',valid),
};
const purchaseOrder=purchaseSupported?targetedPower(88,{
  action:'purchase',shortcut:';',fallback:'◈',
  available:()=>ui?.epoch>=5&&!net&&wasm.leader_power_reserve(wasm.selected_actor_handle()>>>0)>=0,
  count:wasm.leader_purchase_selection_count,pick:wasm.leader_purchase_target_at,issue:wasm.cmd_leader_purchase_at,
},purchaseArt,'economic-action'):null;
const imperialismArt={
  label:()=>imperialSourceAction(3)?.label??'Imperialismus',
  description:()=>imperialSourceAction(3)?.help??'Schützt ein eigenes Gebäude 45 Sekunden vor Angriffen und Eroberung.',
  button:(_action,theme,state)=>imperialSourceAction(theme)?.art?.[state],
  cursor:(_action,valid)=>lateLeaderCursors?.('Imperialism',valid),
};
const imperialismOrder=imperialismSupported?targetedPower(89,{
  action:'imperialism',shortcut:';',fallback:'⌂',
  available:()=>ui?.epoch>=10&&!net&&wasm.leader_power_reserve(wasm.selected_actor_handle()>>>0)>=0,
  count:wasm.imperialism_selection_count,pick:wasm.imperialism_target_at,issue:wasm.cmd_imperialism_at,
},imperialismArt,'imperial-action'):null;
const diversionArt=await diversionPresentation?.loadCommandArt();
const diversionOrder=diversionSupported?targetedPower(87,{
  action:'diversion',shortcut:';',fallback:'◈',label:'Ablenkungsmanöver',
  description:'Erzeugt am gewählten Standort eine steuerbare Phantomformation.',
  available:()=>ui?.epoch>=10&&wasm.leader_diversion_enabled()>0&&wasm.military_leader_reserve(wasm.selected_actor_handle()>>>0)>=0,
  count:()=>wasm.leader_diversion_ready(wasm.selected_actor_handle()>>>0),
  pick:(x,y)=>wasm.leader_diversion_target_at(x,y),issue:wasm.cmd_leader_diversion_at,
},diversionArt,'military-action'):null;
const leaderTargetOrders=[imperialOrder,imperialismOrder,mobilizationOrder,purchaseOrder,diversionOrder].filter(Boolean);
const powerOrders=[...priestOrders,...spyOrders,...leaderTargetOrders];
const extractionAvailable=spyEnabled&&typeof wasm.cmd_spy_extraction==='function';
const selectedPowerType=(selection,definition)=>selection?.def===definition||selection?.types?.some(type=>type[0]===definition);
function militaryLeaderControl(selection,epoch){
  if(!canTargetOrder()||selection?.n!==1||selection.def!==87||typeof wasm.military_leader_reserve!=='function')return null;
  const handle=wasm.selected_actor_handle()>>>0,reserve=wasm.military_leader_reserve(handle);
  if(reserve<0)return null;
  if(epoch<5)return {mode:'rally',ready:!!wasm.leader_rally_ready(handle),
    action:leaderPresentation?.power(87,epoch,nativeMetaEpoch(epoch)),label:'Kundgebung',
    help:'Heilt eigene Landeinheiten im Umkreis.',issue:wasm.cmd_leader_rally};
  if(epoch<10&&typeof wasm.leader_fire_maneuver_ready==='function')
    return {mode:'fire-maneuver',ready:!!wasm.leader_fire_maneuver_ready(handle),
      action:leaderPresentation?.power(87,epoch,nativeMetaEpoch(epoch)),label:'Feuer und Manöver',
      help:'Beschleunigt eigene Landeinheiten und hält feindliche Landeinheiten im Umkreis auf.',issue:wasm.cmd_leader_fire_maneuver};
  return null;
}
function orderMilitaryLeaderPower(){
  const leader=militaryLeaderControl(ui?.sel,ui?.epoch);
  if(!leader?.ready||modalOpen())return;
  cancelMapGesture();cancelBuilding();attackMode=false;
  if(leader.issue())lastUi=0;
}
const activePowerOrder=()=>powerOrders.find(command=>command.order.active());
const activeTargetOrder=()=>activePowerOrder()?.order??(captureOrder?.active()?captureOrder:null);
const tradeRoute=tradeAvailable?createTradeRoute({canvas,caption:$('trade-route-caption'),art:tradeRouteArt,
  canOrder:()=>running&&!bench&&!ui?.win&&wasm.trade_selected_count()>0,
  selectionKey:()=>`${wasm.trade_selected_count()}:${wasm.selected_actor_handle?.()>>>0}`,
  pick:(x,y,home)=>wasm.trade_endpoint_at(x,y,home),issue:(home,destination)=>wasm.cmd_trade_route(home,destination),
  homeValid:typeof wasm.trade_home_valid==='function'?home=>wasm.trade_home_valid(home)!==0:undefined}) : null;
const airMission=airMissionAvailable?createAirMissionOrder({wasm,canvas,caption:$('air-mission-caption'),mapSize:MAP,invalidCursor:airMissionFlag?.invalidCursor,
  host(){
    if(!running||bench||ui?.win||modalOpen())return 0xffffffff;
    const handle=wasm.selected_actor_handle()>>>0;
    return handle!==0xffffffff&&wasm.air_mission_status(handle,0)===1?handle:0xffffffff;
  },
  onChange(){lastUi=0;}
}):null;
let tradePress=null, airPress=null;
function armAirMission() {
  if(!airMission||modalOpen()||!running) return false;
  const inside=mouse.inside;
  cancelMapGesture();cancelBuilding();attackMode=false;playerFlare.cancel();
  mouse.inside=inside;
  return airMission.arm();
}
function airMissionFeedback(ordered) {
  if(ordered&&!selectedUnitVoice('order'))sfx.play(22);
  lastUi=0;
}
function doAirMission(command) {
  if(!running||modalOpen())return;
  airMission?.cancel();
  airMissionFeedback(wasm.cmd_air_mission_action(command));
}
function armTradeRoute() {
  if (!tradeRoute || modalOpen() || !running || !wasm.trade_selected_count()) return false;
  const screen=mouse.inside&&document.elementFromPoint(mouse.x,mouse.y)===canvas?[mouse.x,mouse.y]:null;
  const point=screen?toWorld(...screen):null;
  cancelMapGesture();cancelBuilding();attackMode=false;playerFlare.cancel();
  return tradeRoute.arm(point,screen);
}
function armCapture() {
  if (!captureOrder || modalOpen() || !militaryCaptureCount()) return false;
  const point=mouse.inside?toWorld(mouse.x,mouse.y):null;
  cancelMapGesture();cancelBuilding();attackMode=false;playerFlare.cancel();
  return captureOrder.arm(point);
}
function armPowerOrder(command) {
  if (modalOpen() || !command.count()) return false;
  const point=mouse.inside?toWorld(mouse.x,mouse.y):null;
  cancelMapGesture();cancelBuilding();attackMode=false;playerFlare.cancel();
  return command.order.arm(point);
}
function orderSpyExtraction() {
  if(!extractionAvailable||modalOpen()||!canTargetOrder()||!wasm.spy_extraction_selection_count())return;
  cancelMapGesture();cancelBuilding();attackMode=false;playerFlare.cancel();
  wasm.cmd_spy_extraction();
  lastUi=0;
}
function armPlayerFlare(){
  if(rulesProfile.ruleset!==2||!running||bench||net||!ui||ui.win||modalOpen())return;
  cancelMapGesture();cancelBuilding();attackMode=false;
  playerFlare.arm();
}
$('player-flare-button').onclick=armPlayerFlare;

function warplanRecipient(){
  if(!net||net.roster||net.ls?.alone||!ui||ui.observer)return null;
  const player=ui.diplomacy?.players?.find(row=>row.team===1-ui.local&&row.alive&&!row.ai);
  return player?{relation:player.relation,name:`Spieler ${player.team+1}`}:null;
}
const fullmap = rulesProfile.ruleset===2&&packActorReader ? createFullmap({
  dialog:$('fullmap'),wasm,metadata:packActorReader,size:MAP,teams:TEAM,originalArt:originalEnabled,hillDefinition:kothDefinition,fishingId,
  territoryNameCatalog,cityNameCatalog,bitmapFonts:nativeBitmapFonts.bitmapFonts,
  warplanPeer:()=>net?.warplans,warplanRecipient,warplanClock:()=>wasm.game_tick()*TICK,
  warplanCue:sfx.playWarplanCue,
  pip:fullmapPip,
  onAccept(point){if(!fullmapPip.accept(point)){[cam.x,cam.y]=point;updateCamera();}},
  onToggle(open){keys.clear();playerFlare.cancel();playerFlare.setMapOpen(open);battleFlare?.setMapOpen(open);cancelMapGesture();$('tip').style.display='none';sfx.play(sfx.FULLMAP_CUE);},
  drawFlares:playerFlare.draw,
  drawBattleFlares:battleFlare?.draw,
}) : null;
$('fullmap-button').onclick=()=>{if(running&&!bench&&!valleyWorld&&!modalOpen())fullmap?.open();};

// ---------------------------------------------------------------- starting, saving, loading
function bindWorldTerrainMaterial() {
  if(valleyWorld){worldMaterialPending=null;originalModels.bindNativeTerrainMaterial?.(null);return;}
  worldMaterialPending = null;
  if (rulesProfile.ruleset !== 2 || !originalTerrain.enabled || !originalModels.bindNativeTerrainMaterial) return;
  originalModels.bindNativeTerrainMaterial(null);
  const binding = (async () => {
    const types = u8(wasm.terrain_types_ptr(), MAP * MAP);
    const origin = ownMapTerrainOrigin({width: MAP, height: MAP, types}, originalTerrain.manifest);
    // The procedural world renderer uses temperate materials at full detail.
    await originalModels.bindNativeTerrainMaterial({climate: 'Temperate', detailCode: 2, origin});
  })();
  worldMaterialPending = binding;
  binding.catch(error => console.warn('Gebäudematerial:', error.message)).finally(() => {
    if (worldMaterialPending === binding) {
      worldMaterialPending = null;
      last = performance.now();
    }
  });
}

function begin(tempo = null, useSourceClock = wasm.sim_clock_active?.() === 1, adaptiveStartRate = 0) {
  if(tempo&&!net&&!wasm.configure_solo_command_queue(1))throw Error('Befehlswarteschlange konnte nicht aktiviert werden');
  sourceSoloClock = supportsSourceSoloClock && !net && !bench && useSourceClock;
  const storedRate = wasm.sim_adaptive_rate?.() ?? 0;
  const adaptiveRate = storedRate || adaptiveStartRate;
  adaptiveSoloClock = sourceSoloClock && supportsAdaptiveSoloClock && adaptiveRate
    ? createAdaptiveSourceClock(wasm.game_tick(), adaptiveRate, storedRate ? wasm.sim_adaptive_scale() : tempo?.requested ?? 1)
    : null;
  valleyWorld=valleyPreview?activeValleyGeometry(wasm):null;
  packScene?.reset(wasm.sim_elapsed_seconds());
  mrlsPack?.reset(wasm.sim_elapsed_seconds());
  originalModels.clearResourcePicks();
  takeoffAudio?.reset();
  sfx.resetAirFlightAudio();
  sfx.resetGuidedImpact();
  hercAttack?.reset(wasm.sim_elapsed_seconds());
  guidedImpactFX?.reset(wasm.sim_elapsed_seconds());
  hercBeam?.reset(wasm.sim_elapsed_seconds());
  sfx.resetWorldAmbience();
  ambientCoverage=ambientPresentation&&!bench&&wasm.terrain_types_len?.()===MAP*MAP
    ? liveGeneratedAmbientCoverage(()=>u8(wasm.terrain_types_ptr(),MAP*MAP),MAP,MAP,wasm.resource_kind,performance.now()/1000) : null;
  airMissionFlag?.reset();
  tradeWaitStatus.update(false);
  tradeRoute?.cancel();tradePress=null;
  airMission?.cancel();airPress=null;
  activeTargetOrder()?.cancel();
  previousPriestViews=[];currentPriestViews=[];priestFrameEffects=null;priestAttachments.clear();
  rallyHealGate?.reset();
  spyShimmer?.reset();
  diversionShimmer?.reset();
  priestParticles?.reset();
  blessPreview=null;
  blessGround?.reset();
  playerFlare.reset(wasm.game_tick()>>>0);
  knownHotspotCount=hotspotRevealed();
  battleFlare?.reset(!bench);
  fullmapPip.reset();
  sfx.stopStoneThrowerWheels();
  fullmap?.close();
  fullmap?.warplans.reset();
  fullmap?.invalidate();
  saveSession=Date.now();
  farmAngleMemory.reset();
  ghost = null; attackMode = false; paused = false; acc = 0; fogTick = -1; actSig = '@refresh'; worstMs = 0; ui = null;
  crownProgress?.close();
  pauseReasons.clear();
  originalTempo = tempo;
  speed = tempo?.requested ?? 1;
  pendingTempo = 0;
  showSpeed(); $('pause').textContent = 'Pause';
  $('pause').title=hasOriginalTempo?'Pause (Umschalt+Pause)':'Pause (P)';
  $('game-options-button').hidden=!hasOriginalTempo||!!bench;
  $('pause').style.display = net ? 'none' : '';
  $('save').style.display = !net||net.checkpointHost ? '' : 'none';
  $('save').title=net?.checkpointHost?'Gemeinsamen Spielstand erstellen (Strg+S)':'Speichern (Strg+S) · automatisches Speichern aktiv';
  resetCheckpointDownload();
  $('speed').style.display = net&&!tempo ? 'none' : '';
  $('observer-status').hidden=true;
  updateInitialCeasefire();
  $('menu').style.display = 'none'; $('end').style.display = 'none';
  shellMovie(false);
  $('camera-tools').hidden=!referenceMode;
  $('fullmap-button').hidden=!fullmap||!!bench||!!valleyWorld;
  $('player-flare-button').hidden=!fullmap||!!bench||!!net;
  uploadTerrain();
  bindWorldTerrainMaterial();
  if (rulesProfile.ruleset === 2) {
    const ptr = wasm.ui();
    const state = JSON.parse(dec.decode(u8(ptr, wasm.ui_len())));
    initialMusicEpoch = state.epoch;
    musicCivilization = wasm.player_civilization(state.local) >>> 0;
    sfx.rebindRegionalMusic();
  }
  wasm.events_clear();
  running = true; last = performance.now(); lastAuto = last;
}
function resolvedSkirmishChoices(choices, players) {
  const civilizations = resolveCivilizations(choices, players, {random: () => sfx.presentationWord() % 14});
  const range = resolveEpochRange(+$('start-epoch').value, +$('end-epoch').value, wasm.epoch_count(), sfx.presentationWord);
  return {civilizations, ...range};
}
function configureOriginalPopulation() {
  if (wasm.configure_late_population?.() === 1) return true;
  toast('Die Originalregel für die Bevölkerung konnte nicht aktiviert werden.');
  return false;
}
function start(nAi, seed, benchUnits, humans = 1, local = 0) {
  const originalSkirmish=!benchUnits&&humans===1&&rulesProfile.id==='up15';
  const victoryMode=victoryCondition.value;
  const kothMinutes=originalSkirmish&&victoryMode==='koth'?kothMenu.readMinutes():undefined;
  if(kothMinutes===null){toast('Wähle 2, 5 oder 10 Minuten für Herr des Hügels.');return;}
  const crownVictory=originalSkirmish?crownVictoryMenu.read():undefined;
  const hotspots=originalSkirmish&&hotspotAvailable&&victoryMode==='hotspots'?+$('sk-hotspot-count').value:undefined;
  if(crownVictory===null){toast('Wähle ein gültiges Ziel für den Kronensieg.');return;}
  const capitols=originalSkirmish?capitolsMenu.read():undefined;
  if(capitols===null){toast('Wähle 2, 5 oder 10 Minuten für die Rückeroberung.');return;}
  restartPoint=null;
  bench = benchUnits;
  const climate=benchUnits?0:menuClimate();
  if(kothMinutes===undefined)wasm.init(seed >>> 0, nAi, benchUnits, humans, local);
  else if(!wasm.init_hill_world(seed >>> 0,nAi)){toast('Die Karte für Herr des Hügels konnte nicht erzeugt werden.');return;}
  if(!benchUnits&&humans===1&&heavyMounted3Id!==undefined&&!wasm.configure_heavy_mounted3()){
    toast('Der Minotaurus-HERC konnte nicht für dieses Scharmützel eingerichtet werden.');
    return;
  }
  if(originalSkirmish&&!wasm.configure_guided_herc_rocket()){
    toast('Die HERC-Lenkflugkörper konnten nicht für dieses Scharmützel eingerichtet werden.');
    return;
  }
  if(!benchUnits&&civilizationSetup.enabled){
    const players=Math.max(2,Math.min(4,nAi+humans));
    const territoryPercent=originalSkirmish&&victoryMode==='territory'
      ? +$('sk-territory-percent').value : undefined;
    const regicide=originalSkirmish&&victoryMode==='regicide';
    const soleSurvivor=originalSkirmish&&(victoryMode==='conquest'||regicide||capitols?.mode===5)&&$('sk-sole-survivor').checked;
    try {
      applyMatchSettings(wasm,{...resolvedSkirmishChoices(civilizationSetup.choices(),players),
        popLimit:+$('pop-limit').value,supplies:+$('supplies').value,territoryPercent,regicide,soleSurvivor,kothMinutes,crownVictory,capitols,hotspots},players);
      if(originalSkirmish&&!skirmishTeams.apply(players))throw Error('Die Startteams konnten nicht eingerichtet werden.');
    } catch(error) { toast(error.message); return; }
  }else if(!benchUnits&&humans===1){
    wasm.skirmish_setup(+$('pop-limit').value, +$('supplies').value);
    if(referenceMode)wasm.skirmish_epoch(+$('start-epoch').value);
  }
  if (originalSkirmish && !configureOriginalPopulation()) return;
  seasonSession.configure(menuSeasonSettings(),climate);
  if(!benchUnits)wasm.skirmish_visibility(+$('map-visibility').value);
  if(originalSkirmish&&typeof wasm.configure_sight_terms==='function'&&wasm.configure_sight_terms()!==1){
    toast('Die Sichtregeln konnten nicht eingerichtet werden.');return;
  }
  if(originalSkirmish&&typeof wasm.configure_timed_treaties==='function'&&wasm.configure_timed_treaties(0)!==1){
    toast('Die Bündnisdauer konnte nicht eingerichtet werden.');return;
  }
  if(originalSkirmish&&typeof wasm.configure_border_treaties==='function'&&wasm.configure_border_treaties()!==1){
    toast('Die Grenzregeln konnten nicht eingerichtet werden.');return;
  }
  if(originalSkirmish&&Number.isInteger(fishingId)&&!wasm.configure_fishing_skirmish()){
    toast('Die Fischgründe konnten für dieses Scharmützel nicht eingerichtet werden.');
    return;
  }
  if(!benchUnits&&hasInitialCeasefire&&!wasm.configure_initial_ceasefire(+$('initial-ceasefire').value)) {
    toast('Der anfängliche Waffenstillstand konnte nicht gestartet werden.');
    return;
  }
  if(!benchUnits && humans===1 && rulesProfile.id==='up15') {
    if(nameSettings.get('territoryNames') === '1' && !(territoryNameCatalog && wasm.configure_territory_names?.(territoryNameCatalog.catalog))) {
      toast('Die originalen Gebietsnamen konnten für dieses Scharmützel nicht geladen werden.');
    }
    if(nameSettings.get('cityNames') === '1' && !(cityNameCatalog && wasm.configure_city_names?.(cityNameCatalog.catalog))) {
      toast('Die originalen Städtenamen konnten für dieses Scharmützel nicht geladen werden.');
    }
    if(nameSettings.get('rememberedCities') === '1' && !wasm.configure_city_memory?.()) {
      toast('Die Städtenamen im Nebel konnten für dieses Scharmützel nicht aktiviert werden.');
    }
  }
  if(!benchUnits&&humans===1&&civilizationSetup.enabled&&wasm.stat(local,0)===0
     && typeof wasm.ai9_game_research_setup_manual==='function' && !wasm.ai9_game_research_setup_manual()){
    toast('Originalforschung konnte für dieses Scharmützel nicht gestartet werden.');
  }
  if(!benchUnits&&humans===1&&priestEnabled&&!wasm.configure_priest_skirmish()){
    toast('Priester konnten für dieses Scharmützel nicht eingerichtet werden.');
    return;
  }
  if(!benchUnits&&humans===1&&priestEnabled&&typeof wasm.configure_carrier_surface_pose==='function'){
    wasm.configure_carrier_surface_pose();
  }
  if(originalSkirmish&&typeof wasm.configure_submarine_depth==='function'&&!wasm.configure_submarine_depth()){
    toast('Die Tauchtiefe der U-Boote konnte nicht eingerichtet werden.');
    return;
  }
  if(originalSkirmish&&typeof wasm.configure_torpedo84==='function'&&!wasm.configure_torpedo84()){
    toast('Der Torpedoantrieb konnte nicht eingerichtet werden.');
    return;
  }
  if(!benchUnits&&humans===1&&priestEnabled&&typeof wasm.configure_dock_garrison==='function'
    &&!wasm.configure_dock_garrison()){
    toast('Die Schiffsgarnison konnte für dieses Scharmützel nicht aktiviert werden.');
    return;
  }
  if(!benchUnits&&humans===1&&crownSupported&&!wasm.configure_crowns(Number($('disable-crowns').checked))){
    toast('Die Kronenwertung konnte für dieses Scharmützel nicht eingerichtet werden.');
    return;
  }
  if(!benchUnits&&humans===1&&crownSupported&&priestEnabled&&typeof wasm.configure_city_heal==='function'
    &&!wasm.configure_city_heal()){
    toast('Die Heilung im Stadtgebiet konnte für dieses Scharmützel nicht aktiviert werden.');
    return;
  }
  if(!benchUnits&&humans===1&&crownSupported&&priestEnabled&&typeof wasm.configure_crown_leaders==='function'
    &&!wasm.configure_crown_leaders()){
    toast('Die Kronenführer konnten für dieses Scharmützel nicht aktiviert werden.');
    return;
  }
  if(!benchUnits&&humans===1&&crownSupported&&priestEnabled&&typeof wasm.configure_military_leader_powers==='function'
    &&!wasm.configure_military_leader_powers()){
    toast('Die Fähigkeiten des Militärführers konnten nicht aktiviert werden.');
    return;
  }
  if(!benchUnits&&humans===1&&diversionSupported&&!wasm.configure_leader_diversion()){
    toast('Das Ablenkungsmanöver konnte nicht aktiviert werden.');
    return;
  }
  if(!benchUnits&&humans===1&&crownSupported&&priestEnabled&&typeof wasm.configure_economic_leader_effects==='function'
    &&!wasm.configure_economic_leader_effects()){
    toast('Die Fähigkeiten des Wirtschaftsführers konnten nicht aktiviert werden.');
    return;
  }
  cam.x = benchUnits ? MAP / 2 : wasm.start_x(); cam.y = benchUnits ? MAP / 2 : wasm.start_y(); cam.zoom = benchUnits ? 11 : 28;
  cam.heading=0;
  if(referenceMode)cam.nativeFraction=benchUnits?originalFractionForZoom(11,innerWidth,innerHeight):.2;
  const soloOriginal = hasOriginalTempo && !benchUnits && humans === 1;
  const source = soloOriginal && $('source-solo-clock').checked;
  const adaptive = source && supportsAdaptiveSoloClock && $('source-adaptive-clock').checked;
  begin(soloOriginal ? startingTempo(menuTempo()) : null, source, adaptive ? 30 : 0);
  if(hotspots>0)toast('Ein Krisenherd wurde entdeckt!');
  if(originalTempo){
    const bytes=worldSaveBytes();
    if(bytes)restartPoint={bytes,cam:{...cam},...(sourceSoloClock ? {sourceSoloClock:true} : {}),
      ...(adaptiveSoloClock ? {adaptiveSourceClock:true} : {})};
  }
}

function startValley(authored) {
  if(!valleyPreview?.start(+$('map-visibility').value,authored))return toast('Valley konnte nicht gestartet werden.');
  if (valleyNormal && authored && !configureOriginalPopulation()) return;
  net=null;bench=0;restartPoint=null;
  seasonSession.configure({cycle:false,startMonth:6,minutesPerSeason:5},0);
  if(!wasm.ai9_game_research_setup_manual?.())toast('Originalforschung konnte für dieses Scharmützel nicht gestartet werden.');
  const initial=initialValleyViews(wasm,{primary:[innerWidth,innerHeight],pip:[340,200]});
  Object.assign(cam,initial.primaryPose);
  begin(hasOriginalTempo?startingTempo(menuTempo()):null);
  const bytes=worldSaveBytes();
  if(bytes&&hasOriginalTempo)restartPoint={bytes,cam:{...cam}};
}

const valleyDebugDatabase=!!valleyPreview&&!valleyNormal;
const db = (valley = valleyDebugDatabase) => new Promise((res, rej) => {
  const r = indexedDB.open(rulesProfile.database+(valley?'-valley-preview':''), 1);
  r.onupgradeneeded = () => r.result.createObjectStore('saves');
  r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error);
});
async function readLocalSave(slot) {
  const read = async valley => {
    const d = await db(valley);
    try {
      return await new Promise((res, rej) => {
        const rq = d.transaction('saves','readonly').objectStore('saves').get(slot);
        rq.onsuccess = () => res(rq.result);
        rq.onerror = () => rej(rq.error);
      });
    } finally { d.close(); }
  };
  const normalArchive=valleyNormal&&valleyPreview?.authoredAvailable;
  const preferredValley=valleyDebugDatabase||!!normalArchive&&(running?!!valleyWorld:valleyMapChoice.value===valleyArchiveOption.value);
  const current=await read(preferredValley).catch(()=>null);
  if(normalArchive&&preferredValley&&!valleySaveLocator(current?.bytes))return null;
  if(current||preferredValley||!normalArchive)return current;
  const archive=await read(true).catch(()=>null);
  return valleySaveLocator(archive?.bytes)?archive:null;
}
let saveSession=Date.now();
function worldSaveBytes() {
  const ptr=wasm.save(), length=wasm.save_len();
  return length ? u8(ptr,length).slice() : null;
}
function applyWorldSave(bytes) {
  if(bytes?.[0]===0x56&&bytes[1]===0x43&&bytes[2]===0x4e&&bytes[3]===0x31)return 0;
  const locator=valleySaveLocator(bytes);
  if(locator&&!valleyPreview?.prepareLoad(locator))return 0;
  const ptr=wasm.load_alloc(bytes.length);
  u8(ptr,bytes.length).set(bytes);
  return wasm.load_apply();
}
async function saveGame(silent) {
  if(running&&!bench&&net?.checkpointHost){
    if(!silent&&!rosterLobby.requestCheckpoint())toast('Ein gemeinsamer Spielstand ist gerade nicht verfügbar.');
    return;
  }
  if (!running || bench || net) {
    if(!silent)toast(net?'Gemeinsame Spielstände speichert der Host in einem kompatiblen Mehrspielerraum.':'Spielstand gerade nicht verfügbar.');
    return;
  }
  const bytes=worldSaveBytes();
  if (!bytes) { if (!silent) toast('Spielstand konnte nicht gespeichert werden'); return; }
  try {
    const record={bytes,cam:{...cam},when:Date.now(),epoch:ui?ui.ename:'',session:saveSession,seasons:seasonSession.settings,
      ...(originalTempo?{gameTempo:{...originalTempo,requested:speed}}:{}),
      ...(sourceSoloClock?{sourceSoloClock:true}:{}),
      ...(adaptiveSoloClock&&wasm.game_tick()===0?{adaptiveStartRate:adaptiveSoloClock.rate}:{}),
      ...(restartPoint?{restart:restartPoint}:{}),
      ...(seasonSession.climate?{climate:seasonSession.climate}:{}),rules:rulesProfile.ruleset,profile:rulesProfile.id};
    const d=await db(valleyDebugDatabase||valleyNormal&&!!valleySaveLocator(bytes));
    await new Promise((resolve,reject)=>{
      const tx=d.transaction('saves','readwrite'),store=tx.objectStore('saves'),old=store.get('slot1');
      old.onsuccess=()=>{if(old.result&&old.result.session!==saveSession)store.put(old.result,'previous');store.put(record,'slot1');};
      tx.oncomplete=()=>{d.close();resolve();};tx.onabort=()=>{d.close();reject(tx.error);};tx.onerror=()=>reject(tx.error);
    });
    if (!silent) toast(`Spielstand gespeichert (${(bytes.length / 1024) | 0} KB)`);
    refreshContinue();
  } catch (e) { if (!silent) toast('Could not save: ' + e.message); }
}
async function loadGame(slot='slot1') {
  const s = await readLocalSave(slot);
  if (!s) return toast('No saved game found');
  if(!savedRulesProfileMatches(s,rulesProfile))return toast('Dieser Spielstand gehört zu einer anderen Regelversion. Wähle im Menü die passende Regelversion.');
  let seasons,climate,tempo=null;
  try{seasons=savedSeasonSettings(s);climate=savedClimate(s,climates);}
  catch{return toast('Die Jahreszeiten- oder Klimaeinstellungen des Spielstands sind ungültig.');}
  try{if(hasOriginalTempo)tempo=loadOriginalTempo(s.gameTempo);}
  catch{return toast('Die Geschwindigkeit des Spielstands ist ungültig.');}
  if (!applyWorldSave(s.bytes)) return toast('Dieser Spielstand ist mit dieser Regelversion nicht kompatibel.');
  net = null; bench = 0; Object.assign(cam, s.cam);
  restartPoint=hasOriginalTempo&&s.restart?.bytes instanceof Uint8Array&&s.restart?.cam ? s.restart : null;
  cam.heading=normalizeOriginalHeading(s.cam?.heading??0);
  seasonSession.configure(seasons,climate);setSeasonControls(seasons);climateControl.value=climate;
  if(referenceMode)cam.nativeFraction=Number.isFinite(s.cam?.nativeFraction)?s.cam.nativeFraction:originalFractionForZoom(s.cam?.zoom??28,innerWidth,innerHeight);
  const unstarted = s.sourceSoloClock === true && wasm.game_tick() === 0;
  const adaptiveStartRate = unstarted && Number.isInteger(s.adaptiveStartRate) && s.adaptiveStartRate >= 5 && s.adaptiveStartRate <= 30
    ? s.adaptiveStartRate : 0;
  begin(tempo, wasm.sim_clock_active?.() === 1 || unstarted, adaptiveStartRate);
  toast('Spielstand geladen');
}
let continueRefresh=0;
async function refreshContinue() {
  const request=++continueRefresh;
  const [s,previous]=await Promise.all([readLocalSave('slot1'),readLocalSave('previous')]);
  if(request!==continueRefresh)return;
  $('continue').style.display = savedRulesProfileMatches(s,rulesProfile) ? '' : 'none';
  if (s) $('continue').textContent = `Fortsetzen (${s.epoch || 'Spielstand'}, ${new Date(s.when).toLocaleString()})`;
  $('previous-save').style.display=savedRulesProfileMatches(previous,rulesProfile)?'':'none';
  if(previous)$('previous-save').textContent=`Vorheriger Spielstand (${previous.epoch||'Scharmützel'})`;
}
$('previous-save').onclick=()=>{rosterLobby?.reset();loadGame('previous');};
refreshContinue();

// ---------------------------------------------------------------- simulation stepping & sound
const hillPlayerName=seat=>`Spieler ${seat+1}`;
function playHillCapture(kind,x,y) {
  const code=kind-128, previous=code>>2, holder=code&3, local=ui?.local??0;
  const audience=holder===local?'self':previous===local?'other':'others';
  const cue=kothPresentation?.capture(audience,{fromWorld:previous===4});
  toast(cue?.message.replace('%PLAYER%',hillPlayerName(audience==='self'?previous:holder))??
    (holder===local?'Du hast den Hügel erobert.':`${hillPlayerName(holder)} hat den Hügel erobert.`));
  if(!cue){sfx.play(sfx.CAPTURE_COMPLETE_CUE,1,0);return;}
  const event={...cue.sound,url:new URL(cue.sound.url,import.meta.url),gain:cue.sound.volume};
  if(cue.sound.spatial)void sfx.playVerifiedWorldCue(event,x,y,groundAt(x,y),view);
  else void sfx.playInterfaceCue(event);
}
function advance(dt, now) {
  let visualSeconds = dt * speed;
  const packStartTick=packScene?wasm.game_tick()>>>0:0;
  let packSeconds=null;
  if (net?.sourceRounds) {
    net.refreshLink?.(now);
    let result;
    try { result=net.ls.advance(dt); }
    catch(error) { net.ls.abort(error.message); }
    visualSeconds=result?.gameSeconds??0;
    packSeconds=visualSeconds;
    if(result?.steps){simMs=net.ls.cpuSeconds*1000;waitSince=now;}
  } else if (net) {
    acc = Math.min(acc + dt * speed, TICK * 6);         // allow a short catch-up burst, never a spiral
    let steps = 0; const t0 = performance.now();
    while (acc >= TICK && steps < 8 && net.ls.step()) { acc -= TICK; steps++; }
    if (steps) { simMs += ((performance.now() - t0) / steps - simMs) * 0.1; waitSince = now; }
  } else if (adaptiveSoloClock) {
    const result = advanceAdaptiveSourceFrame(adaptiveSoloClock, dt, paused, adaptiveRenderSample.fps, {
      requested: () => speed,
      beforeAdmit: () => {
        if (pendingTempo) { applyRequestedSpeed(pendingTempo); pendingTempo = 0; }
      },
      nowSeconds: () => performance.now() / 1000,
      admit: (oldRate, rate, scale) => wasm.tick_with_adaptive_source_slot(oldRate, rate, scale) === 1,
    });
    visualSeconds = result.gameSeconds;
    packSeconds=visualSeconds;
    if (result.steps) simMs = adaptiveSoloClock.cpuSeconds * 1000;
  } else if (sourceSoloClock) {
    const t0 = performance.now();
    const result = advanceSourceSoloFrame(acc, dt, paused, () => {
      if (pendingTempo) { applyRequestedSpeed(pendingTempo); pendingTempo = 0; }
      return speed;
    }, delta => wasm.tick_with_source_delta(delta) === 1);
    acc = result.remainder;
    visualSeconds = result.gameSeconds;
    packSeconds=visualSeconds;
    if (result.steps) simMs += ((performance.now() - t0) / result.steps - simMs) * 0.1;
  } else if (!paused) {
    acc += dt * speed;
    let steps = 0; const t0 = performance.now();
    while (acc >= TICK && steps < 8) {
      if(pendingTempo){applyRequestedSpeed(pendingTempo);pendingTempo=0;}
      wasm.tick(); acc -= TICK; steps++;
    }
    if (steps) simMs += ((performance.now() - t0) / steps - simMs) * 0.1;
    if (acc > TICK) acc = TICK;                          // drop time instead of stalling the frame
  }
  if(packScene){
    if(packSeconds===null){
      packSeconds=0;
      const steps=((wasm.game_tick()>>>0)-packStartTick)>>>0,step=wasm.sim_step_seconds();
      for(let i=0;i<steps;i++)packSeconds=storeF32(addF64(packSeconds,step));
    }
    packScene.advance(packSeconds,wasm.sim_elapsed_seconds());
    mrlsPack?.advance(packSeconds,wasm.sim_elapsed_seconds());
  }
  // sound cues the simulation queued up
  const n = wasm.events_len();
  const impacts=hasGuidedImpacts?wasm.guided_herc_impact_prepare():0;
  const beamStarts=wasm.guided_herc_beam_count?.()??0;
  if(beamStarts)hercBeam?.enqueue(wasm,beamStarts);
  if (n) {
    const ev = f32(wasm.events_ptr(), n);
    for (let i = 0; i < n; i += 3) {
      if(hercAttack?.ready(wasm.guided_herc_attack_shot_at(i/3)>>>0))continue;
      const shot=airFlightAudio||referenceMode ? wasm.air_audio_shot_at?.(i/3)>>>0 : 0;
      if(airFlightAudio&&shot&&sfx.airFlightAttackHandled(shot&0xffff,shot>>>16,wasm.air_audio_shot_duration(i/3)))continue;
      const kind = ev[i], x = ev[i + 1], y = ev[i + 2];
      if(kind===34){
        toast('Drei Einwanderer sind am Stadtzentrum angekommen.');
        if(originalEnabled)void sfx.playInterfaceCue(immigrationCue);
        continue;
      }
      if(kind===35){
        toast('Sie haben einen Anführer gewonnen.');
        continue;
      }
      if(kind>=36&&kind<=39){
        const name=wonderPresentation.nameFor(y)||'ein Weltwunder',player=`Spieler ${x+1}`;
        toast([`${player} beginnt den Bau von ${name}.`,`${player} stellt ${name} fertig.`,
          `${name} wurde zerstört.`,`${player} übernimmt ${name}.`][kind-36]);
        continue;
      }
      if(kind>=128&&kind<148){playHillCapture(kind,x,y);continue;}
      if(kind>=148&&kind<155){
        const cue=capitolFeedback(kind,x,y,hillPlayerName,capitolArt);
        toast(cue.message);
        if(cue.sound)void sfx.playInterfaceCue(cue.sound).then(played=>{if(!played)sfx.play(cue.lost?15:sfx.CAPTURE_COMPLETE_CUE);});
        else sfx.play(cue.lost?15:sfx.CAPTURE_COMPLETE_CUE);
        continue;
      }
      const citizen = decodeCitizenDeathCue(kind);
      if (kind >= 12 && !citizen && kind !== sfx.STONE_THROWER_LAUNCH_CUE && kind !== sfx.CAPTURE_COMPLETE_CUE &&
          kind !== sfx.CONVERT_CUE && kind !== sfx.PRIEST_CONVERT_COMPLETE_CUE && kind !== sfx.PRIEST_CONVERT_CAST_CUE) {
        sfx.play(kind, 1, 0);
      } else if (referenceMode && !shot && kind !== sfx.CAPTURE_COMPLETE_CUE) {
        const altitude=groundAt(x,y)+(kind===sfx.PRIEST_CONVERT_CAST_CUE ? .5 : 0);
        sfx.playWorld(kind,x,y,altitude,view,citizen);
      } else {
        const {volume,pan}=sfx.spatialSound(x-cam.x,y-cam.y,cam.zoom);
        if (citizen) {
          if (!sfx.citizenDeath(citizen, volume, pan)) sfx.play(10, volume, pan);
        } else {
          sfx.play(kind, volume, pan);
        }
      }
    }
  }
  if(impacts){
    const rows=f32(wasm.guided_herc_impact_ptr(),impacts*4);
    const pipVP=guidedImpactFX&&fullmapPip.admissionProjection();
    for(let i=0;i<rows.length;i+=4){
      const x=rows[i],y=rows[i+1],z=rows[i+2];
      sfx.playGuidedImpact(x,y,z,view);
      if(guidedImpactFX&&(guidedImpactInside(view.vp,x,y,z)||guidedImpactInside(pipVP,x,y,z)))
        guidedImpactFX.spawn(x,y,z);
    }
  }
  if(n||impacts||beamStarts)wasm.events_clear();
  return visualSeconds;
}
// A hidden tab gets no animation frames, but a multiplayer peer must keep simulating.
setInterval(() => {
  if (!running || worldMaterialPending || !net || !document.hidden) return;
  const now = performance.now(), dt = Math.min(1.5, (now - last) / 1000); last = now;
  for (let t = dt; t > 0; t -= 0.3) advance(Math.min(t, 0.3), now);
}, 200);

// ---------------------------------------------------------------- frame loop
function drawCaptureGlow(vp,actorMetadata){
  if(!captureGlow||!actorMetadata)return;
  const capture=wasm.has_active_capture_glow(),spy=wasm.spy_adverse_glow_present?.();
  if(!capture&&!spy)return;
  const crown=wasm.crown_visuals_present?.();
  const bad=handle=>(capture&&wasm.active_capture_glow(handle))||(spy&&spyAdverseGlow(handle));
  const isMixed=handle=>spyAdvancedArt?.mixedGlow&&bad(handle)
    && (wasm.bless_glow_active?.(handle)||wasm.imperialism_visible?.(handle)||(crown&&(wasm.crown_actor_glow(handle)&1)));
  const projected = captureGlow.project(actorMetadata.actors, handle=>bad(handle)&&!isMixed(handle));
  originalGround.drawCaptureGlow({vp,...projected});
  if(spyAdvancedArt?.mixedGlow){
    const fallback=actorMetadata.actors.filter(actor=>!blessGround?.isGround(originalModels.sourceVisualForActor?.(actor)));
    originalGround.drawMixedGlow({vp,...captureGlow.project(fallback,isMixed)});
  }
}

function spyAdverseGlow(handle) {
  return !!(wasm.building_sabotaged?.(handle)||wasm.spy_toxic_status?.(handle));
}

function drawAirCrownStatus(vp,actorMetadata){
  if(!airCrownGlow||!actorMetadata||!wasm.crown_visuals_present?.())return 0;
  const rows=[];
  for(const actor of actorMetadata.actors){
    if(!blessGround.isAir(originalModels.sourceVisualForActor?.(actor)))continue;
    const flags=wasm.crown_actor_glow(actor.handle);
    if(!flags)continue;
    const good=(flags&1)||wasm.bless_glow_active(actor.handle);
    const bad=(flags&2)||wasm.active_capture_glow(actor.handle)||spyAdverseGlow(actor.handle);
    rows.push({kind:good?(bad?'mixed':'good'):'bad',origin:[actor.renderX,actor.renderAltitude,actor.renderY],yaw:actor.renderYaw});
  }
  return airCrownGlow.draw({vp,rows,phase:blessGround.phase});
}

function drawBlessGround(vp,actorMetadata,previewFrame=null){
  if(!blessGround||!actorMetadata||!originalGround.enabled||!originalModels.sourceVisualForActor)return;
  if(previewFrame){
    const preview=activePowerOrder()?.action==='bless'&&mouse.inside&&document.elementFromPoint(mouse.x,mouse.y)===canvas;
    if(!preview)blessPreview=null;
    else if(!blessPreview||(blessPreview.remaining-=previewFrame.visualSeconds)<=0){
      const [x,y]=toWorld(mouse.x,mouse.y);
      const handles=new Set(actorMetadata.actors.filter(actor=>
        (actor.simX-x)**2+(actor.simY-y)**2<=4&&wasm.bless_preview_eligible(actor.handle)).map(actor=>actor.handle));
      blessPreview={remaining:.2,handles};
    }
  }
  const crown=wasm.crown_visuals_present?.(),capture=wasm.has_active_capture_glow?.();
  let crownGround=0;
  const layer=blessGround.project({actors:actorMetadata.actors,local:ui?.local??0,
    visualForActor:originalModels.sourceVisualForActor,isBlessed:handle=>{
      const crownGood=crown&&(wasm.crown_actor_glow(handle)&1);
      const good=wasm.bless_glow_active(handle)||wasm.imperialism_visible?.(handle)||crownGood;
      if(crownGood)crownGround++;
      return good;
    },
    isMixed:handle=>spyAdvancedArt?.mixedGlow&&((capture&&wasm.active_capture_glow(handle))||spyAdverseGlow(handle)),
    hasPreview:handle=>!!previewFrame&&!!blessPreview?.handles.has(handle)});
  originalGround.drawBlessGlow({vp,...layer});
  originalGround.drawMixedGlow({vp,list:layer.mixed,alpha:layer.alpha});
  if(qaMode&&previewFrame)canvas.dataset.blessGround=`${layer.good.length/5}:${layer.neutral.length/5}`;
  if(qaMode&&previewFrame)canvas.dataset.crownGround=String(crownGround);
}

function preparePriestEffectsFrame(renderTime){
  const count=wasm.priest_effect_count()>>>0,names=['','convert','bless','converted'];
  const records=new DataView(wasm.memory.buffer,wasm.priest_effect_ptr()>>>0,count*12);
  const keys=new Set(),effects=[],heals=[],converted=new Map();
  for(let index=0;index<count;index++){
    const at=index*12,handle=records.getUint32(at,true),type=records.getUint32(at+4,true);
    const started=records.getFloat32(at+8,true);
    if(type===4){if(rallyHealGate)heals.push({handle,kind:'healed',started});continue;}
    const kind=names[type],key=`${handle}:${kind}:${started}`;
    if(!kind||keys.has(key))continue;
    keys.add(key);
    if(!priestAttachments.has(key)){
      const repeated=kind==='converted'&&started-(converted.get(handle)??-Infinity)<2;
      priestAttachments.set(key,!repeated&&previousPriestViews.some(snapshot=>snapshot.forHandle(handle)));
    }
    if(priestAttachments.get(key)!==true)continue;
    if(kind==='converted')converted.set(handle,started);
    effects.push({handle,kind,started,key,elapsed:renderTime-started});
  }
  priestParticles?.advance(effects.filter(effect=>effect.kind!=='bless'&&effect.elapsed>=0));
  return {keys,effects,heals:rallyHealGate?.advance(heals,previousPriestViews,renderTime)??[]};
}

function drawPriestEffects(currentView,actorMetadata,renderTime){
  if(!priestEffects&&!priestParticles&&!rallyHeal)return null;
  if(actorMetadata)currentPriestViews.push(actorMetadata);
  priestFrameEffects??=preparePriestEffectsFrame(renderTime);
  const meshes=[],particles=[];
  for(const {handle,kind,started,key,elapsed} of priestFrameEffects.effects){
    const actor=actorMetadata?.forHandle(handle);
    if(!actor||actor.hpRatio<=0||elapsed<0)continue;
    const scale=originalModels.sourceScaleForActor?.(actor)??(kind==='converted'?null:.13435028493404388);
    const row={key,kind,visible:true,yaw:actor.renderYaw,scale,
      position:[actor.renderX,actor.renderAltitude,actor.renderY]};
    if(kind!=='converted'&&actor.renderTime>=started)meshes.push({...row,elapsed:actor.renderTime-started});
    if(kind!=='bless'&&scale>0)particles.push(row);
  }
  const heals=[];
  for(const effect of priestFrameEffects.heals){
    const actor=actorMetadata?.forHandle(effect.handle);
    if(!actor||actor.hpRatio<=0||actor.renderTime<effect.started)continue;
    const scale=originalModels.sourceScaleForActor?.(actor);
    if(scale>0)heals.push({...effect,visible:true,scale,yaw:actor.renderYaw,elapsed:actor.renderTime-effect.started,
      position:[actor.renderX,actor.renderAltitude,actor.renderY]});
  }
  return {meshDraws:priestEffects?.draw({...currentView,effects:meshes})??0,
    particleQuads:priestParticles?.draw({...currentView,effects:particles})??0,
    rallyDraws:rallyHeal?.draw({...currentView,lighting:originalModels.lighting,effects:heals})??0,
    particleTargets:qaMode?particles.filter(effect=>effect.kind==='converted').length:0};
}

function finishPriestEffectsFrame(){
  if(!priestEffects&&!priestParticles&&!rallyHeal)return;
  for(const key of priestAttachments.keys())if(!priestFrameEffects?.keys.has(key))priestAttachments.delete(key);
  previousPriestViews=currentPriestViews;currentPriestViews=[];priestFrameEffects=null;
}

function drawAirMissionFlags(vp,actorMetadata,mission) {
  if(!mission?.records.length)return;
  const projectHeight=(x,z)=>Math.max(0,sampleGroundHeight(heights,x,z));
  const worldAt=handle=>{
    if(!wasm.entity_alive(handle))return null;
    const slot=handle&65535,x=wasm.ent_x(slot),z=wasm.ent_y(slot);
    return [x,projectHeight(x,z),z];
  };
  const routes=projectAirMissionFlags(mission.records,mission.draft,{
    actorAt:handle=>actorMetadata?.forHandle(handle),
    attachmentAt:actor=>originalModels.rallyAttachment?.(actor),
    hostAt:worldAt,targetAt:worldAt,groundAt:projectHeight
  });
  airMissionFlag.draw({vp,routes,groundAt:projectHeight});
}

function drawSpyExtraction(view,renderTime) {
  if(!spyTimer)return null;
  const count=wasm.spy_extraction_count(),markers=[],ground=[];
  for(let i=0;i<count;i++){
    const marker=[0,1,2,3].map(field=>wasm.spy_extraction_field(i,field));
    if(marker.some(value=>!Number.isFinite(value)))continue;
    markers.push(marker);
    ground.push(marker[0],marker[1],0,1.2,1.2);
  }
  const decals=originalGround.drawExtraction({vp:view.vp,list:ground,tint:TEAM[ui?.local??0]??TEAM[0]});
  const timers=spyTimer.draw(view,markers,renderTime,spyArt?.extractionVisuals.timer);
  return `${decals}:${timers}`;
}

function drawFullmapPipScene(pipView, alpha, renderEpochs, now, mission, preparedSailMetadata) {
  const {vp,eye,zoom,width,height} = pipView;
  const corners = [[0,0],[width,0],[width,height],[0,height]].map(([x,y])=>{
    const ray=originalCameraRay(pipView,x,y,width,height);
    const depth=Math.min(pipView.far,-eye[1]/Math.min(-.0001,ray[1]));
    return [eye[0]+ray[0]*depth,eye[2]+ray[2]*depth];
  });
  const x=corners.map(point=>point[0]),y=corners.map(point=>point[1]);
  const count=(wasm.render_rotated||wasm.render)(alpha,Math.min(...x)-2,Math.min(...y)-2,
    Math.max(...x)+2,Math.max(...y)+6,-1,0,0,0);
  let actorMetadata=null;
  const hercBeamMuzzles=hercBeam?new Map():null;
  originalTerrain.drawTerrain({vao:vaoT,vp,px:1/zoom,fogTexture,territoryTexture,capture:true});
  if(count){
    const data=f32(wasm.inst_ptr(),count*STRIDE),ranges=new Uint32Array(wasm.memory.buffer,wasm.ranges_ptr(),renderNamespace.rangeLength);
    actorMetadata=packMetadata?packMetadata.snapshot(packActorReader.snapshot({count,instances:data,namespace:renderNamespace})):null;
    originalGround.draw({data,ranges,vp,epochs:renderEpochs,isUrbanAt:(x,z)=>!!wasm.ground_urban_at(x,z)});
    gl.bindBuffer(gl.ARRAY_BUFFER,instBuf);
    gl.bufferData(gl.ARRAY_BUFFER,fallbackModelInstances(data,ranges),gl.STREAM_DRAW);
    drawFallbackModels(vp,ranges);
    originalModels.draw({data,ranges,vp,eye,zoom,epochs:renderEpochs,actorMetadata,preparedSailMetadata,capture:true,hercBeamMuzzles,packScene});
  }
  if(originalWater.enabled){
    originalWater.drawWater({vp,time:now/1000,fogTexture});
  }else{
    gl.enable(gl.BLEND);gl.depthMask(false);
    originalTerrain.drawWater({vao:vaoW,vp,time:now/1000,eye,fogTexture});
  }
  boatWake?.draw(vp,guidedImpactVisible);
  drawCaptureGlow(vp,actorMetadata);
  drawBlessGround(vp,actorMetadata);
  const impactDraws=guidedImpactFX?.draw(pipView,guidedImpactVisible);
  const beamDraws=hercBeam?.draw(vp,hercBeamMuzzles,actorMetadata,pipView,wasm);
  if(qaMode){canvas.dataset.guidedHercBeamPip=String(beamDraws??'unavailable');
    canvas.dataset.guidedHercBeamChildrenPip=JSON.stringify(hercBeam?.childDraws??null);}
  if(qaMode)canvas.dataset.guidedImpactPip=impactDraws?.join(':')??'unavailable';
  const crownAir=drawAirCrownStatus(vp,actorMetadata);
  if(qaMode)canvas.dataset.crownAirPip=String(crownAir);
  const extraction=drawSpyExtraction(pipView,wasm.sim_render_seconds?.(alpha));
  if(qaMode&&extraction!==null)canvas.dataset.spyPipExtraction=extraction;
  const priestDraws=drawPriestEffects(pipView,actorMetadata,wasm.sim_render_seconds?.(alpha));
  if(qaMode&&priestDraws){
    canvas.dataset.priestPipParticles=`${priestDraws.particleQuads}:${priestDraws.particleTargets}`;
    canvas.dataset.rallyHealPip=String(priestDraws.rallyDraws);
  }
  drawAirMissionFlags(vp,actorMetadata,mission);
  carrierGarrisonFlag?.draw({vp,actors:actorMetadata?.actors,epochs:renderEpochs,local:ui?.local});
}

function fallbackModelInstances(data,ranges) {
  const fallback=renderNamespace.fallbackInstances(data);
  const masked=originalModels.maskWonderScaffoldFallback?.(data,ranges,fallback)??0;
  if(qaMode)canvas.dataset.wonderScaffoldMasked=String(masked);
  return fallback;
}

function drawFallbackModels(vp,ranges) {
  gl.enable(gl.BLEND);
  gl.useProgram(models.p);gl.bindVertexArray(vaoM);gl.bindBuffer(gl.ARRAY_BUFFER,instBuf);
  gl.uniformMatrix4fv(models.u.uVP,false,vp);gl.uniform1i(models.u.uShadow,0);
  for(let id=0;id<renderNamespace.slotCount;id++){
    if(originalModels.available.has(id))continue;
    const template=renderNamespace.fallbackModelId(id);
    if(template<0||template>=240)continue;
    const count=ranges[id*2+1];if(!count||!mesh.count[template])continue;
    instOffset(ranges[id*2]);gl.drawArraysInstanced(gl.TRIANGLES,mesh.first[template],mesh.count[template],count);
  }
}

function frame(now) {
  const wallSeconds = (now - last) / 1000;
  const dt = Math.min(0.1, Math.max(0, wallSeconds)); last = now;
  if (!running) { sfx.menuMusic(); return; }
  if (worldMaterialPending) return;
  if(gameSettings.active())sfx.menuMusic();
  else sfx.music(!paused, ui?.epoch ?? initialMusicEpoch, musicCivilization);
  frameMs += (dt * 1000 - frameMs) * 0.05;
  worstMs = Math.max(worstMs * 0.995, dt * 1000);

  const pan = fullmap?.isOpen()?0:22 / cam.zoom * 60 * dt * (keys.has('Shift') ? 2 : 1);
  const rotating=referenceMode&&!fullmap?.isOpen()&&!ghost&&keys.has('Control');
  if(rotating){
    // Installed hotkeys bind Ctrl+arrows; angular speed is an input adapter.
    const turn=(keys.has('ArrowLeft')?1:0)-(keys.has('ArrowRight')?1:0);
    if(turn)rotateOriginalCameraHeading(cam,turn*dt*Math.PI/2*(keys.has('Shift')?3:1));
    updateCamera();
  }
  let moveRight=0,moveDown=0;
  if ((!rotating&&keys.has('ArrowLeft')) || (mouse.inside && mouse.x < 4)) moveRight-=pan;
  if ((!rotating&&keys.has('ArrowRight')) || (mouse.inside && mouse.x > innerWidth - 5)) moveRight+=pan;
  if ((!rotating&&keys.has('ArrowUp')) || (mouse.inside && mouse.y < 4)) moveDown-=pan;
  if ((!rotating&&keys.has('ArrowDown')) || (mouse.inside && mouse.y > innerHeight - 5)) moveDown+=pan;
  panCameraScreen(moveRight,moveDown);
  updateCamera();

  const visualSeconds = advance(dt, now);
  guidedImpactFX?.advance(wasm.sim_elapsed_seconds(),airMissionFlag?.normalWind());
  const hercBeamMuzzles=hercBeam?new Map():null;
  updateInitialCeasefire();
  if(ambientCoverage){
    const audible=!paused&&!gameSettings.active()&&!document.hidden;
    sfx.frameOriginalAmbience({world:wasm.render_meta_session(),climate:seasonSession.climate,now:now/1000,
      weather:'Normal',running:audible,refreshForest:ambientCoverage.refreshForest,
      priorities:audible?originalAmbientPriorities(ambientCoverage,view,u8(wasm.fog_ptr(),MAP*MAP)):null});
  }
  if(captureGlow&&!paused&&!(net&&net.ls.waiting)&&wasm.has_active_capture_glow())captureGlow.advance(visualSeconds);
  blessGround?.advance(visualSeconds);
  const interpolation = net?.sourceRounds ? net.ls.interpolation()
    : adaptiveSoloClock ? adaptiveSoloClock.remainder / sourceWallInterval(adaptiveSoloClock.rate)
    : acc / (sourceSoloClock ? SOURCE_SOLO_INTERVAL : TICK);
  const alpha = paused || (net && net.ls.waiting) ? 1 : Math.min(1, interpolation);
  if(wakeEnabled){
    const poses=readNavalWakeActors(wasm,alpha,boatWake.fleet);
    const actors=boatWake.fleet?poses.flatMap(actor=>{
      const source=originalModels.boatWakeSourceForActor(actor,wasm.player_civilization(actor.team));
      const wakeHull=boatWake.hullForActor(actor,source);
      return wakeHull?[{...actor,wakeHull}]:[];
    }):poses;
    boatWake.advance({world:wasm.render_meta_session(),delta:paused||document.hidden||net?.ls.waiting?0:visualSeconds,
      actors,views:[view.vp,fullmapPip.admissionProjection()],visible:guidedImpactVisible});
    if(qaMode)canvas.dataset.boatWake=JSON.stringify({...boatWake.stats,hulls:actors.map(({handle,wakeHull})=>
      ({handle,visual:wakeHull?.visual,tags:wakeHull?.tags.length}))});
  }
  hercBeam?.advance(wasm.sim_elapsed_seconds(),wasm,alpha);
  if (!net && !bench && now - lastAuto > 180000) { lastAuto = now; saveGame(true); }

  const dpr = Math.min(+($('quality').value), devicePixelRatio || 1);
  const w = Math.round(innerWidth * dpr), h = Math.round(innerHeight * dpr);
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  gl.viewport(0, 0, w, h);

  // textures that change
  if (wasm.map_dirty()&&!valleyWorld) { bindTexture(1,mapTexture); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, u8(wasm.map_ptr(), MAP * MAP * 4)); uploadOriginalTerrainClasses(); fullmap?.invalidate(); }
  if (wasm.terr_dirty()) { const p = wasm.terr_ptr(); bindTexture(3,territoryTexture); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, u8(p, MAP * MAP * 4)); fullmap?.invalidateTerritories(); }
  const tickNow = wasm.game_tick()>>>0;
  playerFlare.update(tickNow,dt);
  battleFlare?.update(dt);
  const seasonPhase=seasonSession.update();
  if(qaMode)canvas.dataset.season=JSON.stringify(seasonPhase);
  if (fogTick !== tickNow || fogTick < 0) {
    fogTick = tickNow;
    bindTexture(2,fogTexture); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RED, gl.UNSIGNED_BYTE, u8(wasm.fog_ptr(), MAP * MAP));
  }

  gl.depthMask(true);
  gl.clearColor(0.012, 0.03, 0.05, 1); gl.clearStencil(0); gl.stencilMask(255); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
  gl.enable(gl.DEPTH_TEST); gl.disable(gl.BLEND);
  const px = 1 / cam.zoom;

  const [x0, y0, x1, y1] = viewRect();
  let gid = -1, gx = 0, gy = 0;
  let farmPose=ghost?.nativeCursor?farmCursorPose(undefined,now):null;
  if(farmPose&&wasm.valley_farm_cursor_force_rotation){
    const step=farmAngleMemory.input(keys.has('Control'),now);
    if(step!==null){
      wasm.valley_farm_cursor_force_rotation(step);
      farmPose=farmCursorPose(undefined,now);
    }
  }
  const ghostAt=ghost&&!ghost.nativeCursor?ghostTile():null;
  if (ghostAt) { gid = ghost.id; [gx, gy] = ghostAt; }
  updatePlacement(now,farmPose);
  const n = farmPose?wasm.render_valley_farm_cursor(alpha,x0,y0,x1,y1):
    (wasm.render_rotated||wasm.render)(alpha, x0, y0, x1, y1, gid, gx, gy,ghost?.turn||0);
  if (!n) originalModels.clearResourcePicks();
  const ranges = n ? new Uint32Array(wasm.memory.buffer, wasm.ranges_ptr(), renderNamespace.rangeLength) : null;
  const renderEpochs=[0,1,2,3].map(t=>wasm.stat(t,0));
  if(spyShimmer||diversionShimmer){
    const frame=++worldRenderFrame,seat=ui?.local??0,seconds=wasm.sim_elapsed_seconds();
    spyShimmer?.begin(frame,seat,seconds);
    diversionShimmer?.begin(frame,seat,seconds);
  }
  if(airTakeoff){takeoffSample.tick=wasm.game_tick()>>>0;takeoffSample.alpha=alpha;takeoffSample.now=sourceRenderSeconds(wasm,alpha);}
  const stoneThrowerWheels=originalEnabled&&rulesProfile.ruleset===2&&!paused&&!document.hidden&&!(net&&net.ls.waiting)?[]:null;
  if(rulesProfile.ruleset===2)originalModels.setVisualCivilizations?.([0,1,2,3].map(team=>wasm.player_civilization(team)));

  // Own visual clock: once before every scene pass. Pause/wait freezes cloth;
  // solo speed scales the explicit delta. Native helpers apply their own cap.
  let actorMetadata=sourceSails?.advance({count:n,instances:f32(wasm.inst_ptr(),n*STRIDE),epochs:renderEpochs,
    deltaSeconds:visualSeconds,runPhysics:!paused&&!(net&&net.ls.waiting)})??null;
  if(!actorMetadata&&packMetadata)actorMetadata=packMetadata.snapshot(packActorReader.snapshot({count:n,instances:f32(wasm.inst_ptr(),n*STRIDE),namespace:renderNamespace}));
  takeoffAudio?.update({...takeoffSample,session:actorMetadata?.session,actors:actorMetadata?.actors??[],epochs:renderEpochs,view,
    running:!paused&&!document.hidden&&!(net&&net.ls.waiting)});
  if(animationAudio)sfx.frameAirFlightAudio({...readAirFlightAudio(wasm,alpha),world:wasm.render_meta_session(),view,
    running:!paused&&!document.hidden&&!(net&&net.ls.waiting)});
  mrlsPack?.frame({actors:actorMetadata?.actors??[],epochs:renderEpochs,view,
    running:!paused&&!document.hidden&&!(net&&net.ls.waiting)});
  hercAttack?.update(wasm.sim_elapsed_seconds(),!paused&&!document.hidden);
  let mission=null;
  if(airMissionFlag){
    const draft=airMission?.active()?airMission.preview(mouse.inside&&document.elementFromPoint(mouse.x,mouse.y)===canvas?toWorld(mouse.x,mouse.y):null):null;
    const records=readAirMissionVisualRoutes(wasm).filter(route=>draft?draft.points.length||draft.target!==0xffffffff:
      route.points.length||route.target!==0xffffffff);
    if(airMissionFlag.prepare(records,{deltaSeconds:visualSeconds,runPhysics:!paused&&!(net&&net.ls.waiting),lighting:originalModels.lighting}))mission={records,draft};
  }
  carrierGarrisonFlag?.begin({session:wasm.render_meta_session(),deltaSeconds:visualSeconds,
    runPhysics:!paused&&!(net&&net.ls.waiting),lighting:originalModels.lighting,wind:airMissionFlag?.normalWind(),occupied:carrierOccupied});
  if(qaMode&&sourceSails)canvas.dataset.sourceSails=JSON.stringify(sourceSails.metrics);
  let reflection=null;
  if(!valleyWorld&&originalTerrain.enabled&&!reflectionError){
    const scale=+$('quality').value>1?1:.5;
    try{
      reflection=originalReflection.render({vp:view.vp,eye:view.eye,width:Math.max(1,Math.round(w*scale)),height:Math.max(1,Math.round(h*scale)),
        drawScene:pass=>{
          originalTerrain.drawTerrain({vao:vaoT,px,fogTexture,territoryTexture,...pass});
          if(n)originalModels.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,epochs:renderEpochs,zoom:cam.zoom,actorMetadata,packScene,...pass});
        }});
    }catch(error){reflectionError=String(error);console.warn('Water reflection:',error);}
    if(qaMode){canvas.dataset.reflection=JSON.stringify(originalReflection.metrics);canvas.dataset.reflectionError=reflectionError||'';}
  }

  bindTexture(2,fogTexture);
  let volumeShadowInstances=null;
  if(valleyWorld){
    const quality=+$('quality').value;
    const cities=quality>1&&n&&valleyPreview.cityShadowAvailable ? originalModels.cityShadowCasters?.({
      data:f32(wasm.inst_ptr(),n*STRIDE),ranges,epochs:renderEpochs,zoom:cam.zoom,actorMetadata}) : null;
    volumeShadowInstances=valleyPreview.drawTerrain(view.vp,fogTexture,quality>1,quality>=2,[x0,y0,x1,y1],cities??[]);
    valleyPreview.drawWorld(view,[x0,y0,x1,y1]);
  }else if(originalTerrain.enabled){
    originalTerrain.drawTerrain({vao:vaoT,vp:view.vp,px,fogTexture,territoryTexture});
  }else{
    bindTexture(1,mapTexture);bindTexture(3,territoryTexture);bindTexture(4,originalAssets.terrainTexture,gl.TEXTURE_2D_ARRAY);
    gl.useProgram(terrain.p); gl.bindVertexArray(vaoT);
    gl.uniformMatrix4fv(terrain.u.uVP, false, view.vp); gl.uniform1f(terrain.u.uPx, px);
    gl.drawElements(gl.TRIANGLES, MAP * MAP * 6, gl.UNSIGNED_INT, 0);
  }

  if (n) {
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, fallbackModelInstances(f32(wasm.inst_ptr(), n * STRIDE),ranges), gl.STREAM_DRAW);
    originalGround.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,vp:view.vp,
      epochs:renderEpochs,isUrbanAt:(x,z)=>!!wasm.ground_urban_at(x,z)});
    if(qaMode)canvas.dataset.originalGroundDrawn=String(originalGround.metrics.instances);
    if(!paused && now-lastWorkSound>500){
      lastWorkSound=now;
      const instances=f32(wasm.inst_ptr(),n*STRIDE),workerId=renderNamespace.unitRenderId(8),first=ranges[workerId*2],count=ranges[workerId*2+1];
      let closest=Infinity,hit=-1;
      for(let j=first;j<first+count;j++){
        const k=j*STRIDE,action=instances[k+11];if(action!==2 && action!==3 && action!==5)continue;
        const d=Math.hypot(instances[k]-cam.x,instances[k+1]-cam.y);if(d<closest){closest=d;hit=k;}
      }
      if(hit>=0 && closest<22){const action=instances[hit+11];sfx.play(action===2?23:action===3?24:25,.55*(1-closest/22),(instances[hit]-cam.x)/22);}
    }
    // Planar geometry shadows, stamped once per pixel with stencil. Terrain
    // slopes remain approximate; this adds silhouettes rather than dark circles.
    if(gl.getContextAttributes().stencil && +$('quality').value>1){
      gl.enable(gl.BLEND); gl.depthMask(false); gl.enable(gl.STENCIL_TEST);
      gl.stencilFunc(gl.EQUAL,0,255); gl.stencilOp(gl.KEEP,gl.KEEP,gl.INCR);
      gl.useProgram(models.p); gl.bindVertexArray(vaoM);
      gl.uniformMatrix4fv(models.u.uVP,false,view.vp); gl.uniform1i(models.u.uShadow,1);
      for(let id=0;id<renderNamespace.slotCount;id++){
        if(originalModels.available.has(id))continue;
        const template=renderNamespace.fallbackModelId(id);
        if(template<0||template>=104||(template>=29&&template<100))continue;
        const cnt=ranges[id*2+1]; if(!cnt||!mesh.count[template])continue;
        instOffset(ranges[id*2]); gl.drawArraysInstanced(gl.TRIANGLES,mesh.first[template],mesh.count[template],cnt);
      }
      originalModels.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,vp:view.vp,epochs:renderEpochs,zoom:cam.zoom,eye:view.eye,shadow:true,actorMetadata,packScene,volumeShadowInstances});
      gl.disable(gl.STENCIL_TEST); gl.depthMask(true);
    }
    // one instanced draw per model type
    drawFallbackModels(view.vp,ranges);
    originalModels.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,vp:view.vp,epochs:renderEpochs,zoom:cam.zoom,eye:view.eye,actorMetadata,stoneThrowerWheels,hercBeamMuzzles,packScene});
    if(qaMode){
      canvas.dataset.wonderScaffolds=JSON.stringify(originalModels.metrics.wonderScaffolds??[]);
      canvas.dataset.wonderFences=JSON.stringify(originalModels.metrics.wonderFences??[]);
      canvas.dataset.guidedRocketDraws=String(originalModels.metrics.guidedRocketDraws??0);
    }
  }
  if(rulesProfile.ruleset===2)sfx.updateStoneThrowerWheels(actorMetadata?.session,stoneThrowerWheels,cam,referenceMode?view:undefined);

  // Native base-water depth/alpha followed by the separate reflection overlay.
  gl.enable(gl.BLEND); gl.depthMask(false);
  if(!valleyWorld&&originalWater.enabled){
    originalWater.drawWater({vp:view.vp,time:now/1000,fogTexture,reflectionTexture:reflection?.texture,reflectionVP:reflection?.vp});
    if(qaMode)canvas.dataset.originalWater=JSON.stringify(originalWater.metrics);
  }else if(!valleyWorld&&originalTerrain.enabled){
    originalTerrain.drawWater({vao:vaoW,vp:view.vp,time:now/1000,eye:view.eye,fogTexture,reflectionTexture:reflection?.texture,reflectionVP:reflection?.vp});
  }else if(!valleyWorld){
    bindTexture(1,mapTexture);bindTexture(4,originalAssets.terrainTexture,gl.TEXTURE_2D_ARRAY);
    gl.useProgram(water.p); gl.bindVertexArray(vaoW);
    gl.uniformMatrix4fv(water.u.uVP, false, view.vp); gl.uniform1f(water.u.uTime, now / 1000);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }
  boatWake?.draw(view.vp,guidedImpactVisible);
  drawCaptureGlow(view.vp,actorMetadata);
  drawBlessGround(view.vp,actorMetadata,{visualSeconds});
  const impactDraws=guidedImpactFX?.draw(view,guidedImpactVisible);
  const beamDraws=hercBeam?.draw(view.vp,hercBeamMuzzles,actorMetadata,view,wasm);
  if(qaMode){canvas.dataset.guidedHercBeamMain=String(beamDraws??'unavailable');
    canvas.dataset.guidedHercBeamChildrenMain=JSON.stringify(hercBeam?.childDraws??null);}
  if(qaMode)canvas.dataset.guidedImpactMain=impactDraws?.join(':')??'unavailable';
  const crownAir=drawAirCrownStatus(view.vp,actorMetadata);
  if(qaMode)canvas.dataset.crownAir=String(crownAir);
  const extraction=drawSpyExtraction(view,wasm.sim_render_seconds?.(alpha));
  if(qaMode&&extraction!==null)canvas.dataset.spyMainExtraction=extraction;
  const priestDraws=drawPriestEffects(view,actorMetadata,wasm.sim_render_seconds?.(alpha));
  if(qaMode&&priestDraws){
    canvas.dataset.priestEffects=String(priestDraws.meshDraws);
    canvas.dataset.priestParticles=`${priestDraws.particleQuads}:${priestDraws.particleTargets}`;
    canvas.dataset.rallyHeal=String(priestDraws.rallyDraws);
  }
  drawAirMissionFlags(view.vp,actorMetadata,mission);
  const hotspotDraws=originalGround.drawHotspot({vp:view.vp,points:hotspotPoints()});
  if(qaMode)canvas.dataset.hotspotDraws=String(hotspotDraws);
  carrierGarrisonFlag?.draw({vp:view.vp,actors:actorMetadata?.actors,epochs:renderEpochs,local:ui?.local});

  if (n) {
    gl.useProgram(quads.p); gl.bindVertexArray(vaoQ); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.uniformMatrix4fv(quads.u.uVP, false, view.vp); gl.uniform1f(quads.u.uPx, px);
    gl.uniform3fv(quads.u.uRight,view.right||[1,0,0]); gl.uniform3fv(quads.u.uUp, view.u);
    for (const template of [243, 240, 242, 241]) {
      const id=renderNamespace.quadRenderId(template);            // shadows, rings, explosions, then bars on top of everything
      const cnt = ranges[id * 2 + 1];
      if (!cnt) continue;
      if (template === 241) gl.disable(gl.DEPTH_TEST);
      instOffset(ranges[id * 2]);
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, cnt);
    }
  }

  if (now - lastUi > 100) { lastUi = now; updateUi(n, now); }
  crownVictoryRows.animate(now);
  if (now - lastMini > 250) { lastMini = now; drawMini(); }
  fullmap?.paint(now,ui,corners);
  if(fullmapPip.isOpen())fullmapPip.draw(dt,pipView=>drawFullmapPipScene(pipView,alpha,renderEpochs,now,mission,actorMetadata));
  finishPriestEffectsFrame();
  if(supportsAdaptiveSoloClock)sampleSourceRenderFps(adaptiveRenderSample,wallSeconds);
}

// ---------------------------------------------------------------- HUD
const HOT = ['Q', 'W', 'E', 'R', 'T', 'Z', 'X', 'C', 'V', 'B'];
const unitPresentation=id=>id===kothDefinition?kothPresentation:id===43?artemisPresentation:id===heavyMounted3Id?heavyMounted3Presentation:id===fishingId?fishingPresentation:id>=102&&id<=105?diversionPresentation:id>=87&&id<=89?leaderPresentation:id===86?spyPresentation:id===85?priestPresentation:id===84?carrierPresentation:id===83?fighterBomberPresentation:tradePresentation;
function icon(id, px,context=undefined) {
  const wonderUrl=wonderPresentation.iconFor(id);
  if(wonderUrl){
    const img=document.createElement('img');img.src=wonderUrl;img.alt='';img.width=img.height=px;img.style.objectFit='contain';return img;
  }
  const c = document.createElement('canvas'); c.width = c.height = px;
  const regional = ui && rulesProfile.ruleset === 2
    ? regionalIcons.get(id)?.icon(ui.epoch, wasm.player_civilization(ui.local) >>> 0) : undefined;
  const original=regional??unitPresentation(id)?.icon?.(id,ui?.epoch??0,ui?wasm.player_civilization(ui.local)>>>0:0)??originalIcon(originalAssets,id,ui?.epoch||0,context);
  if(original){
    const scale=Math.min(px/original.width,px/original.height),w=original.width*scale,h=original.height*scale;
    c.getContext('2d').drawImage(original,(px-w)/2,(px-h)/2,w,h);return c;
  }
  const fallbackId=context?.sourceIdentityVersion===1?-1:renderNamespace.unitFallbackId(id);
  if(fallbackId>=0)c.getContext('2d').drawImage(iconAtlas, (fallbackId % 16) * CELL, ((fallbackId / 16) | 0) * CELL, CELL, CELL, 0, 0, px, px);
  return c;
}
const glyph = (ch, px) => { const d = document.createElement('div'); d.style.cssText = `font-size:${px * 0.66}px;height:${px}px;line-height:${px}px`; d.textContent = ch; return d; };
const GLYPH = { e: '⬆', r: '⚗', i: '⬆', u: '⚓', p:'⇄' };
const LABELS = {'Worker':'Bürger','Town Center':'Stadtzentrum','House':'Haus','Farm':'Bauernhof','Barracks':'Kaserne','Workshop':'Werkstatt','Dock':'Hafen','Airfield':'Flugplatz','Watchtower':'Wachturm','Next Epoch':'Nächste Epoche','University':'Universität','Mill':'Mühle','Stable':'Stall','Market':'Markt','Temple':'Tempel','Warehouse':'Lagerhaus','Oil Rig':'Bohrturm','Uranium Mine':'Uranmine','HercFacility':'HERC-Fabrik','Fortress':'Festung','Trade Cart':'Handelskarren','Merchant Ship':'Handelsschiff'};
const label = (name,id,count=1) => sourceResearch.unitName(wonderPresentation.nameFor(id)??unitPresentation(id)?.name(id,ui?.epoch??0,count)??originalLabels.name(id,ui?.epoch??0,LABELS[name]||name,count),id,ui?.local);
const actionUnitId = a => a&&(a.k==='b'||a.k==='t')?a.id:undefined;
const ACTIVITIES = ['Bereit','Unterwegs','Holz fällen','Erz abbauen','Nahrung sammeln','Bauen','Im Kampf'];
function costText(c) {return c.flatMap((value,slot)=>value?[`${value} ${!referenceMode&&slot===2?'Metall':RESOURCE_NAMES[slot]}`]:[]).join(' · ')||'Kostenlos';}

function updateUi(drawn, now) {
  const p = wasm.ui();
  ui = JSON.parse(dec.decode(u8(p, wasm.ui_len())));
  crownProgress?.update(ui.local);
  $('observer-status').hidden=!ui.observer;
  if(ui.observer&&ghost)cancelBuilding();
  tradeRoute?.validate();
  airMission?.validate();
  if(typeof wasm.pack_selected_count==='function'&&wasm.pack_selected_count()>0){
    ui.act.push({k:'p',id:0,name:'Einpacken',ok:true,tip:'Die ausgewählten Einheiten für den Marsch einpacken.'},
      {k:'p',id:1,name:'Auspacken',ok:true,tip:'Die ausgewählten Einheiten in Stellung bringen.'});
  }
  ui.act.push(...sourceResearch.selectedImprovements(ui.epoch));
  for (let i = 0; i < 3; i++) $('r' + i).textContent = ui.res[i];
  economyBar.update(ui);
  $('pop').textContent = `${ui.pop}/${ui.cap}`;
  $('epoch').textContent = ui.ename;
  $('science').textContent = referenceMode ? `${ui.research.points.toFixed(1)} TP` : '';
  $('terr').textContent = ui.terr;
  const territoryVictory=$('territory-victory-status'), territory=ui.territoryVictory;
  territoryVictory.hidden=!territory;
  territoryVictory.textContent=territory
    ? `Kontrolliert ${territory.owned} von ${territory.total} · Ziel ${territory.target} · noch ${Math.max(0,territory.target-territory.owned)}`
    : '';
  const soleStatus=$('sole-survivor-status'), sole=ui.soleSurvivor;
  const regicide=ui.regicide;
  soleStatus.hidden=(!sole&&!regicide)||!!ui.win;
  const soleText=!sole ? '' : sole.seconds>=0
    ? `Bündnisse werden in ${sole.seconds} Sekunden aufgelöst!`
    : sole.players===2 ? 'Einziger Überlebender · zwei Spieler übrig' : `Einziger Überlebender · ${sole.players} Spieler übrig`;
  const regicideText=regicide ? (regicide.kingAlive ? 'Königsmord · König lebt' : 'Königsmord · König gefallen') : '';
  soleStatus.textContent=[regicideText,soleText].filter(Boolean).join(' · ');
  const kothStatus=$('koth-victory-status'), hillHolder=wasm.koth_status?.(2);
  kothStatus.hidden=!(hillHolder>=0&&hillHolder<4)||!!ui.win;
  kothStatus.textContent=kothStatus.hidden?'':formatKothStatus(hillPlayerName(hillHolder),wasm.koth_status(3),
    kothPresentation?.label('tx_ig_koth_status')??undefined);
  const showVictoryStatus=!ui.win&&!fullmap?.isOpen();
  const players=ui.diplomacy?.players;
  const objective=showVictoryStatus?(readCapitolHud(wasm,players,ui.local,sole?.seconds)
    ??readCrownVictoryHud(wasm,players,ui.local)):null;
  crownVictoryRows.update(objective);
  const objectiveFallback=$('crown-victory-hud');
  objectiveFallback.hidden=!objective||typeof nativeHud.updateVictoryStatus==='function';
  if(!objectiveFallback.hidden){
    objectiveFallback.firstElementChild.textContent=objective.caption;
    objectiveFallback.setAttribute('aria-label',objective.caption);
  }
  const hotspotRows=$('hotspot-status-rows'), hotspotStatus=showVictoryStatus&&hotspotTotal()>0;
  hotspotRows.hidden=!hotspotStatus;
  $('hotspot-status-hud').hidden=!hotspotStatus||typeof nativeHud.updateVictoryStatus==='function';
  if(hotspotStatus){
    const text=[`Entdeckte Krisenherde: ${hotspotRevealed()}/${hotspotTotal()}`,...(ui.diplomacy?.players??[])
      .map((player,seat)=>`${hillPlayerName(seat)}: ${wasm.hotspot_status(2,seat)}`)].join('\n');
    if(hotspotRows.textContent!==text)hotspotRows.textContent=text;
  }
  if(hotspotRevealed()>knownHotspotCount)toast('Ein Krisenherd wurde entdeckt!');
  knownHotspotCount=hotspotRevealed();
  nativeHud.updateVictoryStatus?.(wasm.koth_status?.(0)===1&&showVictoryStatus,objective?.caption??null,hotspotStatus);
  $('idle').textContent = `Freie Bürger: ${ui.idle}`;
  $('idle').style.visibility = ui.idle && !bench ? 'visible' : 'hidden';
  let status = `${Math.round(1000 / frameMs)} fps · worst ${worstMs.toFixed(0)} ms · sim ${simMs.toFixed(2)} ms/tick · ${ui.n} entities · ${drawn} drawn`;
  if(bench)status+=` · originals CPU ${(originalModels.metrics.colorMs+originalModels.metrics.shadowMs).toFixed(1)} ms / ${originalModels.metrics.calls} draws / ${(originalModels.metrics.triangles/1000).toFixed(0)}k tris / GPU ${originalModels.metrics.gpuMs.toFixed(1)}ms`;
  if (net) {
    if(net.sourceRounds)status=sourceRoundStatus(net.ls)+' · '+status;
    else if (net.ls.desync) status = '⚠ OUT OF SYNC with the other player · ' + status;
    else if (net.ls.alone) status = 'other player left · ' + status;
    else if (net.ls.waiting && now - waitSince > 600) status = 'waiting for the other player… · ' + status;
    else status = 'multiplayer · ' + status;
  }
  $('perf').textContent = status;

  const s = ui.sel, info = $('info-content');
  const militaryLeader=militaryLeaderControl(s,ui.epoch);
  const powerRead=s.def===85?wasm.priest_power:s.def===86?wasm.spy_power:s.def>=87&&s.def<=89?wasm.leader_power_reserve:null;
  const airStatus=airMission&&!bench&&!ui.win?airMissionSelection(wasm):null;
  tradeWaitStatus.update(!ui.win&&selectedTradeWait(wasm,s),ui.epoch);
  if (!s.n) info.innerHTML = bench ? '<h3>Stress test</h3><span class="dim">Two armies are kept at full strength and thrown at each other. Zoom and pan freely; you can still select and command the blue side.</span>'
    : `<span class="dim">Keine Einheit ausgewählt</span>${ui.techs.length ? '<div class="types">' + ui.techs.map(t => `<span>${t}</span>`).join('') + '</div>' : ''}`;
  else {
    const selectedCapitol=s.n===1&&s.def===0&&wasm.capitol_status?.(0,0)>0&&
      [0,1,2,3].some(seat=>wasm.capitol_status(4,seat)===wasm.selected_actor_handle());
    let h = `<h3>${s.n > 1 && s.types.length > 1 ? s.n + ' Einheiten' : escapeLabelHtml(label(s.name,s.def,s.n)+(selectedCapitol?' (Kapitol)':'')) + (s.n > 1 ? ' ×' + s.n : '')}</h3>`;
    if (s.n === 1) {
      h += `<div class="hp"><i style="width:${100 * s.hp / s.max}%"></i></div>${s.hp} / ${s.max} HP`;
      if (powerRead&&!bench&&!ui.win) {
        const power=powerRead(wasm.selected_actor_handle()>>>0);
        if (power>=0) h+=`<div class="hp priest-power" role="meter" aria-label="Kraftreserve" aria-valuemin="0" aria-valuemax="1" aria-valuenow="${power}"><i style="width:${100*Math.max(.001,power)}%"></i></div>`;
      }
      if (renderNamespace.isMobile(s.def)) h += `<div class="activity">${ACTIVITIES[s.activity] || 'Bereit'}</div>`;
      if (s.dmg) h += ` · ${s.dmg} dmg · range ${s.rng}`;
      if (s.built < 1) h += `<br>Im Bau: ${Math.round(s.built * 100)}%`;
    }
    if(referenceMode&&s.def===5&&s.cargomax)h+=`<br>Liegeplätze: ${s.cargo} / ${s.cargomax}`;
    else if(referenceMode&&s.def===56&&s.cargomax)h+=`<br>Garnison: ${s.cargoUnits??s.cargo} ${(s.cargoUnits??s.cargo)===1?'Einheit':'Einheiten'} · ${s.cargo} / ${s.cargomax} Plätze<br>Angriffsbonus: +${s.garrisonAttackBonus??s.cargo*3} %${s.garrisonHealRate?` · Heilung ${s.garrisonHealRate} HP/s`:""}`;
    else if (s.cargomax) h += s.def===47 ? `<br>Forscher: ${s.cargo} / ${s.cargomax}<br>Forschung insgesamt: +${(ui.research.rate*60).toFixed(1)} TP / Minute` : s.def===53||s.def===54 ? `<br>Förderarbeiter: ${s.cargo} / ${s.cargomax}<br>+${(s.cargo*.75).toFixed(2)} ${s.def===53?'Öl':'Uran'} / Sekunde` : referenceMode&&(s.def===0||s.def===52) ? `<br>Einquartiert: ${s.cargo} / ${s.cargomax}<br>Abgabebonus: +${s.cargo*2} %` : `<br>Passagiere: ${s.cargo} / ${s.cargomax}`;
    if(s.n===1&&airStatus?.kind===1&&airStatus.complete)h+=`<br>Stationierte Flugzeuge ${airStatus.count} / ${airStatus.max}`;
    if(s.n===1&&airStatus?.kind===2)h+=`<br>Munition ${airStatus.count} / ${airStatus.max}`;
    if(s.carrying>0)h+=`<br>${s.returning?'Bringt zurück':'Trägt'}: ${s.carrying.toFixed(1)} ${RESOURCE_NAMES[s.carriedResource-1]}`;
    if (s.types.length > 1) h += '<div class="types">' + s.types.map(t => `<span>${escapeLabelHtml(label(t[2],t[0],t[1]))} ×${t[1]}</span>`).join('') + '</div>';
    if (s.q.length) h += `<div class="q" id="queue"></div><div class="hp"><i style="width:${s.qp * 100}%;background:#4aa3ff"></i></div>`;
    if (s.populationBlocked) h += '<div class="dim">Wartet auf freie Bevölkerungsplätze</div>';
    info.innerHTML = h;
    if (s.n === 1) {
      const context = selectedCitizenContext(wasm, ui);
      const selected=(wasm.selected_actor_handle?.()??0xffffffff)>>>0;
      if(crownStatusArt&&wasm.crown_visuals_present?.()){
        const viewer=ui.local;
        const status=readCrownActorVisuals(wasm,crownStatusArt.rows,{handle:selected,owner:viewer,viewer});
        if(status.activeRows.length){
          const tray=document.createElement('div');tray.className='crown-effect-tray';
          for(const [slot,row] of crownStatusSlots(crownStatusArt.rows,status.activeRows,viewer,viewer).entries()){
            const source=crownStatusArt.badge(row);
            if(!source)continue;
            const badge=document.createElement('img');badge.src=source;badge.width=32;badge.height=32;
            badge.alt=row.title?.text??row.power;badge.title=row.description?.text??badge.alt;
            badge.dataset.crownRow=row.id;badge.style.gridColumn=String(slot+1);tray.append(badge);
          }
          info.querySelector('h3')?.after(tray);
        }
      }
      const toxic=wasm.spy_toxic_status?.(selected)??0;
      const effects=[];
      if(wasm.building_sabotaged?.(selected))effects.push(['sabotage','Sabotiert']);
      if(toxic&1)effects.push(['contamination','Kontaminiert']);
      if(toxic&2)effects.push(['contamination','Toxische Krankheit']);
      if(wasm.imperialism_visible?.(selected))effects.push(['imperialism',imperialismArt.label()]);
      for(const [key,label] of effects){
        const tag=document.createElement('span');tag.className='activity';tag.title=label;tag.dataset.effect=key;
        const source=key==='imperialism'?imperialismBadge:spyAdvancedArt?.status(key);
        if(source){const img=document.createElement('img');img.src=source;img.alt=label;img.width=22;img.height=22;tag.append(img);}
        else tag.textContent=label;
        info.querySelector('h3')?.append(tag);
      }
      if ((context && citizenPresentation.enabled) || regionalPortraits.has(s.def) || wonderPresentation.iconFor(s.def) || unitPresentation(s.def)?.icon?.(s.def,ui.epoch)) {
        const portrait = icon(s.def, 40, context);
        portrait.style.cssText = 'vertical-align:middle;margin-right:6px';
        info.querySelector('h3')?.prepend(portrait);
      }
    }
    const q = $('queue');
    if (q) { for (const id of s.q) {const job=renderNamespace.queueJob(id);q.append(!job?glyph('?',30):job.kind===3?glyph('⬆',30):job.kind===1||job.kind===2?glyph('⚗',30):icon(job.value,30));} q.title = 'Rechtsklick oder Rücktaste: letzten Auftrag abbrechen'; }
  }

  const canCapture=!!captureOrder&&!bench&&!ui.win&&militaryCaptureCount()>0;
  const canTrade=!!tradeRoute&&!bench&&!ui.win&&wasm.trade_selected_count()>0;
  const powerActions=bench||ui.win?[]:powerOrders.filter(command=>selectedPowerType(s,command.definition)&&command.available?.()!==false)
    .map(command=>({command,ready:command.count()>0}));
  const spyExtraction=extractionAvailable&&!bench&&!ui.win&&selectedPowerType(s,86);
  const canExtract=spyExtraction&&wasm.spy_extraction_selection_count()>0;
  const flightActions=airMissionActions(airStatus), airArmed=!!airMission?.active();
  const market=referenceMode&&!bench&&!ui.win&&s.n===1&&s.def===50
    ?marketOrders(wasm,wasm.selected_actor_handle()>>>0):[];
  const dockSelected=s.def===5&&s.n===1;
  const sig = ui.epoch+':'+dockSelected+':'+ui.act.map(a => a.k + a.id + label(a.name,actionUnitId(a))).join(',')+':'+canCapture+':'+canTrade+
    ':'+airStatus?.kind+':'+flightActions.map(action=>action.role+action.ready).join(',')+':'+airStatus?.auto+':'+airArmed+
    ':'+powerActions.map(({command,ready})=>command.action+ready).join(',')+':'+spyExtraction+':'+canExtract+
    ':'+militaryLeader?.mode+':'+militaryLeader?.ready+
    ':'+market.map(order=>`${order.host}/${order.resource}/${order.side}/${order.normal}/${order.shift}`).join(',');
  const acts = $('acts');
  acts.classList.toggle('has-market',market.length>0);
  if (sig !== actSig) {
    actSig = sig; acts.innerHTML = '';
    ui.act.forEach((a, i) => {
      const b = document.createElement('button'); b.className = 'act'; b.dataset.i = i;
      const dockIcon=dockSelected&&a.k==='u'?ungarrisonIcon(dockArt,nativeMetaEpoch(ui.epoch)):null;
      b.append(dockIcon??(GLYPH[a.k] ? glyph(GLYPH[a.k], 44) : icon(a.id, 44)));
      const sm = document.createElement('small'); sm.textContent = label(a.name,actionUnitId(a)); b.append(sm);
      const shortcut=a.k==='u'?'U':a.k==='t'&&a.id===86?'S':airMissionTrainingKey(airStatus,a)??HOT[i];
      if (shortcut && !(canCapture && shortcut === captureOrderText.shortcut) &&
          (airStatus?.kind!==1||!['R','F','T','B'].includes(shortcut)||airMissionTrainingKey(airStatus,a))) {
        const k = document.createElement('kbd'); k.textContent = shortcut; b.append(k);
      }
      b.onclick = () => doAct(i);
      b.oncontextmenu = e => {
        e.preventDefault();const detail=sourceResearch.forAction(ui.act[i]);
        if(detail){if(sourceResearch.cancel(detail.id)){sfx.play(20);lastUi=0;}}
        else wasm.cmd_cancel();
      };
      b.onmouseenter = () => {
        const a = ui.act[i]; if (!a) return;
        const t = $('tip'), detail=sourceResearch.forAction(a);
        const cost=detail?researchCost(detail):a.tp?a.tp+' Technologiepunkte':a.k==='u'||a.k==='p'?'':costText(a.cost);
        const description=unitPresentation(actionUnitId(a))?.description(actionUnitId(a),ui.epoch)??originalLabels.description(actionUnitId(a),ui.epoch);
        const progress=detail&&[2,4].includes(detail.status)?'<br>'+escapeLabelHtml(researchState(detail,''))+' · Rechtsklick zum Abbrechen':'';
        t.innerHTML = `<b>${escapeLabelHtml(label(a.name,actionUnitId(a)))}</b> <span class="c">${cost}</span><br>${description?escapeLabelHtml(description)+'<br>':''}${escapeLabelHtml(a.tip)}${progress}`;
        t.style.display = 'block';
      };
      b.onmouseleave = () => $('tip').style.display = 'none';
      acts.append(b);
    });
    if (canCapture) {
      const b = document.createElement('button'); b.className='act'; b.title=captureOrderText.description;
      captureOrder.decorateButton(b,ui.epoch);
      const caption=document.createElement('small');caption.textContent=captureOrderText.label;b.append(caption);
      const key=document.createElement('kbd');key.textContent=captureOrderText.shortcut;b.append(key);
      b.onclick=armCapture;acts.append(b);
    }
    for (const {command,ready} of powerActions) {
      const b=document.createElement('button');b.className=`act ${command.cssClass}`;b.dataset.action=command.action;
      b.title=command.art?.description?.(command.action)??command.description;
      b.disabled=!ready;b.classList.toggle('no',!ready);
      command.order.decorateButton(b,ui.epoch);
      const caption=document.createElement('small');caption.textContent=command.art?.label?.(command.action)??command.label;b.append(caption);
      const key=document.createElement('kbd');key.textContent=command.shortcut??`Alt+${command.key}`;b.append(key);
      b.onclick=()=>armPowerOrder(command);acts.append(b);
    }
    if(militaryLeader){
      const {action,ready,mode,label,help}=militaryLeader;
      const b=document.createElement('button');b.className='act';b.dataset.action=`leader-${mode}`;
      b.title=action?.help??help;
      b.disabled=!ready;b.classList.toggle('no',!ready);
      const picture=document.createElement('span');picture.className='leader-power-icon';
      picture.setAttribute('aria-hidden','true');
      for(const state of ['idle','hover','pressed'])if(action?.art?.[state])
        picture.style.setProperty(`--leader-power-${state}`,action.art[state]);
      if(!action?.art?.idle)picture.append(glyph('✚',44));
      const caption=document.createElement('small');caption.textContent=action?.label??label;
      const key=document.createElement('kbd');key.textContent=';';
      b.append(picture,caption,key);b.onclick=orderMilitaryLeaderPower;acts.append(b);
    }
    if(spyExtraction){
      const b=document.createElement('button');b.className='act spy-action';b.dataset.action='extraction';
      b.title=spyArt?.description('extraction')??'Platziert einen Rückzugsmarker für den Spion.';
      b.disabled=!canExtract;b.classList.toggle('no',!canExtract);
      const pictures=spyArt&&['idle','hover','pressed'].map(state=>spyArt.button('extraction',nativeMetaEpoch(ui.epoch),state));
      decorateCommandButton(b,{pictures,shortcut:'Alt+X',fallback:'↩'});
      const caption=document.createElement('small');caption.textContent=spyArt?.label('extraction')??'Extraktion';b.append(caption);
      const key=document.createElement('kbd');key.textContent='Alt+X';b.append(key);
      b.onclick=orderSpyExtraction;acts.append(b);
    }
    if (canTrade) {
      const b=document.createElement('button');b.className='act';b.title=tradeRouteText.title;
      tradeRoute.decorateButton(b,ui.epoch);
      const caption=document.createElement('small');caption.textContent=tradeRouteText.label;b.append(caption);
      const key=document.createElement('kbd');key.textContent='⇧T';b.append(key);
      b.onclick=armTradeRoute;acts.append(b);
    }
    for(const {role,ready} of flightActions) {
      const action=airActions[role], b=document.createElement('button');
      b.className='act air-mission-action';
      b.classList.toggle('no',!ready);
      decorateAirMissionButton(b,role,{art:airMissionArt,active:role==='set'&&airArmed,auto:airStatus.auto});
      if(action.key){const k=document.createElement('kbd');k.textContent=action.key;b.append(k);}
      b.onclick=()=>{if(ready){if(role==='set')armAirMission();else doAirMission(action.command);}};
      if(action.rightCommand)b.oncontextmenu=e=>{e.preventDefault();doAirMission(action.rightCommand);};
      acts.append(b);
    }
    if(market.length){
      const grid=document.createElement('div');grid.className='market-grid';
      grid.setAttribute('role','group');grid.setAttribute('aria-label','Marktaufträge');
      grid.append(...market.map(order=>marketOrderButton(order,submitMarketOrder,marketArt,nativeMetaEpoch(ui.epoch))));
      acts.append(grid);
    }
  }
  ui.act.forEach((action,i)=>acts.children[i]?.classList.toggle('no',!action.ok));
  sciencePanel.refresh(ui);
  diplomacy.refresh(ui);
  $('diplomacy-button').disabled=!!bench||!!ui.observer;
  nativeHud.update(ui,seasonSession.settings);

  if (ui.win && !bench && $('end').style.display !== 'flex') {
    fullmap?.close();
    diplomacy.close();
    if(ui.observer)$('endtext').textContent='Partie beendet';
    else if (rulesProfile.id === 'up15' && !net) {
      $('endtext').textContent = ui.win === 1 ? 'Sie haben gewonnen.' : 'Sie haben verloren';
    } else {
      $('endtext').textContent = ui.win === 1 ? 'Victory!' : 'Defeat';
    }
    $('end').style.display = 'flex';
    sfx.play(ui.win === 1 ? 13 : 11);
  }
  nativeBitmapFonts.update();
}

function submitMarketOrder(host,resource,lots){
  if(wasm.cmd_market_trade(host,resource,lots)){sfx.play(20);lastUi=0;}
  else sfx.play(21);
}

function doAct(i, count=1) {
  const a = ui && ui.act[i];
  if (!a) return;
  tradeRoute?.cancel();
  airMission?.cancel();
  if (a.k === 'b') {
    activeTargetOrder()?.cancel();
    const nativeFarm=a.id===2&&wasm.valley_farm_cursor_available?.()===1;
    const nativeHouse=a.id===1&&wasm.valley_house_cursor_reset?.()===1;
    const nativeCursor=nativeFarm||nativeHouse;
    ghost={id:a.id,size:a.size,name:label(a.name,a.id),turn:0,manual:false,nativeCursor};
    if(nativeCursor){
      farmAngleMemory.open(a.id);
      if(nativeFarm)wasm.valley_farm_cursor_reset(FARM_INITIAL_YAW);
    }else farmAngleMemory.close();
    placementKey='';attackMode=false;sfx.play(20);updatePlacement();return;
  }
  if (a.k === 'u') { wasm.cmd_unload(); sfx.play(22); return; }
  if(a.k==='p'){if(typeof wasm.cmd_pack==='function'&&wasm.cmd_pack(a.id)){if(!selectedUnitVoice('order'))sfx.play(20);}else sfx.play(21);lastUi=0;return;}
  if(a.k==='i'&&!a.ok){sfx.play(21);toast(researchState(a,a.hostName));return;}
  let queued=0;
  while(queued<count&&(renderNamespace.wide?wasm.cmd_queue_job(a.id):wasm.cmd_train(a.id)))queued++;
  if (queued) sfx.play(20);
  else { sfx.play(21); toast(a.ok ? 'Warteschlange voll' : a.k==='e' && referenceMode && ui.research.count<6 ? 'Zuerst sechs Technologien dieser Epoche erforschen' : a.tp && ui.research.points<a.tp ? 'Technologiepunkte fehlen – Bürger in Universitäten schicken' : a.k === 't' && !ui.latePopulation && ui.pop >= ui.cap ? 'Mehr Häuser bauen' : 'Rohstoffe fehlen'); }
}

const mini = $('mini'), mctx = mini.getContext('2d');
function drawMini() {
  const width=valleyWorld?.width??MAP,height=valleyWorld?.height??MAP;
  if(mini.width!==width||mini.height!==height){mini.width=width;mini.height=height;mini.style.objectFit=valleyWorld?'contain':'';}
  const pixels=valleyWorld?valleyMinimapSnapshot(wasm,valleyWorld):new Uint8ClampedArray(wasm.memory.buffer,wasm.minimap(),MAP*MAP*4);
  mctx.putImageData(new ImageData(pixels,width,height),0,0);
  for(const hotspot of hotspotPoints()){
    mctx.strokeStyle='#ffe389';mctx.lineWidth=2;mctx.beginPath();mctx.arc(hotspot[0],hotspot[1],4,0,2*Math.PI);mctx.stroke();
  }
  const c = corners();
  mctx.strokeStyle = '#fff'; mctx.lineWidth = 1.5; mctx.beginPath();
  c.forEach((q, i) => i ? mctx.lineTo(q[0], q[1]) : mctx.moveTo(q[0], q[1]));
  mctx.closePath(); mctx.stroke();
  playerFlare.draw(mctx,(x,y)=>[x,y],48*width/Math.max(1,mini.clientWidth));
  battleFlare?.draw(mctx,(x,y)=>[x,y],48*width/Math.max(1,mini.clientWidth));
}
function miniWorld(e) {
  const r=mini.getBoundingClientRect();
  if(valleyWorld)return valleyMinimapPoint(valleyWorld,e.clientX,e.clientY,{left:r.left+mini.clientLeft,top:r.top+mini.clientTop,
    width:mini.clientWidth,height:mini.clientHeight});
  return [(e.clientX-r.left)/r.width*MAP,(e.clientY-r.top)/r.height*MAP];
}
mini.addEventListener('mousedown', e => {
  if(modalOpen()||playerFlare.isArmed())return;
  if(ui?.observer&&e.button===2)return;
  if(airMission?.active()&&e.button===2)return;
  if(e.button===2&&(tradeRoute?.cancel()||activeTargetOrder()?.cancel()))return;
  const point=miniWorld(e);if(!point)return;
  const [x,y]=point;
  if (e.button === 2) { const attack=submitContextOrder(x,y); if (!selectedUnitVoice('order',attack)) sfx.play(22); } else { cam.x = x; cam.y = y; mini.dragging = true; }
});
addEventListener('mousemove',e=>{if(!modalOpen()&&mini.dragging){const point=miniWorld(e);if(point)[cam.x,cam.y]=point;}});
addEventListener('mouseup', () => mini.dragging = false);
mini.addEventListener('contextmenu', e => e.preventDefault());

// ---------------------------------------------------------------- input
const box = $('box');
const modalOpen=()=>$('cheat-dialog').open||$('research-dialog').open||diplomacy.isOpen()||fullmap?.isOpen()||gameOptions?.active();
gameOptions=await mountOriginalGameOptions({
  enabled:hasOriginalTempo,originalArt:originalEnabled,
  view:()=>({epoch:ui?.epoch??initialMusicEpoch,disabled:[...(!restartPoint||net?['restart']:[]),...(net?['load']:[]),
    ...(net&&!net.checkpointHost?['save']:[]),
    ...(net&&(!net.resignation||net.ls.hostLost||ui?.observer)?['resign']:[])]}),
  onOpen(){
    if(!running||bench||ui?.win||$('cheat-dialog').open||$('research-dialog').open||diplomacy.isOpen())return false;
    setPauseHold('options',true);keys.clear();cancelMapGesture();mouse.inside=false;
  },
  onClose(){setPauseHold('options',false);keys.clear();lastUi=0;},
  onAction:handleGameOption,
  onCue:cue=>{void sfx.playInterfaceCue(cue);},
});
function cancelMapGesture(){activeTargetOrder()?.cancel();tradeRoute?.cancel();airMission?.cancel();tradePress=null;airPress=null;mouse.down=false;mouse.drag=false;mouse.pan=false;mouse.inside=false;mini.dragging=false;box.style.display='none';canvas.style.cursor='default';}
const fullscreen=createFullscreenController({buttons:[$('fullscreen'),$('menu-fullscreen')],notify:toast,
  onChange(){keys.clear();cancelMapGesture();updateCamera();}});
const wakeDetailSelect=$('native-settings-effects'),wakeDetailKey='openepoch-effect-detail';
if(wakeEnabled){
  wakeDetailSelect.disabled=false;
  for(const option of wakeDetailSelect.options)option.disabled=!boatWake.supportsDetail(Number(option.value));
  try{boatWake.setDetail(Number(localStorage.getItem(wakeDetailKey)??'1'));}catch{/* optional local setting storage */}
}
const gameSettings=await mountOriginalGameSettings({
  enabled:hasOriginalTempo,originalArt:originalEnabled,
  read:()=>({...sfx.originalAudioSettings(),muted:sfx.isMuted(),voices:sfx.isSpeechEnabled(),
    resolution:$('quality').value,effectDetail:String(boatWake?.detail()??1),fullscreen:fullscreen.active}),
  previewAudio(settings){sfx.setOriginalAudioSettings(settings);updateAudioButtons();},
  apply(settings){
    sfx.setMuted(settings.muted);sfx.commitOriginalAudioSettings(settings);sfx.setSpeechEnabled(settings.voices);
    updateAudioButtons();$('quality').value=settings.resolution;
    if(!wakeDetailSelect.disabled){
      const detail=boatWake.setDetail(Number(settings.effectDetail));
      try{localStorage.setItem(wakeDetailKey,String(detail));}catch{/* optional local setting storage */}
    }
    if(fullscreen.active!==settings.fullscreen)void fullscreen.toggle();
  },
  hotkeys:()=>[
    {group:'Spiel',label:'Optionen',keys:['Esc','Pause']},
    {group:'Spiel',label:'Spiel anhalten oder fortsetzen',keys:'Umschalt+Pause'},
    {group:'Spiel',label:'Spieltempo verringern',keys:'Minus'},
    {group:'Spiel',label:'Spieltempo erhöhen',keys:'Gleich'},
    {group:'Spiel',label:'Starttempo wiederherstellen',keys:'Rücktaste'},
    {group:'Karte',label:'Übersichtskarte',keys:'Tab'},
    {group:'Karte',label:'Kamera bewegen',keys:'Pfeiltasten'},
    {group:'Karte',label:'Diplomatie',keys:'F4'},
  ],
  movie:video=>mountOriginalShellVideo(video,rulesProfile.id),
  onClose(){keys.clear();mouse.inside=false;},
});
addEventListener('pointerdown', sfx.unlock, { once: false });
canvas.addEventListener('contextmenu', e => e.preventDefault());
canvas.addEventListener('mousedown', e => {
  mouse.x = e.clientX; mouse.y = e.clientY;
  if (!running||modalOpen()||playerFlare.isArmed()) return;
  if(ui?.observer&&e.button===2)return;
  if (airMission?.active()) {
    airPress=e.button===0?[e.clientX,e.clientY]:null;
    if(e.button===2)airMissionFeedback(airMission.finish());
    return;
  }
  if (tradeRoute?.active()) {
    tradePress=e.button===0?[e.clientX,e.clientY]:null;
    if (e.button===2) tradeRoute.cancel();
    return;
  }
  const targetOrder=activeTargetOrder();
  if (targetOrder) { if (e.button===2) targetOrder.cancel(); return; }
  if (e.button === 1) { mouse.pan = true; e.preventDefault(); return; }
  if (e.button === 2) {
    if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; return; }
    const point=toWorld(e.clientX,e.clientY);if(!point)return;
    const [wx,wy]=point;
    const workers=ui?.sel?.def===8||ui?.sel?.types?.some(type=>type[0]===8);
    const fishing=Number.isInteger(fishingId)&&(ui?.sel?.def===fishingId||ui?.sel?.def===5||
      ui?.sel?.types?.some(type=>type[0]===fishingId));
    let target=null;
    if(workers||fishing){
      const direction=ray(e.clientX,e.clientY),groundDistance=(groundAt(wx,wy)-view.eye[1])/direction[1];
      target=originalModels.pickResource(view.eye,direction,groundDistance+.02);
      if(referenceMode&&target){
        const kind=wasm.resource_kind(Math.floor(target[1])*MAP+Math.floor(target[0]));
        if(kind===10?!fishing:!workers)target=null;
        else if(kind!==10&&kind&&!wasm.resource_active(ui.local,kind-1)){toast('Dieses Vorkommen ist in dieser Epoche nicht nutzbar.');return;}
        else if(kind===8||kind===9){toast(kind===8?'Öl benötigt einen Bohrturm mit einquartierten Bürgern.':'Uran benötigt eine Uranmine mit einquartierten Bürgern.');return;}
      }
    }
    const attack=submitContextOrder(target?.[0]??wx,target?.[1]??wy);
    if (!selectedUnitVoice('order',attack) && ui && ui.sel.n) sfx.play(22);
    return;
  }
  if (ghost) {
    if(ghost.nativeCursor){
      mouse.inside=true;
      if(!farmCursorPose(true))return;
      if(wasm.valley_farm_cursor_place()===1){
        if(!selectedUnitVoice('build'))sfx.play(20);
        if(!e.shiftKey)cancelBuilding();
        else {placementKey='';updatePlacement();}
      }else{sfx.play(21);toast(placementMessage(wasm.valley_farm_cursor_error()||1,ghost.id));}
      return;
    }
    const tile=ghostTile();if(!tile)return;
    const [tx,ty]=tile,err=wasm.place_error_rotated?wasm.place_error_rotated(ghost.id,tx,ty,ghost.turn):wasm.place_error(ghost.id,tx,ty);
    const placed=!err&&(wasm.cmd_place_rotated?wasm.cmd_place_rotated(ghost.id,tx,ty,ghost.turn):wasm.cmd_place(ghost.id,tx,ty));
    if (placed) { if (!selectedUnitVoice('build')) sfx.play(20); if (!e.shiftKey) cancelBuilding(); } else { sfx.play(21); toast(placementMessage(err||1,ghost.id)); }
    return;
  }
  if (attackMode) {
    const point=toWorld(e.clientX,e.clientY);if(!point)return;
    if(attackMode==='valley-direct'){
      if(!e.altKey||!wasm.valley71_force_attack_at(...point,0x400))return;
    }else wasm.cmd_context(...point,1);
    if(!selectedUnitVoice('attack'))sfx.play(22);attackMode=false;canvas.style.cursor='default';return;
  }
  mouse.down = true; mouse.drag = false; mouse.sx = e.clientX; mouse.sy = e.clientY;
});
addEventListener('mousemove', e => {
  if(modalOpen()){mouse.inside=false;return;}
  if(ghost&&e.target!==canvas&&!mouse.pan){mouse.inside=false;return;}
  if (mouse.pan) panCameraScreen(-e.movementX/cam.zoom,-e.movementY/(cam.zoom*Math.sin(view.pitch??PITCH)));
  mouse.x = e.clientX; mouse.y = e.clientY; mouse.inside = true;
  activeTargetOrder()?.update(e.target===canvas?toWorld(mouse.x,mouse.y):null);
  if (tradeRoute?.active()) {
    if(tradePress&&(e.target!==canvas||Math.abs(mouse.x-tradePress[0])+Math.abs(mouse.y-tradePress[1])>6))tradePress=null;
    tradeRoute.update(e.target===canvas?toWorld(mouse.x,mouse.y):null,e.target===canvas?[mouse.x,mouse.y]:null);
  }
  if(airPress&&(e.target!==canvas||Math.abs(mouse.x-airPress[0])+Math.abs(mouse.y-airPress[1])>6))airPress=null;
  if (mouse.down) {
    if (Math.abs(mouse.x - mouse.sx) + Math.abs(mouse.y - mouse.sy) > 6) mouse.drag = true;
    if (mouse.drag) {
      box.style.display = 'block';
      box.style.left = Math.min(mouse.x, mouse.sx) + 'px'; box.style.top = Math.min(mouse.y, mouse.sy) + 'px';
      box.style.width = Math.abs(mouse.x - mouse.sx) + 'px'; box.style.height = Math.abs(mouse.y - mouse.sy) + 'px';
    }
  }
});
document.addEventListener('mouseleave', () => {mouse.inside=false;tradePress=null;airPress=null;tradeRoute?.update(null,null);});
addEventListener('mouseup', e => {
  if(modalOpen()){cancelMapGesture();return;}
  if(playerFlare.isArmed()){
    const point=e.button===0 ? (e.target===canvas?toWorld(e.clientX,e.clientY):e.target===mini?miniWorld(e):null) : null;
    if(playerFlare.release(e.button,point))return;
  }
  const targetOrder=activeTargetOrder();
  if (targetOrder) {
    const power=activePowerOrder(),voice=power?power.voice:'attack';
    if (e.button===0 && e.target===canvas && targetOrder.release(toWorld(e.clientX,e.clientY))) {
      if (voice&&!selectedUnitVoice(voice)) sfx.play(22);
      lastUi=0;
    }
    return;
  }
  if (airMission?.active()) {
    const click=airPress&&Math.abs(e.clientX-airPress[0])+Math.abs(e.clientY-airPress[1])<=6;
    airPress=null;
    if(e.button===0&&click&&e.target===canvas)airMissionFeedback(airMission.release(toWorld(e.clientX,e.clientY)));
    return;
  }
  if (tradeRoute?.active()) {
    const click=tradePress&&Math.abs(e.clientX-tradePress[0])+Math.abs(e.clientY-tradePress[1])<=6;
    if (e.button===0&&click&&e.target===canvas&&tradeRoute.release(toWorld(e.clientX,e.clientY),[e.clientX,e.clientY])) {
      if(!selectedUnitVoice('order'))sfx.play(22);
      lastUi=0;
    }
    tradePress=null;
    return;
  }
  if (e.button === 1) mouse.pan = false;
  if (e.button !== 0 || !mouse.down) return;
  mouse.down = false; box.style.display = 'none';
  const add = e.shiftKey ? 1 : 0;
  if (mouse.drag) {
    // a screen rectangle is a trapezoid on the ground under a tilted camera
    const l = Math.min(mouse.x, mouse.sx), r = Math.max(mouse.x, mouse.sx), t = Math.min(mouse.y, mouse.sy), b = Math.max(mouse.y, mouse.sy);
    const points=[[l,t],[r,t],[r,b],[l,b]].map(([x,y])=>{
      const point=toWorld(x,y);
      if(point||!valleyWorld)return point;
      const [wx,wy]=toPlane(x,y);
      return [Math.min(valleyWorld.width,Math.max(0,wx)),Math.min(valleyWorld.height,Math.max(0,wy))];
    });
    wasm.select_quad(...points.flat(),add);
  } else {
    const point=toWorld(mouse.x,mouse.y);if(!point)return;
    const [wx,wy]=point;
    const now = performance.now(), dbl = now - lastClick < 300; lastClick = now;
    wasm.select_point(wx, wy, add, dbl ? 1 : 0, ...viewRect());
  }
  if (!selectedUnitVoice('select')) sfx.play(20, .35);
  lastUi = 0;
});
canvas.addEventListener('wheel', e => {
  if(modalOpen())return;
  e.preventDefault();
  if(ghost&&e.shiftKey){const now=performance.now();if(now-lastWheelRotation>80&&e.deltaY){lastWheelRotation=now;rotateBuilding(e.deltaY>0?1:-1,true);}return;}
  const before = toPlane(e.clientX, e.clientY);
  const desiredZoom=cam.zoom*Math.exp(-e.deltaY*.0012);
  if(referenceMode)cam.nativeFraction=originalFractionForZoom(desiredZoom,innerWidth,innerHeight);
  else cam.zoom=desiredZoom;
  updateCamera();
  const after = toPlane(e.clientX, e.clientY);     // keep the point under the cursor fixed
  cam.x += before[0] - after[0]; cam.y += before[1] - after[1]; updateCamera();
}, { passive: false });

function setSpeed(i) { if (net) return; speed = SPEEDS[Math.min(SPEEDS.length - 1, Math.max(0, i))]; showSpeed(); }
function applyRequestedSpeed(rate) { speed=rate;showSpeed(); }
function queueRequestedSpeed(rate) {
  if(originalTempo.locked)return;
  if(net)net.ls.setTempo(rate);
  else pendingTempo=rate;
}
function requestOriginalSpeed(factor) {
  if (!originalTempo) return;
  queueRequestedSpeed(factor==='default' ? originalStartingSpeed(originalTempo) : changeOriginalSpeed(speed,factor));
}
function setPauseHold(reason,hold) {
  if(net)return;
  if(hold)pauseReasons.add(reason);
  else pauseReasons.delete(reason);
  const wasPaused=paused;
  paused=pauseReasons.size>0;
  if(paused&&!wasPaused){sfx.stopStoneThrowerWheels();takeoffAudio?.reset();sfx.resetAirFlightAudio();tradeRoute?.cancel();airMission?.cancel();}
  if(paused!==wasPaused)last=performance.now();
  $('pause').textContent=paused?'Fortsetzen':'Pause';
}
function togglePause() {
  if(!running||net||gameOptions?.active()||$('cheat-dialog').open||$('research-dialog').open||diplomacy.isOpen())return;
  setPauseHold('manual',!pauseReasons.has('manual'));
}
function taskSwitch() {
  if(originalTempo&&running)setPauseHold('task-switch',document.hidden||document.hasFocus?.()===false);
}
document.addEventListener('visibilitychange',()=>{
  if(document.hidden){sfx.stopStoneThrowerWheels();takeoffAudio?.reset();sfx.resetAirFlightAudio();sfx.resetWorldAmbience();}
  taskSwitch();
});
function jumpToSelection() { const x = wasm.sel_center(0); if (x >= 0) { cam.x = x; cam.y = wasm.sel_center(1); } }
function focusNextBattle(pip) {
  if (pip && !fullmapPip.isOpen()) return false;
  const point = battleFlare?.cycle();
  if (!point) return false;
  if (pip) return fullmapPip.focusWorld(point);
  [cam.x,cam.y] = point;
  updateCamera();
  return true;
}
function goHome(){const territory=ui?.diplomacy?.territories?.find(area=>area.owner===ui.local);cam.x=territory?.x??wasm.start_x();cam.y=territory?.y??wasm.start_y();}
function idleWorker() { const i = wasm.select_idle_worker(); if (i !== -1 && i >>> 0 !== 0xffffffff) { tradeRoute?.cancel();airMission?.cancel();cam.x = wasm.ent_x(i); cam.y = wasm.ent_y(i); lastUi = 0; selectedUnitVoice('select'); } }
function updateAudioButtons() {
  $('mute').textContent=sfx.isMuted()?'Ton aus':'Ton an';
  $('music').textContent=sfx.isMusicEnabled()?'Musik an':'Musik aus';
  $('voices').textContent=sfx.isSpeechEnabled()?'Stimmen an':'Stimmen aus';
}
function toggleMute() { sfx.setMuted(!sfx.isMuted());updateAudioButtons(); }
function submitContextOrder(x,y) {
  if(wasm.cmd_context_attack_reply)return wasm.cmd_context_attack_reply(x,y)===1;
  wasm.cmd_context(x,y,0);
  return false;
}
function selectedUnitVoice(action,contextAttack=false) {
  const ptr = wasm.ui(); const state = JSON.parse(dec.decode(u8(ptr, wasm.ui_len()))),selected=state.sel;
  if (selected.n&&diversionPresentation?.hasVoices(selected.def,state.epoch)===false) return true;
  if (selected.n) {
    const context=selectedCitizenContext(wasm,state);
    const submarine=originalEnabled&&rulesProfile.id==='up15'&&selected.def===36&&(state.epoch===12||state.epoch===13);
    if(submarine&&contextAttack)action='attack';
    const replyScope=(context||submarine)&&rulesProfile.id==='up15'?{owner:state.local,world:wasm.render_meta_session()>>>0}:undefined;
    return sfx.acknowledge(selected.def,action,state.epoch,context,replyScope);
  }
  return false;
}

function closeCheats() {
  if (!$('cheat-dialog').open) return;
  $('cheat-dialog').close();setPauseHold('cheat',false);
}
function openCheats() {
  if (!running) return;
  if (net) return toast('Cheats sind nur im Einzelspieler verfügbar.');
  if ($('cheat-dialog').open) return;
  setPauseHold('cheat',true);keys.clear();cancelMapGesture();
  $('cheat-result').textContent = `Cheats ${wasm.cheat_status(0) ? 'aktiv' : 'inaktiv'} · Sofortbau ${wasm.cheat_status(1) ? 'an' : 'aus'} · Technologiepunkte: ${wasm.cheat_status(2)}`;
  $('cheat-dialog').showModal(); $('cheat-input').value = ''; $('cheat-input').focus();
}
$('cheats').onclick = openCheats;
$('cheat-up-commands').hidden=rulesProfile.ruleset!==2;
$('cheat-up-research').hidden=rulesProfile.ruleset!==2;
$('cheat-close').onclick = closeCheats;
$('cheat-dialog').addEventListener('cancel', e => { e.preventDefault(); closeCheats(); });
$('cheat-form').addEventListener('submit', e => {
  e.preventDefault(); const code = parseCheat($('cheat-input').value,rulesProfile.ruleset===2);
  if (code === null) { $('cheat-result').textContent = 'Unbekannter Cheat. Verfügbare Befehle stehen oben.'; return; }
  const result = wasm.cheat(code);
  const messages = ['Cheats aktiviert.', rulesProfile.ruleset===2?'Cheat-Eingabe deaktiviert.':'Cheats deaktiviert. Sofortbau ist aus.', referenceMode?'Bis zu 10.000 Einheiten je Rohstoff erhalten.':'+10.000 Nahrung, Holz und Metall.', result === 2 ? 'Sofortbau aus.' : 'Sofortbau an.', 'Forschungskonto aktualisiert.', 'Bis zu 100 Einheiten je Rohstoff abgezogen.', 'Strafbefehl für die Auswahl ausgeführt.', result===2?'Schadensschutz aus.':'Schadensschutz an.', result===2?'Kriegsnebel aus.':'Kriegsnebel an.', result===4?'Die Endepoche ist erreicht.':'In die nächste Epoche gewechselt.'];
  if (code === 10) messages[code] = result === 5 ? 'Klicke zuerst eine übernehmbare Einheit oder ein Gebäude an.' : 'Übernommen.';
  if (code === 11) messages[code] = 'Kraftreserve der ausgewählten Priester aufgefüllt.';
  $('cheat-result').textContent = result === 3 ? 'Nur im Einzelspieler verfügbar.' : result === 0 ? 'Zuerst icheat eingeben.' : messages[code] + ` Guthaben: ${wasm.cheat_status(2)} Technologiepunkte.`;
  $('cheat-input').value = ''; $('cheat-input').focus(); lastUi = 0;
});
addEventListener('keydown', e => {
  $('acts').classList.toggle('market-shift',e.shiftKey);
  if(e.key==='F11'){e.preventDefault();if(!e.repeat)fullscreen.toggle();return;}
  if(gameSettings.active())return;
  if(gameOptions?.active()){
    if(e.key==='Escape'||(e.key==='Pause'&&!e.shiftKey)){e.preventDefault();if(!e.repeat)gameOptions.close();}
    return;
  }
  if(hasOriginalTempo&&running&&!bench&&e.key==='Pause'&&!e.ctrlKey&&!e.altKey&&!e.metaKey){
    e.preventDefault();
    if(!e.repeat){if(e.shiftKey)togglePause();else gameOptions?.open();}
    return;
  }
  if(fullmap?.isOpen()){
    if(fullmap.warplans.key(e))return;
    if(e.key===' '&&!e.ctrlKey&&!e.altKey&&!e.metaKey&&focusNextBattle(e.shiftKey)){
      e.preventDefault();return;
    }
    if(e.key==='Escape'||(e.key==='Tab'&&!e.shiftKey&&!e.altKey&&!e.ctrlKey&&!e.metaKey)){
      e.preventDefault();if(!e.repeat)fullmap.close();
    } else fullmap.pipKey(e);
    return;
  }
  if ($('cheat-dialog').open || $('research-dialog').open || $('crown-choice').open || diplomacy.isOpen()) return;
  if(crownProgress&&running&&e.code==='KeyC'&&e.altKey&&e.shiftKey&&!e.ctrlKey&&!e.metaKey){
    if(!e.repeat&&crownProgress.toggle())e.preventDefault();
    return;
  }
  if(fullmap&&running&&!bench&&!valleyWorld&&!ui?.win&&e.key==='Tab'&&!e.ctrlKey&&!e.altKey&&!e.metaKey&&!e.shiftKey){
    e.preventDefault();if(!e.repeat)fullmap.open();return;
  }
  if(e.target.closest?.('#building-controls,#camera-tools')&&['Enter',' '].includes(e.key))return;
  if (e.key === 'Enter' && !['INPUT','SELECT','TEXTAREA','BUTTON'].includes(e.target.tagName)) { e.preventDefault(); openCheats(); return; }
  if (['INPUT', 'SELECT', 'TEXTAREA'].includes(e.target.tagName)) return;
  keys.add(e.key);
  if (!running) return;
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if(originalTempo&&!e.ctrlKey&&!e.altKey&&!e.metaKey&&!e.shiftKey&&['Minus','Equal','Backspace'].includes(e.code)){
    const change=e.code==='Backspace'?'default':e.code==='Minus'?.9:1.11111;
    e.preventDefault();requestOriginalSpeed(change);return;
  }
  if(e.altKey&&!e.ctrlKey&&!e.metaKey&&k==='Y'&&rulesProfile.ruleset===2&&!bench&&!net){
    e.preventDefault();if(!e.repeat)armPlayerFlare();return;
  }
  if(k==='Escape'&&playerFlare.cancel()){e.preventDefault();return;}
  if(k==='Escape'&&activeTargetOrder()?.cancel()){e.preventDefault();return;}
  if(k==='Escape'&&airMission?.cancel()){e.preventDefault();airPress=null;return;}
  if(k==='Escape'&&(tradeRoute?.cancel()||e.repeat)){e.preventDefault();tradePress=null;return;}
  if(ghost&&!e.metaKey&&(e.key==='Control'||k==='R'||e.code==='Comma'||e.code==='Period')){
    e.preventDefault();
    if(!e.repeat){
      const direction=e.code==='Comma'||(ghost.nativeCursor&&e.key==='Control')?-1:1;
      const fine=ghost.nativeCursor&&e.key==='Control'?e.altKey:e.shiftKey;
      if(ghost.nativeCursor&&e.key==='Control')farmAngleMemory.controlPressed();
      rotateBuilding(direction,fine);
    }
    return;
  }
  if (e.code.startsWith('Digit')) {
    const n = +e.code[5];
    if (e.ctrlKey) { wasm.group_set(n); toast(`Group ${n} set`); }
    else if (wasm.group_get(n)) { activeTargetOrder()?.cancel();tradeRoute?.cancel();airMission?.cancel();const now = performance.now(); if (lastGroup.n === n && now - lastGroup.t < 400) jumpToSelection(); lastGroup = { n, t: now }; lastUi = 0; }
    e.preventDefault(); return;
  }
  if(!ghost&&!bench&&!ui?.win&&ui?.sel?.n===1&&ui.sel.def===50){
    const order=marketOrderHotkey(wasm,wasm.selected_actor_handle()>>>0,e);
    if(order){
      e.preventDefault();
      keys.delete(e.key);
      if(!e.repeat)submitMarketOrder(order.host,order.resource,order.side*(e.shiftKey?5:1));
      return;
    }
  }
  if(k==='S'&&spyEnabled&&!e.altKey&&!e.metaKey&&ui?.sel?.def===47){
    const index=ui.act.findIndex(action=>action.k==='t'&&action.id===86);
    if(index>=0){
      e.preventDefault();
      if(!e.repeat){
        if(e.ctrlKey){if(wasm.cmd_cancel_spy())lastUi=0;}
        else doAct(index,e.shiftKey?5:1);
      }
      return;
    }
  }
  if ((e.ctrlKey || e.metaKey) && k === 'S') { e.preventDefault(); saveGame(); return; }
  if(referenceMode&&!ghost&&e.ctrlKey&&['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key)){e.preventDefault();keys.add('Control');return;}
  if (e.ctrlKey || e.metaKey) return;
  if(valleyNeutralRequested&&e.code==='KeyA'&&e.shiftKey&&!e.altKey&&wasm.valley71_neutral_opening?.()){
    e.preventDefault();
    if(!e.repeat){
      activeTargetOrder()?.cancel();tradeRoute?.cancel();airMission?.cancel();
      attackMode='valley-direct';ghost=null;canvas.style.cursor='crosshair';
    }
    return;
  }
  if((e.key===';'||e.code==='Semicolon'&&!e.shiftKey)&&!e.altKey){
    if(militaryLeaderControl(ui?.sel,ui?.epoch)){
      e.preventDefault();if(!e.repeat)orderMilitaryLeaderPower();return;
    }
    const target=leaderTargetOrders.find(command=>selectedPowerType(ui?.sel,command.definition)&&command.available());
    if(target){
      e.preventDefault();if(!e.repeat)armPowerOrder(target);return;
    }
  }
  const airStatus=airMission&&!bench&&!ui?.win&&!e.altKey?airMissionSelection(wasm):null;
  if(airStatus?.kind===1){
    if(k==='R'&&!e.shiftKey){e.preventDefault();if(!e.repeat)armAirMission();return;}
    if(k==='U'&&!e.shiftKey){e.preventDefault();if(!e.repeat&&airStatus.complete&&airStatus.count)doAirMission(airActions.scramble.command);return;}
    if(['F','T','B'].includes(k)){
      const index=ui?.act.findIndex(action=>airMissionTrainingKey(airStatus,action)===k)??-1;
      if(index>=0){e.preventDefault();if(!e.repeat)doAct(index,e.shiftKey?5:1);}
      return;
    }
  }
  if(k==='T'&&e.shiftKey&&!e.altKey&&tradeRoute&&wasm.trade_selected_count()){
    e.preventDefault();if(!e.repeat)armTradeRoute();return;
  }
  if(e.altKey&&!e.shiftKey){
    if(e.code==='KeyX'&&extractionAvailable&&selectedPowerType(ui?.sel,86)){
      e.preventDefault();if(!e.repeat)orderSpyExtraction();return;
    }
    const command=powerOrders.find(row=>e.code===`Key${row.key}`&&selectedPowerType(ui?.sel,row.definition)&&row.available?.()!==false);
    if(command){
      e.preventDefault();if(!e.repeat)armPowerOrder(command);return;
    }
  }
  if (k===captureOrderText.shortcut && !e.altKey && captureOrder && militaryCaptureCount()) {
    if(!e.shiftKey){e.preventDefault();if(!e.repeat)armCapture();}return;
  }
  const hi = HOT.indexOf(k);
  if (hi >= 0 && ui && ui.act[hi]) { doAct(hi); return; }
  switch (k) {
    case 'A': activeTargetOrder()?.cancel();tradeRoute?.cancel();airMission?.cancel();attackMode = true; ghost = null; canvas.style.cursor = 'crosshair'; break;
    case 'S': activeTargetOrder()?.cancel();tradeRoute?.cancel();airMission?.cancel();wasm.cmd_stop(); break;
    case 'U': tradeRoute?.cancel();airMission?.cancel();wasm.cmd_unload(); break;
    case 'M': toggleMute(); break;
    case 'Delete': tradeRoute?.cancel();airMission?.cancel();wasm.cmd_delete(); break;
    case 'Backspace': wasm.cmd_cancel(); break;
    case 'Escape':
      if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; }
      else if(hasOriginalTempo&&!bench)gameOptions?.open();
      else wasm.select_none();
      lastUi = 0; break;
    case 'F': idleWorker(); break;
    case 'F4': e.preventDefault();if(!bench)diplomacy.open();break;
    case 'H': goHome(); break;
    case ' ': if(!e.altKey&&!focusNextBattle(e.shiftKey)&&!e.shiftKey)jumpToSelection(); e.preventDefault(); break;
    case 'P': if(!hasOriginalTempo)togglePause(); break;
    case '+': case '=': if(!originalTempo)setSpeed(SPEEDS.indexOf(speed) + 1); break;
    case '-': if(!originalTempo)setSpeed(SPEEDS.indexOf(speed) - 1); break;
    case 'F9': if (!net) { wasm.grant(1000); toast('+1000 resources (sandbox cheat)'); } break;
  }
});
addEventListener('keyup', e => {keys.delete(e.key);$('acts').classList.toggle('market-shift',e.shiftKey);});
addEventListener('blur', () => { keys.clear();$('acts').classList.remove('market-shift');cancelMapGesture();taskSwitch(); });
addEventListener('focus',taskSwitch);

$('idle').onclick = idleWorker;
$('camera-left').onclick=()=>rotateView(Math.PI/4);
$('camera-right').onclick=()=>rotateView(-Math.PI/4);
$('camera-north').onclick=()=>{if(referenceMode&&running){cam.heading=0;updateCamera();mouse.inside=false;}};
$('pause').onclick = togglePause;
$('game-options-button').onclick=()=>gameOptions?.toggle();
$('mute').onclick = toggleMute;
$('music').onclick = () => {sfx.setMusicEnabled(!sfx.isMusicEnabled());updateAudioButtons();};
$('voices').onclick = () => {sfx.setSpeechEnabled(!sfx.isSpeechEnabled());updateAudioButtons();};
$('save').onclick = () => saveGame();
$('speed').onclick = () => {
  if(!originalTempo){setSpeed((SPEEDS.indexOf(speed) + 1) % SPEEDS.length);return;}
  queueRequestedSpeed(ORIGINAL_SPEEDS.find(value=>value>speed)??ORIGINAL_SPEEDS[0]);
};
function handleGameOption(action) {
  if(action==='save'){
    if(net?.checkpointHost)gameOptions.close();
    void saveGame();
  }
  else if(action==='load'){gameOptions.close();void loadGame();}
  else if(action==='settings')gameSettings.open();
  else if(action==='resign'){
    if(net?.ls.hostLost)return;
    if(!window.confirm(net?'Möchtest du aufgeben und als Beobachter weiterspielen?':'Möchtest du aufgeben?'))return;
    if(!(net?net.resignation&&wasm.cmd_resign_network():wasm.cmd_resign_solo()))return;
    gameOptions.close();setPauseHold('manual',false);
  }
  else if(action==='restart'){
    if(net||!restartPoint||!window.confirm('Möchtest du das Spiel neu starten?'))return;
    const adaptiveRate=restartPoint.adaptiveSourceClock ? adaptiveSoloClock?.rate ?? 30 : 0;
    if(!applyWorldSave(restartPoint.bytes)){toast('Dieses Scharmützel konnte nicht neu gestartet werden.');return;}
    Object.assign(cam,restartPoint.cam);
    gameOptions.close();begin({...originalTempo,requested:speed},restartPoint.sourceSoloClock===true,adaptiveRate);
  }
  else if(action==='exit'||action==='desktop'){
    if(!window.confirm('Möchtest du das laufende Spiel beenden?'))return;
    toMenu();
    if(action==='desktop')window.close();
  }
}
function toMenu() {
  cancelMapGesture();
  gameSettings.close();
  gameOptions?.close();
  takeoffAudio?.reset();
  sfx.resetAirFlightAudio();
  sfx.resetGuidedImpact();
  airMissionFlag?.reset();
  tradeWaitStatus.update(false);
  nativeHud.updateVictoryStatus?.(false);
  crownVictoryRows.update(null);
  $('crown-victory-hud').hidden=true;
  $('hotspot-status-hud').hidden=true;
  $('hotspot-status-rows').hidden=true;
  playerFlare.reset(0);
  battleFlare?.reset(false);
  $('player-flare-button').hidden=true;
  fullmap?.close();
  $('fullmap-button').hidden=true;
  $('camera-tools').hidden=true;
  cancelBuilding();
  diplomacy.close();
  if (running && !net && !bench) saveGame(true);
  if(net?.roster)rosterLobby.reset();
  else if(net){try{net.ch.close();net.pc.close();}catch{/* already gone */}}
  net=null;
  resetCheckpointDownload();
  worldMaterialPending = null;
  originalModels.bindNativeTerrainMaterial?.(null);
  running = false; $('menu').style.display = 'flex'; $('end').style.display = 'none'; $('mp').style.display = 'none';
  if(valleyNormal)void refreshContinue();
  shellMovie(true);
  sfx.resetWorldAmbience();
  sfx.menuMusic();
}
$('quit').onclick = toMenu; $('again').onclick = toMenu;
const seedOf = () => { const v = parseInt($('seed').value, 10); return Number.isFinite(v) && v > 0 ? v : (Math.random() * 2 ** 31) | 0 || 1; };
$('play').onclick = () => {
  rosterLobby?.reset();
  if(valleyPreview&&valleyMapChoice.value!=='own')startValley(valleyMapChoice.value==='valley-archive-qa');
  else{net=null;start(+$('nai').value,seedOf(),0);}
};
$('continue').onclick = () => { rosterLobby?.reset(); loadGame(); };
for (const b of document.querySelectorAll('[data-bench]')) b.onclick = () => { net = null; start(1, seedOf(), +b.dataset.bench); };

// ---------------------------------------------------------------- multiplayer lobby
const mpStatus = t => $('mpstatus').textContent = t;
let pending = null;
function sourceRoundStatus(peer) {
  if(peer.desync)return 'Mehrspieler · Synchronisation verloren';
  if(peer.terminal)return 'Mehrspieler · Partie beendet';
  if(peer.hostLost)return 'Hostverbindung verloren · Partie angehalten';
  if(peer.rosterUnavailable)return 'Mitspieler getrennt · Partie angehalten';
  if(peer.rosterConnected!==undefined&&!peer.transportAvailable)return 'Verbindung zum Host unterbrochen · warte';
  const label=peer.alone?'Nach Verbindungsabbruch lokal':peer.waiting?'Warte auf Mitspieler':'Originaler Rundentakt';
  const roster=peer.rosterConnected===undefined?'':peer.sharedOwner||peer.groupedOwners
    ?` · ${peer.sharedOwner?'Gemeinsame Zivilisation':'Gruppen-Koop'} · ${peer.rosterConnected}/${peer.humanSeats} Controller verbunden`
    :` · ${peer.rosterConnected}/${peer.humanSeats} Plätze verbunden`;
  return peer.round<0?label+roster:`${label}${roster} · Runde ${peer.round+1} · ${peer.rate} Hz · ${peer.length} Schritte/Runde`;
}

function channel(local) {
  let ch = null, pc = null, rejected = false, remoteWarplans, remoteCapture = false, remoteCaptureAnimation = false;
  let remoteInfantryRoster = false, remoteProductionRoster = false, remoteTradeRoute = false, remoteAirMission = false, remoteAirRunway = false, remoteAirTakeoff = false, remoteAirLanding = false;
  let remoteAirCarrier = false;
  let remoteOriginalTempo = false;
  let remoteResignation = false;
  let remoteInitialCeasefire = false;
  let remoteCrownRules = false;
  let allowSourceRounds = false, remoteSourceRounds = false, helloSeen = false;
  const supportsCapture = typeof wasm.capture_configure_peer === 'function';
  const supportsCaptureAnimation = supportsCapture && typeof wasm.capture_animation_configure_peer === 'function';
  const supportsInfantryRoster = typeof wasm.infantry_roster_configure_peer === 'function';
  const supportsProductionRoster = typeof wasm.production_roster_configure_peer === 'function';
  const supportsTrade = tradeAvailable&&typeof wasm.trade_route_configure_peer === 'function';
  const supportsAir = airMissionAvailable&&typeof wasm.air_mission_configure_peer === 'function';
  const supportsRunway = supportsAir&&typeof wasm.air_runway_configure_peer === 'function';
  const supportsTakeoff = supportsRunway&&typeof wasm.air_takeoff_configure_peer === 'function';
  const supportsLanding = supportsTakeoff&&typeof wasm.air_landing_configure_peer === 'function';
  const supportsCarrier = supportsLanding&&typeof wasm.air_carrier_configure_peer === 'function';
  return {
    open(c) {
      ch = c;
      allowSourceRounds=supportsSourceNetworkClock&&$('mp-source-rounds').checked&&
        c.ordered===true&&c.maxRetransmits===null&&c.maxPacketLifeTime===null;
      ch.send(JSON.stringify({t:'hello', protocol:PROTOCOL_VERSION, rules:wasm.ruleset(),civilizationChoice:civilizationSetup.choice(0),
        ...(originalEnabled&&fullmap?{warplans:WARPLAN_PEER_VERSION}:{}),
        ...(supportsCapture?{capture:1}:{}),
        ...(supportsCaptureAnimation?{captureAnimation:1}:{}),
        ...(supportsInfantryRoster?{infantryRoster:1}:{}),
        ...(supportsProductionRoster?{productionRoster:1}:{}),
        ...(supportsTrade?{tradeRoute:1}:{}),
        ...(supportsAir?{airMission:1}:{}),
        ...(supportsRunway?{airRunway:1}:{}),
        ...(supportsTakeoff?{airTakeoff:1}:{}),
        ...(supportsLanding?{airLanding:1}:{}),
        ...(supportsCarrier?{airCarrier:1}:{}),
        ...(hasOriginalTempo?{originalTempo:1}:{}),
        ...(hasNetworkResignation?{networkResignation:1}:{}),
        ...(allowSourceRounds?{sourceRounds:SOURCE_ROUND_VERSION}:{}),
        ...(hasInitialCeasefire?{initialCeasefire:1}:{}),
        ...(supportsPeerCrowns?{crownRules:1}:{}),
        ...(climates.length>1?{climates}:{})}));
      mpStatus('Verbunden – Spielversion wird geprüft …');
    },
    message(d) {
      if (typeof d === 'string') {
        let m; try { m = JSON.parse(d);if(!m||typeof m!=='object'||Array.isArray(m))throw new Error('invalid message'); } catch { rejected = true; mpStatus('Ungültige Verbindungsdaten.'); ch.close(); return; }
        if(rejected)return;
        if (m.t === 'hello') {
          if(helloSeen)return;
          helloSeen=true;
          if (m.protocol !== PROTOCOL_VERSION || m.rules !== wasm.ruleset()) { rejected = true; mpStatus('Unterschiedliche Spielversion oder Regelprofile. Beide müssen denselben Stand öffnen.'); ch.close(); return; }
          let guestClimates;
          try{guestClimates=peerClimates(m.climates);}catch{return reject('Ungültige Klimaauswahl des Mitspielers.');}
          if(civilizationSetup.enabled){try{civilizationChoice(m.civilizationChoice);}catch{return reject('Ungültige Völkerwahl des Mitspielers.');}}
          else if(m.civilizationChoice!=null)return reject('Die Völkerwahl passt nicht zur Regelversion.');
          remoteWarplans=m.warplans;
          remoteCapture=m.capture===1;
          remoteCaptureAnimation=remoteCapture&&m.captureAnimation===1;
          remoteInfantryRoster=m.infantryRoster===1;
          remoteProductionRoster=m.productionRoster===1;
          remoteTradeRoute=m.tradeRoute===1;
          remoteAirMission=m.airMission===1;
          remoteAirRunway=remoteAirMission&&m.airRunway===1;
          remoteAirTakeoff=remoteAirRunway&&m.airTakeoff===1;
          remoteAirLanding=remoteAirTakeoff&&m.airLanding===1;
          remoteAirCarrier=remoteAirLanding&&m.airCarrier===1;
          remoteOriginalTempo=m.originalTempo===1;
          remoteResignation=m.networkResignation===1;
          remoteInitialCeasefire=m.initialCeasefire===1;
          remoteCrownRules=m.crownRules===1;
          remoteSourceRounds=m.sourceRounds===SOURCE_ROUND_VERSION;
          if (local === 0 && !net) {
            try{
              const ai=+$('mpai').value,players=2+ai,choices=civilizationSetup.choices();
              if(choices)choices[1]=m.civilizationChoice;
              const resolved=choices?resolvedSkirmishChoices(choices,players):{civilizations:null,epoch:0,finalEpoch:null};
              const climate=menuClimate();
              if(!guestClimates.includes(climate))return reject('Die Originalgrafiken für dieses Klima fehlen beim Mitspieler.');
              const ceasefire=hasInitialCeasefire?+$('initial-ceasefire').value:0;
              if(ceasefire&&!remoteInitialCeasefire)return reject('Der Mitspieler benötigt eine Version mit anfänglichem Waffenstillstand.');
              const cfg = {t:'start',protocol:PROTOCOL_VERSION,rules:wasm.ruleset(),seed:(seedOf()>>>0)||1,ai,visibility:+$('map-visibility').value,seasons:menuSeasonSettings(),
                ...(climate?{climate}:{}),...(supportsCapture&&remoteCapture?{capture:1}:{}),
                ...(supportsCaptureAnimation&&remoteCaptureAnimation?{captureAnimation:1}:{}),
                ...(supportsInfantryRoster&&remoteInfantryRoster?{infantryRoster:1}:{}),
                ...(supportsProductionRoster&&remoteProductionRoster?{productionRoster:1}:{}),
                ...(supportsTrade&&remoteTradeRoute?{tradeRoute:1}:{}),
                ...(supportsAir&&remoteAirMission?{airMission:1}:{}),
                ...(supportsRunway&&remoteAirRunway?{airRunway:1}:{}),
                ...(supportsTakeoff&&remoteAirTakeoff?{airTakeoff:1}:{}),
                ...(supportsLanding&&remoteAirLanding?{airLanding:1}:{}),
                ...(supportsCarrier&&remoteAirCarrier?{airCarrier:1}:{}),
                ...(hasOriginalTempo&&remoteOriginalTempo?{gameTempo:menuTempo()}:{}),
                ...(hasNetworkResignation&&remoteResignation?{resignationPolicy:+$('mp-resign-policy').value}:{}),
                ...(allowSourceRounds&&remoteSourceRounds&&remoteOriginalTempo&&remoteResignation?{sourceRounds:SOURCE_ROUND_VERSION}:{}),
                ...(ceasefire?{ceasefire}:{}),
                ...(supportsPeerCrowns&&remoteCrownRules?{crownsDisabled:$('disable-crowns').checked}:{}),
                ...resolved,popLimit:choices?+$('pop-limit').value:null,supplies:choices?+$('supplies').value:null};
              checkStart(cfg);ch.send(JSON.stringify(cfg));launch(cfg);
            }catch{return reject('Die Scharmützeleinstellungen konnten nicht übernommen werden.');}
          }
        } else if (m.t === 'start' && local === 1 && !net) {
          try{checkStart(m);launch(m);}catch{return reject('Ungültige Spielversion, Völker- oder Scharmützeleinstellungen.');}
        } else if (net?.ch===ch) net.warplans?.receive(m);
      }
      else if (net?.ch===ch) {
        if(net.sourceRounds){
          try { if(!net.carrier.receive(d))net.ls.abort('Netzwerkpaket wurde abgelehnt'); }
          catch(error) { net.ls.abort(error.message); }
        } else net.ls.receive(d);
      }
    },
    transport(available) {
      if(net?.ch!==ch||!net.sourceRounds||net.ls.alone)return false;
      net.ls.setTransportAvailable(available);
      return true;
    },
    close() {
      if(net?.ch===ch&&!net.ls.alone){
        if(net.sourceRounds&&!net.ls.dropPeer())return;
        if(!net.sourceRounds)net.ls.dropPeer();
        net.warplans?.cancelPending();
        if(net.resignation)wasm.network_peer_disconnected();
        if(net.sourceRounds)showSpeed();
        toast('Die Verbindung zum Mitspieler wurde getrennt.');
      }else if(!net&&!rejected)mpStatus('Verbindung geschlossen.');
    },
    setPc(p) { pc = p; },
  };
  function reject(message){rejected=true;mpStatus(message);ch.close();}
  function checkStart(cfg){
    if(cfg.protocol!==PROTOCOL_VERSION||cfg.rules!==wasm.ruleset()||![0,1,2].includes(cfg.visibility)||!Number.isInteger(cfg.seed)||cfg.seed<1||cfg.seed>0xffffffff||![0,1,2].includes(cfg.ai))throw Error('Invalid new match settings');
    peerSeasonSettings(cfg.seasons);
    checkedClimate(cfg.climate,climates);
    if(cfg.capture!==undefined&&(cfg.capture!==1||!supportsCapture||!remoteCapture))throw Error('Unsupported capture command');
    if(cfg.captureAnimation!==undefined&&(cfg.captureAnimation!==1||cfg.capture!==1||!supportsCaptureAnimation||!remoteCaptureAnimation))throw Error('Unsupported capture animation');
    if(cfg.infantryRoster!==undefined&&(cfg.infantryRoster!==1||!supportsInfantryRoster||!remoteInfantryRoster))throw Error('Unsupported infantry roster');
    if(cfg.productionRoster!==undefined&&(cfg.productionRoster!==1||!supportsProductionRoster||!remoteProductionRoster))throw Error('Unsupported production roster');
    if(cfg.tradeRoute!==undefined&&(cfg.tradeRoute!==1||!supportsTrade||!remoteTradeRoute))throw Error('Unsupported trade route');
    if(cfg.airMission!==undefined&&(cfg.airMission!==1||!supportsAir||!remoteAirMission))throw Error('Unsupported air mission');
    if(cfg.airRunway!==undefined&&(cfg.airRunway!==1||cfg.airMission!==1||!supportsRunway||!remoteAirRunway))throw Error('Unsupported air runway');
    if(cfg.airTakeoff!==undefined&&(cfg.airTakeoff!==1||cfg.airRunway!==1||!supportsTakeoff||!remoteAirTakeoff))throw Error('Unsupported air takeoff');
    if(cfg.airLanding!==undefined&&(cfg.airLanding!==1||cfg.airTakeoff!==1||!supportsLanding||!remoteAirLanding))throw Error('Unsupported air landing');
    if(cfg.airCarrier!==undefined&&(cfg.airCarrier!==1||cfg.airLanding!==1||!supportsCarrier||!remoteAirCarrier))throw Error('Unsupported aircraft carrier');
    if(cfg.gameTempo!==undefined){
      if(!hasOriginalTempo||!remoteOriginalTempo)throw Error('Unsupported game speed settings');
      checkOriginalTempo(cfg.gameTempo);
    }
    if(cfg.resignationPolicy!==undefined&&(!hasNetworkResignation||!remoteResignation||
      !Number.isInteger(cfg.resignationPolicy)||cfg.resignationPolicy<0||cfg.resignationPolicy>7))throw Error('Unsupported resignation policy');
    if(cfg.sourceRounds!==undefined&&(cfg.sourceRounds!==SOURCE_ROUND_VERSION||!allowSourceRounds||!remoteSourceRounds||
      !cfg.gameTempo||cfg.resignationPolicy===undefined))throw Error('Unsupported source rounds');
    if(cfg.ceasefire!==undefined&&(!hasInitialCeasefire||!remoteInitialCeasefire||
      !Number.isInteger(cfg.ceasefire)||cfg.ceasefire<1||cfg.ceasefire>10))throw Error('Unsupported initial ceasefire');
    if(cfg.crownsDisabled!==undefined&&(!supportsPeerCrowns||!remoteCrownRules||typeof cfg.crownsDisabled!=='boolean'))
      throw Error('Unsupported Crown settings');
    if(civilizationSetup.enabled){
      checkedCivilizations(cfg.civilizations,2+cfg.ai);
      if(![250,500,1000].includes(cfg.popLimit)||![0,1,2].includes(cfg.supplies))throw Error('Invalid skirmish setup');
      checkedEpochRange(cfg.epoch,cfg.finalEpoch,wasm.epoch_count());
    }else if(cfg.civilizations!==null||cfg.popLimit!==null||cfg.supplies!==null||cfg.epoch!==0||cfg.finalEpoch!==null)throw Error('Unsupported civilization setup');
  }
  function launch(cfg) {
    checkStart(cfg);
    wasm.init(cfg.seed >>> 0, cfg.ai, 0, 2, local);
    if(civilizationSetup.enabled)applyMatchSettings(wasm,cfg,2+cfg.ai);
    wasm.skirmish_visibility(cfg.visibility);
    if(cfg.capture===1&&!wasm.capture_configure_peer(1))throw Error('Capture setup failed');
    if(cfg.captureAnimation===1&&!wasm.capture_animation_configure_peer(1))throw Error('Capture animation setup failed');
    if(cfg.infantryRoster===1&&!wasm.infantry_roster_configure_peer(1))throw Error('Infantry roster setup failed');
    if(cfg.productionRoster===1&&!wasm.production_roster_configure_peer(1))throw Error('Production roster setup failed');
    if(cfg.tradeRoute===1&&!wasm.trade_route_configure_peer(1))throw Error('Trade route setup failed');
    if(cfg.airMission===1&&!wasm.air_mission_configure_peer(1))throw Error('Air mission setup failed');
    if(cfg.airRunway===1&&!wasm.air_runway_configure_peer(1))throw Error('Air runway setup failed');
    if(cfg.airTakeoff===1&&!wasm.air_takeoff_configure_peer(1))throw Error('Air takeoff setup failed');
    if(cfg.airLanding===1&&!wasm.air_landing_configure_peer(1))throw Error('Air landing setup failed');
    if(cfg.airCarrier===1&&!wasm.air_carrier_configure_peer(1))throw Error('Aircraft carrier setup failed');
    if(cfg.resignationPolicy!==undefined&&!wasm.configure_network_resignation(cfg.resignationPolicy))throw Error('Resignation setup failed');
    if(cfg.ceasefire!==undefined&&!wasm.configure_initial_ceasefire(cfg.ceasefire))throw Error('Initial ceasefire setup failed');
    if(cfg.crownsDisabled!==undefined&&!wasm.configure_crowns_peer(Number(cfg.crownsDisabled)))throw Error('Crown setup failed');
    seasonSession.configure(peerSeasonSettings(cfg.seasons),cfg.climate);setSeasonControls(seasonSession.settings);climateControl.value=seasonSession.climate;
    const sourceRounds = cfg.sourceRounds===SOURCE_ROUND_VERSION;
    net = {ch,pc,ls:null,resignation:cfg.resignationPolicy!==undefined,...(sourceRounds?{sourceRounds:true}:{})};
    const tempo = cfg.gameTempo ? {locked:cfg.gameTempo.locked,apply:applyRequestedSpeed} : null;
    const send = buf => { if(ch.readyState==='open')ch.send(buf); };
    if(sourceRounds){
      const session=net;
      let roundTrip=0,nextStats=0;
      const carrier=createSourceRoundCarrier(send,message=>session.ls.receive(1-local,message));
      session.carrier=carrier;
      session.ls=new SourceRoundPeer(wasm,local,carrier.send,{
        requested:originalStartingSpeed(cfg.gameTempo),locked:cfg.gameTempo.locked,
        renderFps:()=>adaptiveRenderSample.fps,linkMs:()=>roundTrip,
        nowSeconds:()=>performance.now()/1000,onRequested:applyRequestedSpeed,
      });
      if(local===0&&typeof pc?.getStats==='function')session.refreshLink=now=>{
        if(session.ls.alone||session.ls.terminal||now<nextStats)return;
        nextStats=now+1000;
        pc.getStats().then(stats=>{
          const measurement=selectedRoundTripMilliseconds(stats);
          if(net===session&&measurement!==null)roundTrip=measurement;
        }).catch(()=>{});
      };
    } else net.ls = new Lockstep(wasm, local, send, tempo);
    if(originalEnabled&&fullmap&&remoteWarplans===WARPLAN_PEER_VERSION)net.warplans=createWarplanPeer({
      local,peerVersion:remoteWarplans,end:MAP-1,recipientAvailable:()=>!!warplanRecipient(),
      post:packet=>{if(ch.readyState!=='open')return false;ch.send(JSON.stringify(packet));return true;},
      onChange:fullmap.warplans.refresh,onCommit:fullmap.warplans.commit,
    });
    bench = 0; cam.x = wasm.start_x(); cam.y = wasm.start_y(); cam.zoom = 28;
    cam.heading=0;
    if(referenceMode)cam.nativeFraction=.2;
    begin(cfg.gameTempo ? startingTempo(cfg.gameTempo) : null);
    if(sourceRounds&&local===1)net.ls.activate();
    toast(allowSourceRounds&&!sourceRounds?'Die Partie verwendet den kompatiblen Mehrspieler-Takt.'
      :local === 0 ? 'You are blue. Good luck!' : 'You are red. Good luck!');
  }
}
$('mpopen').onclick = () => { $('mp').style.display = $('mp').style.display === 'block' ? 'none' : 'block'; };
$('mphost').onclick = async () => {
  rosterLobby?.reset();
  mpStatus('Creating invite…');
  try {
    const on = channel(0); pending = await rtcHost(on); on.setPc(pending.pc);
    $('mpout').value = pending.code; $('mpin').value = ''; $('mpin').placeholder = 'Paste the reply code from the other player here'; $('mpgo').textContent = 'Connect';
    $('mpgo').onclick = async () => { try { await pending.accept($('mpin').value); mpStatus('Connecting…'); } catch { mpStatus('That reply code is not valid.'); } };
    mpStatus('1. Send the invite code below to the other player.  2. Paste their reply code and press Connect.');
  } catch (e) { mpStatus('WebRTC is not available: ' + e.message); }
};
$('mpjoin').onclick = () => {
  rosterLobby?.reset();
  $('mpout').value = ''; $('mpin').value = ''; $('mpin').placeholder = 'Paste the invite code from the host here'; $('mpgo').textContent = 'Create reply code';
  mpStatus('Paste the host’s invite code, then press “Create reply code”.');
  $('mpgo').onclick = async () => {
    try {
      const on = channel(1); pending = await rtcJoin($('mpin').value, on); on.setPc(pending.pc);
      $('mpout').value = pending.code; mpStatus('Send the reply code below back to the host. The game starts when they connect.');
    } catch { mpStatus('That invite code is not valid.'); }
  };
};
$('mpcopy').onclick = () => { $('mpout').select(); navigator.clipboard?.writeText($('mpout').value).then(() => mpStatus('Copied to the clipboard.'), () => {}); };

const rosterFeatures=Object.freeze(Object.fromEntries([
  ['capture','capture_configure_peer'],['captureAnimation','capture_animation_configure_peer'],
  ['infantryRoster','infantry_roster_configure_peer'],['productionRoster','production_roster_configure_peer'],
  ...(tradeAvailable?[['tradeRoute','trade_route_configure_peer']]:[]),
  ...(airMissionAvailable?[['airMission','air_mission_configure_peer'],['airRunway','air_runway_configure_peer'],
    ['airTakeoff','air_takeoff_configure_peer'],['airLanding','air_landing_configure_peer'],['airCarrier','air_carrier_configure_peer']]:[]),
  ...(supportsPeerCrowns?[['crowns','configure_crowns_peer']]:[]),
].filter(([,name])=>typeof wasm[name]==='function')));
const supportedRosterFeatures=Object.keys(rosterFeatures);

function checkRosterSettings(cfg,humanSeats,resuming=false) {
  if(!cfg||cfg.humanSeats!==humanSeats||![2,3,4].includes(humanSeats)||!Number.isInteger(cfg.ai)||cfg.ai<0||
    !Number.isInteger(cfg.seed)||cfg.seed<1||cfg.seed>0xffffffff||![0,1,2].includes(cfg.visibility)||
    ![250,500,1000].includes(cfg.popLimit)||![0,1,2].includes(cfg.supplies)||!Number.isInteger(cfg.resignationPolicy)||
    cfg.resignationPolicy<0||cfg.resignationPolicy>7||!Number.isInteger(cfg.ceasefire)||cfg.ceasefire<0||cfg.ceasefire>10||
    (cfg.ceasefire>0&&!hasInitialCeasefire))throw Error('Ungültige Scharmützeleinstellungen für diesen Raum.');
  const sharedOwner=cfg.sharedOwner===true;
  if(cfg.sharedOwner!==undefined&&(!sharedOwner||!supportsSharedOwner||humanSeats!==2||cfg.ai!==1))
    throw Error('Ungültige Einstellungen für die gemeinsame Zivilisation.');
  const archivedWorldHumans=resuming&&cfg.groupedOwners!==undefined?cfg.groupedWorldHumans:null;
  if((cfg.groupedWorldHumans!==undefined&&archivedWorldHumans===null)||
      (cfg.groupedOwners!==undefined&&(!supportsGroupedCoop||sharedOwner||
        (resuming&&!Number.isInteger(archivedWorldHumans))||!groupedOwnerLayout(cfg.groupedOwners,humanSeats,archivedWorldHumans))))
    throw Error('Ungültige Aufteilung der Koop-Gruppen.');
  const worldHumans=archivedWorldHumans??(sharedOwner?1:cfg.groupedOwners?Math.max(...cfg.groupedOwners)+1:humanSeats);
  if(cfg.ai>4-worldHumans)throw Error('Zu viele Zivilisationen für dieses Scharmützel.');
  const features=cfg.features;
  if(!Array.isArray(features)||new Set(features).size!==features.length||features.some(key=>!Object.hasOwn(rosterFeatures,key)))
    throw Error('Der Raum verwendet nicht verfügbare Spielbefehle.');
  if(features.includes('crowns')?typeof cfg.crownsDisabled!=='boolean':cfg.crownsDisabled!==undefined)
    throw Error('Ungültige Kroneneinstellungen für diesen Raum.');
  let crownVictory;
  if(cfg.crownVictory!==undefined){
    if(!supportsNetworkCrownVictory||!features.includes('crowns')||sharedOwner||cfg.groupedOwners)
      throw Error('Kronen-Siegbedingungen benötigen einzelne Spielerplätze und eine passende UP-Spielversion.');
    crownVictory=checkCrownVictorySettings(cfg.crownVictory);
  }
  let capitolVictory;
  if(cfg.capitolVictory!==undefined){
    const {mode,minutes,sole}=cfg.capitolVictory??{};
    if(!supportsNetworkCapitols||crownVictory||![5,6].includes(mode)||![2,5,10].includes(minutes)||typeof sole!=='boolean'||
      (mode===6&&sole)||sharedOwner||cfg.groupedOwners||!features.includes('captureAnimation')||
      (requiresCapitolRoster(mode,humanSeats,cfg.ai)&&!supportsNetworkCapitolRoster))
      throw Error('Kapitole benötigen getrennte Zivilisationen und eine passende Eroberungsregel.');
    capitolVictory={mode,minutes,sole};
  }
  const featureDeps={captureAnimation:'capture',airRunway:'airMission',airTakeoff:'airRunway',airLanding:'airTakeoff',airCarrier:'airLanding'};
  for(const [key,parent] of Object.entries(featureDeps))if(features.includes(key)&&!features.includes(parent))
    throw Error('Unvollständige Spielbefehle für diesen Raum.');
  const captureCheckpoint=cfg.captureCheckpoint===true;
  if(cfg.captureCheckpoint!==undefined&&(!captureCheckpoint||!supportsCaptureCheckpoint||!features.includes('capture')||
    sharedOwner||cfg.groupedOwners||capitolVictory))throw Error('Dieser Raum benötigt Unterstützung für Eroberungen nach dem Laden.');
  const startingTeams=checkStartingTeams(cfg.startingTeams,worldHumans+cfg.ai);
  if(startingTeams&&(!supportsNetworkTeams||sharedOwner||cfg.groupedOwners||crownVictory||capitolVictory))
    throw Error('Startteams benötigen getrennte Spielerplätze in Eroberung.');
  if(cfg.sightTerms!==undefined&&(cfg.sightTerms!==true||!supportsNetworkSightTerms||sharedOwner||cfg.groupedOwners))
    throw Error('Die Sichtregeln benötigen getrennte Spielerplätze und eine passende Spielversion.');
  if(cfg.timedTreaties!==undefined&&(cfg.timedTreaties!==true||!supportsNetworkTimedTreaties||!cfg.sightTerms))
    throw Error('Befristete Bündnisse benötigen eine passende Spielversion.');
  if(cfg.borderTreaties!==undefined&&(cfg.borderTreaties!==true||!supportsNetworkBorderTreaties||!cfg.timedTreaties))
    throw Error('Bündnisgrenzen benötigen eine passende Spielversion.');
  if(cfg.fishing!==undefined&&(cfg.fishing!==true||!supportsNetworkFishing||sharedOwner||cfg.groupedOwners))
    throw Error('Fischerei benötigt getrennte Spielerplätze und eine passende Spielversion.');
  checkedEpochRange(cfg.epoch,cfg.finalEpoch,wasm.epoch_count());
  return {humanSeats,ai:cfg.ai,seed:cfg.seed,visibility:cfg.visibility,popLimit:cfg.popLimit,supplies:cfg.supplies,
    resignationPolicy:cfg.resignationPolicy,ceasefire:cfg.ceasefire,epoch:cfg.epoch,finalEpoch:cfg.finalEpoch,
    civilizations:checkedCivilizations(cfg.civilizations,worldHumans+cfg.ai),
    climate:checkedClimate(cfg.climate,climates),seasons:peerSeasonSettings(cfg.seasons),
    gameTempo:checkOriginalTempo(cfg.gameTempo),features:supportedRosterFeatures.filter(key=>features.includes(key)),
    ...(features.includes('crowns')?{crownsDisabled:cfg.crownsDisabled}:{}),
    ...(crownVictory?{crownVictory}:{}),
    ...(capitolVictory?{capitolVictory}:{}),
    ...(captureCheckpoint?{captureCheckpoint:true}:{}),
    ...(startingTeams?{startingTeams}:{}),
    ...(cfg.sightTerms?{sightTerms:true}:{}),
    ...(cfg.timedTreaties?{timedTreaties:true}:{}),
    ...(cfg.borderTreaties?{borderTreaties:true}:{}),
    ...(cfg.fishing?{fishing:true}:{}),
    ...(sharedOwner?{sharedOwner:true}:cfg.groupedOwners?{groupedOwners:cfg.groupedOwners.slice(),
      ...(resuming?{groupedWorldHumans:worldHumans}:{})}:{})};
}

function checkpointRules(cfg){
  const crowns=cfg.crownsDisabled===undefined?0:cfg.crownsDisabled?2:1;
  const capture=cfg.captureCheckpoint?1+Number(cfg.features.includes('captureAnimation')):0;
  const rules=crowns|(capture<<25)|(Number(cfg.sightTerms===true)<<27)|(Number(cfg.timedTreaties===true)<<28)|
    (Number(cfg.borderTreaties===true)<<29)|(Number(cfg.fishing===true)<<30);
  const crown=cfg.crownVictory, capitol=cfg.capitolVictory;
  if(crown)return rules|(crown.mode<<8)|(crown.target<<16);
  if(capitol)return rules|(capitol.mode<<8)|(capitol.minutes<<16)|(Number(capitol.sole)<<24);
  return rules;
}

function startRosterGame(cfg,seat,send,links,{archive,onCheckpoint}) {
  const sharedOwner=cfg.sharedOwner===true;
  const groupedOwners=cfg.groupedOwners??null;
  const worldHumans=sharedOwner?1:groupedOwners?Math.max(...groupedOwners)+1:cfg.humanSeats;
  if(archive)applyNetworkCheckpoint(wasm,archive.decoded,seat,cfg.humanSeats,false,checkpointRules(cfg),checkpointStartingTeams(cfg.startingTeams));
  else{
    const {crownVictory,capitolVictory,...matchSettings}=cfg;
    if(sharedOwner){
      if(wasm.init_network_shared_owner(cfg.seed,seat)!==1)throw Error('Der Koop-Controller konnte nicht gestartet werden.');
    }else if(groupedOwners){
      const packed=groupedOwnerLayout(groupedOwners,cfg.humanSeats)&255;
      if(wasm.init_network_grouped_coop(cfg.seed,cfg.ai,cfg.humanSeats,packed,seat)!==1)
        throw Error('Die Koop-Gruppe konnte nicht gestartet werden.');
    }else if(cfg.humanSeats===2)wasm.init(cfg.seed,cfg.ai,0,2,seat);
    else if(wasm.init_network_roster(cfg.seed,cfg.ai,cfg.humanSeats,seat)!==1)throw Error('Dieser Spielerplatz konnte nicht gestartet werden.');
    applyMatchSettings(wasm,matchSettings,worldHumans+cfg.ai);
    if(cfg.startingTeams&&wasm.configure_network_starting_teams(packStartingTeams(cfg.startingTeams),Number(cfg.startingTeams.locked))!==1)
      throw Error('Die Startteams konnten nicht eingerichtet werden.');
    wasm.skirmish_visibility(cfg.visibility);
    if(cfg.sightTerms&&wasm.configure_sight_terms()!==1)
      throw Error('Die Sichtregeln konnten nicht eingerichtet werden.');
    if(cfg.timedTreaties&&wasm.configure_timed_treaties(1)!==1)
      throw Error('Die Bündnisdauer konnte nicht eingerichtet werden.');
    if(cfg.borderTreaties&&wasm.configure_border_treaties()!==1)
      throw Error('Die Grenzregeln konnten nicht eingerichtet werden.');
    if(cfg.fishing&&wasm.configure_network_fishing()!==1)
      throw Error('Die Fischgründe konnten nicht eingerichtet werden.');
    if((!sharedOwner&&!wasm.configure_network_resignation(cfg.resignationPolicy))||
      (cfg.ceasefire&&!wasm.configure_initial_ceasefire(cfg.ceasefire)))throw Error('Die Regeln für diesen Raum konnten nicht aktiviert werden.');
    for(const name of cfg.features){
      const value=name==='crowns'?Number(cfg.crownsDisabled):1;
      if(wasm[rosterFeatures[name]](value)!==1)throw Error('Ein Spielbefehl konnte nicht aktiviert werden.');
    }
    if(cfg.captureCheckpoint&&wasm.configure_network_capture_checkpoint()!==1)
      throw Error('Eroberungen nach dem Laden konnten nicht aktiviert werden.');
    if(crownVictory&&wasm.configure_network_crown_victory(crownVictory.mode,crownVictory.target)!==1)
      throw Error('Die Kronen-Siegbedingung konnte nicht aktiviert werden.');
    if(capitolVictory){
      const {mode,minutes,sole}=capitolVictory;
      const configured=requiresCapitolRoster(mode,cfg.humanSeats,cfg.ai)?
        wasm.configure_network_capitols_roster(mode,minutes,Number(sole)):wasm.configure_network_capitols(minutes,Number(sole));
      if(configured!==1)throw Error('Die Kapitol-Siegbedingung konnte nicht aktiviert werden.');
    }
  }
  seasonSession.configure(cfg.seasons,cfg.climate);setSeasonControls(seasonSession.settings);climateControl.value=seasonSession.climate;
  const roundTrip=new Map();
  let nextStats=0;
  const resume=archive?.decoded.meta??null;
  const requested=resume?.requested??originalStartingSpeed(cfg.gameTempo);
  const locked=resume?.locked??cfg.gameTempo.locked;
  const peer=new SourceRoundPeer(wasm,seat,send,{humanSeats:cfg.humanSeats,
    requested,locked,resume,onCheckpoint,sharedOwner,groupedOwners,
    renderFps:()=>adaptiveRenderSample.fps,linkMs:other=>roundTrip.get(other)??0,
    nowSeconds:()=>performance.now()/1000,onRequested:applyRequestedSpeed});
  const session={roster:true,sourceRounds:true,ls:peer,resignation:!sharedOwner&&!groupedOwners,checkpointHost:seat===0&&!!onCheckpoint};
  if(seat===0)session.refreshLink=now=>{
    if(peer.terminal||peer.desync||now<nextStats)return;
    nextStats=now+1000;
    for(const link of links())link.pc?.getStats().then(stats=>{
      const ms=selectedRoundTripMilliseconds(stats);
      if(net===session&&ms!==null)roundTrip.set(link.seat,ms);
    }).catch(()=>{});
  };
  net=session;
  bench=0;cam.x=wasm.start_x();cam.y=wasm.start_y();cam.zoom=28;cam.heading=0;
  if(referenceMode)cam.nativeFraction=.2;
  peer.rosterConnected=cfg.humanSeats;
  begin({...startingTempo(cfg.gameTempo),requested,locked});
  toast(archive?`Spielstand fortgesetzt · Platz ${seat+1}`:sharedOwner?'Koop · Gemeinsame Zivilisation gegen die KI':
    groupedOwners?`Gruppen-Koop · Zivilisation ${groupedOwners[seat]+1}`:`Mehrspieler · Platz ${seat+1}`);
  return {peer,connected:count=>{
    peer.rosterConnected=count;
    if((sharedOwner||groupedOwners)&&seat===0&&onCheckpoint){
      session.checkpointHost=sharedOwner?count===cfg.humanSeats:count>=2;
      $('save').style.display=session.checkpointHost?'':'none';
    }
  }};
}

const supportsSourceRoster=supportsSourceNetworkClock&&civilizationSetup.enabled&&
  typeof wasm.init_network_roster==='function'&&typeof wasm.network_roster_peer_disconnected==='function';
const supportsNetworkCheckpoint=supportsSourceRoster&&!!globalThis.crypto?.subtle&&
  ['save_network_resume','load_network_resume','validate_network_resume','network_resume_pending'].every(name=>typeof wasm[name]==='function');
const supportsNetworkTeams=supportsNetworkCheckpoint&&
  ['configure_network_starting_teams','load_network_resume_with_teams','validate_network_resume_with_teams'].every(name=>typeof wasm[name]==='function');
const supportsNetworkSightTerms=supportsNetworkCheckpoint&&typeof wasm.configure_sight_terms==='function';
const supportsNetworkTimedTreaties=supportsNetworkSightTerms&&typeof wasm.configure_timed_treaties==='function';
const supportsNetworkBorderTreaties=supportsNetworkTimedTreaties&&typeof wasm.configure_border_treaties==='function';
const supportsNetworkFishing=supportsNetworkCheckpoint&&typeof wasm.configure_network_fishing==='function';
const supportsSharedOwner=supportsSourceRoster&&typeof wasm.init_network_shared_owner==='function'&&typeof wasm.network_shared_owner==='function';
const supportsSharedCheckpoint=supportsSharedOwner&&supportsNetworkCheckpoint&&
  ['save_shared_owner_resume','load_shared_owner_resume','validate_shared_owner_resume'].every(name=>typeof wasm[name]==='function');
const supportsGroupedCoop=supportsSourceRoster&&['init_network_grouped_coop','network_grouped_coop_layout',
  'net_apply_grouped_controller','network_grouped_controller_disconnected'].every(name=>typeof wasm[name]==='function');
const supportsGroupedCheckpoint=supportsGroupedCoop&&supportsNetworkCheckpoint&&
  ['grouped_resume_layout','save_grouped_resume','load_grouped_resume','validate_grouped_resume'].every(name=>typeof wasm[name]==='function');
// The ordinary roster hello keeps its Native46 simulation compatibility key.
const rosterProfile='openepoch-ee2-up15.wasm?v=native46';
let checkpointDownload=null;
function resetCheckpointDownload(){checkpointDownload=null;$('mp-checkpoint-download').hidden=true;}
$('mp-checkpoint-download').onclick=()=>{
  if(!checkpointDownload)return;
  const url=URL.createObjectURL(new Blob([checkpointDownload.bytes],{type:'application/octet-stream'}));
  const link=document.createElement('a');
  link.href=url;link.download=`OpenEpoch-Scharmuetzel-${checkpointDownload.tick}.openepoch-mp`;
  link.click();setTimeout(()=>URL.revokeObjectURL(url),30000);
};
$('mp-roster-resume-file').disabled=!supportsNetworkCheckpoint;
$('mp-roster-count').querySelector('[value="2"]').hidden=!supportsNetworkCheckpoint;
const networkTeamPicker=mountNetworkTeamPicker({document,anchor:$('mp-roster-start'),victory:victoryCondition,available:supportsNetworkTeams});
const rosterLobby=supportsSourceRoster?mountSourceRosterLobby($('mp-roster'),{
  rtcHost,rtcJoin,
  onHostRoster:networkTeamPicker.configure,
  describeRoom:archive=>archive?{climate:archive.climate,epoch:archive.epoch,finalEpoch:archive.finalEpoch}:
    {climate:menuClimate(),epoch:+$('start-epoch').value,finalEpoch:+$('end-epoch').value},
  epochName:epoch=>epoch===-1?'Zufall':epoch>=0&&epoch<wasm.epoch_count()?
    dec.decode(u8(wasm.epoch_name_ptr(epoch),wasm.epoch_name_len(epoch))):`Epoche ${epoch+1}`,
  localHello:()=>({protocol:PROTOCOL_VERSION,profile:rosterProfile,rules:wasm.ruleset(),
    civilization:civilizationSetup.choice(0),climates,features:supportedRosterFeatures,
    ...(supportsNetworkCrownVictory?{crownVictory:true}:{}),
    ...(supportsNetworkCapitols?{capitolVictory:true}:{}),
    ...(supportsNetworkCapitolRoster?{capitolRoster:true}:{}),
    ...(supportsCaptureCheckpoint?{captureCheckpoint:true}:{}),
    ...(supportsNetworkTeams?{startingTeams:true}:{}),
    ...(supportsNetworkSightTerms?{sightTerms:true}:{}),
    ...(supportsNetworkTimedTreaties?{timedTreaties:true}:{}),
    ...(supportsNetworkBorderTreaties?{borderTreaties:true}:{}),
    ...(supportsNetworkFishing?{fishing:true}:{}),
    checkpoints:supportsNetworkCheckpoint,sharedOwner:supportsSharedOwner,sharedOwnerCheckpoint:supportsSharedCheckpoint,
    groupedCoop:supportsGroupedCoop,groupedCheckpoint:supportsGroupedCheckpoint}),
  checkHello(hello){
    if(hello?.protocol!==PROTOCOL_VERSION||hello.profile!==rosterProfile||hello.rules!==wasm.ruleset()||
      !Array.isArray(hello.features)||hello.features.length>11||hello.features.some(key=>typeof key!=='string'))
      throw Error('Alle Teilnehmer brauchen dieselbe UP-Spielversion.');
    return {civilization:civilizationChoice(hello.civilization),climates:peerClimates(hello.climates),features:hello.features,
      crownVictory:hello.crownVictory===true,
      capitolVictory:hello.capitolVictory===true,
      capitolRoster:hello.capitolRoster===true,
      captureCheckpoint:hello.captureCheckpoint===true,
      startingTeams:hello.startingTeams===true,
      sightTerms:hello.sightTerms===true,
      timedTreaties:hello.timedTreaties===true,
      borderTreaties:hello.borderTreaties===true,
      fishing:hello.fishing===true,
      checkpoints:hello.checkpoints===true,sharedOwner:hello.sharedOwner===true,
      sharedOwnerCheckpoint:hello.sharedOwner===true&&hello.sharedOwnerCheckpoint===true,groupedCoop:hello.groupedCoop===true,
      groupedCheckpoint:hello.groupedCoop===true&&hello.groupedCheckpoint===true};
  },
  makeSettings(humanSeats,ai,guests,sharedOwner,groupedOwners){
    const choices=civilizationSetup.choices(),climate=menuClimate();
    if(!guests.every(guest=>guest.climates.includes(climate)))throw Error('Einem Mitspieler fehlen die Grafiken für dieses Klima.');
    const worldHumans=sharedOwner?1:groupedOwners?Math.max(...groupedOwners)+1:humanSeats;
    if(groupedOwners){
      for(let owner=1;owner<worldHumans;owner++)choices[owner]=guests[groupedOwners.indexOf(owner)-1].civilization;
    }else if(!sharedOwner)for(const [index,guest] of guests.entries())choices[index+1]=guest.civilization;
    const features=supportedRosterFeatures.filter(key=>guests.every(guest=>guest.features.includes(key)));
    const crownVictory=crownVictoryMenu.read();
    if(crownVictory===null)throw Error('Ungültiges Kronen-Siegziel.');
    if(crownVictory&&(!supportsNetworkCrownVictory||!features.includes('crowns')||sharedOwner||groupedOwners||
      guests.some(guest=>!guest.crownVictory)))
      throw Error('Kronen-Siegbedingungen benötigen einzelne Spielerplätze und eine passende UP-Spielversion.');
    const capitol=capitolsMenu.read();
    if(capitol===null)throw Error('Ungültige Dauer für die Kapitol-Rückeroberung.');
    if(capitol){
      const roster=requiresCapitolRoster(capitol.mode,humanSeats,ai);
      if(!supportsNetworkCapitols||sharedOwner||groupedOwners||!features.includes('captureAnimation')||
        (roster&&!supportsNetworkCapitolRoster)||guests.some(guest=>!guest.capitolVictory||(roster&&!guest.capitolRoster)))
        throw Error('Kapitole benötigen getrennte Zivilisationen und eine passende Eroberungsregel.');
    }
    const capitolVictory=capitol?{...capitol,sole:capitol.mode===5&&$('sk-sole-survivor').checked}:undefined;
    const captureCheckpoint=supportsCaptureCheckpoint&&!sharedOwner&&!groupedOwners&&!capitolVictory&&
      features.includes('capture')&&guests.every(guest=>guest.captureCheckpoint);
    const startingTeams=networkTeamPicker.read();
    if(startingTeams&&guests.some(guest=>!guest.startingTeams))throw Error('Alle Mitspieler benötigen eine Spielversion mit Startteams.');
    const sightTerms=supportsNetworkSightTerms&&!sharedOwner&&!groupedOwners&&guests.every(guest=>guest.sightTerms);
    const timedTreaties=sightTerms&&supportsNetworkTimedTreaties&&guests.every(guest=>guest.timedTreaties);
    const borderTreaties=timedTreaties&&supportsNetworkBorderTreaties&&guests.every(guest=>guest.borderTreaties);
    const fishing=supportsNetworkFishing&&!sharedOwner&&!groupedOwners&&guests.every(guest=>guest.fishing);
    const resolved=resolvedSkirmishChoices(choices,worldHumans+ai);
    return {humanSeats,ai,seed:(seedOf()>>>0)||1,visibility:+$('map-visibility').value,
      popLimit:+$('pop-limit').value,supplies:+$('supplies').value,resignationPolicy:+$('mp-resign-policy').value,
      ceasefire:+$('initial-ceasefire').value,climate,seasons:menuSeasonSettings(),gameTempo:menuTempo(),
      features,...(features.includes('crowns')?{crownsDisabled:$('disable-crowns').checked}:{}),
      ...(crownVictory?{crownVictory}:{}),
      ...(capitolVictory?{capitolVictory}:{}),
      ...(captureCheckpoint?{captureCheckpoint:true}:{}),
      ...(startingTeams?{startingTeams}:{}),
      ...(sightTerms?{sightTerms:true}:{}),
      ...(timedTreaties?{timedTreaties:true}:{}),
      ...(borderTreaties?{borderTreaties:true}:{}),
      ...(fishing?{fishing:true}:{}),
      ...resolved,
      ...(sharedOwner?{sharedOwner:true}:groupedOwners?{groupedOwners}:{})};
  },
  checkSettings:checkRosterSettings,
  validateArchive:supportsNetworkCheckpoint?(archive,seat,seats)=>{
    if(archive.decoded.meta.groupedOwners&&!supportsGroupedCheckpoint)throw Error('Dieser Spielstand benötigt Unterstützung für Gruppen-Koop-Spielstände.');
    if(archive.decoded.meta.sharedOwner&&!supportsSharedCheckpoint)throw Error('Dieser Spielstand benötigt Koop-Spielstandunterstützung.');
    applyNetworkCheckpoint(wasm,archive.decoded,seat,seats,true,checkpointRules(archive.settings),checkpointStartingTeams(archive.settings.startingTeams));
  }:null,
  sharedOwnerAvailable:supportsSharedOwner,
  sharedCheckpointAvailable:supportsSharedCheckpoint,
  groupedCoopAvailable:supportsGroupedCoop,
  groupedCheckpointAvailable:supportsGroupedCheckpoint,
  onCheckpointState(message,archive){
    toast(message);
    if(archive){
      checkpointDownload=archive;
      const button=$('mp-checkpoint-download');
      button.textContent=`Spielstand herunterladen · Schritt ${archive.tick}`;
      button.hidden=false;
    }
  },
  launch:startRosterGame,
  onOpen(){pending?.pc.close();pending=null;},
  onDeparture(seat,grouped){toast(`Verbindung zu ${grouped?'Controller':'Spieler'} ${seat+1} wurde getrennt.`);},
  onHostLoss({peer}){
    if(peer.terminal)return;
    peer.hostLost=true;
    peer.setTransportAvailable(false);
    toast('Hostverbindung verloren. Die Partie wurde angehalten.');
  },
}):null;

// ?bench=2000 or ?seed=42 start immediately (handy for profiling and screenshots)
const qs = new URLSearchParams(location.search);
if (qs.has('bench')) start(1, +qs.get('seed') || 7, +qs.get('bench') || 2000);
else if (qs.has('seed')) start(+qs.get('ai') || 1, +qs.get('seed') || 1, 0);
// ?warm=600 fast-forwards that many ticks; ?zoom= ?x= ?y= place the camera
for (let i = +qs.get('warm') || 0; i > 0 && running; i--) {
  if (adaptiveSoloClock) {
    if (!advance(sourceWallInterval(adaptiveSoloClock.rate) + 0.0000001, performance.now())) break;
  } else if (sourceSoloClock) {
    if (!wasm.tick_with_source_delta(sourceStepDelta(speed))) break;
  } else wasm.tick();
}
for (const k of ['zoom', 'x', 'y']) if (qs.has(k)) cam[k] = +qs.get(k);
window.openepoch = { wasm, rulesProfile, renderNamespace, cam, gl, frame, start, saveGame, loadGame, toWorld, channel, seasonSession, nativeDiplomacy, nativeBitmapFonts, fullmapPip, get net() { return net; } };
const loop = now => { requestAnimationFrame(loop); frame(now); };
requestAnimationFrame(loop);
