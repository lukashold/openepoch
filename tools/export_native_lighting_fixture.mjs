// Production GLSL plus immutable original source values for independent GLES QA.
import fs from 'node:fs';
import {MODEL_SHADERS} from '../docs/original-renderer.js';
if(!process.argv[2])throw Error('Expected output path');
const lighting=JSON.parse(fs.readFileSync(new URL('../docs/local-original/climate-lighting.json',import.meta.url)));
fs.writeFileSync(process.argv[2],JSON.stringify({build:'native5',programs:[{name:'native5-model-lighting',...MODEL_SHADERS}],lighting}));
