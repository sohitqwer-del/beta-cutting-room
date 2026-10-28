// Local-only 0.8 launch editor. Media and intermediate files must live outside git.
import { spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { readFileSync, writeFileSync, mkdirSync, statSync, readdirSync, existsSync } from 'node:fs';
import { resolve, join, dirname, extname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { cleanVoice, duration, normalize, loudness } from './audio.mjs';
import { social, walkthrough, link } from './launch-scenes.mjs';
const here = dirname(fileURLToPath(import.meta.url));
const args = Object.fromEntries(process.argv.slice(3).flatMap((v,i,a)=>v.startsWith('--')?[[v.slice(2),a[i+1]]]:[]));
const out = resolve(args.out ?? 'launch8-video'), work=join(out,'work8');
mkdirSync(work,{recursive:true});
const stage=process.argv[2]??'all', lines=[...social,...walkthrough];
const run=(cmd,a)=>{const r=spawnSync(cmd,a,{stdio:'inherit',windowsHide:true});if(r.status!==0)throw Error(`${cmd} failed (${r.status})`);};
const ff=a=>run('ffmpeg',['-hide_banner','-loglevel','error','-y','-threads','3','-filter_threads','2','-filter_complex_threads','2',...a]);
const enc=['-c:v','libx264','-preset','fast','-crf','24','-pix_fmt','yuv420p','-r','30','-threads','3'];
const esc=p=>p.replaceAll('\\','/').replaceAll(':','\\:').replaceAll("'","\\'");
const wait=ms=>new Promise(r=>setTimeout(r,ms));
const json=(name,v)=>writeFileSync(join(work,name),JSON.stringify(v,null,2));
async function tts(){
  for(const [name,seq,wpm] of [['social',social,145],['walk',walkthrough,165]]){
    const dir=join(work,'voice',name);mkdirSync(dir,{recursive:true});
    const f=join(work,`${name}-lines.json`);writeFileSync(f,JSON.stringify(seq));
    run(args.python??process.env.NARRATION_PYTHON??'python',[join(here,'neural-tts.py'),'--models',resolve(args.models??process.env.NARRATION_MODELS),'--lines',f,'--out',dir,'--voice','af_heart','--wpm',String(wpm)]);
    for(const b of seq)cleanVoice(join(dir,`${b.id}.wav`),join(work,`${b.id}.wav`));
  }
}
async function record(){
  const root=resolve(args.build);
  if(!existsSync(join(root,'index.html')))throw Error('Build the worktree static app first.');
  const mime={'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.css':'text/css','.svg':'image/svg+xml','.png':'image/png','.json':'application/json'};
  const server=createServer((req,res)=>{try{const p=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname));if(!p.startsWith(root+ '/'.replace('/',process.platform==='win32'?'\\':'/'))&&p!==root)throw Error('path');const f=statSync(p).isDirectory()?join(p,'index.html'):p;res.setHeader('Content-Type',mime[extname(f)]??'application/octet-stream');res.end(readFileSync(f));}catch{res.statusCode=404;res.end('Not found');}});
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  const url=`http://127.0.0.1:${server.address().port}/`;
  const pw=await import(pathToFileURL(join(resolve(args.playwright??process.env.PLAYWRIGHT_CORE),'index.mjs')).href);
  let browser;const report={sourceCommit:spawnSync('git',['rev-parse','HEAD'],{cwd:here,encoding:'utf8',windowsHide:true}).stdout.trim(),staticBuildSha256:createHash('sha256').update(readFileSync(join(root,'index.html'))).digest('hex'),errors:[],external:[],clips:{},downloads:[]};
  try{
    browser=await pw.chromium.launch({headless:true,executablePath:args.chrome??process.env.CHROME_PATH??'C:/Program Files/Google/Chrome/Application/chrome.exe',args:['--hide-scrollbars']});
    const ctx=await browser.newContext({viewport:{width:1280,height:720},acceptDownloads:true,locale:'en-US',colorScheme:'light'});
    await ctx.route('**/*',route=>{const u=route.request().url();if(/^https?:/.test(u)&&!u.startsWith(url)){report.external.push(u);return route.abort();}return route.continue();});
    const p=await ctx.newPage();p.on('pageerror',e=>report.errors.push(e.message));
    const click=async s=>{await p.locator(s).first().click({timeout:30000});await wait(350);};
    const ready=async s=>p.locator(s).first().waitFor({state:'visible',timeout:90000});
    const capture=async(name,action,minimum=4,force=false)=>{
      if(!force&&args.reuse==='1'&&!['photo','review','tape','paperMarker'].includes(name)&&existsSync(join(work,`${name}.mp4`))){console.log('Reuse',name);await action();report.clips[name]={duration:duration(join(work,`${name}.mp4`)),reused:true};return;}
      console.log('Capture',name);const dir=join(work,'frames',name);mkdirSync(dir,{recursive:true});
      let done=false,frames=[],start=performance.now();
      const loop=(async()=>{while(!done){const t=(performance.now()-start)/1000;const f=join(dir,`${String(frames.length).padStart(5,'0')}.jpg`);await p.screenshot({path:f,type:'jpeg',quality:90});frames.push({f,t});await wait(90);}})();
      try{await action();await wait(Math.max(0,minimum*1000-(performance.now()-start)));}finally{done=true;await loop;}
      const length=(performance.now()-start)/1000;
      const list=join(work,`${name}.ffconcat`);writeFileSync(list,'ffconcat version 1.0\n'+frames.map((f,i)=>`file '${f.f.replaceAll('\\','/')}'\nduration ${((frames[i+1]?.t??length)-f.t).toFixed(5)}`).join('\n')+`\nfile '${frames.at(-1).f.replaceAll('\\','/')}\u0027\n`);
      ff(['-safe','0','-f','concat','-i',list,'-t',String(length),...enc,join(work,`${name}.mp4`)]);
      report.clips[name]={duration:length,frames:frames.length};
    };
    await p.goto(url);await ready('#home-title');await capture('home',async()=>{},5);
    await p.goto(url+'#/quick');await ready('#q-gallery-btn');
    await click('#q-sample');await ready('.q-photo-card');await p.waitForFunction(()=>!document.querySelector('.q-reading'),null,{timeout:90000});
    await click('#q-to-review');await p.locator('.q-review').scrollIntoViewIfNeeded();await capture('review',async()=>{await ready('.q-piece');},8);
    await click('.q-piece');await ready('#q-sheet');
    await capture('tape',async()=>{await p.locator('.q-tape').scrollIntoViewIfNeeded();await wait(1500);
    // Known distance on this original synthetic test image; never infer a real tape measurement from a scan.
    const indices=await p.locator('#q-tape-svg path').first().evaluate(el=>{
      const nums=el.getAttribute('d').match(/-?\d+(?:\.\d+)?/g).map(Number),pts=[];
      for(let i=0;i<nums.length;i+=2)pts.push({x:nums[i],y:nums[i+1],index:i/2+1});
      const left=Math.min(...pts.map(p=>p.x)),edge=pts.filter(p=>p.x<left+.3).sort((a,b)=>a.y-b.y);
      return [edge[0].index,edge.at(-1).index];
    });
    await p.locator('#q-tape-a').fill(String(indices[0]));await p.locator('#q-tape-a').dispatchEvent('change');
    await p.locator('#q-tape-b').fill(String(indices[1]));await p.locator('#q-tape-b').dispatchEvent('change');
    await p.locator('#q-tape-mm').fill(String(Number(args['tape-mm']??143.8546)/10));
    await click('[data-q="check-tape"]');console.log('Tape',await p.locator('.q-tape').innerText());await ready('.q-tape-result.ok');await wait(2000);
    },10);
    await click('#q-sheet-close');await click('[data-q="goto"][data-step="capture"]');
    await capture('photo',async()=>{await p.locator('.q-photo-card').scrollIntoViewIfNeeded();},7);
    report.photoStatus=await p.locator('.q-photo-card').innerText();
    if(await p.locator('.q-photo-card .q-warnings').count())throw Error('Sample has quality warnings');
    await click('#q-to-review');await click('#q-accept-all');await click('#q-to-order');
    await click('#q-budget-20');await click('#q-make');await p.waitForFunction(()=>document.querySelector('#screen-11')?.dataset.step==='use',null,{timeout:90000});
    await p.locator('.q-result').evaluate(el=>{el.style.scrollMarginTop='64px';el.scrollIntoView({block:'start'});});await wait(500);
    await capture('paperMarker',async()=>{},8);
    report.photoMarker=await p.locator('.q-result').innerText();
    report.photoEfficiency=Number(report.photoMarker.match(/([\d.]+)%/)?.[1]);
    if(!(report.photoEfficiency>=65&&report.photoEfficiency<=100))throw Error('Synthetic sample marker fabric use is unexpectedly low');
    if(args['photos-only']==='1'){
      const prior=JSON.parse(readFileSync(join(work,'record-report.json'),'utf8'));
      report.clips={...prior.clips,...report.clips};report.downloads=prior.downloads;
      report.notes='Photo path refreshed from current build; other source clips retained.';json('record-report.json',report);
      if(report.errors.length||report.external.length)throw Error('Photo recording failed');return;
    }
    await p.goto(url+'#/tour');await ready('#tour[data-step="pattern"]');
    await capture('pattern',async()=>{},8);
    await click('#tour-next');await capture('grade',async()=>{},8);
    await click('#tour-next');await capture('lay',async()=>{},7);
    await capture('marker',async()=>{await click('#tour-next');await p.waitForFunction(()=>window.__tourTimes?.marker_done!==undefined,null,{timeout:90000});},16);
    await click('#tour-next');await capture('cut',async()=>{},8);
    await click('#tour-next');await ready('[data-tour-choice="keep"]');await click('[data-tour-choice="keep"]');
    await click('#st-step-grade');await capture('grade',async()=>{},8,true);
    await click('#st-step-pattern');await capture('pattern',async()=>{},8,true);
    await click('#st-step-cut');
    await capture('files',async()=>{
      await p.locator('[data-export="marker-plt"]').scrollIntoViewIfNeeded();
      for(const ext of ['plt','dxf','pdf']){const event=p.waitForEvent('download');await click(`[data-export="marker-${ext}"]`);const d=await event;const f=join(work,`sample-export.${ext}`);await d.saveAs(f);report.downloads.push({format:ext,bytes:statSync(f).size});await wait(1000);}
    },9);
    await click('[data-st="cut-plan"]');await ready('#make-plan');
    await p.evaluate(()=>{document.querySelector('#plan-time').value='0.3';});
    await capture('plan',async()=>{
      for(const input of await p.locator('#plan-quantities input').all())await input.fill('40');
      await p.locator('#max-garments').fill('1');
      await click('#make-plan');await p.waitForFunction(()=>document.querySelector('#plan-error')?.textContent.includes('Plan ready'),null,{timeout:90000});
      await p.locator('#plan-result').scrollIntoViewIfNeeded();
    },8);
    await p.goto(url);await ready('#home-title');await capture('limits',async()=>{},6);
    await click('#feedback-button');await capture('feedback',async()=>{},7);
    json('record-report.json',report);
    if(report.errors.length||report.external.length)throw Error('Recording errors; inspect record-report.json');
  }catch(e){report.failure=e.stack;json('record-report.json',report);const pages=browser?.contexts().flatMap(c=>c.pages())??[];for(const p of pages)await p.screenshot({path:join(work,'record-failure.png')}).catch(()=>{});throw e;
  }finally{await browser?.close();await new Promise(r=>server.close(r));}
}
const time=(s,sep=',')=>{const ms=Math.round(s*1000);return `${String(Math.floor(ms/3600000)).padStart(2,'0')}:${String(Math.floor(ms/60000)%60).padStart(2,'0')}:${String(Math.floor(ms/1000)%60).padStart(2,'0')}${sep}${String(ms%1000).padStart(3,'0')}`;};
function wrap(t,max=43){if(t.length<=max)return t;const stop=t.indexOf('. ');if(stop>=0&&stop+1<=max&&t.length-stop-2<=max)return t.slice(0,stop+1)+'\n'+t.slice(stop+2);const words=t.split(' ');let best=null;for(let i=1;i<words.length;i++){const a=words.slice(0,i).join(' '),b=words.slice(i).join(' '),score=Math.max(a.length,b.length);if(!best||score<best.score)best={score,text:a+'\n'+b};}return best.text;}
function cues(seq,walk=false){let at=0;const slots=[5,5,5,5,5,6,8,6];return seq.map((b,i)=>{const speech=duration(join(work,b.id+'.wav'));const len=walk?Math.ceil((speech+.7)*30)/30:slots[i];if(speech>len-.15)throw Error(`Voice too long ${b.id}: ${speech}/${len}`);const v={...b,at,len,speech};at+=len;return v;});}
function subtitles(seq,file){const ev=[];for(const b of seq){const words=b.say.split(' ');let chunks=[],line='';for(const w of words){if((line+' '+w).length>82){chunks.push(line);line=w;}else line+=(line?' ':'')+w;}chunks.push(line);let pos=0;for(const text of chunks){const n=text.split(' ').length;const from=b.at+b.speech*pos/words.length;pos+=n;ev.push({from,to:pos===words.length?b.at+b.len:b.at+b.speech*pos/words.length,text:wrap(text)});}}
  writeFileSync(join(out,file+'.srt'),ev.map((e,i)=>`${i+1}\n${time(e.from)} --> ${time(e.to)}\n${e.text}\n`).join('\n'));
  writeFileSync(join(out,file+'.vtt'),'WEBVTT\n\n'+ev.map(e=>`${time(e.from,'.')} --> ${time(e.to,'.')}\n${e.text}\n`).join('\n'));return ev;
}
function ass(events,w,h,file){const at=s=>time(s,'.').replace(/^(\d\d):/,'$1:').slice(0,-1);const header=`[Script Info]\nScriptType: v4.00+\nPlayResX: ${w}\nPlayResY: ${h}\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Caption,Segoe UI,${w===1920?48:42},&H00FFFFFF,&H00FFFFFF,&H001C2310,&H001C2310,1,0,0,0,100,100,0,0,3,10,0,2,45,45,${h===1080&&w===1920?36:90},1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n`;
  writeFileSync(file,header+events.map(e=>`Dialogue: 0,${at(e.from)},${at(e.to)},Caption,,0,0,0,,${e.text.replaceAll('\n','\\N')}`).join('\n'));
}
function render(seq,w,h,name,voiced=true){
 console.log('Render',name);
 const events=subtitles(seq,name), total=seq.at(-1).at+seq.at(-1).len, parts=[];
 const cached=join(work,`${name}-picture.mp4`);
 const reusePicture=args.recaption==='all'||(args.recaption==='1'&&w===1920);
 if(!reusePicture||!existsSync(cached)){
 for(const [i,b]of seq.entries()){
  const f=join(work,`${name}-${i}.mp4`),end=b.clip==='end';
  const changed=w===1920?['photo','paperMarker']:['home','photo','paperMarker','pattern','grade','socialCut'];
  if(args.patch==='1'&&!changed.includes(b.clip)&&existsSync(f)){parts.push(f);continue;}
  const input=end?join(work,'end.mp4'):join(work,b.clip+'.mp4');
  const offset=!end&&['paperMarker','cut','files'].includes(b.clip)?Math.max(0,duration(input)-b.len):0;
  const imageH=w===1920?900:(end?608:720);
  const y=w===1920?65:Math.round((h-imageH)/2)-30;
  const title=join(work,`${name}-title.txt`);writeFileSync(title,'BETA CUTTING ROOM  0.8  /  '+(b.clip==='end'?'TRY THE SAMPLE TOUR':b.clip.toUpperCase()));
  const linkFile=join(work,'link.txt');writeFileSync(linkFile,link);
  const headlines={home:'Free cutting-room software',photo:'Your paper patterns. Digitized.',paperMarker:'From photo to marker',pattern:'Start with the T-shirt block',grade:'One style. Every size.',marker:'The marker draws itself',socialCut:'Plan the cut. Get your files.',end:'Try BETA. Help improve it.'};
  const hf=join(work,`${name}-${i}-headline.txt`);writeFileSync(hf,headlines[b.clip]??b.clip);
  const heading=w===1920?'':`,drawtext=fontfile='C\\:/Windows/Fonts/segoeuib.ttf':textfile='${esc(hf)}':fontcolor=white:fontsize=56:x=(w-tw)/2:y=${Math.round(y/2)-20}`;
  const footer=end?`,drawbox=x=0:y=${h-265}:w=iw:h=90:color=0x10231c:t=fill,drawtext=fontfile='C\\:/Windows/Fonts/segoeuib.ttf':textfile='${esc(linkFile)}':fontcolor=0x9de4c2:fontsize=${w===1920?48:36}:x=(w-tw)/2:y=${h-250}`:'';
  // Crop complete important regions, retaining whole labels rather than filling with desktop chrome.
  const rois={paperMarker:[920,500,215,60],pattern:[590,380,270,260],grade:[560,290,112,250],marker:[780,510,85,195]};
  const roi=w===1080?rois[b.clip]:null;
  const crop=roi?`crop=${roi.join(':')},`:'';
  const vf=`${crop}scale=${w}:${imageH}:force_original_aspect_ratio=decrease,pad=${w}:${h}:(ow-iw)/2:${y}:color=0x10231c,drawtext=fontfile='C\\:/Windows/Fonts/segoeuib.ttf':textfile='${esc(title)}':fontcolor=0x9de4c2:fontsize=${w===1920?26:24}:x=40:y=22${heading}${footer},tpad=stop_mode=clone:stop_duration=${b.len}`;
  if(b.clip==='photo'){
    const wide=w===1920,iw=wide?1000:(h===1080?700:1000),ih=iw*.8,ix=wide?80:(w-iw)/2,iy=wide?140:(h===1080?155:190);
    const sx=wide?1160:(h===1080?90:40),sy=wide?380:(h===1080?735:1000),sw=wide?650:(h===1080?900:1000),sh=Math.round(sw*55/350/2)*2;
    const stamp=join(work,'sample-stamp.txt');writeFileSync(stamp,'SYNTHETIC TEACHING SET - 30 PERCENT SCALE');
    ff(['-i',join(work,'review.mp4'),'-i',input,'-filter_complex',`color=c=0x10231c:s=${w}x${h}:r=30:d=${b.len}[bg];[0:v]crop=500:400:218:110,scale=${iw}:${ih},tpad=stop_mode=clone:stop_duration=${b.len}[p];[1:v]crop=350:55:330:333,scale=${sw}:${sh},tpad=stop_mode=clone:stop_duration=${b.len}[s];[bg][p]overlay=${ix}:${iy}[a];[a][s]overlay=${sx}:${sy},drawtext=fontfile='C\\:/Windows/Fonts/segoeuib.ttf':textfile='${esc(stamp)}':fontcolor=0x9de4c2:fontsize=${wide?32:26}:x=${ix}:y=${iy-42}[v]`,'-map','[v]','-t',String(b.len),'-an',...enc,f]);
  }else ff(['-ss',String(offset),'-i',input,'-vf',vf,'-t',String(b.len),'-an',...enc,f]);parts.push(f);
 }
 const list=join(work,`${name}-parts.txt`);writeFileSync(list,parts.map(f=>`file '${f.replaceAll('\\','/')}'`).join('\n'));
 const picture=join(work,`${name}-picture.mp4`);ff(['-f','concat','-safe','0','-i',list,'-c','copy',picture]);
 }
 const picture=cached;
 const af=join(work,`${name}.ass`);ass(events,w,h,af);
 let inputs=['-i',picture],audio=[];
 if(voiced){
  const mix=join(work,`${name}-mix.wav`);const filters=seq.map((b,i)=>`[${i}:a]adelay=${Math.round(b.at*1000)}:all=1[a${i}]`);filters.push(seq.map((b,i)=>`[a${i}]`).join('')+`amix=inputs=${seq.length}:normalize=0,apad,atrim=0:${total}[a]`);
  ff([...seq.flatMap(b=>['-i',join(work,b.id+'.wav')]),'-filter_complex',filters.join(';'),'-map','[a]','-ar','48000',mix]);const norm=join(work,`${name}-norm.wav`);normalize(mix,norm);inputs.push('-i',norm);audio=['-map','1:a','-c:a','aac','-b:a','128k'];
 }else{inputs.push('-f','lavfi','-i','anullsrc=r=48000:cl=stereo');audio=['-map','1:a','-c:a','aac','-b:a','64k'];}
 const labels={home:'START HERE',photo:'PHOTO TO MARKER',paperMarker:'PAPER PATTERNS',socialCut:'CUT PLAN + FILES',pattern:'PATTERN',grade:'GRADE',lay:'FABRIC AND LAY',marker:'LIVE MARKER',cut:'CUT',files:'FILES',limits:'HONEST LIMITS',feedback:'FEEDBACK',end:'TRY THE SAMPLE TOUR'};
 const top=seq.map((b,i)=>{const f=join(work,`${name}-${i}-label.txt`);writeFileSync(f,'BETA CUTTING ROOM  0.8  /  '+labels[b.clip]);return `drawtext=fontfile='C\\:/Windows/Fonts/segoeuib.ttf':textfile='${esc(f)}':fontcolor=0x9de4c2:fontsize=${w===1920?26:24}:x=40:y=22:enable='gte(t,${b.at})*lt(t,${b.at+b.len})'`;});
 ff([...inputs,'-map','0:v','-vf',`drawbox=x=0:y=0:w=iw:h=56:color=0x10231c:t=fill,${top.join(',')},ass='${esc(af)}'`,...enc,...audio,'-t',String(total),'-movflags','+faststart',join(out,name+'.mp4')]);
}
async function cards(){
 // End card is editorial only. All feature footage comes from the real app.
 const pw=await import(pathToFileURL(join(resolve(args.playwright??process.env.PLAYWRIGHT_CORE),'index.mjs')).href);
 const b=await pw.chromium.launch({headless:true,executablePath:args.chrome??'C:/Program Files/Google/Chrome/Application/chrome.exe'});
 try{const p=await b.newPage({viewport:{width:1280,height:720}});await p.setContent(`<body style="margin:0;background:#10231c;color:white;font-family:Segoe UI;padding:70px;box-sizing:border-box"><p style="color:#9de4c2;font-size:28px">FREE · OPEN SOURCE · BETA 0.8</p><h1 style="font-size:70px;line-height:1.05">Your patterns.<br>Your cutting room.</h1><p style="font-size:28px;color:#9de4c2">${link}</p><p style="font-size:30px">Built with modern AI tools. Help us improve it.</p></body>`);await p.screenshot({path:join(work,'end.png')});}finally{await b.close();}
 ff(['-loop','1','-i',join(work,'end.png'),'-t','6',...enc,join(work,'end.mp4')]);
}
async function build(){await cards();const s=cues(social),w=cues(walkthrough,true);json('edit-timeline.json',{social:s,walkthrough:w});
 // Social cut: a real cut-plan result, then the actual three-file download scene.
 for(const [clip,offset,len]of [['plan',Math.max(0,duration(join(work,'plan.mp4'))-4),4],['files',Math.max(0,duration(join(work,'files.mp4'))-4),4]])ff(['-ss',String(offset),'-i',join(work,clip+'.mp4'),'-t',String(len),'-an',...enc,join(work,'social-'+clip+'.mp4')]);
 const cl=join(work,'social-cut.txt');writeFileSync(cl,['plan','files'].map(c=>`file '${join(work,'social-'+c+'.mp4').replaceAll('\\','/')}'`).join('\n'));ff(['-safe','0','-f','concat','-i',cl,'-c','copy',join(work,'socialCut.mp4')]);
 for(const b of s)if(b.clip==='cut')b.clip='socialCut';
 const target=args.target??'all';
 for(const [key,fn]of Object.entries({wide:()=>render(s,1920,1080,'beta-08-social-voice'),silent:()=>render(s,1920,1080,'beta-08-social-silent',false),square:()=>render(s,1080,1080,'beta-08-social-square',false),portrait:()=>render(s,1080,1350,'beta-08-social-4x5',false),walk:()=>render(w,1920,1080,'beta-08-walkthrough')}))if(target==='all'||target.split(',').includes(key))fn();
}
async function stills(){
 for(const [w,h]of [[1280,720],[1080,1080]]){
 const source=join(out,w===1280?'beta-08-social-voice.mp4':'beta-08-social-square.mp4');
 const title=join(work,'poster-title.txt');writeFileSync(title,'FREE CUTTING-ROOM SOFTWARE');
 ff(['-ss','28','-i',source,'-vf',`scale=${w}:${h},drawbox=x=0:y=0:w=iw:h=${w===1280?90:170}:color=0x10231c:t=fill,drawtext=fontfile='C\\:/Windows/Fonts/segoeuib.ttf':textfile='${esc(title)}':fontcolor=0x9de4c2:fontsize=${w===1280?42:40}:x=(w-tw)/2:y=${w===1280?25:65}`,'-frames:v','1',join(out,`beta-08-poster-${w}x${h}.png`)]);
 }
 const source=join(work,'marker.mp4');ff(['-ss','1','-i',source,'-t','7','-vf','scale=960:-2','-an',...enc,'-movflags','+faststart',join(out,'beta-08-marker-loop.mp4')]);
 ff(['-i',join(out,'beta-08-marker-loop.mp4'),'-filter_complex','fps=8,scale=640:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96[p];[b][p]paletteuse=dither=bayer','-loop','0',join(out,'beta-08-marker-loop.gif')]);
}
function check(){
 for(const [stem,seq]of [['beta-08-social-voice',social],['beta-08-social-silent',social],['beta-08-social-square',social],['beta-08-social-4x5',social],['beta-08-walkthrough',walkthrough]]){
   const text=readFileSync(join(out,stem+'.srt'),'utf8').split('\n').filter(l=>l&&!/^\d+$/.test(l)&&!l.includes('-->')).join(' ').replace(/\s+/g,' ').trim();
   if(text!==seq.map(b=>b.say).join(' '))throw Error('Caption/narration mismatch '+stem);
 }
 const result=[];for(const f of readdirSync(out).filter(f=>/\.(mp4|gif|png|srt|vtt)$/.test(f))){const path=join(out,f),bytes=statSync(path).size;const item={file:f,bytes,sha256:createHash('sha256').update(readFileSync(path)).digest('hex')};
 if(/mp4$/.test(f)){item.duration=duration(path);if(/voice|walkthrough/.test(f)){item.loudness=loudness(path);if(Math.abs(item.loudness.lufs+16)>1)throw Error('Loudness '+f);}
 const probe=spawnSync('ffprobe',['-v','error','-show_streams','-show_format','-of','json',path],{encoding:'utf8',windowsHide:true});item.probe=JSON.parse(probe.stdout);
 const data=readFileSync(path);item.faststart=data.indexOf(Buffer.from('moov'))<data.indexOf(Buffer.from('mdat'));if(!item.faststart)throw Error('Missing faststart '+f);
 if(!item.probe.streams.some(s=>s.codec_name==='h264'))throw Error('Not H.264 '+f);
 if(!/loop/.test(f)&&!item.probe.streams.some(s=>s.codec_name==='aac'))throw Error('Missing AAC '+f);
 const log=spawnSync('ffmpeg',['-hide_banner','-i',path,'-vf','blackdetect=d=0.1:pix_th=0.02','-an','-f','null','-'],{encoding:'utf8',windowsHide:true});if(log.status!==0||/black_start:/.test(log.stderr))throw Error('Decode/black frame '+f);
 if(/social/.test(f)&&(bytes>30000000||item.duration<40||item.duration>50))throw Error('Social delivery constraint '+f);
 ff(['-i',path,'-vf',`fps=1/5,scale=384:-1,tile=5x${Math.ceil(item.duration/25)}`,'-frames:v','1',join(work,f+'.contact.png')]);
 }if(/gif$/.test(f)&&bytes>8000000)throw Error('GIF too large');result.push(item);}
 json('qa-media.json',result);writeFileSync(join(out,'manifest.json'),JSON.stringify(result,null,2));
 const rows=result.map(x=>`| ${x.file} | ${x.duration?.toFixed(2)??'-'} | ${(x.bytes/1000000).toFixed(2)} | \`${x.sha256}\` |`).join('\n');
 writeFileSync(join(out,'VIDEO-0.8.md'),`# BETA 0.8 launch videos\n\nLocal review kit. Do not post until 0.8 is live.\n\nLinkedIn: social-square (1080 square) or social-4x5 (1080x1350), both silent with burned captions. X/YouTube: social-voice or social-silent (1920x1080). Walkthrough: narrated 16:9 with burned captions and matching SRT/VTT. Posters are PNG. Marker loop: 7 seconds; GIF repeats forever, MP4 can be looped by the player.\n\nNo music. Original app capture and editorial cards: project Apache-2.0. Kokoro-82M af_heart weights/voices and Misaki dictionaries: Apache-2.0, pinned hashes in AUDIO-SOURCES.md. Segoe UI is a system font, rendered here; no font files redistributed. Synthetic test image only; no factory data.\n\n| File | Seconds | MB (decimal) | SHA-256 |\n|---|---:|---:|---|\n${rows}\n\nChecks: full decode and blackdetect on every MP4; final AAC loudness measurements in manifest.json. Contact sheets every five seconds in work8. Recording errors, network requests and exported sample file sizes in work8/record-report.json. Narration text and caption text share one source. Human listening remains needed for pronunciation and voice quality.\n`);
}
function script(){writeFileSync(join(here,'SCRIPT.md'),'# BETA Cutting Room 0.8 launch script\n\nGenerated from launch-scenes.mjs. Real static browser app; synthetic data only.\n\n## Social cut (45 seconds)\n\n'+social.map(b=>`- **${b.clip}**: ${b.say}`).join('\n')+'\n\n## Walkthrough (three paths)\n\n'+walkthrough.map(b=>`- **${b.clip}**: ${b.say}`).join('\n')+'\n\nEnd card: '+link+'\n');}
for(const s of stage==='all'?['tts','record','build','stills','check','script']:stage.split(',')){if(!({tts,record,build,stills,check,script})[s])throw Error('Unknown step '+s);await ({tts,record,build,stills,check,script})[s]();}
