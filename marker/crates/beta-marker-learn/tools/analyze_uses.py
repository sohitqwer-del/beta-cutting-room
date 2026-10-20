"""Offline evaluation of possible planner and time-allocation uses.
These diagnostics do not alter the fitted predictor or product search defaults.
"""
import argparse
import csv
import hashlib
import json
from collections import defaultdict
from pathlib import Path
import numpy as np
from scipy.stats import spearmanr


def csv_rows(path):
    with path.open(encoding='utf-8-sig',newline='') as f:
        return {int(r['id']):r for r in csv.DictReader(f)}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--evidence',type=Path,required=True)
    p.add_argument('--bench-root',type=Path,required=True)
    args=p.parse_args()
    rows={r['id']:r for r in (json.loads(line) for line in (args.evidence/'features.jsonl').open())}
    pred={r['id']:r['prediction'] for r in (json.loads(line) for line in (args.evidence/'training/rust-predictions.jsonl').open())}
    report={}
    source_paths=[]
    # Identical demanded shape counts and orientation specifications, varying width.
    # No garment identities are guessed, and non-rotation constraints are excluded.
    groups=defaultdict(list)
    for line in (args.evidence/'tasks.jsonl').open():
        r=json.loads(line)
        if r['split']=='test' and r['sheet_code']==0 and r['extra_constraints']==0:
            signature=tuple(tuple(p) for p in r['pieces'])
            groups[signature].append(r['id'])
    pairs=[]
    for group in groups.values():
        group=sorted(group)
        for a,b in zip(group,group[1:]):
            fa,fb=rows[a]['features'],rows[b]['features']
            if fa['width_mm']==fb['width_mm']:
                continue
            true_a=fa['area_mm2']/fa['width_mm']/rows[a]['target']
            true_b=fb['area_mm2']/fb['width_mm']/rows[b]['target']
            est_a=fa['area_mm2']/fa['width_mm']/pred[a]
            est_b=fb['area_mm2']/fb['width_mm']/pred[b]
            constant_a=fa['area_mm2']/fa['width_mm']
            constant_b=fb['area_mm2']/fb['width_mm']
            if abs(true_a/true_b-1)>.01:
                pairs.append({'ids':[a,b], 'learned_correct':bool((est_a<est_b)==(true_a<true_b)),
                              'constant_correct':bool((constant_a<constant_b)==(true_a<true_b))})
    report['same_demand_width_ranking']={'pairs':len(pairs),
        'learned_accuracy':float(np.mean([r['learned_correct'] for r in pairs])) if pairs else None,
        'constant_accuracy':float(np.mean([r['constant_correct'] for r in pairs])) if pairs else None,
        'note':'Adjacent ID pairs within identical shape/count/orientation groups; only test rows, code 0, no extra constraints, different width, implied reference length gap >1%. No independence claim across pairs.'}
    # Existing matched 60s / longer experiments: retrospective, no new solver load.
    source_paths += [args.bench_root/'results/2026-10-18-lectra-round2-60s/results.csv',
                     args.bench_root/'results/2026-10-18-lectra-round2-match/results.csv']
    short,long=map(csv_rows,source_paths)
    selected=[]
    for i in sorted(short.keys()&long.keys()):
        a,b=short[i],long[i]
        if rows[i]['split']!='test' or a['valid']!='true' or b['valid']!='true':continue
        gap=pred[i]*100-float(a['our_efficiency'])
        gain=float(b['our_efficiency'])-float(a['our_efficiency'])
        selected.append({'id':i,'predicted_gap_pp':gap,'longer_gain_pp':gain})
    near=[r for r in selected if r['predicted_gap_pp']<=1.0]
    report['stopping_diagnostic']={'heldout_pairs':len(selected),
        'spearman_gap_vs_longer_gain':float(spearmanr([r['predicted_gap_pp'] for r in selected],[r['longer_gain_pp'] for r in selected]).statistic) if len(selected)>2 else None,
        'within_one_pp_of_prediction':len(near),
        'near_prediction_but_longer_gain_above_one_pp':sum(r['longer_gain_pp']>1.0 for r in near),
        'rows':selected,'decision':'Do not enable learned stopping: reference efficiency is not a bound; these are separate stochastic runs, not continuation curves.'}
    # Small historical paired strategy dataset. Fit only a single decision stump per arm.
    # Apply the same selected stump to held-out IDs. Keep only if test lower 95% bootstrap >0.
    arms=['compression30','compression40','sequential3','race3','samples-light']
    policies=[]
    for arm in arms:
        root=args.bench_root/'quality-v03'/('paired-'+arm)/'effective'
        source_paths += [root/'baseline/results.csv',root/'variant/results.csv']
        a,b=csv_rows(root/'baseline/results.csv'),csv_rows(root/'variant/results.csv')
        invalid=[i for i in sorted(a.keys()&b.keys()) if a[i]['valid']!='true' or b[i]['valid']!='true']
        if set(invalid)-{12127}:
            policies.append({'arm':arm,'invalid_pair_ids':invalid,'decision':'Reject; unexpected invalid pairs'})
            continue
        records=[]
        for i in sorted(a.keys()&b.keys()):
            if a[i]['valid']!='true' or b[i]['valid']!='true':continue
            records.append((i,[rows[i]['features']['piece_count'],pred[i]*100],float(b[i]['our_efficiency'])-float(a[i]['our_efficiency'])))
        train=[r for r in records if rows[r[0]]['split']=='train']
        val=[r for r in records if rows[r[0]]['split']=='val']
        test=[r for r in records if rows[r[0]]['split']=='test']
        if not train or not val or not test:
            policies.append({'arm':arm,'counts':[len(train),len(val),len(test)],'decision':'insufficient supplied split labels'})
            continue
        best=(0.,None)
        for f in range(2):
            for threshold in sorted(set(r[1][f] for r in train)):
                for direction in [-1,1]:
                    chosen=[r[2] if direction*(r[1][f]-threshold)>=0 else 0. for r in train]
                    score=float(np.mean(chosen))
                    if score>best[0]:best=(score,(f,threshold,direction))
        rule=best[1]
        def gains(data):
            return np.array([r[2] if rule is not None and rule[2]*(r[1][rule[0]]-rule[1])>=0 else 0. for r in data])
        vg=gains(val)
        policies.append({'arm':arm,'counts':[len(train),len(val),len(test)],'rule':rule,'invalid_pair_ids':invalid,
                         'train_mean_gain_pp':best[0],'validation_mean_gain_pp':float(vg.mean()),
                         '_test_gains':gains(test).tolist()})
    eligible=[r for r in policies if r.get('validation_mean_gain_pp',0)>0 and r.get('rule') is not None]
    winner=max(eligible,key=lambda r:r['validation_mean_gain_pp']) if eligible else None
    if winner:
        gains=np.array(winner['_test_gains']);rng=np.random.default_rng(7030786)
        boot=rng.choice(gains,size=(10000,len(gains)),replace=True).mean(axis=1)
        ci=np.quantile(boot,[.025,.975]).tolist()
        report['strategy_selection']={'arm':winner['arm'],'test_pairs':len(gains),
            'mean_gain_pp':float(gains.mean()),'bootstrap95_pp':ci,
            'decision':'Research signal only; require fresh paired confirmation' if ci[0]>0 else 'Reject; retain existing default'}
    else:
        report['strategy_selection']={'decision':'Reject; no policy beat zero gain on supplied validation tasks'}
    for r in policies:r.pop('_test_gains',None)
    report['strategy_candidates']=policies
    report['source_sha256']={str(path.relative_to(args.bench_root)).replace('\\','/'):hashlib.sha256(path.read_bytes()).hexdigest() for path in source_paths}
    report['limitations']='Historical development tasks were stratified by target, host was shared, five arms use separate paired baselines, small split counts, no new nesting run or independent factory validation. Do not treat these as 100k strategy labels.'
    (args.evidence/'training/use-diagnostics.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    main()
