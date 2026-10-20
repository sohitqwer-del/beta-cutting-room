#!/usr/bin/env python3
"""Test the ordinary-strip model against full source orientation sets.

No nesting or geometry export. Read verified parts with the restricted loader;
recover source outlines by reversing the converter's rigid transformations.
"""
import collections
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import runpy
import sys


def convex_hull(points):
    points = sorted(set(tuple(p) for p in points))
    def cross(o,a,b):
        return (a[0]-o[0])*(b[1]-o[1])-(a[1]-o[1])*(b[0]-o[0])
    lower, upper = [], []
    for p in points:
        while len(lower)>1 and cross(lower[-2],lower[-1],p)<=0:
            lower.pop()
        lower.append(p)
    for p in reversed(points):
        while len(upper)>1 and cross(upper[-2],upper[-1],p)<=0:
            upper.pop()
        upper.append(p)
    return lower[:-1]+upper[:-1]


def spans(points, angle):
    co, si = math.cos(angle), math.sin(angle)
    xs = [co*x-si*y for x,y in points]
    ys = [si*x+co*y for x,y in points]
    return max(xs)-min(xs),max(ys)-min(ys)


def minimum_length(points, states, width):
    """Exact projection bound up to floating-point tolerances.

    Support vertices change when a hull edge is horizontal or vertical.
    Between those angles, each span is A*cos(theta)+B*sin(theta), positive
    and concave. Its minimum on a feasible component is at an endpoint.
    Enumerate support changes, interval ends and width-boundary roots.
    """
    best = math.inf
    for mirror, arcs in states.items():
        polygon = convex_hull([(-x if int(mirror) else x,y) for x,y in points])
        switches = {0.0, 2*math.pi}
        for a,b in zip(polygon,polygon[1:]+polygon[:1]):
            edge = math.atan2(b[1]-a[1],b[0]-a[0])
            switches.update((step*math.pi/2-edge)%(2*math.pi) for step in range(4))
        for start,end in arcs:
            lo,hi = math.radians(start),math.radians(end)
            breaks = sorted({lo,hi,*[a for a in switches if lo<a<hi]})
            candidates = list(breaks)
            for a,b in zip(breaks,breaks[1:]):
                mid = (a+b)/2
                co,si = math.cos(mid),math.sin(mid)
                low = min(polygon,key=lambda p:si*p[0]+co*p[1])
                high = max(polygon,key=lambda p:si*p[0]+co*p[1])
                dx,dy = high[0]-low[0],high[1]-low[1]
                radius = math.hypot(dx,dy)
                if radius>0 and width<=radius:
                    phase = math.atan2(dx,dy)
                    root = math.acos(max(-1,min(1,width/radius)))
                    for sign in (-1,1):
                        angle = (phase+sign*root)%(2*math.pi)
                        if a-1e-10<=angle<=b+1e-10:
                            candidates.append(angle)
            for angle in candidates:
                xspan,yspan = spans(polygon,angle)
                if yspan<=width+1e-7:
                    best = min(best,xspan)
    return best


def self_test():
    rectangle = [[0,0],[3,0],[3,2],[0,2]]
    assert abs(minimum_length(rectangle,{'0':[[0,360]]},2)-3)<1e-7
    assert abs(minimum_length(rectangle,{'0':[[0,360]]},3)-2)<1e-7
    assert math.isinf(minimum_length(rectangle,{'0':[[0,360]]},1))
    assert abs(minimum_length(rectangle,{'1':[[0,0]]},2)-3)<1e-7
    assert math.isinf(minimum_length(rectangle,{'0':[[90,90]]},2))
    triangle = [[0,0],[4,0],[0,1]]
    # Compare analytic minima with a dense angular scan. The analytic bound
    # must be no larger, and should approach the grid result from below.
    for points in (rectangle,triangle):
        for width in (1.1,2.1,3.1,4.1):
            exact = minimum_length(points,{'0':[[0,360]]},width)
            grid = [spans(points,i*2*math.pi/36000) for i in range(36000)]
            dense = min((x for x,y in grid if y<=width),default=math.inf)
            assert exact<=dense+1e-7
            if math.isfinite(exact):
                assert abs(exact-dense)<.003


