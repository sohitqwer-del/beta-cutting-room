// Build-time licence gate. All SPDX alternatives choose an allowed licence.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
const manifest = fileURLToPath(new URL('../src-tauri/Cargo.toml', import.meta.url));
const rustc = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
const targetIndex = process.argv.indexOf('--target');
const target = (targetIndex >= 0 ? process.argv[targetIndex + 1] : undefined) ?? process.env.CARGO_BUILD_TARGET ?? rustc.stdout.match(/^host: (.+)$/m)?.[1];
if (!target?.startsWith('x86_64-pc-windows-')) throw new Error('This package targets Windows x64.');
const result = spawnSync('cargo', ['metadata', '--manifest-path', manifest, '--format-version', '1', '--filter-platform', target], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
if (result.status !== 0) throw new Error(result.stderr);
const metadata = JSON.parse(result.stdout);
const allowed = new Set(['MIT', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause', 'ISC', 'Zlib', 'Unicode-3.0', 'Unicode-DFS-2016', '0BSD']);
function permitted(expression) {
  const tokens=expression.replaceAll('/', ' OR ').match(/\(|\)|AND|OR|[A-Za-z0-9.+-]+/g) ?? [];let index=0;
  const atom=()=>{if(tokens[index]==='('){index++;const value=or();if(tokens[index++]!==')')throw new Error('Invalid SPDX');return value;}return allowed.has(tokens[index++]);};
  const and=()=>{let value=atom();while(tokens[index]==='AND'){index++;const right=atom();value=value&&right;}return value;};
  const or=()=>{let value=and();while(tokens[index]==='OR'){index++;const right=and();value=value||right;}return value;};
  const accepted=or();return accepted&&index===tokens.length;
}
const rejected=[];
for(const pkg of metadata.packages){
 if(pkg.name==='jagua-rs'&&pkg.version==='1.0.0'&&pkg.license==='MPL-2.0')continue;
 if(pkg.name==='sparrow'&&pkg.version==='0.3.0'&&!pkg.license){if(readFileSync(join(dirname(pkg.manifest_path),'LICENSE'),'utf8').startsWith('MIT License'))continue;}
 if(!pkg.license||!permitted(pkg.license))rejected.push(`${pkg.name} ${pkg.version}: ${pkg.license??'no licence'}`);
}
if(rejected.length)throw new Error(`Disallowed dependency licences:\n${rejected.join('\n')}`);
console.log(`Licence gate passed: ${metadata.packages.length} Rust packages for ${target}. Only the existing jagua-rs exception uses MPL-2.0.`);
