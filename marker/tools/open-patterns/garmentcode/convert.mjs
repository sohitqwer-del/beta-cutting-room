// One-time GarmentCode JSON data converter. SPDX-License-Identifier: Apache-2.0
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {panelOutline,allowance} from './panels.mjs';
import {job,preview,roundPt,stats,writeJob} from '../geometry.mjs';

const here=path.dirname(fileURLToPath(import.meta.url));
const args=process.argv.slice(2);
const root=process.env.BETA_DATA_DIR;
const input=args[0]||(root&&path.join(root,'datasets','garmentcodedata','specs'));
const output=args[1]||(root&&path.join(root,'marker-bench','open-patterns','garmentcode'));
const examples=args[2]||path.resolve(here,'../../../testdata/open/garmentcode');
if(!input||!output)throw new Error('Set BETA_DATA_DIR or pass INPUT_DIR OUTPUT_DIR [EXAMPLES_DIR]');
await fs.mkdir(output,{recursive:true});await fs.mkdir(examples,{recursive:true});

const report={sample_count:0,converted:0,rejected:[],curve_types:[],seam_allowance_mm:10,garments:[]},curveTypes=new Set();
let manifest;
try{manifest=JSON.parse(await fs.readFile(path.join(input,'../sample-manifest.json'),'utf8'));}catch{}
const files=(await fs.readdir(input)).filter(f=>f.endsWith('.json')).sort();report.sample_count=files.length;
for(const file of files){
  try{
    const raw=await fs.readFile(path.join(input,file)),spec=JSON.parse(raw.toString('utf8'));
    if(!spec.pattern?.panels)throw new Error('Missing pattern.panels');
    const style=file.replace(/_specification(?:_shifted)?\.json$/,''),pieces=[];
    for(const [name,panel] of Object.entries(spec.pattern.panels)){
      const seam=panelOutline(panel,spec.properties,curveTypes),outline=allowance(seam);
      // Panels are already explicit, including left/right counterparts. No
      // symmetry constraint is encoded in this source, so do not infer pairs.
      const xs=seam.map(p=>p[0]),ys=seam.map(p=>p[1]),cx=(Math.min(...xs)+Math.max(...xs))/2,lo=Math.min(...ys),hi=Math.max(...ys);
      const grain=[[cx,lo+(hi-lo)*0.2],[cx,lo+(hi-lo)*0.8]].map(roundPt);
      pieces.push({id:`${style.toUpperCase()}-${name.toUpperCase()}-MTM`,name,style,size:'MTM',material:'fabric',outline,grain,quantity:1,mirror_quantity:0,rotation:'inherit',buffer:0,marks:[{kind:'line',points:seam},{kind:'text',points:[grain[0]],text:name}]});
    }
    const st=stats(pieces);if(st.area_m2<0.05||st.area_m2>20)throw new Error(`Implausible area ${st.area_m2} m2`);
    const entry=manifest?.samples?.find(s=>(s.file||path.posix.basename(s.path))===file);
    const meta={source:'GarmentCodeData',source_url:'https://doi.org/10.3929/ethz-b-000690432',mirror_url:manifest?.source_url||'https://huggingface.co/georgeNakayama/AIpparel',source_archive_path:entry?.archive_path||null,source_file:file,source_sha256:createHash('sha256').update(raw).digest('hex'),license:'CC-BY-4.0',attribution:'Korosteleva et al., GarmentCodeData: A Dataset of 3D Made-to-Measure Garments With Sewing Patterns, ECCV 2024',source_units_in_meter:spec.properties.units_in_meter,seam_allowance_mm:10,flatten_tolerance_mm:0.15,offset_disk_sagitta_mm:0.05,grain_assumption:'Panel local +y axis (up in its 3D placement); no textile grain annotation in dataset',pairing:'Each source panel once. Left/right panels remain explicit; no source symmetry flag.',changes:'Converted to mm; flattened curves; added uniform round 10 mm allowance. Original stitch boundary kept as line mark. No manufacturing notches or drills inferred.'};
    const value=job(`GarmentCodeData ${style}`,pieces,{MTM:1},meta);
    await writeJob(path.join(output,`${style}.job.json`),value);
    if(report.converted<10)await writeJob(path.join(examples,`${style}.job.json`),value);
    if(report.converted<3)await preview(path.join(examples,`${style}.svg`),`GarmentCodeData ${style}`,pieces);
    report.garments.push({id:style,...st});report.converted++;
  }catch(error){report.rejected.push({file,error:error.message});}
  if((report.converted+report.rejected.length)%25===0)console.log(`${report.converted} converted, ${report.rejected.length} rejected`);
}
report.curve_types=[...curveTypes].sort();report.source_archive_bytes=manifest?.archive_bytes;report.specification_bytes=manifest?.specification_bytes;
await fs.writeFile(path.join(output,'conversion-report.json'),JSON.stringify(report,null,2)+'\n');
await fs.writeFile(path.join(examples,'report.json'),JSON.stringify({...report,garments:report.garments.slice(0,10)},null,2)+'\n');
console.log(JSON.stringify({converted:report.converted,rejected:report.rejected.length,curve_types:report.curve_types}));
if(!report.converted)process.exitCode=1;
