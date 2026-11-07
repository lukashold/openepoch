// Render-only calendar. Tick rate comes from the deterministic simulation.
// Native Calendar constructor006C8B60 uses a1200-second year and cycling on;
// terrain headers require twelve uniformly interpolated monthly images.
// Starting in summer is this application's choice, not a native saved preset.
export const MINUTES_PER_SEASON=Object.freeze([1,5,10,20,30]);
export const MONTH_NAMES=Object.freeze(['Januar','Februar','März','April','Mai','Juni','Juli','August','September','Oktober','November','Dezember']);
export const DEFAULT_SEASON_SETTINGS=Object.freeze({cycle:true,startMonth:6,minutesPerSeason:5});
const TICKS_PER_SECOND=20;

export function normalizeSeasonSettings(options={}){
 const {cycle=DEFAULT_SEASON_SETTINGS.cycle,startMonth=DEFAULT_SEASON_SETTINGS.startMonth,minutesPerSeason=DEFAULT_SEASON_SETTINGS.minutesPerSeason}=options;
 if(typeof cycle!=='boolean')throw new TypeError('Season cycle must be boolean');
 if(!Number.isInteger(startMonth)||startMonth<0||startMonth>11)throw new RangeError('Start month must be an integer 0..11');
 if(!MINUTES_PER_SEASON.includes(minutesPerSeason))throw new RangeError('Unsupported minutes per season');
 return Object.freeze({cycle,startMonth,minutesPerSeason});
}

export function seasonAtTick(tick,options=DEFAULT_SEASON_SETTINGS){
 if(!Number.isSafeInteger(tick)||tick<0)throw new RangeError('Season tick must be a non-negative safe integer');
 const settings=normalizeSeasonSettings(options);
 const ticksPerYear=settings.minutesPerSeason*4*60*TICKS_PER_SECOND;
 // Reduce integral ticks first: long sessions do not accumulate float deltas.
 const elapsedMonths=settings.cycle?(tick%ticksPerYear)*12/ticksPerYear:0;
 return seasonPhase(elapsedMonths,ticksPerYear,settings);
}

function seasonPhase(elapsedMonths,ticksPerYear,settings){
 const monthPosition=(settings.startMonth+elapsedMonths)%12;
 const month0=Math.floor(monthPosition),month1=(month0+1)%12,blend=monthPosition-month0;
 return Object.freeze({month0,month1,blend,monthPosition,monthName:MONTH_NAMES[month0],
  secondsPerYear:ticksPerYear/TICKS_PER_SECOND,ticksPerYear,settings});
}

export function createSeasonClock(options={}){
 const settings=normalizeSeasonSettings(options);
 const secondsPerYear=settings.minutesPerSeason*4*60;
 return Object.freeze({settings,atTick:tick=>seasonAtTick(tick,settings),atSeconds(seconds){
   if(!Number.isFinite(seconds)||seconds<0)throw new RangeError('Season time must be finite and nonnegative');
   const elapsedMonths=settings.cycle?(seconds%secondsPerYear)*12/secondsPerYear:0;
   return seasonPhase(elapsedMonths,secondsPerYear*TICKS_PER_SECOND,settings);
 }});
}
