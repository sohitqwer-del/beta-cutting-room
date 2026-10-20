"""Train offline; export small numerical trees for independent Rust inference.
No pickle model is written. Supplied split flags are never reassigned.
"""
import argparse
import copy
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import subprocess
import sys
import time

# Respect the shared laptop even when this script is called without environment setup.
os.environ.setdefault('OMP_NUM_THREADS', '2')
os.environ.setdefault('OPENBLAS_NUM_THREADS', '1')
os.environ.setdefault('MKL_NUM_THREADS', '1')
import numpy as np
import sklearn
from sklearn.ensemble import HistGradientBoostingRegressor
from sklearn.linear_model import Ridge
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import mean_absolute_error, mean_squared_error, r2_score
from scipy.stats import spearmanr
from threadpoolctl import threadpool_limits


def metrics(y, pred):
    error = np.abs(y-pred)
    return {'n':len(y), 'mae_pp':float(mean_absolute_error(y,pred)*100),
            'rmse_pp':float(np.sqrt(mean_squared_error(y,pred))*100), 'r2':float(r2_score(y,pred)),
            'bias_pp':float(np.mean(pred-y)*100), 'p90_absolute_error_pp':float(np.quantile(error,.9)*100),
            'length_mape_pct':float(np.mean(np.abs(y/pred-1))*100)}


