import { describe, expect, it } from 'vitest';
import fixtures from './fixtures/transform.json';
import contract from './fixtures/contract.json';
import { bbox, transform } from '../src/geometry';
import type { Pt, Placement } from '../src/types';
import type { CutOrder, CutPlan, Job, Layout, MarkerResult, MatchAxes, Nap, NestOptions, PlanOptions, ShadeAxis } from '../src/types';
describe('drawing matches the shared Rust contract', () => {
  it('keeps separate layer bundles and explicit surplus in the wire contract', () => {
    const placements: Placement[] = contract.layer_allocation_examples.map(p => ({ ...p, translate: p.translate as Pt }));
    expect(placements[0]!.layer_allocations?.map(a => a?.bundle)).toEqual([0, 1]);
    expect(placements[1]!.layer_allocations?.[1]).toBeNull();
  });
  for (const [index, fixture] of fixtures.entries()) it(`transform fixture ${index}`, () => {
    const actual = transform(fixture.points as Pt[], fixture.mirrored, fixture.rotation_deg, fixture.translate as Pt);
    expect(actual).toHaveLength(fixture.expected.length);
    actual.forEach((point, i) => point.forEach((value, axis) => expect(value).toBeCloseTo(fixture.expected[i]![axis]!, 9)));
  });
  it('bounds handles negative and empty geometry', () => {
    expect(bbox([])).toBeNull();
    expect(bbox([[-2, 10], [5, -1]])).toEqual([-2, -1, 5, 10]);
  });
  it('reads the same job/result fixture as the Rust JSON API', () => {
    // Narrow only enum literals; the rest must satisfy the complete TS wire types.
    const job: Job = { ...contract.job, fabric: { ...contract.job.fabric, nap: 'none' },
      pieces: contract.job.pieces.map(p => ({ ...p, outline: p.outline as Pt[], grain: p.grain as [Pt, Pt], rotation: 'inherit',
        marks: p.marks.map(m => ({ ...m, kind: m.kind as 'notch' | 'drill' | 'line' | 'text', points: m.points as Pt[] })) })) };
    const result: MarkerResult = { ...contract.result, placements: contract.result.placements.map(p => ({ ...p, translate: p.translate as Pt })) };
    expect(job.pieces[0]!.mirror_quantity).toBe(1);
    expect(result.placements[0]!.translate).toEqual([1, 1]);
  });
  it('typechecks every v0.2 field against the shared Rust fixture', () => {
    const v = contract.v02;
    const job: Job = { ...v.job, fabric: { ...v.job.fabric, nap: 'one_way_per_bundle', layout: 'folded',
      shade: { ...v.job.fabric.shade, axis: 'length' }, defects: v.job.fabric.defects.map(z => ({ ...z, polygon: z.polygon as Pt[] })) },
      pieces: v.job.pieces.map(p => ({ ...p, rotation: 'inherit', outline: p.outline as Pt[], grain: p.grain as [Pt, Pt], fold_line: p.fold_line as [Pt, Pt],
        match_points: p.match_points.map(m => ({ ...m, point: m.point as Pt })),
        marks: p.marks.map(m => ({ ...m, kind: m.kind as 'notch' | 'drill' | 'line' | 'text', points: m.points as Pt[] })) })),
      match_rules: v.job.match_rules.map(r => ({ ...r, axes: 'both', offset: r.offset as Pt })) };
    const result: MarkerResult = { ...v.result, placements: v.result.placements.map(p => ({ ...p, translate: p.translate as Pt })) };
    const options: NestOptions = { ...v.options, fixed: result.placements, warm_start: result };
    const order: CutOrder = { ...v.order, job };
    const planOptions: PlanOptions = v.plan_options;
    const plan: CutPlan = { ...v.plan, lays: v.plan.lays.map(l => ({ ...l, result })) };
    const enum_values = { layouts: ['open', 'folded', 'tubular'] satisfies Layout[], naps: ['none', 'one_way', 'one_way_per_bundle'] satisfies Nap[],
      shade_axes: ['length', 'width'] satisfies ShadeAxis[], match_axes: ['x', 'y', 'both'] satisfies MatchAxes[] };
    expect({ job, result, options, order, plan_options: planOptions, plan, enum_values }).toEqual(v);
  });
});
