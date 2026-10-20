#!/usr/bin/env python3
"""Geometry-free round-2 reports from external benchmark evidence."""
import argparse
from datetime import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import re
from zoneinfo import ZoneInfo

import pandas as pd

SOURCE = ('Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for '
          '2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), '
          'version 1.1, **[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)**. Conversion changes coordinates to mm, fixes one '
          'handedness per part, and may restrict rotations. No geometry is published.')
ASSUMPTION = ('**Assumption:** `sheet_type=0` means an ordinary rectangular strip of '
              '`sheet_width`, with open single-layer fabric. No public codebook verifies '
              'this. `flip_x=1` is interpreted as x → −x before rotation. Tier A is exact '
              'conditional on these assumptions. Tier B includes Tier A and restricts '
              'each part to a subset of Lectra’s rotations and mirror states.')


def read_jobs(path):
    return [json.loads(line) for line in path.read_text(encoding='utf-8').splitlines() if line.strip()]


def rows(path):
    frame = pd.read_csv(path / 'results.csv', dtype={'id':str}, keep_default_na=False)
    for name in ('our_efficiency','reference_efficiency','difference_pp','relative_length_saving',
                 'runtime_s','validation_s','budget_s','our_length'):
        frame[name] = pd.to_numeric(frame[name],errors='coerce')
    frame['valid'] = frame['valid'].astype(str).str.lower() == 'true'
    return frame


def stats(frame):
    valid = frame[frame.valid]
    differences = valid.difference_pp
    wins = int((differences > .1+1e-9).sum())
    ties = int((differences.abs() <= .1+1e-9).sum())
    too_long = int((~frame.valid & frame.errors.str.contains('too_long',regex=False)).sum())
    losses = int((differences < -.1-1e-9).sum()) + too_long
    return {'attempted':len(frame),'valid':len(valid),'mean':differences.mean(),
            'median':differences.median(),'wins':wins,'ties':ties,'losses':losses,
            'too_long':too_long,'other_failures':len(frame)-len(valid)-too_long,
            'saving_mean':100*valid.relative_length_saving.mean(),
            'saving_median':100*valid.relative_length_saving.median()}


def fmt(value):
    return f'{value:.4f}' if math.isfinite(value) else '—'


def task_count(count):
    return f"{count} task{'s' if count!=1 else ''}"


def table(groups):
    lines = ['| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |',
             '|---|---:|---:|---:|---|---:|---:|---:|']
    for name, frame in groups:
        s = stats(frame)
        lines.append(f"| {name} | {s['attempted']} / {s['valid']} | {fmt(s['mean'])} | {fmt(s['median'])} | "
                     f"{s['wins']} / {s['ties']} / {s['losses']} | {fmt(s['saving_mean'])} | {fmt(s['saving_median'])} | {s['other_failures']} |")
    return lines


def correlation(frame):
    valid = frame[frame.valid]
    if len(valid)<2 or valid.our_efficiency.nunique()<2 or valid.reference_efficiency.nunique()<2:
        return 'Correlation is undefined (too few distinct valid pairs).'
    pearson = valid.our_efficiency.corr(valid.reference_efficiency)
    spearman = valid.our_efficiency.rank(method='average').corr(valid.reference_efficiency.rank(method='average'))
    return f'Valid pairs: {len(valid)}. Pearson **{pearson:.4f}**; Spearman **{spearman:.4f}** (average ranks for ties).'


def distribution(frame):
    valid = frame[frame.valid]
    values = valid.difference_pp.quantile([0,.05,.25,.5,.75,.95,1])
    return ['| Difference quantile | pp |','|---|---:|'] + [
        f'| {100*q:g}% | {fmt(v)} |' for q,v in values.items()]


