// Export production shader text for the no-UI native EGL/GLES regression test.
import fs from 'node:fs';
import {MODEL_SHADERS} from '../docs/original-renderer.js';
if(!process.argv[2])throw new Error('Usage: node export_original_model_shaders.mjs output.json');
fs.writeFileSync(process.argv[2],JSON.stringify({programs:[{name:'original-models',...MODEL_SHADERS}]}));
