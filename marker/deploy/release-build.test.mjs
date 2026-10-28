import { test } from 'node:test';
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { releaseEnvironment, localPathLeaks } from './release-build.mjs';

test('remaps Cargo and the whole source tree with spaces, preserving existing flags', () => {
  const env = releaseEnvironment({ RUSTFLAGS: '-C debuginfo=0' }, 'source tree', 'cargo home');
  const flags = env.CARGO_ENCODED_RUSTFLAGS.split('\x1f');
  assert.deepEqual(flags.slice(0, 2), ['-C', 'debuginfo=0']);
  assert.ok(flags.includes(`--remap-path-prefix=${resolve('source tree')}=/build`));
  assert.ok(flags.includes(`--remap-path-prefix=${resolve('cargo home')}=/cargo`));
  assert.equal(env.RUSTFLAGS, undefined);
  assert.equal(env.CARGO_INCREMENTAL, '0');
});

test('uses Cargo encoded-flag precedence without breaking an argument containing spaces', () => {
  const env = releaseEnvironment({ RUSTFLAGS: 'ignored', CARGO_ENCODED_RUSTFLAGS: '-L\x1fpath with spaces' });
  assert.deepEqual(env.CARGO_ENCODED_RUSTFLAGS.split('\x1f').slice(0, 2), ['-L', 'path with spaces']);
  assert.ok(!env.CARGO_ENCODED_RUSTFLAGS.includes('ignored'));
});

test('detects native/slash and UTF-16 local paths, but permits remapped paths', () => {
  const root = resolve('test source');
  for (const path of [root, root.replaceAll('\\', '/'), 'C:\\Users\\builder', 'c:/users/builder', '/home/builder/source']) {
    for (const encoding of ['utf8', 'utf16le']) assert.ok(localPathLeaks(Buffer.from(`before ${path} after`, encoding), [root]).length);
    assert.ok(localPathLeaks(Buffer.concat([Buffer.from([0xff]), Buffer.from(path, 'utf16le')]), [root]).length);
  }
  assert.deepEqual(localPathLeaks(Buffer.from('/cargo/registry/src/lib.rs /build/marker/src/main.rs'), [root]), []);
});

test('scans even and odd binary lengths at both UTF-16 alignments', () => {
  for (const prefix of [Buffer.alloc(0), Buffer.from([0xff])]) {
    for (const suffix of [Buffer.alloc(0), Buffer.from([0xff])]) {
      const bytes = Buffer.concat([prefix, Buffer.from('C:/Users/builder', 'utf16le'), suffix]);
      assert.ok(localPathLeaks(bytes).length);
    }
  }
  assert.deepEqual(localPathLeaks(Buffer.from([0xff])), []);
  assert.deepEqual(localPathLeaks(Buffer.alloc(0)), []);
});
