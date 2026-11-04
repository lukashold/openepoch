import fs from 'node:fs';
import {ORIGINAL_GROUND_SHADERS} from '../docs/original-ground.js';
if(!process.argv[2])throw new Error('Usage: node export_original_ground_shaders.mjs output.json');
fs.writeFileSync(process.argv[2],JSON.stringify({programs:[{name:'original-ground',...ORIGINAL_GROUND_SHADERS}]}));
