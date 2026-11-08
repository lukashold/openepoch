// Numerical rule profiles are explicit; assets keep the declared visual adapters.
// Enable only after the imported UP binary and its source/ABI oracle are released.
export const UP15_PROFILE_ENABLED=true;
const PROFILES=Object.freeze({
 own:Object.freeze({id:'own',ruleset:0,label:'OpenEpoch-Regeln',wasm:'openepoch.wasm?v=clock38',database:'openepoch-v2'}),
 base:Object.freeze({id:'base',ruleset:1,label:'EE2 · Basisdaten',wasm:'openepoch-ee2.wasm?v=combat-season1',database:'openepoch-ee2-reference-v2'}),
 up15:Object.freeze({id:'up15',ruleset:2,label:'EE2 · installierte UP-Regeln',wasm:'openepoch-ee2-up15.wasm?v=native82',schemaRevision:9,definitionCount:109,packSchemaRevision:4,requiredExports:['configure_match_settings','configure_match_epoch_range','configure_match_territory_victory','init_hill_world','configure_match_koth','koth_definition','koth_status','configure_match_crown_victory','crown_victory_status','configure_match_capitols','capitol_status','configure_match_sole_survivor','configure_match_regicide','match_final_epoch','render_stone_thrower_attack_elapsed','pack_selected_count','cmd_pack','render_pack_version','navigation_history_unknown','projectile_history_unknown','source_identity_version','actor_source_identity','selected_actor_handle','source_rng_origin','source_rng_history_unknown','ai9_game_research_setup_manual','ai9_game_research_mode','ai9_game_research_info','ai9_game_research_status','ai9_game_research_elapsed','ai9_game_research_cancel','ai9_game_improvement_job','ai9_game_improvement_family_job','ai9_game_unit_rank','air_mission_status','air_mission_points_ptr','cmd_air_mission_route','cmd_air_mission_action','air_runway_configure_peer','air_takeoff_configure_peer','air_takeoff_start_tick','air_landing_configure_peer','air_carrier_configure_peer','configure_solo_command_queue','cmd_resign_solo','configure_network_resignation','cmd_resign_network','network_peer_disconnected','configure_initial_ceasefire','initial_ceasefire_seconds','air_mission_visual_routes','air_mission_visual_routes_ptr','configure_priest_skirmish','priest_power','capture_selection_count_for','capture_target_at_for','bless_selection_count','bless_target_at','cmd_bless_at','spy_power','spy_disguised','spy_selection_count','spy_intel_target_at','cmd_spy_intel_at','spy_visual_state','spy_visual_elapsed','cmd_spy_extraction','spy_extraction_count','spy_extraction_field','spy_extraction_selection_count','cmd_cancel_spy','spy_operation_unlocked','spy_operation_selection_count','spy_operation_target_at','cmd_spy_operation_at','building_sabotaged','spy_toxic_status','spy_adverse_glow_present','configure_crowns','configure_crown_leaders','configure_dock_garrison','configure_military_leader_powers','configure_economic_leader_effects','leader_power_reserve','cmd_leader_rally','cmd_leader_fire_maneuver','configure_leader_diversion','leader_diversion_enabled','leader_diversion_ready','leader_diversion_target_at','cmd_leader_diversion_at','leader_diversion_visual','configure_heavy_mounted3','heavy_mounted3_definition','configure_guided_herc_rocket','cmd_imperial_construction_at','cmd_leader_mobilization_at','leader_purchase_selection_count','leader_purchase_target_at','cmd_leader_purchase_at','imperialism_selection_count','imperialism_target_at','cmd_imperialism_at','imperialism_visible','crown_rating','crown_holder','crown_technology','crown_choose_power','crown_chosen_power','crown_can_choose_power','crown_visuals_present','crown_actor_visual','crown_actor_glow','wonder_footprint'],sourceIdentityVersion:1,database:'openepoch-ee2-up15-v1'}),
});

