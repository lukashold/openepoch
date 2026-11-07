export const ORIGINAL_AUDIO_DEFAULTS=Object.freeze({music:true,ambience:true,musicVolume:1,soundVolume:.75});

export function originalAudioLevel(value,fallback){
  return Number.isFinite(value)?Math.fround(Math.min(1,Math.max(0,value))):fallback;
}

// Both original Miles bus writers store ln(1 + 7*x) / ln(8) as binary32.
export const originalAudioGain=level=>Math.fround(Math.log1p(7*level)/Math.log(8));
