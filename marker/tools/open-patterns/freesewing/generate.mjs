// SPDX-License-Identifier: Apache-2.0
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {Simon} from '@freesewing/simon';
import {Teagan} from '@freesewing/teagan';
import {Titan} from '@freesewing/titan';
import {Hugo} from '@freesewing/hugo';
import {Bella} from '@freesewing/bella';
import * as models from '@freesewing/models';
import clipping from 'polygon-clipping';
import {area,bezier,clean,dist,job,outerBoundary,preview,roundPt,stats,validatePolygon,writeJob} from '../geometry.mjs';

const here=path.dirname(fileURLToPath(import.meta.url));
const output=path.resolve(process.argv[2]||path.join(here,'../../../testdata/open/freesewing'));
await fs.mkdir(output,{recursive:true});
const ratio={S:1,M:2,L:2,XL:1};
const male={S:'cisMaleAdult36',M:'cisMaleAdult38',L:'cisMaleAdult40',XL:'cisMaleAdult42'};
const female={S:'cisFemaleAdult32',M:'cisFemaleAdult34',L:'cisFemaleAdult36',XL:'cisFemaleAdult38'};
const designs=[['simon',Simon,male],['teagan',Teagan,male],['titan',Titan,male],['hugo',Hugo,male],['bella',Bella,female]];
const pt=p=>[p.x,p.y];
function flatten(p){
  const result=[];let line=[],start;
  for(const op of p.ops){
    if(op.type==='move'){if(line.length>1)result.push(line);line=[pt(op.to)];start=pt(op.to);}
    else if(op.type==='line')line.push(pt(op.to));
    else if(op.type==='curve')line.push(...bezier([line.at(-1),pt(op.cp1),pt(op.cp2),pt(op.to)]).slice(1));
    else if(op.type==='close'){if(start&&dist(line.at(-1),start)>1e-8)line.push(start);}
    else throw new Error(`Unsupported FreeSewing op: ${op.type}`);
  }
  if(line.length>1)result.push(line);return result;
}
function cutOutline(part){
  const paths=Object.values(part.paths).filter(p=>!p.hidden&&/\bsa\b/.test(p.attributes.get('class')||''));
  const seam=part.paths.seam;if(!seam)return null;
  const sas=paths.flatMap(flatten);
  if(sas.length===1&&dist(sas[0][0],sas[0].at(-1))<0.001){let ring=clean(sas[0]);if(area(ring)<0)ring.reverse();return ring;}
  return outerBoundary([...flatten(seam).map(p=>dist(p[0],p.at(-1))<0.001?p:[...p,p[0]]),...sas]);
}
function reflect(p,[a,b]) {const u=[b[0]-a[0],b[1]-a[1]],t=((p[0]-a[0])*u[0]+(p[1]-a[1])*u[1])/(u[0]**2+u[1]**2);return [2*(a[0]+t*u[0])-p[0],2*(a[1]+t*u[1])-p[1]];}
function unfold(outline,fold){
  const mirror=outline.map(p=>roundPt(reflect(p,fold)));
  const union=clipping.union([[...outline,outline[0]]],[[...mirror,mirror[0]]]);
  if(union.length!==1||union[0].length!==1)throw new Error('Unfold produced disconnected pieces or holes');
  let ring=clean(union[0][0]);if(area(ring)<0)ring.reverse();return ring;
}
function getGrain(part,cut,outline,assumptions) {
  const path=Object.entries(part.paths).find(([k])=>k.startsWith('__macro_grainline_'))?.[1];
  if(path){const line=flatten(path)[0];if(dist(line[0],line.at(-1))>1)return [line[0],line.at(-1)];}
  if(cut.grainOrigin==='cutonfold'&&cut.cutOnFold)return cut.cutOnFold.map(pt);
  if(Number.isFinite(cut.grain)){const c=outline[0],r=cut.grain*Math.PI/180;return [c,[c[0]+100*Math.cos(r),c[1]-100*Math.sin(r)]];}
  // Simon does not annotate the grain of the cuff or sleeve plackets.
  // Their long axis is x in the draft. Record this explicit assumption.
  if(/^simon\.(cuff|sleevePlacketOverlap|sleevePlacketUnderlap)$/.test(part.name)){assumptions.push(`${part.name}: no grain macro; use drafted x axis (length of cuff/placket).`);return [[0,0],[100,0]];}
  throw new Error(`No grain for ${part.name}`);
}
const report={source_version:'4.10.2',flatten_tolerance_mm:0.15,styles:{}};
for(const [style,Design,sizes] of designs){
  const byMaterial={},assumptions=new Set();
  for(const [size,model] of Object.entries(sizes)){
    const pattern=new Design({measurements:models[model],sa:10}).draft();
    const errors=[...pattern.store.logs.error,...pattern.setStores[0].logs.error];
    if(errors.length)throw new Error(`FreeSewing draft failed (${style}/${size}): ${errors.join('; ')}`);
    const cuts=pattern.setStores[0].get('cutlist');
    for(const [name,part] of Object.entries(pattern.parts[0])){
      const cut=cuts[name];if(part.hidden||!cut?.materials)continue;
      let outline=cutOutline(part);if(!outline)continue;
      const localAssumptions=[];const grain=getGrain(part,cut,outline,localAssumptions);
      localAssumptions.forEach(x=>assumptions.add(x));
      const marks=Object.values(part.snippets).flatMap(s=>s.def.includes('notch')?[{kind:'notch',points:[pt(s.anchor)],text:s.def}]:/^(button|buttonhole)$/.test(s.def)?[{kind:'drill',points:[pt(s.anchor)],text:s.def}]:[]);
      const partName=name.split('.').at(-1);
      if(name==='simon.collar')assumptions.add('simon.collar: collar and undercollar use the shared full cut envelope; the optional smaller undercollar trim is not applied.');
      // Two inconsistent v4.10.2 cutlists are resolved against the official
      // design cutting instructions, with the correction kept in job metadata.
      if(name==='hugo.pocket'){
        cut.materials.fabric=[{cut:1,identical:true,onFold:true,onBias:false}];
        assumptions.add('hugo.pocket: v4.10.2 says cut 2 but draws a half pocket with a fold macro; official Hugo cutting instructions say cut 1 on fold. Use one unfolded pocket.');
      }
      if(name==='bella.back'){
        cut.materials.fabric=[{cut:2,identical:false,onFold:false,onBias:false}];
        assumptions.add('bella.back: v4.10.2 onFold=true has no fold macro and a curved centre back; official Bella instructions say cut 2 back parts. Use a mirrored pair.');
      }
      for(const [material,instructions] of Object.entries(cut.materials)){
        let quantity=0,mirror_quantity=0;const folded=instructions.some(i=>i.onFold)&&!!cut.cutOnFold;
        if(folded&&instructions.some(i=>!i.onFold))throw new Error(`Mixed fold instructions: ${name}`);
        for(const i of instructions){if(i.onBias)throw new Error(`Unmapped bias cut ${name}`);if(i.identical||folded){quantity+=i.cut;}else{quantity+=Math.ceil(i.cut/2);mirror_quantity+=Math.floor(i.cut/2);}}
        if(!quantity&&!mirror_quantity)continue;
        // FreeSewing's offset can leave microscopic loops near tangent joins.
        // Resolve its nonzero fill before validating the cut polygon.
        const resolved=clipping.union([[...outline,outline[0]]]);
        if(resolved.length!==1||resolved[0].length!==1)throw new Error(`Cut path has holes/disconnected regions: ${name}`);
        const resolvedOutline=clean(resolved[0][0]);
        if(Math.abs(Math.abs(area(resolvedOutline))-Math.abs(area(outline)))>0.01)assumptions.add(`${name}: resolved self-crossing seam-allowance offset with nonzero polygon union.`);
        let polygon=folded?unfold(resolvedOutline,cut.cutOnFold.map(pt)):resolvedOutline;
        let pieceMarks=marks;
        if(folded)pieceMarks=[...marks,...marks.map(m=>({...m,points:m.points.map(p=>reflect(p,cut.cutOnFold.map(pt)))}))];
        // FreeSewing uses SVG y-down coordinates. Reflect all geometry together
        // into the contract's y-up coordinates (including drill/notch anchors).
        const yUp=p=>roundPt([p[0],-p[1]]);
        polygon=clean(polygon.map(yUp));if(area(polygon)<0)polygon.reverse();validatePolygon(polygon,`${style}-${partName}-${size}`);
        const piece={id:`${style.toUpperCase()}-${partName.toUpperCase()}-${size}`,name:partName,style,size,material,outline:polygon,grain:grain.map(yUp),quantity,mirror_quantity,rotation:'inherit',buffer:0,marks:[...pieceMarks.map(m=>({...m,points:m.points.map(yUp)})),{kind:'text',points:[yUp(grain[0])],text:`${style} ${partName} ${size}`} ]};
        (byMaterial[material]??=[]).push(piece);
      }
    }
  }
  const materials={};
  for(const [material,pieces] of Object.entries(byMaterial)){
    const basename=style+(material==='fabric'?'':`-${material}`),meta={source:'FreeSewing',source_url:`https://freesewing.eu/docs/designs/${style}/`,version:'4.10.2',license:'MIT',copyright:'Joost De Cock and FreeSewing contributors',measurement_sets:sizes,seam_allowance_mm:10,cut_line:'FreeSewing seam allowance paths; design-specific hems and flat-felled allowances retained',flatten_tolerance_mm:0.15,unfolded:true,assumptions:[...assumptions]};
    await writeJob(path.join(output,`${basename}-ratio.job.json`),job(`${basename} S1 M2 L2 XL1`,pieces,ratio,meta));
    const medium=pieces.filter(p=>p.size==='M');await writeJob(path.join(output,`${basename}-M.job.json`),job(`${basename} M1`,medium,{M:1},meta));
    materials[material]=Object.fromEntries(Object.keys(sizes).map(s=>[s,stats(pieces.filter(p=>p.size===s))]));
    for(const [s,st] of Object.entries(materials[material]))if(material==='fabric'&&(st.area_m2<0.2||st.area_m2>5))throw new Error(`Implausible garment area ${style} ${s}: ${st.area_m2}`);
  }
  await preview(path.join(output,`${style}.svg`),`FreeSewing ${style} (all materials)`,Object.values(byMaterial).flat().filter(p=>p.size==='M'));
  report.styles[style]={measurement_sets:sizes,materials,assumptions:[...assumptions]};console.log(style,JSON.stringify(materials));
}
await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n');
