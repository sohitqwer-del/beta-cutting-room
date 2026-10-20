// Independent numerical checks for the one-time converters.
// SPDX-License-Identifier: Apache-2.0
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {area,bezier,dist,validatePolygon,outerBoundary} from './geometry.mjs';
import {arc,panelOutline,allowance} from './garmentcode/panels.mjs';

function distanceToPolyline(p,line){
  return Math.min(...line.slice(1).map((b,i)=>{const a=line[i],den=dist(a,b)**2,t=Math.max(0,Math.min(1,((p[0]-a[0])*(b[0]-a[0])+(p[1]-a[1])*(b[1]-a[1]))/den));return dist(p,[a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])]);}));
}
test('Bezier flattening bounds an inflection and a closed-end loop',()=>{
  for(const nodes of [[[0,0],[60,150],[-20,-100],[100,0]],[[0,0],[100,100],[-100,100],[0,0]]]){
    const line=bezier(nodes);
    for(let i=0;i<=2000;i++){const t=i/2000,s=1-t,p=[0,1].map(k=>s**3*nodes[0][k]+3*s*s*t*nodes[1][k]+3*s*t*t*nodes[2][k]+t**3*nodes[3][k]);assert.ok(distanceToPolyline(p,line)<=0.151);}
  }
});
test('GarmentCode relative control point uses the left perpendicular',()=>{
  const panel={vertices:[[0,0],[10,0],[10,-10],[0,-10]],edges:[{endpoints:[0,1],curvature:{type:'quadratic',params:[[0.5,0.2]]}},{endpoints:[1,2]},{endpoints:[2,3]},{endpoints:[3,0]}]};
  const ring=panelOutline(panel,{units_in_meter:100,curvature_coords:'relative'},new Set());
  assert.ok(Math.max(...ring.map(p=>p[1]))>9.9);assert.equal(Math.min(...ring.map(p=>p[1])),-100);
});
test('Circle flags select all four major/minor and sweep combinations',()=>{
  const radius=80,a=[0,0],b=[100,0];
  for(const large of [0,1])for(const sweep of [0,1]){
    const line=arc(a,b,[radius,large,sweep]);const length=line.slice(1).reduce((s,p,i)=>s+dist(p,line[i]),0);
    assert.equal(length>Math.PI*radius,Boolean(large));
    if(!large)assert.equal(Math.sign(line[Math.floor(line.length/2)][1]),sweep?-1:1);
    for(let i=1;i<line.length;i++){const chord=dist(line[i-1],line[i]);assert.ok(radius-Math.sqrt(radius*radius-chord*chord/4)<=0.151);}
  }
});
test('10 mm round allowance has the independently known rectangle area',()=>{
  const p=[[0,0],[100,0],[100,60],[0,60]],out=allowance(p);
  // Exact round dilation area: A + perimeter*r + pi*r*r. Disk approximation
  // is inscribed, so permit its known sub-3 mm2 rounding error.
  const expected=6000+320*10+Math.PI*100;
  assert.ok(Math.abs(area(out)-expected)<3);validatePolygon(out);
  assert.equal(Math.min(...out.map(p=>p[0])),-10);assert.equal(Math.max(...out.map(p=>p[1])),70);
});
test('Self crossings, touching edges and repeated closure are rejected',()=>{
  assert.throws(()=>validatePolygon([[0,0],[10,0],[0,10],[10,10]]));
  assert.throws(()=>validatePolygon([[0,0],[10,0],[10,10],[5,0],[0,10]]));
  assert.throws(()=>validatePolygon([[0,0],[10,0],[0,10],[0,0]]));
});
test('Composite cut-line graph keeps an open-fold boundary and variable hem',()=>{
  const seam=[[0,0],[100,0],[100,60],[0,60],[0,0]];
  const out=outerBoundary([seam,[[0,-10],[110,-10],[110,80],[0,80]],[[0,0],[0,-10]],[[0,60],[0,80]]]);
  validatePolygon(out);assert.equal(area(out),9900);
});
