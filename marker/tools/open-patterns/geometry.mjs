// One-time open-data conversion helpers. SPDX-License-Identifier: Apache-2.0
import fs from 'node:fs/promises';

export const dist = (a,b) => Math.hypot(a[0]-b[0],a[1]-b[1]);
export const cross = (a,b,c) => (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]);
export const area = p => p.reduce((s,a,i)=>{const b=p[(i+1)%p.length];return s+a[0]*b[1]-b[0]*a[1];},0)/2;
export const roundPt = p => p.map(x=>Math.round(x*10000)/10000);
export function clean(p) {
  const out=[];
  for(const a of p.map(roundPt)) if(!out.length || dist(a,out.at(-1))>0.0001)out.push(a);
  if(out.length>1 && dist(out[0],out.at(-1))<0.0001)out.pop();
  return out;
}
function segmentDistance(p,a,b) {
  const d=dist(a,b)**2;
  const t=d?Math.max(0,Math.min(1,((p[0]-a[0])*(b[0]-a[0])+(p[1]-a[1])*(b[1]-a[1]))/d)):0;
  return dist(p,[a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])]);
}
// The Bezier convex hull bounds the curve. All control points must be within
// tolerance of the finite chord, so loops and coincident endpoints are safe.
export function bezier(nodes,tolerance=0.15,depth=0) {
  if(nodes.slice(1,-1).every(p=>segmentDistance(p,nodes[0],nodes.at(-1))<=tolerance))return [nodes[0],nodes.at(-1)];
  if(depth>24)throw new Error('Bezier subdivision did not converge');
  const left=[nodes[0]],right=[nodes.at(-1)];let row=nodes;
  while(row.length>1){row=row.slice(1).map((p,i)=>[(p[0]+row[i][0])/2,(p[1]+row[i][1])/2]);left.push(row[0]);right.unshift(row.at(-1));}
  return [...bezier(left,tolerance,depth+1).slice(0,-1),...bezier(right,tolerance,depth+1)];
}
function intersects(a,b,c,d) {
  if(Math.max(a[0],b[0])+1e-8<Math.min(c[0],d[0])||Math.max(c[0],d[0])+1e-8<Math.min(a[0],b[0])||Math.max(a[1],b[1])+1e-8<Math.min(c[1],d[1])||Math.max(c[1],d[1])+1e-8<Math.min(a[1],b[1]))return false;
  const x=cross(a,b,c),y=cross(a,b,d),z=cross(c,d,a),w=cross(c,d,b);
  return x*y<=1e-12&&z*w<=1e-12;
}
export function validatePolygon(p,label='polygon') {
  if(p.length<3||p.some(a=>a.length!==2||a.some(x=>!Number.isFinite(x))))throw new Error(`${label}: invalid vertices`);
  if(dist(p[0],p.at(-1))<1e-8)throw new Error(`${label}: repeated closing vertex`);
  if(area(p)<=1e-4)throw new Error(`${label}: non-positive area`);
  for(let i=0;i<p.length;i++)for(let j=i+1;j<p.length;j++){
    if(j===i+1||(i===0&&j===p.length-1))continue;
    if(intersects(p[i],p[(i+1)%p.length],p[j],p[(j+1)%p.length]))throw new Error(`${label}: self intersection at ${i},${j}`);
  }
}
export function validateJob(job) {
  if(job.format!=='beta-marker.job/v1'||job.fabric.width!==1500||!job.pieces.length)throw new Error('Invalid job');
  const ids=new Set();
  for(const p of job.pieces){
    if(ids.has(p.id))throw new Error(`Duplicate id ${p.id}`);ids.add(p.id);
    validatePolygon(p.outline,p.id);
    if(!p.grain||p.grain.length!==2||p.grain.flat().some(x=>!Number.isFinite(x))||dist(...p.grain)<1)throw new Error(`${p.id}: missing grain`);
    if(!Number.isInteger(p.quantity)||!Number.isInteger(p.mirror_quantity)||p.quantity<0||p.mirror_quantity<0||p.quantity+p.mirror_quantity<1)throw new Error(`${p.id}: bad cut count`);
    if(!(p.size in job.order.sizes))throw new Error(`${p.id}: missing size demand`);
    if(p.marks.some(m=>!['notch','drill','line','text'].includes(m.kind)||m.points.flat().some(x=>!Number.isFinite(x))))throw new Error(`${p.id}: invalid marks`);
  }
}
export function job(name,pieces,sizes,meta={}) {
  return {format:'beta-marker.job/v1',name,fabric:{width:1500,max_length:null,nap:'none',allow_flip:false,tilt_deg:0},spacing:{piece_gap:0,edge_gap:0},pieces,order:{sizes},meta:{units:'mm',...meta}};
}
export async function writeJob(file,value) {
  validateJob(value);const data=JSON.stringify(value)+'\n';
  if(Buffer.byteLength(data)>=1_000_000)throw new Error(`Job exceeds 1 MB: ${file}`);
  await fs.writeFile(file,data);
}
export function stats(pieces) {
  return {piece_types:pieces.length,cut_pieces:pieces.reduce((s,p)=>s+p.quantity+p.mirror_quantity,0),area_m2:Number((pieces.reduce((s,p)=>s+area(p.outline)*(p.quantity+p.mirror_quantity),0)/1e6).toFixed(6))};
}
// Trace the unbounded face of seam + cut-line linework. Split intersections,
// including endpoints on edges, before traversing the planar half-edge graph.
export function outerBoundary(polylines) {
  const segments=polylines.flatMap(p=>p.slice(1).map((b,i)=>[p[i],b])).filter(([a,b])=>dist(a,b)>1e-6);
  const splits=segments.map(()=>[0,1]);
  const dot=(a,b)=>a[0]*b[0]+a[1]*b[1];
  const sub=(a,b)=>[a[0]-b[0],a[1]-b[1]];
  for(let i=0;i<segments.length;i++)for(let j=i+1;j<segments.length;j++){
    const [a,b]=segments[i],[c,d]=segments[j],u=sub(b,a),v=sub(d,c),w=sub(c,a),den=u[0]*v[1]-u[1]*v[0];
    if(Math.abs(den)>1e-9){const t=(w[0]*v[1]-w[1]*v[0])/den,s=(w[0]*u[1]-w[1]*u[0])/den;if(t>=-1e-7&&t<=1+1e-7&&s>=-1e-7&&s<=1+1e-7){splits[i].push(Math.max(0,Math.min(1,t)));splits[j].push(Math.max(0,Math.min(1,s)));}}
    else if(Math.abs(cross(a,b,c))<1e-5){for(const p of [c,d]){const t=dot(sub(p,a),u)/dot(u,u);if(t>0&&t<1)splits[i].push(t);}for(const p of [a,b]){const t=dot(sub(p,c),v)/dot(v,v);if(t>0&&t<1)splits[j].push(t);}}
  }
  const nodes=[],adj=[],byKey=new Map();
  function node(p){const key=p.map(x=>Math.round(x*10000)).join(',');if(byKey.has(key))return byKey.get(key);const n=nodes.length;byKey.set(key,n);nodes.push(roundPt(p));adj.push(new Set());return n;}
  segments.forEach(([a,b],i)=>{const ts=[...new Set(splits[i])].sort((a,b)=>a-b);for(let j=1;j<ts.length;j++){const at=t=>[a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])],u=node(at(ts[j-1])),v=node(at(ts[j]));if(u!==v){adj[u].add(v);adj[v].add(u);}}});
  const sorted=adj.map((list,i)=>[...list].sort((a,b)=>Math.atan2(nodes[a][1]-nodes[i][1],nodes[a][0]-nodes[i][0])-Math.atan2(nodes[b][1]-nodes[i][1],nodes[b][0]-nodes[i][0])));
  const seen=new Set(),rings=[];
  for(let a=0;a<nodes.length;a++)for(const b of sorted[a]){if(seen.has(`${a},${b}`))continue;let u=a,v=b;const ring=[];for(let n=0;n<segments.length*6;n++) {const key=`${u},${v}`;if(seen.has(key))break;seen.add(key);ring.push(nodes[u]);const next=sorted[v][(sorted[v].indexOf(u)-1+sorted[v].length)%sorted[v].length];u=v;v=next;}if(ring.length>=3)rings.push(ring);}
  if(!rings.length)throw new Error('No closed face in cut-line graph');
  let ring=rings.sort((a,b)=>Math.abs(area(b))-Math.abs(area(a)))[0];
  // Remove zero-area spurs, which are internal lines touching the perimeter.
  let changed=true;while(changed){changed=false;for(let i=0;i<ring.length;i++)if(dist(ring[(i+ring.length-1)%ring.length],ring[(i+1)%ring.length])<0.001){ring.splice(i,1);ring=clean(ring);changed=true;break;}}
  ring=clean(ring);if(area(ring)<0)ring.reverse();return ring;
}
export async function preview(file,title,pieces) {
  let x=25,y=70,row=0;const shapes=[];
  const esc=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('"','&quot;');
  for(const p of pieces){const xs=p.outline.map(a=>a[0]),ys=p.outline.map(a=>a[1]);const minX=Math.min(...xs),minY=Math.min(...ys),w=Math.max(...xs)-minX,h=Math.max(...ys)-minY;
    if(x+w>1800){x=25;y+=row+70;row=0;}
    const maxY=minY+h;
    const points=p.outline.map(a=>roundPt([a[0]-minX+x,maxY-a[1]+y]));
    const g=p.grain.map(a=>roundPt([a[0]-minX+x,maxY-a[1]+y]));
    shapes.push(`<g><title>${esc(p.id)}</title><polygon points="${points.map(a=>a.join(',')).join(' ')}" fill="#e4eef8" stroke="#243e54" stroke-width="1.5"/><path d="M${g[0]} L${g[1]}" stroke="#a52a36" stroke-width="2" marker-end="url(#arrow)"/><text x="${x}" y="${y-12}" font-size="16">${esc(p.name)} / ${esc(p.material)} (${p.quantity}+${p.mirror_quantity} mirrored)</text></g>`);x+=w+45;row=Math.max(row,h);
  }
  await fs.writeFile(file,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1850 ${y+row+40}" role="img" aria-label="${esc(title)}"><title>${esc(title)}</title><defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10z" fill="#a52a36"/></marker></defs><rect width="100%" height="100%" fill="white"/><g font-family="sans-serif"><text x="25" y="32" font-size="25">${esc(title)} — millimetres, separate pieces</text>${shapes.join('')}</g></svg>\n`);
}
