// Download JSON data only; do not import or execute any dataset contents.
// SPDX-License-Identifier: Apache-2.0
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import {createGunzip} from 'node:zlib';
import {createHash} from 'node:crypto';
import {Readable} from 'node:stream';
import {pipeline} from 'node:stream/promises';

const root=process.argv[2]||process.env.BETA_DATA_DIR;
if(!root)throw new Error('Set BETA_DATA_DIR or pass the data root as argument 1.');
const count=Number(process.argv[3]||300);
if(!Number.isInteger(count)||count<1||count>1000)throw new Error('Sample count must be 1..1000');
const revision='f238540ebc7f98dcf5cb7ed1154e8396bc5285fb';
const name='garment_specifications_archive.tar.gz';
const url=`https://huggingface.co/georgeNakayama/AIpparel/resolve/${revision}/${name}`;
const expectedBytes=251722049,expectedHash='fa7d016bde8bf036ce74575b1e8995dc85fc4a86729fad743d8514115d819c86';
const dir=path.join(root,'datasets','garmentcodedata'),archive=path.join(dir,name);
await fsp.mkdir(path.join(dir,'specs'),{recursive:true});
if(!fs.existsSync(archive)){
  const head=await fetch(url,{method:'HEAD',redirect:'follow'});if(!head.ok)throw new Error(`HEAD ${head.status}`);
  const bytes=Number(head.headers.get('content-length'));
  if(bytes!==expectedBytes)throw new Error(`Unexpected archive size ${bytes}; refusing download`);
  const response=await fetch(url);if(!response.ok)throw new Error(`Download ${response.status}`);
  await pipeline(Readable.fromWeb(response.body),fs.createWriteStream(archive+'.part'));
  await fsp.rename(archive+'.part',archive);
}
if((await fsp.stat(archive)).size!==expectedBytes)throw new Error('Archive byte count differs');
const hash=createHash('sha256');for await(const chunk of fs.createReadStream(archive))hash.update(chunk);
if(hash.digest('hex')!==expectedHash)throw new Error('Archive SHA-256 differs');

// Bounded tar reader: regular JSON files only, no links, no path extraction.
// Stop once count entries have been copied; never unpack the full archive.
let buf=Buffer.alloc(0),pending=null,remaining=0;const samples=[];
const input=fs.createReadStream(archive),gunzip=input.pipe(createGunzip());
try {
  sampling:for await(const chunk of gunzip){
    buf=Buffer.concat([buf,chunk]);
    while(true){
      if(pending){
        if(buf.length<remaining)break;
        if(pending.regular&&pending.name.endsWith('_specification_shifted.json')){
          const data=buf.subarray(0,pending.size);JSON.parse(data.toString('utf8'));
          const basename=path.posix.basename(pending.name);
          if(samples.some(x=>x.file===basename))throw new Error(`Duplicate sample name ${basename}`);
          await fsp.writeFile(path.join(dir,'specs',basename),data);
          samples.push({archive_path:pending.name,file:basename,bytes:data.length,sha256:createHash('sha256').update(data).digest('hex')});
          if(samples.length===count)break sampling;
        }
        buf=buf.subarray(remaining);pending=null;
      }else{
        if(buf.length<512)break;
        const head=buf.subarray(0,512);buf=buf.subarray(512);
        if(head.every(x=>x===0))break sampling;
        const size=parseInt(head.subarray(124,136).toString().replace(/\0.*/,''),8);
        if(!Number.isSafeInteger(size)||size<0||size>10_000_000)throw new Error('Oversize/invalid tar member');
        const type=head[156];
        pending={name:head.subarray(0,100).toString().replace(/\0.*/,''),size,regular:type===0||type===48};
        remaining=Math.ceil(size/512)*512;
      }
    }
  }
}finally{input.destroy();gunzip.destroy();}
if(samples.length!==count)throw new Error(`Only found ${samples.length}/${count} specifications`);
const manifest={source_url:url,mirror:'AIpparel authors / GarmentCodeData specifications',license:'CC-BY-4.0 (original GarmentCodeData)',archive_bytes:expectedBytes,archive_sha256:expectedHash,sampling:'First N regular specification JSON entries in archive order; not random or representative',sample_count:samples.length,specification_bytes:samples.reduce((s,x)=>s+x.bytes,0),samples};
await fsp.writeFile(path.join(dir,'sample-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({archive_bytes:expectedBytes,sample_count:samples.length,specification_bytes:manifest.specification_bytes}));
