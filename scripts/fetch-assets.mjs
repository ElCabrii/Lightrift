import {mkdir,writeFile} from 'node:fs/promises';
const out=new URL('../assets/',import.meta.url);
await mkdir(out,{recursive:true});
async function get(url){const r=await fetch(url);if(!r.ok)throw new Error(`${r.status}: ${url}`);return r;}
const versions=await (await get('https://ddragon.leagueoflegends.com/api/versions.json')).json();
const patch=versions[0];
await writeFile(new URL('patch.txt',out),patch);
const files=await Promise.all(['champion','item','runesReforged','summoner'].map(async name=>{const j=await(await get(`https://ddragon.leagueoflegends.com/cdn/${patch}/data/en_US/${name}.json`)).json();await writeFile(new URL(`${name}.json`,out),JSON.stringify(j));return j;}));
const sprites=[...new Set(files.flatMap(j=>Object.values(j.data??{}).map(x=>x.image?.sprite).filter(Boolean)))];
await Promise.all(sprites.map(async name=>{await writeFile(new URL(name,out),Buffer.from(await(await get(`https://ddragon.leagueoflegends.com/cdn/${patch}/img/sprite/${name}`)).arrayBuffer()));}));
const lines=sprites.map(name=>`    (${JSON.stringify(name)}, include_bytes!("../assets/${name}") as &[u8]),`).join('\n');
await writeFile(new URL('../src/sprites.rs',out),`pub const SPRITES: &[(&str, &[u8])] = &[\n${lines}\n];\n`);
await writeFile(new URL('riotgames.pem',out),await(await get('https://static.developer.riotgames.com/docs/lol/riotgames.pem')).text());
console.log(JSON.stringify({patch,champions:Object.keys(files[0].data).length,items:Object.keys(files[1].data).length,sprites:sprites.length}));
