// Drives the wasm build through a full wizard run. Usage: node wasm/smoke-test.mjs [path/to/phoenix_wasm.wasm]
import fs from 'fs'; import crypto from 'crypto';
const bytes = fs.readFileSync(process.argv[2] || new URL('./target/wasm32-unknown-unknown/release/phoenix_wasm.wasm', import.meta.url));
const mod = await WebAssembly.compile(bytes);
console.log('imports:', WebAssembly.Module.imports(mod).map(i=>i.module+'.'+i.name));
const { exports: x } = await WebAssembly.instantiate(mod, {});
const enc = new TextEncoder(), dec = new TextDecoder();
function call(req) {
  const b = enc.encode(JSON.stringify(req)); const p = x.mem_alloc(b.length);
  new Uint8Array(x.memory.buffer, p, b.length).set(b);
  const n = x.call(p, b.length); x.mem_free(p, b.length);
  return JSON.parse(dec.decode(new Uint8Array(x.memory.buffer, x.out_ptr(), n)));
}
const seed = () => [...crypto.randomBytes(32)];
let a = {};
a = call({op:'patch', answers:a, patch:{name:'Ada Kowalski', pronouns:'she/her', age:48}}).ok;
for (const c of ['characteristics','edu_checks','luck']) a = call({op:'roll', answers:a, request:{ceremony:c}, seed:seed()}).ok;
console.log('dice', JSON.stringify(a.dice));
console.log('top careers', call({op:'careers', characteristics:a.dice.characteristics}).ok.slice(0,3).map(c=>c.id+':'+c.fit));
console.log('early finalize', JSON.stringify(call({op:'finalize', answers:a}).error));
a = call({op:'patch', answers:a, patch:{careerId:'funerary-director', hobbyIds:['occult-curiosity','people-watching','poker-nights','bookworm']}}).ok;
const s = call({op:'finalize', answers:a}).ok;
console.log('derived', JSON.stringify(s.derived)); console.log('pools', JSON.stringify(s.pools));
console.log('bad declare', JSON.stringify(call({op:'roll', answers:a, request:{ceremony:'declared', characteristics:{str:52,con:60,siz:30,dex:55,app:50,pow:65,int:70,edu:75}}, seed:seed()}).error));