def dump(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False)+'\n', encoding='utf-8')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--features',type=Path,required=True)
    p.add_argument('--executable',type=Path,required=True)
    p.add_argument('--out',type=Path,required=True)
    args=p.parse_args()
    if sklearn.__version__!='1.7.2':
        raise ValueError('Tree export is verified against scikit-learn 1.7.2 only')
    args.out.mkdir(parents=True,exist_ok=True)
    rows=[json.loads(line) for line in args.features.open(encoding='utf-8')]
    x=np.array([r['features']['values'] for r in rows],dtype=np.float64)
    y=np.array([r['target'] for r in rows])
    ids=np.array([r['id'] for r in rows])
    split=np.array([r['split'] for r in rows])
    names=json.loads(subprocess.check_output([str(args.executable),'schema'],text=True))
    assert len(rows)==100000 and len(set(ids))==100000 and x.shape==(100000,len(names))
    assert np.isfinite(x).all() and np.isfinite(y).all() and np.all((y>0)&(y<=1))
    train=split=='train'; val=split=='val'; test=split=='test'
    tune=val & (ids%2==0); calibrate=val & (ids%2==1)
    assert np.all(train|val|test) and min(sum(train),sum(tune),sum(calibrate),sum(test))>0
    report={'feature_schema':names,'counts':{k:int(v.sum()) for k,v in [('train',train),('val',val),('val_selection',tune),('val_calibration',calibrate),('test',test)]},
            'versions':{k:importlib.metadata.version(k) for k in ['numpy','pandas','scikit-learn','scipy','joblib','threadpoolctl','cloudpickle']},
            'features_sha256':hashlib.sha256(args.features.read_bytes()).hexdigest(),
            'selection_rule':'Lowest validation-selection MAE; no test access until selection is frozen.',
            'candidates':[]}
    start=time.perf_counter()
    baseline_mean=float(y[train].mean()); baseline_median=float(np.median(y[train]))
    report['baseline_mean']=baseline_mean; report['baseline_median']=baseline_median
    for alpha in [1.,100.]:
        ridge=make_pipeline(StandardScaler(),Ridge(alpha=alpha))
        ridge.fit(x[train],y[train]); pred=np.clip(ridge.predict(x[tune]),.01,1)
        report['candidates'].append({'kind':'ridge','alpha':alpha,'selection':metrics(y[tune],pred)})
    best_score=float('inf'); best=None; best_config=None
    # Warm-start checkpoints use train data only. Selection never uses the test split.
    for leaves in [15,31,63]:
        model=HistGradientBoostingRegressor(loss='squared_error',learning_rate=.06,
            max_leaf_nodes=leaves,min_samples_leaf=25,l2_regularization=2.,early_stopping=False,
            warm_start=True,random_state=7030786)
        for iterations in [150,350,600]:
            model.set_params(max_iter=iterations)
            model.fit(x[train],y[train])
            score=metrics(y[tune],np.clip(model.predict(x[tune]),.01,1))
            config={'kind':'hist_gradient_boosting','leaves':leaves,'iterations':iterations,
                    'learning_rate':.06,'min_samples_leaf':25,'l2_regularization':2.}
            report['candidates'].append(dict(config,selection=score))
            print(config,score,flush=True)
            if score['mae_pp']<best_score:
                best_score=score['mae_pp']; best_config=config
                # Capture numbers now; later warm-start fitting must not change the winner.
                trees=[]
                for predictors in model._predictors:
                    assert len(predictors)==1
                    nodes=[]
                    for node in predictors[0].nodes:
                        assert not node['is_categorical']
                        if node['is_leaf']:
                            nodes.append([-1,0.,0,0,float(node['value'])])
                        else:
                            nodes.append([int(node['feature_idx']),float(node['num_threshold']),
                                          int(node['left']),int(node['right']),0.])
                    trees.append(nodes)
                best={'format':'beta-marker-learn/1','feature_names':names,
                      'intercept':float(model._baseline_prediction[0,0]),'trees':trees,
                      'residual_q90':0.,'feature_min':x[train].min(axis=0).tolist(),
                      'feature_max':x[train].max(axis=0).tolist(),
                      'provenance':'Lallier, Vezard, Pinaud and Blin (2022), Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation, Zenodo 7030786 v1.1, https://doi.org/10.5281/zenodo.7030786. Trained only on supplied is_train; selected on even-id is_val; interval calibrated on odd-id is_val. Target is recorded Lectra efficiency, not a ceiling. Model '+json.dumps(config,sort_keys=True),
                      'license':'CC-BY-4.0, https://creativecommons.org/licenses/by/4.0/'}
                best_model=copy.deepcopy(model)
    # Freeze winner, then calculate calibration and held-out measurements.
    ridge_best = min(r['selection']['mae_pp'] for r in report['candidates'] if r['kind']=='ridge')
    if best_score >= ridge_best:
        raise ValueError('Trees did not beat the ridge baseline; do not promote this model')
    best_predictions=np.clip(best_model.predict(x),.01,1)
    report['selected']=best_config
    residuals=np.sort(np.abs(y[calibrate]-best_predictions[calibrate]))
    rank=min(len(residuals)-1,int(np.ceil((len(residuals)+1)*.90))-1)
    best['residual_q90']=float(residuals[rank])
    model_path=args.out/'lectra-v1.json'
    model_path.write_text(json.dumps(best,separators=(',',':'),allow_nan=False)+'\n',encoding='utf-8')
    assert model_path.stat().st_size<5000000
    report['model_bytes']=model_path.stat().st_size
    report['model_sha256']=hashlib.sha256(model_path.read_bytes()).hexdigest()
    report['training_seconds']=time.perf_counter()-start
    report['residual_q90_pp']=best['residual_q90']*100
    report['metrics']={}
    for name,mask in [('train',train),('val',val),('val_selection',tune),('val_calibration',calibrate),('test',test),('all',np.ones(len(y),dtype=bool))]:
        m=metrics(y[mask],best_predictions[mask]);m['interval_coverage']=float(np.mean(np.abs(y[mask]-best_predictions[mask])<=best['residual_q90']))
        report['metrics'][name]=m
    report['test_baselines']={k:metrics(y[test],np.full(test.sum(),v)) for k,v in [('train_mean',baseline_mean),('train_median',baseline_median),('planner_75_percent',.75)]}
    count=np.array([r['features']['piece_count'] for r in rows])
    seen=np.array([r['seen_shape_fraction'] for r in rows])
    report['test_subgroups']={}
    for name,mask in [('pieces_1_25',count<=25),('pieces_26_75',(count>25)&(count<=75)),('pieces_76_150',(count>75)&(count<=150)),('pieces_151_plus',count>150),
                      ('sheet_0',(x[:,7]==0)&(x[:,8]==0)),('sheet_2',x[:,7]==1),('sheet_3',x[:,8]==1),
                      ('extra_constraints',x[:,9]>0),('no_extra_constraints',x[:,9]==0),
                      ('all_shapes_unseen',seen==0),('some_shapes_unseen',(seen>0)&(seen<1)),('all_shapes_seen',seen==1)]:
        mask=mask&test
        if mask.sum()>1:
            report['test_subgroups'][name]=metrics(y[mask],best_predictions[mask])
    report['shape_overlap']={'test_mean_physical_fraction_seen_in_train':float(seen[test].mean()),
                             'test_tasks_all_shapes_seen':int((test&(seen==1)).sum()),
                             'test_tasks_all_shapes_unseen':int((test&(seen==0)).sum())}
    train_vectors={hashlib.sha256(row.tobytes()).digest() for row in x[train]}
    report['test_exact_feature_duplicates_in_train']=sum(hashlib.sha256(row.tobytes()).digest() in train_vectors for row in x[test])
    report['test_spearman_efficiency']=float(spearmanr(y[test],best_predictions[test]).statistic)
    rng=np.random.default_rng(7030786)
    indices=np.flatnonzero(test); a,b=rng.choice(indices,size=(2,20000),replace=True)
    keep=np.abs(y[a]-y[b])>=.01
    report['random_test_pair_ranking']={'eligible_pairs':int(keep.sum()),'minimum_target_gap_pp':1.,
        'accuracy':float(np.mean(np.sign(y[a[keep]]-y[b[keep]])==np.sign(best_predictions[a[keep]]-best_predictions[b[keep]]))),
        'note':'Independent marker efficiency ranking; not a cut-order candidate comparison.'}
    # All-row parity verifies export thresholds and Rust traversal, not just golden examples.
    rust_path=args.out/'rust-predictions.jsonl'
    completed=subprocess.run([str(args.executable),'evaluate',str(model_path),str(args.features),str(rust_path)],check=True,capture_output=True,text=True)
    print(completed.stderr,flush=True)
    rust=[json.loads(line) for line in rust_path.open()]
    assert [r['id'] for r in rust]==ids.tolist()
    difference=np.abs(np.array([r['prediction'] for r in rust])-best_predictions)
    report['rust_parity_max_absolute_error']=float(difference.max())
    report['rust_timing']=completed.stderr.strip()
    assert difference.max()<1e-12
    golden=[]
    for i in indices[np.linspace(0,len(indices)-1,12,dtype=int)]:
        golden.append({'features':rows[i]['features'],'prediction':float(best_predictions[i])})
    dump(args.out/'golden.json',golden)
    dump(args.out/'metrics.json',report)
    print(json.dumps({'selected':best_config,'test':report['metrics']['test'],'bytes':report['model_bytes']},indent=2),flush=True)


if __name__=='__main__':
    with threadpool_limits(limits=2):
        main()