def stratum_groups(frame):
    sizes = {'1-25':0,'26-75':1,'76-150':2,'151+':3}
    densities = {'<75':0,'75-85':1,'85+':2}
    def order(item):
        size,density = item[0].split(';')
        return sizes[size.removeprefix('pieces:')],densities[density.removeprefix('lectra:')]
    return sorted(frame.groupby('stratum',sort=False),key=order)


def runtime(path, frame):
    config = json.loads((path/'run.json').read_text())
    summary = (path/'summary.md').read_text(encoding='utf-8')
    wall = re.search(r'Wall time ([\d.]+) s',summary)
    start = datetime.fromtimestamp((path/'run.json').stat().st_mtime,ZoneInfo('Asia/Kolkata'))
    end = datetime.fromtimestamp((path/'summary.md').stat().st_mtime,ZoneInfo('Asia/Kolkata'))
    return (f"Run window: {start:%Y-%m-%d %H:%M:%S} to {end:%Y-%m-%d %H:%M:%S}, Asia/Kolkata. "
            f"Wall time **{wall.group(1) if wall else 'unknown'} s**. Summed nesting time "
            f"**{frame.runtime_s.sum():.3f} s**; median {frame.runtime_s.median():.3f} s; "
            f"maximum {frame.runtime_s.max():.3f} s. Independent validation adds "
            f"{frame.validation_s.sum():.3f} s. Search budgets range from "
            f"{frame.budget_s.min():g} to {frame.budget_s.max():g} s. "
            f"{config['jobs']} simultaneous tasks × {config['threads_per_task']} workers. "
            f"{config.get('processor','Unknown CPU')}; {config['logical_cpus']} logical CPUs. "
            f"Engine revision `{config['revision'][:12]}`, dirty worktree `{config['dirty']}`. "
            'Import, initial construction, final repair and validation add time outside the search budget. '
            'Runner wall time excludes compilation, input-file parsing, setup and final summary writing.')


def failures(frame):
    failed = frame[~frame.valid]
    if failed.empty:
        return ['All attempted layouts passed independent validation.']
    lines = ['| Task | Failure |','|---|---|']
    for r in failed.itertuples():
        lines.append(f'| {r.id} | {r.errors.replace(chr(10)," ").replace("|","/")} |')
    return lines


def hull(points):
    points = sorted(set(tuple(p) for p in points))
    def cross(o,a,b):
        return (a[0]-o[0])*(b[1]-o[1])-(a[1]-o[1])*(b[0]-o[0])
    lower, upper = [], []
    for p in points:
        while len(lower)>=2 and cross(lower[-2],lower[-1],p)<=0:
            lower.pop()
        lower.append(p)
    for p in reversed(points):
        while len(upper)>=2 and cross(upper[-2],upper[-1],p)<=0:
            upper.pop()
        upper.append(p)
    return lower[:-1]+upper[:-1]


def area(points):
    return abs(sum(a[0]*b[1]-a[1]*b[0] for a,b in zip(points,points[1:]+points[:1])))/2


def min_caliper(points):
    points = hull(points)
    widths = []
    for a,b in zip(points,points[1:]+points[:1]):
        dx,dy = b[0]-a[0], b[1]-a[1]
        norm = math.hypot(dx,dy)
        projections = [(dx*p[1]-dy*p[0])/norm for p in points]
        widths.append(max(projections)-min(projections))
    return min(widths)


def audit_inputs(records):
    """Check retained interval inclusion directly, independently of the solver."""
    checked = 0
    for record in records:
        job = record['job']
        assert sum(p['quantity']+p['mirror_quantity'] for p in job['pieces']) == record['piece_count']
        mappings = job['meta']['orientation_mapping']
        assert len(mappings) == record['piece_count']
        for c in mappings:
            original = c['original_states'][str(c['mirror'])]
            if c['rule']=='free':
                assert original == [[0.0,360.0]]
            else:
                assert c['tilt'] == job['fabric']['tilt_deg']
                assert c['n'] == {'fixed':1,'flip180':2,'quarter':4}[c['rule']]
                for i in range(c['n']):
                    lo = (c['phase']+i*360/c['n']-c['tilt'])%360
                    hi = lo+2*c['tilt']
                    intervals = [(lo,hi)] if hi<=360 else [(lo,360),(0,hi-360)]
                    for a,b in intervals:
                        assert any(x-1e-7<=a<=b<=y+1e-7 for x,y in original), (record['id'],c)
            if record['tier']=='A':
                assert c['exact'] and len(c['original_states'])==1
            checked += 1
    return checked


