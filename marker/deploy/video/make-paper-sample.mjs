// Original 30%-scale teaching set, generated from the real Rust T-shirt block.
// SVG is presentation only; Rust WASM supplies every cut outline and grain line.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [pkgArg,outArg,pwArg]=process.argv.slice(2),pkg=resolve(pkgArg),out=resolve(outArg);
mkdirSync(out,{recursive:true});
const raw=await import(pathToFileURL(join(pkg,'beta_pattern_wasm.js')).href);
raw.initSync({module:readFileSync(join(pkg,'beta_pattern_wasm_bg.wasm'))});
const call=r=>JSON.parse(raw.style_command(JSON.stringify(r)));
const chart=call({action:'size_chart',preset:'indian_adult'});chart.sizes=['M'];chart.base_size='M';chart.measurements={M:chart.measurements.M};
const doc=call({action:'create',request:{name:'Synthetic T-shirt teaching set',block:'t_shirt',chart}});
const job=call({action:'to_job',document:doc,settings:{fabric:{width:900,layout:'open',nap:'none'},spacing:{piece_gap:2,edge_gap:5},ratio:{M:1}}});
writeFileSync(join(out,'sample-block.job.json'),JSON.stringify(job,null,2));
console.log(job.pieces.map(p=>({name:p.name,n:p.quantity,outline:p.outline.length,grain:p.grain})));
const specs=[['front',25,30],['back',440,30],['sleeve',25,320],['sleeve',440,320],['neck',230,420]];
let svg='<svg xmlns="http://www.w3.org/2000/svg" width="2400" height="1920" viewBox="0 0 640 512"><defs><linearGradient id="table" x2="1" y2="1"><stop stop-color="#294142"/><stop offset="1" stop-color="#24393b"/></linearGradient></defs><rect width="640" height="512" fill="url(#table)"/>';
svg+='<rect x="205" y="80" width="210" height="297" fill="#f4f2eb"/>';
const truth=[];
for(const [name,x,y]of specs){const p=job.pieces.find(p=>p.name.toLowerCase().includes(name));if(!p)throw Error('Missing '+name);const xs=p.outline.map(p=>p[0]),ys=p.outline.map(p=>p[1]),minX=Math.min(...xs),maxY=Math.max(...ys);const s=.30;
 const points=p.outline.map(([px,py])=>[x+(px-minX)*s,y+(maxY-py)*s]);
 const path='M'+points.map(q=>q.join(',')).join('L')+'Z';
 let grain=p.grain.map(([px,py])=>[x+(px-minX)*s,y+(maxY-py)*s]);
 if(name==='neck'){grain=[[x+15,y+8],[x+85,y+8]];} const [a,b]=grain;
 svg+=`<path d="${path}" fill="#ded5bf"/><path d="M${a}L${b}" stroke="#263b43" stroke-width=".8"/>`;
 // Small open arrowhead stays slender enough for the scanner's line detector.
 svg+=`<path d="M${b[0]-1.4},${b[1]-3}L${b}L${b[0]+1.4},${b[1]-3}" fill="none" stroke="#263b43" stroke-width=".6"/>`;
 truth.push({name,points,grain});
}
svg+='</svg>';writeFileSync(join(out,'synthetic-tshirt.svg'),svg);writeFileSync(join(out,'sample-truth.json'),JSON.stringify({scale:.30,reference:[205,80,210,297],pieces:truth},null,2));
const pw=await import(pathToFileURL(join(resolve(pwArg),'index.mjs')).href),browser=await pw.chromium.launch({headless:true,executablePath:process.env.CHROME_PATH??'C:/Program Files/Google/Chrome/Application/chrome.exe'});
try{const p=await browser.newPage({viewport:{width:2400,height:1920}});await p.setContent(svg);await p.evaluate(()=>{document.body.style.margin='0';document.querySelector('svg').style.transform='perspective(12000px) rotateX(.15deg) rotateY(-.15deg)';});await p.screenshot({path:join(out,'synthetic-tshirt.png')});}finally{await browser.close();}
