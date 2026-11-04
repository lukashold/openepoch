import {createSeasonClock,normalizeSeasonSettings} from './original-seasons.js';

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

// Both renderers read the same integer simulation tick. Pausing, network waits,
// save/load and different display refresh rates therefore keep their phase.
export function createSeasonSession(wasm,terrain,ground){
  let clock=createSeasonClock();
  return Object.freeze({
    configure(settings){clock=createSeasonClock(settings);},
    get settings(){return clock.settings;},
    update(){
      const phase=clock.atTick(wasm.game_tick()>>>0);
      terrain.setSeasonPhase(phase);
      ground.setSeasonPhase(phase);
      return phase;
    },
  });
}