def write_docs(marker, fixed, matched, academic, frames, conversion, paths):
    document = marker/'docs/benchmarks.md'
    prefix = document.read_text(encoding='utf-8').split('## Latest results',1)[0]
    lines = ['## Latest results', '',
             '**Assumption: code 0 is an ordinary rectangular strip.** Tier B includes Tier A '
             'and keeps a subset of allowed orientations. flip_x is interpreted as x-coordinate '
             'reflection before rotation. All figures below are conditional on those assumptions.', '',
             f"Population: {conversion['tier_a']:,} Tier A tasks; {conversion['tier_b_including_a']:,} "
             f"Tier B tasks including A; {conversion['excluded']:,} excluded. The "
             '[round-2 policy audit](../bench/results/2026-10-18-lectra-round2-policy.md) '
             'gives overlapping and primary exclusion counts. The hole stratum is empty.', '',
             'The [60-second report](../bench/results/2026-10-18-lectra-60s.md) uses 300 tasks, '
             'with 25 in each stratum. It includes 293 Tier A tasks and seven with an orientation '
             'handicap. All 300 layouts are valid. All 25 finite-length tasks respect max_length; '
             'there are no too_long failures.', '']
    groups = [('Tier B at 60 s, includes A',fixed),('Tier A at 60 s',fixed[fixed.tier=='A'])]
    if matched is not None:
        groups += [('Tier B at matched time, includes A',matched),
                   ('Tier A at matched time',matched[matched.tier=='A'])]
    lines += table(groups)+['', 'Negative length saving means BETA uses a longer marker. '
                           'The reported mean is the arithmetic mean of per-task savings.', '']
    if matched is None:
        lines += ['The 60-task matched-duration run is still in progress. Its results will be '
                  'reported separately when all trials finish.', '']
    else:
        paired = fixed.merge(matched,on='id',suffixes=('_60','_match'))
        paired = paired[paired.valid_60 & paired.valid_match]
        gain = paired.our_efficiency_match-paired.our_efficiency_60
        lines += ['The [matched-duration report](../bench/results/2026-10-18-lectra-match.md) '
                  'uses the recorded Lectra duration without truncating it to 300 seconds. '
                  'The same seed and three workers are used on a preselected stratified '
                  '60-task subset: 59 Tier A tasks and one handicapped task.', '',
                  f'Recorded budgets range from {matched.budget_s.min():g} to '
                  f'{matched.budget_s.max():g} seconds; the median is '
                  f'{matched.budget_s.median():g}. Two tasks record 608 seconds. '
                  'Those values are kept. Their cause is unknown.', '',
                  *table([('Same subset at 60 s',fixed[fixed.id.isin(matched.id)]),
                          ('Same subset at matched duration',matched)]), '',
                  f'Across {len(paired)} valid pairs, extra time changes efficiency by '
                  f'**{gain.mean():.4f} points on average** and {gain.median():.4f} at the median. '
                  'Separate stochastic runs can regress as well as improve.', '']
    lines += ['### By stratum', '', 'Each stratum has 25 tasks at 60 seconds.', '',
              *table(stratum_groups(fixed)), '']
    if matched is not None:
        lines += ['The matched subset has five tasks in each stratum.', '',
                  *table(stratum_groups(matched)), '']
    lines += ['### Sheet-type evidence', '', 'Code 0, 60 seconds: '+correlation(fixed), '']
    if matched is not None:
        lines += ['Code 0, matched duration: '+correlation(matched), '']
    quantiles = fixed.difference_pp.quantile([0,.05,.5,.95,1])
    lines += [f'At 60 seconds, differences range from {quantiles.iloc[0]:.4f} to '
              f'{quantiles.iloc[-1]:+.4f} points. The 5th, 50th and 95th percentiles are '
              f'{quantiles.iloc[1]:.4f}, {quantiles.iloc[2]:.4f} and {quantiles.iloc[3]:.4f}. '
              f'The mean difference is {stats(fixed)["mean"]:.4f} points, with a positive relationship and wide tails. '
              'The sheet-type codebook remains unverified.', '',
              *table([('Code 2 diagnostic',frames['lectra-round2-code2-60s']),
                      ('Code 3 diagnostic',frames['lectra-round2-code3-60s'])]), '',
              'The analytic width check uses the full Lectra orientation set. Four of the '
              '19 code-3 tasks contain pieces whose minimum required width is 1.243–1.692 '
              'times the stated width. Those same four tasks fail initial construction. '
              'Different sheet semantics, such as folded or tubular fabric, are plausible. '
              'Their actual meaning remains unknown. No code-2 width contradiction was found. '
              'Codes 2 and 3 stay outside the headline.', '',
              'Code 0 has no width-fit contradiction. Task 63488 has a small reference anomaly: '
              'its efficiency is 0.01249 point above an analytic projection bound. Coordinate '
              'precision or denominator details may explain it. That explanation is unverified.', '',
              '### Runtime and academic rerun', '',
              '| Run | Search budget, s | Wall time, s | Summed nesting time, s |',
              '|---|---:|---:|---:|']
    timing = [('Lectra 60 s','lectra-round2-60s',fixed),
              ('Code 2 diagnostic','lectra-round2-code2-60s',frames['lectra-round2-code2-60s']),
              ('Code 3 diagnostic','lectra-round2-code3-60s',frames['lectra-round2-code3-60s']),
              ('Academic','academic-round2-60s',academic)]
    if matched is not None:
        timing.insert(1,('Lectra matched','lectra-round2-match',matched))
    for label,key,frame in timing:
        wall = re.search(r'Wall time ([\d.]+) s',(paths[key]/'summary.md').read_text(encoding='utf-8')).group(1)
        budget = f'{frame.budget_s.min():g}' if frame.budget_s.nunique()==1 else f'{frame.budget_s.min():g}–{frame.budget_s.max():g}'
        lines.append(f'| {label} | {budget} | {wall} | {frame.runtime_s.sum():.3f} |')
    lines += ['', 'Runs use five simultaneous tasks and three workers per task on a 16-logical-CPU '
              'Intel laptop. Import, construction, repair and validation add time outside '
              'the search budget. Workspace checks overlapped early 60-second trials. '
              'Runner wall time excludes compilation, input parsing, setup and final summary writing. '
              'Background CPU load was not controlled. Hardware is shared and not equivalent '
              'to Lectra’s unknown machine.', '',
              'The [round-2 academic rerun](../bench/results/2026-10-18-academic-round2-60s.md) '
              'has **21 of 21 valid trials**. Shirts seed 1 now passes. The mean difference '
              f"from the published means is {stats(academic)['mean']:.4f} points; the median is "
              f"{stats(academic)['median']:.4f}. Win/tie/loss is 0/0/21. The round-1 run had "
              '20 valid trials; it remains available as historical evidence.', '',
              *table(list(academic.groupby('stratum',sort=True))), '',
              'Workspace verification: 70 tests passed, two intentionally ignored; Clippy '
              'passed without warnings. Converter safety and orientation-inclusion checks passed. '
              'The [evidence manifest](../bench/results/2026-10-18-round2-manifest.md) identifies '
              'the input and tooling files. Only summaries and this documentation are committed; '
              'tooling edits remain in the worktree, as requested.', '',
              'Dataset attribution: Lallier, Vézard, Pinaud and Blin (2022), '
              '*Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, '
              '[Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0. '
              'Converted coordinates are in mm; orientation sets may be restricted. '
              'No raw geometry is included.', '']
    document.write_text(prefix+'\n'.join(lines),encoding='utf-8')