export function resolveRulesProfile(input,{up15Enabled=UP15_PROFILE_ENABLED}={}){
 const url=new URL(input),reference=url.searchParams.get('rules')==='ee2';
 const id=reference?(url.searchParams.get('profile')??(up15Enabled?'up15':'base')):'own';
 if(!Object.hasOwn(PROFILES,id)||reference&&id==='own')throw Error('Unbekannte Regelversion. Wähle Basisdaten oder installierte UP-Regeln.');
 if(id==='up15'&&!up15Enabled)throw Error('Die installierten UP-Regeln sind noch nicht freigegeben. Wähle vorerst EE2 · Basisdaten.');
 const qa=url.searchParams.has('qa');
 return Object.freeze({...PROFILES[id],reference,qa,database:PROFILES[id].database+(qa?'-qa':'')});
}

export function rulesProfileURL(input,id,{up15Enabled=UP15_PROFILE_ENABLED}={}){
 if(!Object.hasOwn(PROFILES,id))throw Error('Unbekannte Regelversion.');
 if(id==='up15'&&!up15Enabled)throw Error('Die installierten UP-Regeln sind noch nicht freigegeben.');
 const url=new URL(input);
 if(id==='own'){url.searchParams.delete('rules');url.searchParams.delete('profile');}
 else{url.searchParams.set('rules','ee2');url.searchParams.set('profile',id);}
 return url;
}

export function assertRulesProfile(wasm,profile){
 if(typeof wasm?.ruleset!=='function'||wasm.ruleset()!==profile.ruleset)throw Error(`Falsche Spieldatei für ${profile.label}. Entpacke das vollständige Paket erneut; Regelprofile werden nicht automatisch ersetzt.`);
 if(profile.schemaRevision!==undefined&&(typeof wasm.schema_revision!=='function'||wasm.schema_revision()!==profile.schemaRevision))throw Error('Falsches Speicher-/Einheitenformat. Entpacke das vollständige aktuelle Paket erneut.');
 if(profile.definitionCount!==undefined&&(typeof wasm.definition_count!=='function'||wasm.definition_count()!==profile.definitionCount))throw Error('Falsches Speicher-/Einheitenformat: Einheitensatz stimmt nicht überein.');
 if(profile.requiredExports?.some(name=>typeof wasm[name]!=='function'))throw Error('Falsches Speicher-/Einheitenformat: Spielbefehle oder Starteinstellungen fehlen.');
 if(profile.sourceIdentityVersion!==undefined&&(typeof wasm.source_identity_version!=='function'||wasm.source_identity_version()!==profile.sourceIdentityVersion))throw Error('Falsches Speicher-/Einheitenformat: Bürgeridentitäten fehlen.');
 if(profile.packSchemaRevision!==undefined&&(typeof wasm.pack_schema_revision!=='function'||wasm.pack_schema_revision()!==profile.packSchemaRevision))throw Error('Falsches Speicher-/Einheitenformat: Wegfindungs- und Aufgabenstand stimmt nicht überein.');
 return wasm;
}

/** One requested file, with MIME-compatible decoding fallback of the same bytes. */
export async function loadRulesWasm(profile,base,{fetchFile=globalThis.fetch,webAssembly=globalThis.WebAssembly}={}){
 try{
  const response=await fetchFile(new URL(profile.wasm,base));
  if(!response.ok)throw Error(`Datei fehlt oder ist nicht lesbar (HTTP ${response.status}).`);
  let result;
  if(typeof webAssembly.instantiateStreaming==='function'&&typeof response.clone==='function'){
   try{result=await webAssembly.instantiateStreaming(response.clone(),{});}catch{result=await webAssembly.instantiate(await response.arrayBuffer(),{});}
  }else result=await webAssembly.instantiate(await response.arrayBuffer(),{});
  return assertRulesProfile(result.instance.exports,profile);
 }catch(error){throw Error(`${profile.label} konnte nicht geladen werden. ${error.message}`);}
}

// Legacy records in the existing own/BASE databases predate these two fields.
// The new UP database has no implicit legacy import or cross-profile migration.
export function savedRulesProfileMatches(record,profile){
 if(!record||typeof record!=='object')return false;
 if(record.rules===undefined&&record.profile===undefined)return profile.ruleset!==2;
 return record.rules===profile.ruleset&&record.profile===profile.id;
}
