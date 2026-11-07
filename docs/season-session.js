import {createSeasonClock,normalizeSeasonSettings} from './original-seasons.js?v=clock38';
import {checkedClimate,CLIMATE_NAMES} from './skirmish-climates.js';

// Saves made before the calendar existed displayed fixed summer. Preserve that
// appearance instead of unexpectedly advancing an old game's calendar.
export const LEGACY_SEASON_SETTINGS=Object.freeze({cycle:false,startMonth:6,minutesPerSeason:5});

export function peerSeasonSettings(value){
  if(!value||typeof value!=='object'||
     !['cycle','startMonth','minutesPerSeason'].every(key=>Object.hasOwn(value,key)))
    throw new TypeError('Missing season settings');
  return normalizeSeasonSettings(value);
}

export function savedSeasonSettings(record){
  return record.seasons===undefined?LEGACY_SEASON_SETTINGS:peerSeasonSettings(record.seasons);
}

// Both renderers read the same simulation clock so pause, save/load and
// different display refresh rates preserve their season phase.
export function createSeasonSession(wasm,terrain,ground,models=null,climates=[0]){
  let clock=createSeasonClock(),climate=0;
  return Object.freeze({
    configure(settings,requestedClimate=0){
      const nextClock=createSeasonClock(settings);
      const value=checkedClimate(requestedClimate,climates);
      if(value!==climate){
        const name=CLIMATE_NAMES[value];
        terrain.setSeasonClimate(name);
        ground.setSeasonClimate(name);
        models.setSeasonClimate(name);
        models.setLightingSettings({profile:'native155',climate:name});
        terrain.setLightingContext(models.lighting);
        climate=value;
      }
      clock=nextClock;
    },
    get settings(){return clock.settings;},
    get climate(){return climate;},
    update(){
      const phase=wasm.sim_clock_active?.()===1?clock.atSeconds(wasm.sim_elapsed_seconds()):clock.atTick(wasm.game_tick()>>>0);
      terrain.setSeasonPhase(phase);
      ground.setSeasonPhase(phase);
      models?.setSeasonPhase?.(phase);
      return phase;
    },
  });
}
