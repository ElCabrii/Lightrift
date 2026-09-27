import {readFile,writeFile,mkdir} from 'node:fs/promises';
const root=new URL('../',import.meta.url);
const trees=JSON.parse(await readFile(new URL('assets/runesReforged.json',root),'utf8'));
const icons=trees.flatMap(t=>[{id:t.id,icon:t.icon},...t.slots.flatMap(s=>s.runes)]);
await mkdir(new URL('assets/runes/',root),{recursive:true});
for(let i=0;i<icons.length;i+=8){await Promise.all(icons.slice(i,i+8).map(async r=>{const res=await fetch('https://ddragon.leagueoflegends.com/cdn/img/'+r.icon);if(!res.ok)throw new Error(`Rune ${r.id}: ${res.status}`);await writeFile(new URL(`assets/runes/${r.id}.png`,root),Buffer.from(await res.arrayBuffer()));}));}
await writeFile(new URL('src/rune_icons.rs',root),'pub const RUNE_ICONS: &[(&str, &[u8])] = &[\n'+icons.map(r=>`    ("rune${r.id}", include_bytes!("../assets/runes/${r.id}.png") as &[u8]),`).join('\n')+'\n];\n');
console.log(`Downloaded ${icons.length} official rune icons`);
