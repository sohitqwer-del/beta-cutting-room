#!/usr/bin/env python3
"""One-time Lectra pickle audit and conversion. Run Python with -I outside data dir."""
import argparse
import collections
import gzip
import hashlib
import importlib
import json
import math
import os
from pathlib import Path
import pickle
import pickletools
import random
import sys
import urllib.request

RECORD = 'https://zenodo.org/api/records/7030786'
FILES = ('tasks.gz', 'parts.gz', 'constraints.gz', 'shapes.gz')
CHECKSUMS = {
    'tasks.gz': 'ac18fc58408a3fc832cfd6757b4b16ca',
    'parts.gz': 'd8b51403f0cab79ec990b95a40911c1c',
    'constraints.gz': 'e12581851bd2a357145a9dfccdad5363',
    'shapes.gz': 'ff1623f24adf031710450a30e72984f2',
}
# Exact reconstruction globals only. Never allow whole modules or dynamic fallbacks.
ALLOWED = {
    ('pandas.core.frame', 'DataFrame'),
    ('pandas.core.internals.managers', 'BlockManager'),
    ('pandas._libs.internals', '_unpickle_block'),
    ('pandas.core.indexes.base', '_new_Index'),
    ('pandas.core.indexes.base', 'Index'),
    ('pandas.core.indexes.range', 'RangeIndex'),
    ('pandas.core.indexes.numeric', 'Int64Index'),
    ('numpy.core.multiarray', '_reconstruct'),
    ('numpy._core.multiarray', '_reconstruct'),
    ('numpy.core.multiarray', 'scalar'),
    ('numpy._core.multiarray', 'scalar'),
    ('numpy.core.numeric', '_frombuffer'),
    ('numpy._core.numeric', '_frombuffer'),
    ('numpy', 'ndarray'), ('numpy', 'dtype'), ('builtins', 'slice'),
}


def audit(path):
    """Symbolic stack: resolve GLOBAL/STACK_GLOBAL without executing any opcode."""
    stack, memo, found = [], {}, set()
    memo_count = 0
    mark, unknown = object(), object()
    with gzip.open(path, 'rb') as source:
        for op, arg, pos in pickletools.genops(source):
            n = op.name
            if n in ('PROTO', 'FRAME'):
                continue
            if n in ('BININT','BININT1','BININT2','LONG','LONG1','LONG4','INT','FLOAT','BINFLOAT',
                     'NONE','NEWTRUE','NEWFALSE','BYTEARRAY8','BINBYTES','BINBYTES8','SHORT_BINBYTES',
                     'EMPTY_LIST','EMPTY_DICT','EMPTY_TUPLE','EMPTY_SET'):
                stack.append(unknown)
                continue
            if n == 'APPEND':
                stack.pop()
                continue
            if n == 'SETITEM':
                del stack[-2:]
                continue
            if n in ('GLOBAL', 'INST'):
                module, name = arg.split(' ', 1)
                found.add((module, name))
                if n == 'INST':
                    raise ValueError('Legacy INST is refused')
                stack.append(unknown)
            elif n == 'STACK_GLOBAL':
                name, module = stack.pop(), stack.pop()
                if not isinstance(module, str) or not isinstance(name, str):
                    raise ValueError(f'Unresolved STACK_GLOBAL at {pos}')
                found.add((module, name))
                stack.append(unknown)
            elif n in ('EXT1', 'EXT2', 'EXT4', 'PERSID', 'BINPERSID'):
                raise ValueError(f'Forbidden opcode {n} at {pos}')
            elif n == 'MARK':
                stack.append(mark)
            elif n == 'MEMOIZE':
                if isinstance(stack[-1], str):
                    memo[memo_count] = stack[-1]
                memo_count += 1
            elif n in ('PUT', 'BINPUT', 'LONG_BINPUT'):
                index = int(arg)
                if index > memo_count:
                    raise ValueError('Sparse legacy memo is refused')
                memo_count += index == memo_count
                if isinstance(stack[-1], str):
                    memo[index] = stack[-1]
                else:
                    memo.pop(index, None)
            elif n in ('GET', 'BINGET', 'LONG_BINGET'):
                stack.append(memo.get(int(arg), unknown))
            elif n == 'DUP':
                stack.append(stack[-1])
            elif n in ('UNICODE', 'BINUNICODE', 'SHORT_BINUNICODE', 'BINUNICODE8', 'STRING'):
                stack.append(arg)
            else:
                before = op.stack_before
                if pickletools.markobject in before:
                    i = len(stack) - 1 - stack[::-1].index(mark)
                    fixed = before.index(pickletools.markobject)
                    del stack[i-fixed:]
                elif before:
                    del stack[-len(before):]
                stack.extend(unknown for _ in op.stack_after)
    return sorted(found)


class RestrictedUnpickler(pickle.Unpickler):
    def find_class(self, module, name):
        if (module, name) not in ALLOWED:
            raise pickle.UnpicklingError(f'Refused global {module}.{name}')
        # pandas 2 removed Int64Index. The public Index reconstructs the integer index.
        if (module, name) == ('pandas.core.indexes.numeric', 'Int64Index'):
            from pandas import Index
            return Index
        aliases = {'numpy.core.numeric': 'numpy._core.numeric',
                   'numpy.core.multiarray': 'numpy._core.multiarray'}
        return getattr(importlib.import_module(aliases.get(module, module)), name)

    def persistent_load(self, pid):
        raise pickle.UnpicklingError('Persistent ids are refused')


