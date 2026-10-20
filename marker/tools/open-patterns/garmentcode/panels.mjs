// SPDX-License-Identifier: Apache-2.0
import clipping from 'polygon-clipping';
import {area,bezier,clean,cross,dist,roundPt,validatePolygon} from '../geometry.mjs';

// GarmentCode relative control points use chord and its left perpendicular as
// their basis. Keep native y-up panel coordinates; do not use renderer y flips.
const rel=(a,b,p)=>[a[0]+p[0]*(b[0]-a[0])-p[1]*(b[1]-a[1]),a[1]+p[0]*(b[1]-a[1])+p[1]*(b[0]-a[0])];
export function arc(a,b,[radius,large,sweep],tolerance=0.15){
  if(!(radius>0)||dist(a,b)<1e-8)throw new Error('Invalid circle edge');
  const dx=(a[0]-b[0])/2,dy=(a[1]-b[1])/2,half2=dx*dx+dy*dy;
  if(radius*radius<half2-0.001)throw new Error('Circle radius smaller than half chord');
  radius=Math.max(radius,Math.sqrt(half2));
  const sign=Boolean(large)===Boolean(sweep)?-1:1,coef=sign*Math.sqrt(Math.max(0,(radius*radius-half2)/half2));
  const cx=(a[0]+b[0])/2+coef*dy,cy=(a[1]+b[1])/2-coef*dx;
  const start=Math.atan2(a[1]-cy,a[0]-cx);let delta=Math.atan2(b[1]-cy,b[0]-cx)-start;
  if(sweep&&delta<0)delta+=2*Math.PI;if(!sweep&&delta>0)delta-=2*Math.PI;
  const step=2*Math.acos(Math.max(-1,Math.min(1,1-tolerance/radius)));
  const n=Math.max(1,Math.ceil(Math.abs(delta)/step));
  return Array.from({length:n+1},(_,i)=>i===0?a:i===n?b:[cx+radius*Math.cos(start+delta*i/n),cy+radius*Math.sin(start+delta*i/n)]);
}
export function panelOutline(panel,properties,curveTypes){
  if(properties.curvature_coords!=='relative')throw new Error('Only relative curvature coordinates supported');
  const scale=1000/properties.units_in_meter;
  if(!(scale>0&&scale<10000))throw new Error('Missing/invalid units_in_meter');
  const vertices=panel.vertices.map(p=>p.map(x=>x*scale));let ring=[],first,previous;
  for(const edge of panel.edges){
    const [i,j]=edge.endpoints,a=vertices[i],b=vertices[j];if(!a||!b)throw new Error('Invalid endpoint index');
    if(previous!==undefined&&i!==previous)throw new Error('Panel edges are not an ordered closed loop');first??=i;previous=j;
    let points=[a,b];const c=edge.curvature;
    if(c){
      const type=Array.isArray(c)?'quadratic':c.type,params=Array.isArray(c)?[c]:c.params;curveTypes.add(type);
      if(type==='quadratic'||type==='cubic'){
        if(params.length!==(type==='quadratic'?1:2))throw new Error(`Invalid ${type} controls`);
        points=bezier([a,...params.map(p=>rel(a,b,p)),b]);
      }else if(type==='circle')points=arc(a,b,[params[0]*scale,params[1],params[2]]);
      else throw new Error(`Unsupported curvature ${type}`);
    }
    ring.push(...points.slice(0,-1));
  }
  if(previous!==first)throw new Error('Panel does not close');
  ring=clean(ring);if(area(ring)<0)ring.reverse();validatePolygon(ring,'source panel');return ring;
}
export function allowance(p,distance=10){
  // Minkowski sum with a polygonal disk: union the filled polygon, edge
  // rectangles and round convex vertex caps. Disk sagitta is <= 0.05 mm.
  const polys=[[[...p,p[0]]]],n=Math.ceil(Math.PI/Math.acos(1-0.05/distance));
  for(let i=0;i<p.length;i++){
    const a=p[i],b=p[(i+1)%p.length],len=dist(a,b),normal=[(b[1]-a[1])*distance/len,-(b[0]-a[0])*distance/len];
    const aa=[a[0]+normal[0],a[1]+normal[1]],bb=[b[0]+normal[0],b[1]+normal[1]];
    polys.push([[a,b,bb,aa,a]]);
    if(cross(p[(i+p.length-1)%p.length],a,b)>0){const disk=Array.from({length:n},(_,j)=>[a[0]+distance*Math.cos(j*2*Math.PI/n),a[1]+distance*Math.sin(j*2*Math.PI/n)]);polys.push([[...disk,disk[0]]]);}
  }
  // Snap all constructed vertices to the same 0.0001 mm lattice before the
  // sweep. Trigonometric near-zero values otherwise make coincident cap/edge
  // boundaries numerically inconsistent.
  const merged=clipping.union(...polys.map(poly=>poly.map(ring=>ring.map(roundPt))));
  if(merged.length!==1||merged[0].length!==1)throw new Error('Offset has multiple components or holes');
  const ring=clean(merged[0][0]);if(area(ring)<0)ring.reverse();validatePolygon(ring,'offset panel');
  if(area(ring)<area(p))throw new Error('Offset reduced area');return ring;
}
