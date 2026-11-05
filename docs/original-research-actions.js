import {RESOURCE_NAMES} from './economy-bar.js';

// Native wallet order follows NATIVE_IDS in the simulation's economy module.
const RESOURCE_WORDS = [0, 1, 5, 2, 3, 4, 6, 7, 8];
const IMPROVEMENTS = ['Veteran', 'Elite']; // text_tt_upgrade_veteran / text_tt_upgrade_elite
const IMPROVEMENT_EFFECT = 'Leichte Infanterie: +10 Prozentpunkte Lebenspunkte und Schaden';

export function researchCost({cost, tp}) {
 const parts = cost.flatMap((value, slot) => value ? [`${value} ${RESOURCE_NAMES[slot]}`] : []);
 if (tp) parts.push(`${tp} TP`);
 return parts.join(' · ') || 'Kostenlos';
}

export function researchState(detail, hostName) {
 switch (detail.status) {
  case 2: return `Forschung läuft · ${Math.ceil(detail.seconds - detail.elapsed)} Sek. verbleiben`;
  case 3: return '✓ Erforscht';
  case 4: return 'Bezahlt · wartet auf den Forschungsplatz';
  case 5: return 'Benötigt Veteran';
  default: return !detail.host ? `Benötigt ${hostName}` : detail.ok ? 'Forschen' : 'Rohstoffe oder Technologiepunkte fehlen';
 }
}

export function originalResearchActions(wasm) {
 const available = typeof wasm.ai9_game_research_setup_manual === 'function';
 const manual = () => available && wasm.ai9_game_research_mode() === 2;

 function describe(id, selected = false) {
  if (!manual()) return null;
  const status = wasm.ai9_game_research_status(id);
  if (!status) return null;
  const info = field => wasm.ai9_game_research_info(id, field);
  return {
   status, host: info(selected ? 1 : 0) === 1, ok: info(selected ? 3 : 2) === 1,
   seconds: info(4), elapsed: status === 2 ? wasm.ai9_game_research_elapsed(id) : 0,
   cost: RESOURCE_WORDS.map(word => info(16 + word)), tp: info(25),
  };
 }

 function forAction(action) {
  if (!manual() || !action) return null;
  const id = action.k === 'i' ? action.id
   : action.k === 'r' && wasm.queue_item_kind(action.id) === 1 ? wasm.queue_item_value(action.id) : null;
  const detail = id === null ? null : describe(id, true);
  return detail && {id, ...detail};
 }

 function improvements(epoch) {
  if (!manual() || epoch > 1) return [];
  return IMPROVEMENTS.map((name, index) => ({
   id: wasm.ai9_game_improvement_job(index + 1), name, effect: IMPROVEMENT_EFFECT,
   host: 'Leichte Infanterie', branch: 1, epoch,
  }));
 }

 function selectedImprovements(epoch) {
  return improvements(epoch).flatMap(row => {
   const detail = describe(row.id, true);
   if (!detail?.host || detail.status === 3) return [];
   return [{k: 'i', id: row.id, name: row.name, ...detail,
    tip: `${row.effect} · ${detail.seconds} Sek.${detail.status === 5 ? ' · Benötigt Veteran' : ''}`}];
  });
 }

 function unitName(name, id, team) {
  if (!manual() || !Number.isInteger(id) || !Number.isInteger(team)) return name;
  // tx_utn_veteran_modify_name / tx_utn_elite_modify_name in the German table.
  switch (wasm.ai9_game_unit_rank(team, id)) {
   case 1: return `Veteran ${name}`;
   case 2: return `Elite-${name}`;
   default: return name;
  }
 }

 return Object.freeze({describe, forAction, improvements, selectedImprovements, unitName,
  cancel: id => manual() && wasm.ai9_game_research_cancel(id) === 1});
}