def download(folder):
    folder.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(RECORD, timeout=90) as response:
        record = json.load(response)
    (folder / 'record.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    manifest = {}
    for entry in record['files']:
        name = entry['key']
        if name not in FILES:
            continue
        path = folder / name
        algorithm, expected = entry['checksum'].split(':')
        def digest():
            with path.open('rb') as stream:
                return hashlib.file_digest(stream, algorithm).hexdigest()
        if not path.exists() or digest() != expected:
            temp = path.with_suffix('.partial')
            with urllib.request.urlopen(f'{RECORD}/files/{name}/content', timeout=120) as source, temp.open('wb') as target:
                while block := source.read(1024 * 1024):
                    target.write(block)
            temp.replace(path)
        if digest() != expected:
            raise ValueError(f'Checksum mismatch: {name}')
        manifest[name] = entry['checksum']
        print(name, 'verified', flush=True)
    return manifest


def load_tables(folder, out):
    if folder.resolve() == Path.cwd().resolve():
        raise ValueError('Run from a directory other than the download folder')
    report = {}
    cache = out / 'pickle-globals.json'
    if cache.exists():
        report = json.loads(cache.read_text(encoding='utf-8'))
    for name in FILES:
        with (folder / name).open('rb') as stream:
            digest = hashlib.file_digest(stream, 'md5').hexdigest()
        if digest != CHECKSUMS[name]:
            raise ValueError(f'Checksum mismatch: {name}')
        if name in report:
            continue
        globals_used = audit(folder / name)
        report[name] = ['.'.join(x) for x in globals_used]
        print(name, report[name], flush=True)
    (out / 'pickle-globals.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    for name in FILES:
        refused = set(tuple(x.rsplit('.', 1)) for x in report[name]) - ALLOWED
        if refused:
            raise ValueError(f'Unreviewed globals in {name}: {refused}')
    tables = {}
    for name in FILES:
        with gzip.open(folder / name, 'rb') as source:
            tables[name[:-3]] = RestrictedUnpickler(source).load()
    return tables


def profile(tables, out):
    report = {}
    for name, table in tables.items():
        report[name] = {'rows': len(table), 'columns': {str(k): str(v) for k, v in table.dtypes.items()},
                        'head': table.head(3).to_dict(orient='records')}
    c = tables['constraints']
    report['constraint_types'] = c['type'].value_counts(dropna=False).to_dict()
    report['constraint_column_presence'] = c.groupby('type').count().to_dict(orient='index')
    report['sheet_types'] = tables['tasks']['sheet_type'].value_counts(dropna=False).to_dict()
    report['rotation_examples'] = c.loc[c['r1_start'].notna()].head(20).to_dict(orient='records')
    (out / 'profile.json').write_text(json.dumps(report, default=lambda x: x.tolist() if hasattr(x, 'tolist') else str(x), indent=2), encoding='utf-8')
    print('Rows:', {k: len(v) for k, v in tables.items()}, flush=True)
    print('Constraints:', report['constraint_types'], flush=True)


def main():
    if not sys.flags.isolated:
        raise SystemExit('Python -I is required')
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('mode', choices=['download', 'inspect', 'profile', 'convert', 'self-test', 'report'])
    p.add_argument('--data-dir', type=Path, default=Path(os.environ['BETA_DATA_DIR']) if 'BETA_DATA_DIR' in os.environ else None)
    p.add_argument('--seed', type=int, default=7030786)
    p.add_argument('--sample-size', type=int, default=300)
    p.add_argument('--out', type=Path, help='New evidence directory; do not overwrite previous conversions')
    p.add_argument('--reference-sample', type=Path, help='Preserve confirmation IDs from an existing sample')
    p.add_argument('--reference-match', type=Path, help='Preserve matched-duration subset IDs')
    args = p.parse_args()
    if args.data_dir is None:
        p.error('set BETA_DATA_DIR or pass --data-dir outside the checkout')
    if args.sample_size <= 0:
        p.error('--sample-size must be positive')
    folder = args.data_dir / 'datasets' / 'lectra-nesting-7030786'
    out = args.out or args.data_dir / 'marker-bench' / 'lectra'
    out.mkdir(parents=True, exist_ok=True)
    if args.mode == 'self-test':
        self_test(args.data_dir)
        return
    if args.mode == 'download':
        download(folder)
        return
    if args.mode == 'inspect':
        report = {}
        for name in FILES:
            report[name] = ['.'.join(x) for x in audit(folder/name)]
            print(name, report[name], flush=True)
        (out/'pickle-globals.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(report, indent=2))
        return
    if args.mode == 'report':
        write_summary(out)
        return
    tables = load_tables(folder, out)
    profile(tables, out)
    if args.mode == 'convert':
        convert(tables, out, args.seed, args.sample_size, args.reference_sample, args.reference_match)


def present(value):
    return value is not None and not (isinstance(value, float) and math.isnan(value))


def sequence(value):
    if not present(value):
        return []
    return list(value) if hasattr(value, '__iter__') and not isinstance(value, (str, bytes)) else [value]


def rotation_rule(starts, ends, flips):
    starts, ends, flips = sequence(starts), sequence(ends), sequence(flips)
    if not starts or len(starts) != len(ends) or len(starts) != len(flips):
        return None, 'missing_or_malformed_rotation'
    if any(f != 0 for f in flips):
        # Whether flip_x means x-coordinate reflection or reflection about x is not specified.
        # Optional handedness also cannot be represented by fixed mirror quantities.
        return None, 'flip_semantics_unverified'
    if any(not math.isfinite(float(a)) for a in starts + ends):
        return None, 'nonfinite_rotation'
    if len(starts) == 1 and abs(float(ends[0])-float(starts[0])) == 360:
        return 'free', None
    if any(float(a) != float(b) for a, b in zip(starts, ends)):
        return None, 'rotation_interval_not_exactly_supported'
    angles = frozenset(float(a) % 360 for a in starts)
    rules = {frozenset([0]): 'fixed', frozenset([0,180]): 'flip180',
             frozenset([0,90,180,270]): 'quarter'}
    return (rules[angles], None) if angles in rules else (None, 'rotation_set_not_exactly_supported')


def merge_arcs(arcs):
    merged = []
    for lo, hi in sorted(arcs):
        if merged and lo <= merged[-1][1] + 1e-9:
            merged[-1][1] = max(merged[-1][1], hi)
        else:
            merged.append([lo, hi])
    return merged


def circle_arc(start, length):
    if length >= 360 - 1e-9:
        return [[0.0, 360.0]]
    start %= 360
    end = start + length
    if end > 360:
        return [[0.0, end - 360], [start, 360.0]]
    # Duplicate the seam point so intersections at 0/360 work for point sets.
    arcs = [[start, end]]
    if start == 0:
        arcs.append([360.0, 360.0])
    if end == 360:
        arcs.append([0.0, 0.0])
    return merge_arcs(arcs)


def intersect_arcs(a, b):
    return merge_arcs([[max(x, u), min(y, v)] for x, y in a for u, v in b
                       if max(x, u) <= min(y, v) + 1e-9])


def orientation_candidates(starts, ends, flips):
    """All regular 1/2/4-angle subsets, plus free, in either handedness.

    A phase is baked into the outline. flip_x=1 is assumed to mean x -> -x,
    before rotation. This is the lead's conservative policy, not a codebook.
    """
    starts, ends, flips = sequence(starts), sequence(ends), sequence(flips)
    if not starts or len(starts) != len(ends) or len(starts) != len(flips):
        return None, 'missing_or_malformed_rotation'
    if any(not math.isfinite(float(a)) for a in starts + ends):
        return None, 'nonfinite_rotation'
    if any(f not in (0, 1) for f in flips):
        return None, 'unknown_flip_code'
    states = {}
    candidates = []
    for mirror in sorted(set(int(f) for f in flips)):
        arcs = []
        for start, end, flip in zip(starts, ends, flips):
            if flip != mirror:
                continue
            span = float(end) - float(start)
            if span < 0 and abs(span) < 360:
                # Wrapped counter-clockwise interval, e.g. 358 -> 2.
                span %= 360
            arcs.extend(circle_arc(float(start), abs(span)))
        arcs = merge_arcs(arcs)
        states[mirror] = arcs
        if arcs == [[0.0, 360.0]]:
            candidates.append({'rule': 'free', 'mirror': mirror, 'phase': 0.0,
                               'capacity': 180.0, 'n': 0})
            continue
        for n, rule in ((4, 'quarter'), (2, 'flip180'), (1, 'fixed')):
            possible = arcs
            for step in range(1, n):
                shift = step * 360 / n
                shifted = merge_arcs([piece for lo, hi in arcs
                                      for piece in circle_arc(lo - shift, hi - lo)])
                possible = intersect_arcs(possible, shifted)
            components = [list(x) for x in possible]
            if len(components) > 1 and components[0][0] == 0 and components[-1][1] == 360:
                components = [[components[-1][0] - 360, components[0][1]]] + components[1:-1]
            for lo, hi in components:
                candidates.append({'rule': rule, 'mirror': mirror, 'phase': ((lo + hi) / 2) % 360,
                                   'capacity': max(0.0, (hi - lo) / 2), 'n': n})
    return {'states': states, 'candidates': candidates}, None


def represented_arcs(c):
    if c['rule'] == 'free':
        return [[0.0, 360.0]]
    return merge_arcs([arc for i in range(c['n'])
        for arc in circle_arc(c['phase'] + i * 360 / c['n'] - c['tilt'], 2 * c['tilt'])])


def arcs_equal(a, b):
    return len(a) == len(b) and all(abs(x-u) < 1e-7 and abs(y-v) < 1e-7
        for (x,y),(u,v) in zip(a,b))


def reflected_arcs(arcs):
    return merge_arcs([arc for lo,hi in arcs for arc in circle_arc(-hi, hi-lo)])


def choose_orientations(specs):
    """Choose each piece's own tilt and optional reflection, without enlarging its set.

    After baking R(phase) M(mirror) into the outline, an optional reflection
    negates phase. Intersect with that negated opposite-handed set before
    enabling Piece.flip. Independent reflection/rotation sets cannot express
    every source orientation union; those remain Tier B, with the loss recorded.
    """
    selected = {}
    for part_id, spec in specs.items():
        choices = [dict(c, flip=False) for c in spec['candidates']]
        if set(spec['states']) == {0,1}:
            joint = intersect_arcs(spec['states'][0], reflected_arcs(spec['states'][1]))
            if joint:
                shared, error = orientation_candidates([a for a,b in joint], [b for a,b in joint], [0]*len(joint))
                if error:
                    raise ValueError(error)
                choices += [dict(c, flip=True) for c in shared['candidates']]
        for c in choices:
            c['tilt'] = 0.0 if c['rule']=='free' else c['capacity']
            represented = represented_arcs(c)
            states = {c['mirror']: represented}
            if c['flip']:
                states[1-c['mirror']] = reflected_arcs(represented)
            c['exact'] = set(states)==set(spec['states']) and all(arcs_equal(states[f], spec['states'][f]) for f in states)
            c['dropped_mirror_state'] = set(states)!=set(spec['states'])
            c['dropped_angles_or_tilt'] = any(not arcs_equal(states[f], spec['states'][f]) for f in states)
        def score(c):
            count = 2 if c['flip'] else 1
            measure = 360 if c['rule']=='free' else min(360, 2*c['tilt']*c['n'])
            cardinality = 360 if c['rule']=='free' else c['n']
            return (c['exact'], measure*count, cardinality*count, -c['mirror'], -c['phase'])
        selected[part_id] = max(choices, key=score)
    # Fabric tilt is zero; every piece records its independent override.
    return 0.0, selected


def shape_outline(raw, sizes):
    points = sequence(raw)
    counts = [int(n) for n in sequence(sizes)]
    if not counts:
        return None, 'missing_subshape_sizes', 0
    if points and not hasattr(points[0], '__iter__'):
        # Zenodo v1.1 stores flat alternating x/y scalars. Its sizes count
        # scalar entries per sub-shape, not vertices (verified against raw).
        if len(points) % 2 or any(n % 2 for n in counts):
            return None, 'odd_subshape_coordinate_count', 0
        if sum(counts) != len(points):
            return None, 'inconsistent_subshape_sizes', 0
        counts = [n//2 for n in counts]
        points = list(zip(points[::2], points[1::2]))
    if sum(counts) != len(points) or min(counts) < 3:
        return None, 'inconsistent_subshape_sizes', 0
    outer = [[float(x)*0.1, float(y)*0.1] for x,y in points[:counts[0]]]
    if outer and outer[0] == outer[-1]:
        outer.pop()
    if len(outer) < 3 or any(not math.isfinite(q) for p in outer for q in p):
        return None, 'invalid_outline', 0
    area = abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(outer, outer[1:]+outer[:1]))) / 2
    if area <= 0:
        return None, 'zero_area_outline', 0
    # Retain the first ring for diagnostic jobs, but do not certify that all other
    # sub-shapes are holes: neither Zenodo nor the author's card establishes it.
    return outer, ('subshape_topology_unverified' if len(counts)>1 else None), len(counts)-1

def stratum(count, efficiency):
    size = '1-25' if count <= 25 else '26-75' if count <= 75 else '76-150' if count <= 150 else '151+'
    density = '<75' if efficiency < 75 else '75-85' if efficiency < 85 else '85+'
    return f'pieces:{size};lectra:{density}'


def stratified_sample(records, count, seed):
    # Equal allocation by round-robin, without replacement. Not population weighted.
    buckets = collections.defaultdict(list)
    for record in records:
        buckets[record['stratum']].append(record)
    rng = random.Random(seed)
    for key in sorted(buckets):
        buckets[key].sort(key=lambda r: int(r['id']))
        rng.shuffle(buckets[key])
    chosen = []
    while len(chosen) < min(count, len(records)):
        for key in sorted(buckets):
            if buckets[key] and len(chosen) < count:
                chosen.append(buckets[key].pop())
    return chosen


def compact(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False)


def convert(tables, out, seed, sample_size, reference_sample=None, reference_match=None):
    tasks, parts, constraints, shapes = (tables[k] for k in ('tasks','parts','constraints','shapes'))
    if not tasks.tasks_index.is_unique or not shapes.shape_hash.is_unique:
        raise ValueError('Duplicate task or shape join keys')
    part_groups = parts.groupby('tasks_index', sort=False).indices
    constraint_groups = constraints.groupby('tasks_index', sort=False).indices
    shape_index = {int(row.shape_hash): i for i,row in enumerate(shapes.itertuples(index=False))}
    if set(part_groups)-set(int(x) for x in tasks.tasks_index) or set(constraint_groups)-set(int(x) for x in tasks.tasks_index):
        raise ValueError('Orphan task join keys')
    # Each task gets overlapping reasons plus a stable primary reason. Small reports
    # contain ids and numbers only; sampled Job geometry stays on the data drive.
    records, reason_counts, primary_counts = [], collections.Counter(), collections.Counter()
    shape_cache, rotation_cache = {}, {}
    shape_rows = list(shapes.itertuples(index=False))
    columns = list(constraints.columns)
    rotation_fields = {'r1_start','r1_end','r1_flip_x'}
    identifiers = {'type','tasks_index','parts_1'}
    unknown_columns = set(columns)-identifiers-rotation_fields-{
        'p1_x','p1_y','parts_2','p2_x','p2_y','x_offset','y_offset','motif_order',
        'x_alignment_type','y_alignment_type','proximity_type','max_distance',
        'y_min','y_max','groups_relative_orientation','is_frozen'}
    if unknown_columns:
        raise ValueError(f'Unreviewed constraint columns: {unknown_columns}')
    extra_fields = [k for k in columns if k not in identifiers|rotation_fields]
    task_info = {}
    for t in tasks.itertuples(index=False):
        task_info[int(t.tasks_index)] = t
    # Precompute type summaries without constructing millions of row dictionaries.
    type_counts = constraints.groupby(['tasks_index','type'], sort=False).size()
    summaries = collections.defaultdict(dict)
    for (task_id, kind), count in type_counts.items():
        summaries[int(task_id)][str(kind)] = int(count)
    def evaluate(task_id, build=False):
        t = task_info[task_id]
        reasons = set()
        sheet_type = int(t.sheet_type)
        if sheet_type != 0:
            reasons.add('sheet_type_not_headline')
        width = float(t.sheet_width)*0.1
        if not math.isfinite(width) or width <= 0:
            reasons.add('invalid_width')
        efficiency = float(t.efficiency)
        if not math.isfinite(efficiency) or not 0 < efficiency <= 100:
            reasons.add('invalid_reference_efficiency')
        p = parts.iloc[part_groups.get(task_id, [])]
        if p.empty or not p.part_id.is_unique:
            reasons.add('missing_or_duplicate_parts')
        part_ids = {int(x) for x in p.part_id}
        rotations = {}
        for kind in summaries[task_id]:
            if kind != 's1':
                reasons.add('unsupported_constraint:'+kind)
        for c in constraints.iloc[constraint_groups.get(task_id, [])].itertuples(index=False):
            if c.type != 's1':
                continue
            if any(present(getattr(c,k)) for k in extra_fields):
                reasons.add('extra_parameters_on_rotation')
            ids = [int(x) for x in sequence(c.parts_1)]
            if not ids or any(x not in part_ids for x in ids):
                reasons.add('invalid_constraint_part_reference')
            key = tuple(tuple(sequence(getattr(c,k))) for k in ('r1_start','r1_end','r1_flip_x'))
            if key not in rotation_cache:
                rotation_cache[key] = orientation_candidates(*key)
            spec, error = rotation_cache[key]
            if error:
                reasons.add(error)
            for part_id in ids:
                if part_id in rotations:
                    reasons.add('multiple_rotation_constraints')
                rotations[part_id] = spec
        if set(rotations) != part_ids:
            reasons.add('missing_rotation_constraints')
        tilt, selected = choose_orientations({k:v for k,v in rotations.items() if v is not None})
        grouped = collections.Counter()
        orientation_changes = []
        exact = True
        for part_id, mapping in selected.items():
            exact = exact and mapping['exact']
            if build:
                orientation_changes.append({'part_id':part_id, **mapping,
                    'original_states':rotations[part_id]['states']})
        dropped = 0
        for part_id, shape_hash in zip(p.part_id, p.shape_hash):
            shape_hash = int(shape_hash)
            if shape_hash not in shape_index:
                reasons.add('missing_shape'); continue
            if shape_hash not in shape_cache:
                row = shape_rows[shape_index[shape_hash]]
                _, error, holes = shape_outline(row.raw, row.sizes)
                shape_cache[shape_hash] = (error, holes)
            error, holes = shape_cache[shape_hash]
            dropped += holes
            if error:
                reasons.add(error)
            mapping = selected.get(int(part_id))
            signature = (mapping['rule'],mapping['mirror'],mapping['phase'],mapping['tilt'],mapping['flip']) if mapping else ('fixed',0,0.0,0.0,False)
            grouped[(shape_hash, signature)] += 1
        job = None
        if build:
            pieces = []
            for i,((shape_hash, signature),quantity) in enumerate(sorted(grouped.items())):
                shape_row = shape_rows[shape_index[shape_hash]]
                outline, _, _ = shape_outline(shape_row.raw, shape_row.sizes)
                if outline is None:
                    continue
                rule, mirror, phase, piece_tilt, flip = signature
                radians = math.radians(phase)
                co, si = math.cos(radians), math.sin(radians)
                outline = [[co*(-x if mirror else x)-si*y, si*(-x if mirror else x)+co*y] for x,y in outline]
                pieces.append({'id':str(i),'name':str(shape_hash),'outline':outline,'grain':None,'quantity':quantity,
                               'mirror_quantity':0,'rotation':rule or 'fixed','buffer':0,'marks':[],
                               'tilt_deg':piece_tilt,'flip':flip})
            length = float(t.sheet_length)
            job = {'format':'beta-marker.job/v1','name':f'lectra-{task_id}',
                'fabric':{'width':width,'max_length':length*0.1 if math.isfinite(length) and length>0 else None,
                          'nap':'none','allow_flip':False,'tilt_deg':tilt},'spacing':{'piece_gap':0,'edge_gap':0},
                'pieces':pieces,'order':None,'meta':{'source':'Zenodo 7030786 / CC BY 4.0','sheet_type':int(t.sheet_type),
                    'diagnostic_only':bool(reasons),'subshapes_omitted':dropped,
                    'policy':'code-0 ordinary rectangular strip assumed; per-piece optional reflection and tilt',
                    'flip_axis_assumption':'flip_x means x -> -x before rotation',
                    'orientation_mapping':orientation_changes}}
        return {'id':str(task_id),'job':job,'lectra_efficiency':efficiency if math.isfinite(efficiency) else None,
            'lectra_duration_s':float(t.duration),'constraint_summary':summaries[task_id],
            'comparable':not reasons,'reason':';'.join(sorted(reasons)), 'reasons':sorted(reasons),
            'tier':'A' if exact and not dropped else 'B', 'sheet_type':sheet_type,
            'diagnostic_eligible':reasons <= {'sheet_type_not_headline'},
            'orientation_handicap':not exact,'tilt_deg':tilt,
            'dropped_mirror_parts':sum(c['dropped_mirror_state'] for c in selected.values()),
            'dropped_angle_parts':sum(c['dropped_angles_or_tilt'] for c in selected.values()),
            'piece_count':len(p),'stratum':stratum(len(p),efficiency),'reference_area_ratio':1.0,
            'subshapes_omitted':dropped}
    with (out/'eligibility.jsonl').open('w', encoding='utf-8', newline='\n') as audit_file:
        for index, task_id in enumerate(sorted(task_info)):
            record = evaluate(task_id)
            reasons = record['reasons']
            reason_counts.update(reasons)
            if reasons:
                primary_counts[reasons[0]] += 1
            audit_file.write(compact({k:v for k,v in record.items() if k != 'job'})+'\n')
            records.append(record)
            if index % 5000 == 0:
                print('classified',index,'/',len(task_info),flush=True)
    comparable = [r for r in records if r['comparable']]
    (out/'comparable-task-ids.txt').write_text(''.join(r['id']+'\n' for r in comparable), encoding='utf-8')
    def preserve(path):
        ids = [int(json.loads(line)['id']) for line in path.read_text(encoding='utf-8').splitlines() if line.strip()]
        by_id = {int(r['id']):r for r in comparable}
        if any(i not in by_id for i in ids):
            raise ValueError('A reference task lost eligibility; retain and report it explicitly')
        return [by_id[i] for i in ids]
    selected = preserve(reference_sample) if reference_sample else stratified_sample(comparable, sample_size, seed)
    excluded_ids = {r['id'] for r in selected}
    if reference_match:
        excluded_ids.update(r['id'] for r in preserve(reference_match))
    development = stratified_sample([r for r in comparable if r['id'] not in excluded_ids], 40, seed+3)
    with (out/'development-40.jsonl').open('w',encoding='utf-8',newline='\n') as stream:
        for r in development:
            stream.write(compact(evaluate(int(r['id']),build=True))+'\n')
    (out/'split-policy.json').write_text(json.dumps({'seed':seed+3,'development_ids':[r['id'] for r in development],
        'confirmation_ids':[r['id'] for r in selected], 'disjoint':True},indent=2),encoding='utf-8')
    sample_path = out/f'sample-{sample_size}.jsonl'
    with sample_path.open('w',encoding='utf-8',newline='\n') as stream:
        for r in selected:
            stream.write(compact(evaluate(int(r['id']),build=True))+'\n')
    match_sample = preserve(reference_match) if reference_match else stratified_sample(selected,60,seed)
    with (out/'sample-match-60.jsonl').open('w',encoding='utf-8',newline='\n') as stream:
        for r in match_sample:
            stream.write(compact(evaluate(int(r['id']),build=True))+'\n')
    diagnostics = []
    for code, count in ((2,20),(3,27)):
        candidates = [r for r in records if r['sheet_type'] == code and r['diagnostic_eligible']]
        chosen = stratified_sample(candidates,count,seed)
        with (out/f'diagnostic-code-{code}.jsonl').open('w',encoding='utf-8',newline='\n') as stream:
            for r in chosen:
                stream.write(compact(evaluate(int(r['id']),build=True))+'\n')
        diagnostics.extend(chosen)
    summary = {'tasks':len(records),'comparable':len(comparable),'sample_size':len(selected),'requested_sample_size':sample_size,
        'seed':seed,'tier_a':sum(r['tier']=='A' for r in comparable),
        'tier_b_including_a':len(comparable),'tier_b_only':sum(r['tier']=='B' for r in comparable),
        'excluded':len(records)-len(comparable),
        'sample_tiers':dict(collections.Counter(r['tier'] for r in selected)),
        'verified_hole_tasks':0,'multi_ring_tasks':sum(r['subshapes_omitted']>0 for r in records),
        'diagnostic_sample_size':len(diagnostics),
        'diagnostic_counts':dict(collections.Counter(str(r['sheet_type']) for r in diagnostics)),
        'diagnostic_candidate_counts':dict(collections.Counter(str(r['sheet_type']) for r in records
            if r['sheet_type'] in (2,3) and r['diagnostic_eligible'])),
        'population_handicap_tasks':{
            'optional_mirror':sum(r['dropped_mirror_parts']>0 for r in comparable),
            'angles_or_tilt':sum(r['dropped_angle_parts']>0 for r in comparable)},
        'exclusion_reasons_overlapping':dict(sorted(reason_counts.items())), 'primary_reasons':dict(sorted(primary_counts.items())),
        'constraint_types':{str(k):int(v) for k,v in constraints.type.value_counts().items()},
        'sheet_types':{str(k):int(v) for k,v in tasks.sheet_type.value_counts().items()},
        'rows':{k:len(v) for k,v in tables.items()},'sample_strata':dict(collections.Counter(r['stratum'] for r in selected)),
        'diagnostic_strata':dict(collections.Counter(r['stratum'] for r in diagnostics)),
        'notice':'ASSUMPTION: sheet_type 0 is an ordinary rectangular strip. Tier A is exact conditional on that assumption and flip_x meaning x -> -x. Tier B includes A and uses a subset of orientations; codes 2/3 remain diagnostic only.'}
    (out/'conversion-report.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
    print(json.dumps(summary,indent=2),flush=True)
    write_summary(out)

def self_test(data_dir):
    import io
    import tempfile
    temp = data_dir / 'tmp'
    temp.mkdir(parents=True, exist_ok=True)
    for payload, expected in [
        (b'cos\nsystem\n.', [('os','system')]),
        (b'\x80\x05\x8c\x08builtins\x94\x8c\x05slice\x94\x93.', [('builtins','slice')]),
    ]:
        with tempfile.TemporaryDirectory(dir=temp) as folder:
            path = Path(folder)/'test.gz'
            with gzip.open(path,'wb') as stream:
                stream.write(payload)
            assert audit(path) == expected
    try:
        RestrictedUnpickler(io.BytesIO(b'cos\nsystem\n.')).load()
    except pickle.UnpicklingError:
        pass
    else:
        raise AssertionError('Forbidden global was allowed')
    assert rotation_rule([0],[0],[0]) == ('fixed',None)
    assert rotation_rule([180,0],[180,0],[0,0]) == ('flip180',None)
    assert rotation_rule([0,90,180,270],[0,90,180,270],[0,0,0,0]) == ('quarter',None)
    assert rotation_rule([0],[360],[0]) == ('free',None)
    assert rotation_rule([0],[180],[0])[0] is None
    assert rotation_rule([0],[0],[1])[0] is None
    assert rotation_rule([],[],[])[0] is None
    def mapped(starts, ends, flips):
        spec, error = orientation_candidates(starts, ends, flips)
        assert error is None
        tilt, chosen = choose_orientations({1:spec})
        return tilt, chosen[1]
    assert mapped([180],[180],[0])[1]['phase'] == 180
    assert mapped([0,180],[0,180],[0,0])[1]['rule'] == 'flip180'
    assert mapped([0,180,0,180],[0,180,0,180],[0,0,1,1])[1]['flip']
    assert mapped([0],[360],[1])[1]['mirror'] == 1
    tilt, mapping = mapped([-2,178],[2,182],[0,0])
    assert mapping['tilt'] == 2 and mapping['rule'] == 'flip180' and mapping['exact']
    tilt, mapping = mapped([0],[180],[0])
    assert mapping['tilt'] == 90 and mapping['rule'] == 'fixed' and mapping['phase'] == 90
    assert mapped([0,120,240],[0,120,240],[0,0,0])[1]['rule'] == 'fixed'
    # Independently check inclusion for every sampled angle and handedness.
    rng = random.Random(7030786)
    for _ in range(250):
        starts = [rng.randrange(-360,360) for _ in range(rng.randrange(1,6))]
        ends = [a+rng.choice([0,2,4,90,180,360]) for a in starts]
        flips = [rng.randrange(2) for _ in starts]
        spec, error = orientation_candidates(starts,ends,flips)
        assert error is None
        tilt, chosen = choose_orientations({1:spec})
        c = chosen[1]
        tilt = c['tilt']
        angles = range(360) if c['rule']=='free' else [
            (i*360/c['n']+offset)%360
            for i in range(c['n']) for offset in (-tilt,-tilt/2,0,tilt/2,tilt)]
        for reflected in ([False,True] if c['flip'] else [False]):
            for angle in angles:
                original_angle = (angle + (-c['phase'] if reflected else c['phase']))%360
                handedness = c['mirror'] ^ reflected
                assert any(f==handedness and (abs(b-a)>=360 or (original_angle-a)%360 <= (b-a)%360+1e-7
                           or abs((original_angle-a)%360-360)<1e-7)
                           for a,b,f in zip(starts,ends,flips)), (starts,ends,flips,c,original_angle,handedness)
    # Independent tilts no longer force every piece to the smallest tolerance.
    a,_ = orientation_candidates([-2,178],[2,182],[0,0])
    b,_ = orientation_candidates([0],[0],[0])
    _, chosen = choose_orientations({1:a,2:b})
    assert chosen[1]['tilt']==2 and chosen[2]['tilt']==0
    # Reflection negates the baked phase: this pair is exactly representable.
    _, c = mapped([30,330],[30,330],[0,1])
    assert c['flip'] and c['exact']
    outline,error,holes = shape_outline([0,0,10,0,10,10,0,10],[8])
    assert error is None and holes == 0 and outline[2] == [1.0,1.0]
    assert shape_outline([0,0,10,0,0,10],[4])[1] == 'inconsistent_subshape_sizes'
    assert shape_outline([0,0,100,0,0,100,1,1,2,1,1,2],[6,6])[1] == 'subshape_topology_unverified'
    records = [{'id':str(i),'stratum':str(i%3)} for i in range(30)]
    sample = stratified_sample(records,12,7030786)
    assert sample == stratified_sample(records,12,7030786)
    assert len({r['id'] for r in sample}) == 12
    assert set(collections.Counter(r['stratum'] for r in sample).values()) == {4}
    print('Converter self-test: opcode audit, forbidden globals, rotations, units, rings, deterministic strata passed')
def write_summary(out):
    report = json.loads((out/'conversion-report.json').read_text(encoding='utf-8'))
    if 'tier_a' in report:
        lines = ['# Lectra conversion audit', '', report['notice'], '',
                 'Source: Lallier, Vezard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0.', '',
                 f"Population {report['tasks']:,}; Tier A {report['tier_a']:,}; Tier B including A {report['tier_b_including_a']:,}; excluded {report['excluded']:,}.", '',
                 f"Seed {report['seed']}. The {report['sample_size']}-task sample contains {report['sample_tiers'].get('A',0)} Tier A tasks and {report['sample_tiers'].get('B',0)} Tier B-only tasks. The diagnostic samples contain {report['diagnostic_counts'].get('2',0)} code-2 tasks and {report['diagnostic_counts'].get('3',0)} code-3 tasks.", '',
                 f"There are {report['diagnostic_candidate_counts'].get('2',0)} otherwise eligible code-2 tasks and {report['diagnostic_candidate_counts'].get('3',0)} code-3 tasks. Population handicap counts overlap: {report['population_handicap_tasks']['optional_mirror']:,} tasks drop optional mirror states; {report['population_handicap_tasks']['angles_or_tilt']:,} restrict angles or tilt.", '',
                 'Tier B pre-transforms arbitrary base angles and uses the largest regular 1/2/4-angle subset or continuous free rotation. Each piece has its own tilt; optional reflection is retained when the opposite-handed set contains the reflected retained angles. Each sampled job records the original and retained set per part. flip_x is assumed to reflect x before rotation. All other constraint classes and extra parameters remain excluded. Multiple rings remain excluded unless verified; none is silently dropped from a headline job.', '',
                 '| Sheet-type code | Tasks |', '|---|---:|']
        lines += [f'| {key} | {value:,} |' for key,value in sorted(report['sheet_types'].items())]
        lines += ['',
                 '| Exclusion reason (overlapping) | Tasks |', '|---|---:|']
        lines += [f'| {key} | {value:,} |' for key,value in report['exclusion_reasons_overlapping'].items()]
        lines += ['', '| Primary exclusion reason | Tasks |', '|---|---:|']
        lines += [f'| {key} | {value:,} |' for key,value in report['primary_reasons'].items()]
        lines += ['', '| Sample stratum | Tasks |', '|---|---:|']
        lines += [f'| {key} | {value} |' for key,value in sorted(report['sample_strata'].items())]
        lines += ['', f"Multi-ring tasks: {report['multi_ring_tasks']}; verified-hole tasks: {report['verified_hole_tasks']}.", '',
                  'All four pinned MD5 checks passed before restricted unpickling. Geometry and the complete per-task audit stay outside git. This stratified sample is unweighted, not a population estimate.', '']
        (out/'conversion-summary.md').write_text('\n'.join(lines),encoding='utf-8')
        return
    globals_used = json.loads((out/'pickle-globals.json').read_text(encoding='utf-8'))
    lines = ['# Lectra conversion audit — 18 October 2026', '',
        'Source: Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0. Downloads matched all four API MD5 checksums. No raw geometry is included here.', '',
        f"Tasks: **{report['tasks']:,}**. Strictly comparable: **{report['comparable']:,}**. Requested sample: {report['requested_sample_size']}; actual eligible sample: **{report['sample_size']}**.", '',
        f"There are {report['diagnostic_candidates']:,} mechanically convertible candidates whose only remaining blocker is unknown sheet-type semantics. The separate diagnostic sample contains {report['diagnostic_sample_size']} tasks and remains `comparable: false`. The benchmark runner rejects it. No Lectra performance number is claimed.", '',
        'No public codebook for sheet_type was found in the Zenodo description, author dataset card, or full HAL paper. A task is not treated as an exact match by assuming that code 0 means plain open-width fabric. Nonzero flip settings and unverified multi-ring topology are also excluded.', '',
        '## Rows', '', '| Table | Rows |', '|---|---:|']
    for key,value in report['rows'].items():
        lines.append(f'| {key} | {value:,} |')
    lines += ['', '## Constraint types', '', 'The class labels below are observed values. Their vendor meanings are not invented. Only s1 records with rotation fields alone are candidates for exact mapping.', '', '| Type | Records |', '|---|---:|']
    for key,value in sorted(report['constraint_types'].items()):
        lines.append(f'| {key} | {value:,} |')
    lines += ['', '## Sheet types', '', '| Code | Tasks |', '|---|---:|']
    for key,value in sorted(report['sheet_types'].items()):
        lines.append(f'| {key} | {value:,} |')
    lines += ['', '## Exclusion reasons', '', 'Counts overlap: one task can have several reasons. Every task is blocked by the unverified sheet-type meaning.', '', '| Reason | Tasks |', '|---|---:|']
    for key,value in sorted(report['exclusion_reasons_overlapping'].items()):
        lines.append(f'| {key} | {value:,} |')
    lines += ['', 'Primary reasons are the lexicographically first reason per task; their counts are mutually exclusive.', '', '| Primary reason | Tasks |', '|---|---:|']
    for key,value in sorted(report['primary_reasons'].items()):
        lines.append(f'| {key} | {value:,} |')
    lines += ['', '## Pickle globals', '', 'All GLOBAL and STACK_GLOBAL opcodes were resolved with pickletools.genops before restricted loading. Only explicit pandas/numpy reconstruction globals and builtins.slice were accepted. Python ran with -I outside the download directory.']
    for name,values in globals_used.items():
        lines += ['', f'### {name}', '']+[f'- `{value}`' for value in values]
    lines += ['', '## Sampling', '', f"Seed: {report['seed']}. Equal round-robin allocation by piece count (1–25, 26–75, 76–150, 151+) and recorded efficiency (<75%, 75% to <85%, 85%+), without replacement. This is not a weighted population estimate.", '',
        '| Diagnostic stratum | Sample tasks |', '|---|---:|']
    for key,value in sorted(report['diagnostic_strata'].items()):
        lines.append(f'| {key} | {value} |')
    lines += ['', 'The complete comparable-task id list, eligibility metadata for all tasks and sampled Job JSON remain on the data drive. Multi-ring diagnostic jobs keep the first sub-shape and record the omission; none enters a headline comparison. Dropping holes changes both feasibility and the efficiency numerator, so any future inclusion must preserve net-area accounting.', '',
        'The full paper states a nominal five-minute nesting budget using the same algorithm. Software version, hardware and worker count are unknown. The paper describes area utilization but does not settle hole accounting or all finite-sheet denominator conventions.', '']
    (out/'conversion-summary.md').write_text('\n'.join(lines),encoding='utf-8')
if __name__ == '__main__':
    main()