def main():
    if not sys.flags.isolated:
        raise SystemExit('Python -I is required')
    self_test()
    marker = Path(__file__).resolve().parents[2]
    converter = runpy.run_path(str(marker/'bench/tools/lectra_convert.py'))
    data = Path(os.environ['BETA_DATA_DIR'])
    folder = data/'marker-bench/lectra'
    inputs = [(0,'sample-300.jsonl'),(2,'diagnostic-code-2.jsonl'),(3,'diagnostic-code-3.jsonl')]
    groups = [(code,[json.loads(line) for line in (folder/name).read_text(encoding='utf-8').splitlines()])
              for code,name in inputs]
    source = data/'datasets/lectra-nesting-7030786/parts.gz'
    with source.open('rb') as stream:
        assert hashlib.file_digest(stream,'md5').hexdigest()==converter['CHECKSUMS']['parts.gz']
    with gzip.open(source,'rb') as stream:
        parts = converter['RestrictedUnpickler'](stream).load()
    ids = {int(record['id']) for _,records in groups for record in records}
    selected = parts.loc[parts.tasks_index.isin(ids),['tasks_index','part_id','shape_hash']]
    part_shapes = {(int(t),int(p)):int(s) for t,p,s in selected.itertuples(index=False,name=None)}
    del parts,selected
    shapes,cache = {},{}
    report = []
    for code,records in groups:
        result = []
        for record in records:
            task_id = int(record['id'])
            job = record['job']
            mappings = job['meta']['orientation_mapping']
            counts = collections.Counter()
            for c in mappings:
                shape_hash = part_shapes[task_id,c['part_id']]
                counts[(shape_hash,(c['rule'],c['mirror'],c['phase']))] += 1
            assert len(counts)==len(job['pieces'])
            for ((shape_hash,(rule,mirror,phase)),quantity),piece in zip(sorted(counts.items()),job['pieces']):
                assert piece['name']==str(shape_hash) and piece['rotation']==rule and piece['quantity']==quantity
                if shape_hash not in shapes:
                    co,si = math.cos(math.radians(phase)),math.sin(math.radians(phase))
                    shapes[shape_hash] = [[(-1 if mirror else 1)*(co*x+si*y),-si*x+co*y]
                                          for x,y in piece['outline']]
            bound = 0
            cannot_fit = 0
            required_width_ratio = 0
            for c in mappings:
                shape_hash = part_shapes[task_id,c['part_id']]
                key = (shape_hash,job['fabric']['width'],json.dumps(c['original_states'],sort_keys=True))
                if key not in cache:
                    cache[key] = minimum_length(shapes[shape_hash],c['original_states'],job['fabric']['width'])
                minimum = cache[key]
                if not math.isfinite(minimum):
                    cannot_fit += 1
                    # A quarter-turn exchanges projection spans. For mirrored
                    # shapes its sign reverses, which leaves the span unchanged.
                    required = minimum_length([[y,-x] for x,y in shapes[shape_hash]],
                                              c['original_states'],math.inf)
                    required_width_ratio = max(required_width_ratio,required/job['fabric']['width'])
                else:
                    bound = max(bound,minimum)
            def area(points):
                return abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(points,points[1:]+points[:1])))/2
            total_area = sum(area(p['outline'])*p['quantity'] for p in job['pieces'])*record['reference_area_ratio']
            implied = total_area/(job['fabric']['width']*record['lectra_efficiency']/100)
            result.append({'id':record['id'],'width_infeasible_parts':cannot_fit,
                           'minimum_required_width_over_stated':required_width_ratio if cannot_fit else None,
                           'reference_length_below_bound':not cannot_fit and implied<bound*(1-1e-6),
                           'reference_efficiency_above_bound_pp':record['lectra_efficiency']*(1-implied/bound) if bound and not cannot_fit and implied<bound else 0.0,
                           'reference_length_over_bound':implied/bound if bound else None})
        ratios = [r['reference_length_over_bound'] for r in result if not r['width_infeasible_parts'] and r['reference_length_over_bound']]
        report.append({'sheet_type':code,'tasks':len(result),
                       'width_infeasible_tasks':sum(r['width_infeasible_parts']>0 for r in result),
                       'reference_length_below_bound':sum(r['reference_length_below_bound'] for r in result),
                       'minimum_ratio':min(ratios,default=None),'details':result})
        print(code,'checked',len(result),'tasks',flush=True)
    output = {'method':'Full source rotation/mirror intervals; analytic width-constrained projection bound. No placement search. Quantity and inverse-transform joins checked. Tolerances 1e-7 mm, reference ratio 1e-6.',
              'assumption':'Test ordinary rectangular strip at stated sheet_width; flip_x reflects x before rotation.',
              'results':report}
    (folder/'geometry-check-round2.json').write_text(json.dumps(output,indent=2),encoding='utf-8')
    print(json.dumps({str(r['sheet_type']):{k:v for k,v in r.items() if k!='details'} for r in report},indent=2))


if __name__=='__main__':
    main()