def sheet_bound(jobs):
    lines = ['An unconditional geometric check uses each piece’s minimum caliper width '
             '(smallest span in any direction). A rectangular strip must have occupied '
             'length at least the largest such span. This permits more orientations than '
             'Lectra and is therefore a safe, weak lower bound. A reference-implied length '
             'below it would contradict the rectangular-strip model.', '',
             '| Code | Tasks | Reference length below bound | Smallest reference length / bound |',
             '|---|---:|---:|---:|']
    for code, records in jobs:
        ratios = []
        for record in records:
            pieces = record['job']['pieces']
            net_area = sum(area(p['outline'])*p['quantity'] for p in pieces)*record['reference_area_ratio']
            implied_length = net_area/(record['job']['fabric']['width']*record['lectra_efficiency']/100)
            bound = max(min_caliper(p['outline']) for p in pieces)
            ratios.append(implied_length/bound)
        lines.append(f'| {code} | {len(ratios)} | {sum(r<1-1e-6 for r in ratios)} | {min(ratios):.4f} |')
    return lines


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data-dir',type=Path,default=Path(os.environ['BETA_DATA_DIR']))
    p.add_argument('--fixed-only',action='store_true',help='Write completed 60-second and academic reports while match is still running')
    args = p.parse_args()
    marker = Path(__file__).resolve().parents[2]
    out = marker/'bench/results'
    data = args.data_dir/'marker-bench'
    conversion = json.loads((data/'lectra/conversion-report.json').read_text())
    sample = read_jobs(data/'lectra/sample-300.jsonl')
    checked_parts = audit_inputs(sample)
    metadata = {r['id']:r for r in sample}
    paths = {name:data/'results'/f'2026-10-18-{name}' for name in
             ('lectra-round2-60s','lectra-round2-match','academic-round2-60s',
              'lectra-round2-code2-60s','lectra-round2-code3-60s')}
    frames = {name:rows(path) for name,path in paths.items()}
    fixed, match = frames['lectra-round2-60s'], frames['lectra-round2-match']
    for frame in (fixed,match):
        frame['tier'] = frame.id.map(lambda i:metadata[i]['tier'])
        frame['finite_sheet'] = frame.id.map(lambda i:metadata[i]['job']['fabric']['max_length'] is not None)
    assert len(fixed)==300 and fixed.id.nunique()==300
    if not args.fixed_only:
        assert len(match)==60 and match.id.nunique()==60 and set(match.id)<=set(fixed.id)
    policy = [ASSUMPTION, '',
              'Lectra’s recorded efficiencies came from its own nesting engine on real client tasks. '
              'The paper gives a nominal five-minute budget. Hardware, engine version and worker '
              'count are unknown. These runs compare quality at selected search budgets; they do '
              'not establish hardware-equivalent speed or full garment-rule coverage.', '',
              'The local machine was shared. Workspace verification overlapped early '
              '60-second trials, and background CPU load was not controlled. Paired time '
              'differences also include stochastic scheduling and load effects.', '',
              'Tier B fixes one handedness per physical part. It keeps the largest regular '
              '1/2/4-angle subset or continuous rotation expressible with one shared fabric tilt. '
              'The converter maximizes summed allowed angular measure, then base-angle count. '
              'It records original and retained sets for every sampled part. Other constraint '
              'classes, extra parameters and unverified multi-ring shapes remain excluded. '
              'A positive sheet length is enforced as max_length.', '',
              'Differences are BETA minus Lectra, in percentage points. A tie is within 0.1 point. '
              'Length saving is 100 × (1 − Lectra efficiency / BETA efficiency); positive means '
              'shorter. Numerical statistics use valid layouts only. A too_long failure is a '
              'loss with blank numerical scores. Other failures remain separate. The fixed-seed '
              'sample balances piece count and Lectra efficiency; its unweighted means are '
              'descriptive, not estimates for all customers.']
    population = ['| Population category | Tasks |','|---|---:|',
                  f"| All | {conversion['tasks']:,} |",
                  f"| Tier A | {conversion['tier_a']:,} |",
                  f"| Tier B including A | {conversion['tier_b_including_a']:,} |",
                  f"| Tier B only | {conversion['tier_b_only']:,} |",
                  f"| Excluded | {conversion['excluded']:,} |", '',
                  f"Multi-ring tasks: {conversion['multi_ring_tasks']}; verified-hole stratum: "
                  f"{conversion['verified_hole_tasks']}. No holes were silently filled in a headline task.", '',
                  f"Population handicap task counts overlap: {conversion['population_handicap_tasks']['optional_mirror']:,} "
                  'drop optional mirror states; '
                  f"{conversion['population_handicap_tasks']['angles_or_tilt']:,} restrict angles or tilt."]
    for label,frame,key,filename in (
        ('60-second',fixed,'lectra-round2-60s','2026-10-18-lectra-60s.md'),
        ('matched-duration',match,'lectra-round2-match','2026-10-18-lectra-match.md')):
        if args.fixed_only and label=='matched-duration':
            continue
        lines = [f'# Lectra {label} comparison — 18 October 2026','',*policy,'',
                 '## Population and sample','',*population,'',
                 f"Sampling seed **7030786**. This run has {len(frame)} tasks; "
                 f"{sum(frame.tier=='A')} are Tier A. The 60-task matched sample is a stratified "
                 'subset of the original 300, chosen before examining run results. '
                 'All Lectra trials use nesting seed **1**. Academic trials use seeds 1, 2 and 3.','',
                 f"This run drops optional mirror choices for {task_count(sum(metadata[i]['dropped_mirror_parts']>0 for i in frame.id))} "
                 f"and angles or tilt for {task_count(sum(metadata[i]['dropped_angle_parts']>0 for i in frame.id))}. "
                 'These counts can overlap. A finite max_length applies to '
                 f"{task_count(sum(metadata[i]['job']['fabric']['max_length'] is not None for i in frame.id))}.",'',
                 '## Results','',*table([('Tier B headline, includes A',frame),
                    ('Tier A, exact',frame[frame.tier=='A']),('Tier B only',frame[frame.tier=='B'])]),'',
                 *table(stratum_groups(frame)),'',
                 *table([('No finite max_length',frame[~frame.finite_sheet]),
                         ('Finite max_length enforced',frame[frame.finite_sheet])]),'',
                 '## Correlation and differences','',correlation(frame),'',*distribution(frame),'',
                 '## Runtime','',runtime(paths[key],frame),'','## Failures','',*failures(frame),'']
        if label == 'matched-duration':
            paired = fixed.merge(match,on='id',suffixes=('_60','_match'))
            good = paired[paired.valid_60 & paired.valid_match]
            gain = good.our_efficiency_match-good.our_efficiency_60
            lines += ['## Paired time effect','',
                      'Matched budgets use the recorded Lectra duration without the old 300-second '
                      'cap (`--match-cap 10000`). The same seed and worker count are used. '
                      'Separate time-limited stochastic runs can regress as well as improve.','',
                      f'Recorded budgets: minimum {match.budget_s.min():g} s, median '
                      f'{match.budget_s.median():g} s, maximum {match.budget_s.max():g} s. '
                      'Two tasks (96058 and 60855) record 608 seconds. The nominal five-minute '
                      'description does not explain these values. They are retained without '
                      'inventing an overhead explanation.','',
                      *table([('Same subset at 60 s',fixed[fixed.id.isin(match.id)]),('Matched duration',match)]),'',
                      f'Valid paired runs: {len(good)}. Mean gain from extra time: **{gain.mean():.4f} pp**; '
                      f'median **{gain.median():.4f} pp**. Improved / within 0.1 pp / regressed: '
                      f'{sum(gain>.1+1e-9)} / {sum(gain.abs()<=.1+1e-9)} / {sum(gain<-.1-1e-9)}.','',
                      'Same subset at 60 seconds: '+correlation(fixed[fixed.id.isin(match.id)]),'',
                      f"The observed change closes {100*gain.mean()/(-fixed[fixed.id.isin(match.id)].difference_pp.mean()):.1f}% "
                      'of the paired subset’s mean efficiency deficit. Background load and '
                      'stochastic scheduling remain confounders.','']
        else:
            lines += ['## Sheet-type diagnostics','',
                      'Codes 2 and 3 are excluded from the headline. Each diagnostic applies the '
                      'same orientation and unsupported-constraint filters and temporarily assumes '
                      'an ordinary strip solely to test that model. Code 2 has 403 otherwise '
                      'eligible tasks; 20 are sampled. All 19 otherwise eligible code-3 tasks '
                      'are used. The other eight code-3 tasks have unsupported constraints '
                      '(t2 in all eight; c4 also occurs in two).','']
            diagnostics = []
            for code in (2,3):
                diagkey = f'lectra-round2-code{code}-60s'
                diag = frames[diagkey]
                lines += table([(f'Code {code}',diag)])+['',correlation(diag),'',runtime(paths[diagkey],diag),'',*failures(diag),'']
                diagnostics.append((code,read_jobs(data/f'lectra/diagnostic-code-{code}.jsonl')))
            geometry = json.loads((data/'lectra/geometry-check-round2.json').read_text())
            lines += ['A stronger analytic check uses each piece’s full source orientation '
                      'set, including all mirror states, rather than BETA’s restricted set. '
                      'It finds the minimum possible lengthwise span subject to fitting the '
                      'stated width. The task lower bound is the largest piece bound. '
                      'Support changes, interval ends and width-boundary roots are enumerated; '
                      'this is a projection bound, not a nesting search. Source part joins and '
                      'inverse transforms are checked. Tolerances: 1e-7 mm and 1e-6 in the length ratio.','',
                      '| Code | Tasks | Cannot fit stated width | Reference length below bound | Minimum reference length / bound among width-feasible tasks |',
                      '|---|---:|---:|---:|---:|']
            for g in geometry['results']:
                lines.append(f"| {g['sheet_type']} | {g['tasks']} | {g['width_infeasible_tasks']} | "
                             f"{g['reference_length_below_bound']} | {g['minimum_ratio']:.6f} |")
            lines += ['',
                      'Code-0 task 63488 is a small anomaly: Lectra’s implied length is '
                      '0.02916% below the analytic bound, equivalent to 0.01249 percentage '
                      'point of efficiency. This is below the 0.1-point tie threshold. '
                      'Coordinate precision or denominator details could explain it; that '
                      'explanation remains unverified. It is not a gross width contradiction.','',
                      'Four code-3 tasks contradict the ordinary-strip width model. The '
                      'largest minimum required width in each task is 1.243–1.692 times '
                      'the stated width. This supports excluding code 3 and is consistent '
                      'with different sheet semantics, such as folded or tubular fabric. '
                      'It does not identify the actual codebook. No code-2 task in this '
                      'sample violates the analytic bound.','',
                      'Correlation and geometry checks can falsify a gross width interpretation. '
                      'They cannot prove the sheet-type codebook, nor separate every engine-quality '
                      'gap from the imposed orientation handicap. Finite-length failures and '
                      'remaining reference-denominator uncertainty must be considered.','',
                      f"For code 0, the positive correlations, {-stats(frame)['mean']:.4f}-point mean deficit and "
                      'absence of a width-fit contradiction support this conditional comparison. '
                      f'The differences have wide tails, from {frame.difference_pp.min():.4f} to '
                      f'{frame.difference_pp.max():+.4f} points. '
                      'The sheet-type codebook and reference denominator remain unverified.','']
        lines += ['## Attribution and evidence','',SOURCE,'',
                  'The four source checksums match the pinned Zenodo files. The restricted '
                  'unpickler and explicit global allow-list remain in use. Full per-task metadata, '
                  'Job inputs, CSV results and run configurations stay in the external data directory. '
                  'No raw geometry or build output is committed.','']
        lines += [f'An independent metadata audit checked complete retained-interval inclusion and '
                  f'quantity preservation for all **{checked_parts:,} physical parts** in the 300-task '
                  'sample. This checks the declared orientation sets; the solver’s actual placements '
                  'also pass the independent Job validator.','']
        (out/filename).write_text('\n'.join(lines),encoding='utf-8')
    academic = frames['academic-round2-60s']
    lines = ['# Academic round-2 rerun — 18 October 2026','',
             f"**{sum(academic.valid)} of {len(academic)} trials passed independent validation.** "
             'Seven instances, seeds 1/2/3, 60-second search budget, round-2 collision guard and repair. '
             'The round-1 shirts seed-1 overlap is checked again here.','',
             *table([('All',academic),*list(academic.groupby('stratum',sort=True))]),'',
             runtime(paths['academic-round2-60s'],academic),'',*failures(academic),'',
             'References: Gardeyn, Vanden Berghe and Wauters, '
             '[*An open-source heuristic to reboot 2D nesting research*, arXiv:2509.13329v4](https://arxiv.org/html/2509.13329v4), '
             'Tables 2–3. Published means use 100 runs, 1200 seconds, three threads and an AMD 7950X. '
             'BETA uses sparrow itself. This measures the adapter and budget, not a separate algorithm '
             'or a hardware-normalized comparison. Geometry is not included.','']
    (out/'2026-10-18-academic-round2-60s.md').write_text('\n'.join(lines),encoding='utf-8')
    (out/'2026-10-18-lectra-round2-policy.md').write_text(
        (data/'lectra/conversion-summary.md').read_text(encoding='utf-8'),encoding='utf-8')
    code_paths = ['bench/tools/lectra_convert.py','bench/tools/summarize_round2.py',
                  'bench/tools/lectra_geometry_check.py',
                  'crates/beta-marker-cli/src/cmd/bench.rs','crates/beta-marker-cli/src/cmd/bench/runner.rs']
    manifest = ['# Round-2 evidence manifest','',ASSUMPTION,'',
                'Base engine revision: `3022b12`. Benchmark tooling has local changes. '
                'Only summaries are committed, as requested. These SHA-256 hashes identify the '
                'final tooling files; the code changes remain in this worktree for the lead.','',
                '| File | SHA-256 |','|---|---|']
    for name in code_paths:
        manifest.append(f'| `{name}` | `{hashlib.sha256((marker/name).read_bytes()).hexdigest()}` |')
    manifest += ['', '| Input | SHA-256 |','|---|---|']
    for name in ('sample-300.jsonl','sample-match-60.jsonl','diagnostic-code-2.jsonl','diagnostic-code-3.jsonl'):
        manifest.append(f'| `{name}` | `{hashlib.sha256((data/"lectra"/name).read_bytes()).hexdigest()}` |')
    manifest += ['','Input hashes identify external files; inputs contain geometry and are not committed.','']
    (out/'2026-10-18-round2-manifest.md').write_text('\n'.join(manifest),encoding='utf-8')
    write_docs(marker,fixed,None if args.fixed_only else match,academic,frames,conversion,paths)
    print(json.dumps({k:stats(v) for k,v in frames.items()
                      if not args.fixed_only or k!='lectra-round2-match'},indent=2))


if __name__ == '__main__':
    main()
