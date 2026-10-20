#!/usr/bin/env python3
"""Bounded factory-workflow audit using only original fixtures and public FreeSewing.

Run from the repository root: python marker/deploy/audit-cli.py BINARY OUTPUT_DIR
The output folder must be new. No service is installed and no machine is driven.
"""
import argparse
import copy
import hashlib
import http.client
import json
import os
from pathlib import Path
import subprocess
import socket
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    marker = Path(__file__).resolve().parents[1]
    env = dict(os.environ, BETA_DATA_DIR=str(out / 'empty-local-datasets'))
    records = []

    def save(name, value):
        path = out / name
        path.write_text(json.dumps(value, ensure_ascii=False), encoding='utf-8')
        return path

    def run(*arguments, expected=0, timeout=90, executable=binary):
        start = time.perf_counter()
        command = [str(executable), *map(str, arguments)]
        try:
            result = subprocess.run(command, env=env, cwd=marker, capture_output=True,
                                    text=True, encoding='utf-8', errors='replace', timeout=timeout)
            record = dict(args=list(map(str, arguments)), binary=executable.name,
                          seconds=time.perf_counter() - start, exit=result.returncode,
                          stdout=result.stdout, stderr=result.stderr)
            records.append(record)
            if result.returncode != expected:
                raise AssertionError(f"{arguments}: exit {result.returncode}, expected {expected}: {result.stderr}")
            return result.stdout
        except subprocess.TimeoutExpired:
            records.append(dict(args=list(map(str, arguments)), seconds=time.perf_counter() - start, timed_out=True))
            raise
        finally:
            (out / 'commands.json').write_text(json.dumps(records, indent=2), encoding='utf-8')

    helps = ['', 'import', 'nest', 'validate', 'export', 'serve', 'plan', 'plan pipeline',
             'pipeline', 'batch', 'roll', 'roll inspect', 'roll group', 'roll allocate',
             'roll spread', 'roll defects-to-marker', 'cutroom', 'cutroom bundle',
             'cutroom tickets', 'cutroom stickers', 'cutroom recut', 'cutroom report',
             'cutroom cost', 'queue', 'queue add', 'queue run', 'queue cancel', 'queue pause',
             'queue resume', 'queue status', 'bench', 'bench academic', 'bench lectra',
             'bench jobs', 'bench replay', 'bench audit', 'bench finalize']
    for command in helps:
        run(*command.split(), '--help', timeout=10)
    run('--version')
    run('bench', 'academic', '--instances', 'shirts', '--time', '0.1', '--seeds', '1',
        '--out', out / 'academic-missing', expected=1, timeout=10)
    assert 'runtime downloads are disabled' in records[-1]['stderr']
    assert not (out / 'empty-local-datasets').exists()
    # Loopback startup measurement; use only a process and port owned by this run.
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', 0))
        port = probe.getsockname()[1]
    start = time.perf_counter()
    server_env = {key: value for key, value in env.items() if key != 'BETA_TOKEN'}
    child = subprocess.Popen([str(binary), 'serve', '--port', str(port), '--web-dir', str(marker / 'apps/web/dist')],
                             env=server_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(200):
            connection = http.client.HTTPConnection('127.0.0.1', port, timeout=1)
            try:
                connection.request('GET', '/api/health')
                response = connection.getresponse()
                assert response.status == 200
                assert json.loads(response.read())['ok']
                startup_seconds = time.perf_counter() - start
                break
            except OSError:
                assert child.poll() is None, 'Owned server exited during startup'
                time.sleep(0.025)
            finally:
                connection.close()
        else:
            raise AssertionError('Server startup timed out')
    finally:
        child.terminate()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)
    fixture = json.loads((marker / 'packages/marker-wasm/test/fixtures/cutting-room.json').read_text())
    job = save('original.job.json', fixture['job'])
    result = save('original.result.json', fixture['result'])
    run('validate', job, result, '--json')
    for fmt in ['svg', 'plt', 'pdf', 'report', 'iso', 'dxf', 'xlsx', 'labels', 'mini']:
        run('export', job, result, '--format', fmt, '-o', out / f'original.{fmt}')
    run('import', marker / 'testdata/dxf/shirt-aama-mm.dxf', '--pairs', '--width', '1500', '-o', out / 'shirt.job.json')
    run('nest', out / 'shirt.job.json', '--time', '0.2', '--seed', '42', '--workers', '2', '-o', out / 'shirt.result.json')
    run('validate', out / 'shirt.job.json', out / 'shirt.result.json', '--json')
    run('import', out / 'original.plt', '--width', '100', '--result-out', out / 'roundtrip.result.json', '-o', out / 'roundtrip.job.json')
    run('validate', out / 'roundtrip.job.json', out / 'roundtrip.result.json', '--json')
    public = marker / 'testdata/open/freesewing/simon-ratio.job.json'
    run('nest', public, '--time', '1', '--seed', '42', '--workers', '2', '-o', out / 'simon.result.json', timeout=120)
    run('validate', public, out / 'simon.result.json', '--json')
    simon = json.loads((out / 'simon.result.json').read_text())
    simon['placements'][0]['locked'] = True
    fixed = save('simon.locked.json', simon)
    run('nest', public, '--time', '1', '--seed', '42', '--workers', '2', '--fixed', fixed,
        '--warm-start', fixed, '-o', out / 'simon.renested.json', timeout=30)
    run('validate', public, out / 'simon.renested.json', '--json')
    # Nesting may reorder placements; the exact locked placement must be retained.
    assert simon['placements'][0] in json.loads((out / 'simon.renested.json').read_text())['placements']

    order = save('cutorder.json', dict(format='beta-marker.cutorder/v1', name='Original square order', job=fixture['job'],
                                     quantities={'M': 20}, max_plies=10, max_garments_per_marker=1, overcut_pct=0))
    run('plan', order, '--time-per-marker', '0.1', '--max-markers', '1', '--search-nodes', '100', '-o', out / 'plan.json')
    register = copy.deepcopy(fixture['register'])
    for roll in register['rolls']:
        roll['defects'] = []
    rolls = save('rolls.json', register)
    run('roll', 'inspect', rolls, '--format', 'csv', '-o', out / 'inspection.csv')
    run('roll', 'group', rolls, '-o', out / 'groups.json')
    run('roll', 'spread', rolls, out / 'plan.json', '--width', '100', '-o', out / 'spread.json')
    run('roll', 'defects-to-marker', rolls, out / 'spread.json', '-o', out / 'defects.json')
    run('pipeline', order, '--rolls', rolls, '--time-per-marker', '0.1', '--final-time', '0.1',
        '--max-markers', '1', '--search-nodes', '100', '-o', out / 'pipeline', timeout=120)
    lay = save('lay.json', fixture['lay'])
    run('cutroom', 'bundle', lay, '-o', out / 'bundles.json')
    run('cutroom', 'tickets', out / 'bundles.json', '-o', out / 'tickets.pdf')
    run('cutroom', 'stickers', out / 'bundles.json', '--paper', 'label', '-o', out / 'stickers.pdf')
    run('cutroom', 'report', save('actuals.json', [fixture['actuals']]), '--format', 'csv', '-o', out / 'actuals.csv')
    run('cutroom', 'cost', job, '--garments', '1', '-o', out / 'cost.json')
    run('batch', job, '--widths', '100,120', '--time', '0.1', '-o', out / 'batch')
    submission = dict(id='audit-queue', name='Original public square', priority=0,
                      time=dict(tier='overnight', seconds=0.2), task=dict(kind='nest', job=fixture['job'], options=dict(time_limit_s=0.2, seed=42, workers=2, runs=1)))
    request = save('submission.json', submission)
    queue = ['queue', '--folder', out / 'queue']
    run(*queue, 'add', request)
    for operation in ['status', 'pause', 'resume', 'cancel', 'resume']:
        run(*queue, operation, 'audit-queue')
    run(*queue, 'run', '--workers', '2')
    status = json.loads(run(*queue, 'status', 'audit-queue'))
    assert status['status'] == 'completed', status

    # Exact original rectangles isolate validation/export scaling from search quality.
    large = copy.deepcopy(fixture['job'])
    large['name'] = 'Original 1000-square validation probe'
    large['fabric'] = dict(width=400)
    large.pop('order', None)
    large['pieces'][0]['quantity'] = 1000
    large['pieces'][0]['mirror_quantity'] = 0
    large_result = copy.deepcopy(fixture['result'])
    placement = large_result['placements'][0]
    large_result.update(job_name=large['name'], width=400, length=1000, piece_area=400000, efficiency=1)
    large_result['placements'] = [dict(placement, copy=i, bundle=0, translate=[20 * (i % 50), 20 * (i // 50)]) for i in range(1000)]
    large_job = save('large.job.json', large)
    large_marker = save('large.result.json', large_result)
    run('validate', large_job, large_marker, '--json', timeout=30)
    run('export', large_job, large_marker, '--format', 'svg', '-o', out / 'large.svg', timeout=30)
    run('export', large_job, large_marker, '--format', 'xlsx', '-o', out / 'large.xlsx', timeout=30)
    floor = binary.with_name('beta-marker-floor' + binary.suffix)
    if floor.exists():
        run('--help', executable=floor)
        state = dict(version=1, orders=[], register=register, lays=[], issues=[])
        prepare = dict(id='AUDIT-FLOOR', order='PO1', colour='Blue', date='2026-10-20', table='T1',
                       job=fixture['job'], marker=fixture['result'], plies=2, end_allowance_mm=10,
                       roll_ids=[register['rolls'][0]['id']], splice_zones=[], splice_overlap_m=0.1, pieces_per_bundle=20)
        for op, data in [('prepare', prepare), ('dashboard', dict(date='2026-10-20'))]:
            reply = json.loads(run(save(f'floor-{op}.json', dict(op=op, state=state, data=data)), executable=floor))
            state = reply['state']
        save('floor-state.json', state)
    summary = dict(invocations=len(records), passed=sum('exit' in r for r in records),
                   expected_negative_checks=sum(r.get('exit') != 0 for r in records),
                   serve_startup_seconds=startup_seconds,
                   help_checks=len(helps), binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                   slowest=sorted([dict(args=r['args'], seconds=r['seconds']) for r in records], key=lambda r: r['seconds'], reverse=True)[:8])
    save('summary.json', summary)
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
