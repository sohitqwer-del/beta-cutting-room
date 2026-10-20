"""Restricted, checksum-verified Lectra transport. Geometry stays under BETA_DATA_DIR.
Run with Python -I. Feature mathematics and inference are implemented in Rust.
"""
import argparse
import gc
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import time


def main():
    if not sys.flags.isolated:
        raise SystemExit('Python -I is required')
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--data-dir', type=Path, default=Path(os.environ['BETA_DATA_DIR']) if 'BETA_DATA_DIR' in os.environ else None)
    p.add_argument('--stage', choices=['shapes', 'tasks'], required=True)
    args = p.parse_args()
    if args.data_dir is None:
        p.error('set BETA_DATA_DIR or pass --data-dir')
    args.out.mkdir(parents=True, exist_ok=True)
    converter = Path(__file__).resolve().parents[3] / 'bench/tools/lectra_convert.py'
    spec = importlib.util.spec_from_file_location('lectra_convert', converter)
    conv = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(conv)
    folder = args.data_dir / 'datasets/lectra-nesting-7030786'
    audit_path = args.data_dir / 'marker-bench/lectra/pickle-globals.json'
    audits = json.loads(audit_path.read_text())

    def load(name):
        path = folder / (name + '.gz')
        with path.open('rb') as f:
            if hashlib.file_digest(f, 'md5').hexdigest() != conv.CHECKSUMS[path.name]:
                raise ValueError('Checksum mismatch: ' + name)
        used = {tuple(s.rsplit('.', 1)) for s in audits[path.name]}
        if used - conv.ALLOWED:
            raise ValueError('Unreviewed pickle global')
        print('loading', name, flush=True)
        with gzip.open(path, 'rb') as f:
            return conv.RestrictedUnpickler(f).load()

    def dump(f, value):
        f.write(json.dumps(value, separators=(',', ':'), allow_nan=False) + '\n')

    if args.stage == 'shapes':
        shapes = load('shapes')
        with (args.out / 'shapes.jsonl').open('x', encoding='utf-8') as f:
            for i, row in enumerate(shapes.itertuples(index=False)):
                outline, error, omitted = conv.shape_outline(row.raw, row.sizes)
                if error or omitted:
                    raise ValueError(f'Shape {row.shape_hash}: {error}')
                dump(f, {'id': int(row.shape_hash), 'outline': outline})
                if i % 50000 == 0:
                    print('shapes', i, flush=True)
        return

    import numpy as np
    import pandas as pd
    tasks = load('tasks')
    assert tasks.tasks_index.is_unique and len(tasks) == 100000
    flags = tasks[['is_train','is_val','is_test']].to_numpy(dtype=int)
    assert np.all(flags.sum(axis=1) == 1)
    parts = load('parts')
    assert not parts.duplicated(['tasks_index','part_id']).any()
    assert set(parts.tasks_index) <= set(tasks.tasks_index)
    constraints = load('constraints')
    assert set(constraints.tasks_index) <= set(tasks.tasks_index)
    extra = constraints[constraints.type != 's1'].groupby('tasks_index').size().to_dict()
    c = constraints.loc[constraints.type == 's1', ['tasks_index','parts_1','r1_start','r1_end','r1_flip_x']]
    del constraints
    gc.collect()
    # Deduplicate identical orientation specifications without changing their meaning.
    profiles, ids, part_ids, task_ids = {}, np.empty(len(c), dtype=np.int32), np.empty(len(c), dtype=np.int64), np.empty(len(c), dtype=np.int64)
    with (args.out / 'rotations.jsonl').open('x', encoding='utf-8') as f:
        for i, row in enumerate(c.itertuples(index=False)):
            if len(row.parts_1) != 1:
                raise ValueError('s1 must reference one physical part')
            key = (tuple(row.r1_start), tuple(row.r1_end), tuple(row.r1_flip_x))
            if key not in profiles:
                rid = len(profiles)
                profiles[key] = rid
                dump(f, {'id': rid, 'intervals': [[float(a),float(b),int(m)] for a,b,m in zip(*key)]})
                assert len(key[0]) == len(key[1]) == len(key[2]) > 0
            ids[i], part_ids[i], task_ids[i] = profiles[key], row.parts_1[0], row.tasks_index
            if i % 1000000 == 0:
                print('rotations', i, flush=True)
    del c
    gc.collect()
    rotation = pd.DataFrame({'part_id':part_ids, 'tasks_index':task_ids, 'rotation':ids})
    parts = parts.merge(rotation, on=['part_id','tasks_index'], how='left', validate='one_to_one')
    assert len(parts) == 7360718 and parts.rotation.notna().all() and len(rotation) == len(parts)
    del rotation, ids, part_ids, task_ids
    gc.collect()
    grouped = parts.groupby(['tasks_index','shape_hash','rotation'], sort=True).size().rename('count').reset_index()
    train_ids = set(tasks.loc[tasks.is_train, 'tasks_index'])
    train_shapes = set(parts.loc[parts.tasks_index.isin(train_ids), 'shape_hash'])
    del parts
    gc.collect()
    by_task = grouped.groupby('tasks_index', sort=False)
    assert set(by_task.indices) == set(tasks.tasks_index)
    with (args.out / 'tasks.jsonl').open('x', encoding='utf-8') as f:
        for row in tasks.itertuples(index=False):
            g = by_task.get_group(row.tasks_index)
            pieces = [[int(r.shape_hash),int(r.count),int(r.rotation)] for r in g.itertuples(index=False)]
            split = 'train' if row.is_train else 'val' if row.is_val else 'test'
            n = sum(x[1] for x in pieces)
            dump(f, {'id':int(row.tasks_index), 'split':split, 'target':float(row.efficiency)/100,
                     'width':float(row.sheet_width)*0.1, 'sheet_code':int(row.sheet_type),
                     'extra_constraints':int(extra.get(row.tasks_index,0)), 'pieces':pieces,
                     'seen_shape_fraction':sum(x[1] for x in pieces if x[0] in train_shapes)/n})
    print('tasks complete', len(tasks), 'profiles', len(profiles), flush=True)


if __name__ == '__main__':
    main()
