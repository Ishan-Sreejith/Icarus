import fs from 'node:fs';
import init, { CoReEngine, get_sample_code } from './pkg/forge.js';

try {
  const bytes = fs.readFileSync(new URL('./pkg/forge_bg.wasm', import.meta.url));
  await init(bytes);
  const engine = new CoReEngine();
  const code = get_sample_code();
  const out = engine.execute(code);
  console.log('OK output:', out);
} catch (e) {
  console.error('ERR', e);
  process.exit(1);
}
