// Keep one graded size from an openly licensed ASCII DXF. No vendor code.
// SPDX-License-Identifier: Apache-2.0
import fs from 'node:fs/promises';

const [input,output,size='M']=process.argv.slice(2);
if(!input||!output||!/^[A-Za-z0-9]+$/.test(size))throw new Error('Usage: node extract-dxf-size.mjs INPUT OUTPUT [M]');
const lines=(await fs.readFile(input,'utf8')).trimEnd().split(/\r?\n/);
if(lines.length%2)throw new Error('DXF must contain group-code/value pairs');
const pairs=[];for(let i=0;i<lines.length;i+=2){const code=Number(lines[i].trim());if(!Number.isInteger(code))throw new Error('Invalid DXF group code');pairs.push({code,value:lines[i+1],raw:lines[i]+'\n'+lines[i+1]+'\n'});}
const records=[];for(const p of pairs){if(p.code===0||!records.length)records.push([]);records.at(-1).push(p);}
const field=(record,code)=>record.find(p=>p.code===code)?.value.trim();
const retained=new Set();let block=null,keep=true,section=null;const out=[];
for(const record of records){
  const type=field(record,0);
  if(type==='SECTION')section=field(record,2);
  if(type==='BLOCK'){
    block=field(record,2);keep=block.endsWith('_'+size);if(keep)retained.add(block);
  }
  if(block===null&&section==='ENTITIES'){
    if(type==='INSERT'){if(!retained.has(field(record,2)))continue;}
    else if(!['SECTION','ENDSEC','TEXT'].includes(type))throw new Error(`Unexpected top-level entity ${type}; refusing to guess its size`);
  }
  if(keep)out.push(...record.map(p=>p.raw));
  if(type==='ENDBLK'){block=null;keep=true;}
  if(type==='ENDSEC')section=null;
}
if(!retained.size)throw new Error(`No _${size} blocks`);
const result=out.join('');if(Buffer.byteLength(result)>=1_000_000)throw new Error('Filtered DXF exceeds commit size limit');
await fs.writeFile(output,result);console.log(JSON.stringify({size,blocks:retained.size,bytes:Buffer.byteLength(result)}));
