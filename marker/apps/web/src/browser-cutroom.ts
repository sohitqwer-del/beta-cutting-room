/** Local cutting-room routes for the static browser edition. Rust WASM answers
 * every route; nothing leaves the device. Null means another transport owns the route. */
import { createCutPlanner, loadCutRoomCore, nest, type BrowserNestOptions, type CutPlan, type CutPlanner,
  type CutRoomCore, type Job, type MarkerResult, type NestOptions, type PlanRequest } from '@beta/marker-wasm';

/** Routes answered by the lazily loaded cutting-room module. */
const MODULE_ROUTES = new Set(['/api/roll/inspect', '/api/roll/spread', '/api/cutroom/lay', '/api/cutroom/from-spread',
  '/api/cutroom/bundles', '/api/cutroom/tickets', '/api/cutroom/stickers', '/api/cutroom/cost', '/api/cutroom/recut',
  '/api/cutroom/report', '/api/cutroom/report-marker', '/api/validate']);
/** Factory formats stay in the marker module (browser-export.ts); these report formats come from here. */
const REPORT_FORMATS = new Set(['report', 'xlsx', 'mini', 'labels']);
/** Sent while a browser cut plan runs: `detail` is the latest checked partial plan. */
export const PLAN_PROGRESS_EVENT = 'beta-plan-progress';

export interface CutRoomDeps {
  core(): Promise<CutRoomCore>;
  plan(request: PlanRequest, onProgress: (plan: CutPlan) => void): Promise<CutPlan>;
  nest(job: Job, options: BrowserNestOptions): Promise<MarkerResult>;
}
let core: Promise<CutRoomCore> | undefined;
let planner: CutPlanner | undefined;
const defaults: CutRoomDeps = {
  core: () => (core ??= loadCutRoomCore().catch(error => { core = undefined; throw error; })),
  async plan(request, onProgress) {
    // One Worker per plan: a stopped plan terminates its Worker.
    planner = createCutPlanner();
    try { return await planner.plan(request, onProgress); } finally { planner.dispose(); planner = undefined; }
  },
  nest: (job, options) => nest(job, options),
};
/** Stop a running browser cut plan; it resolves with the best checked partial plan. */
export function stopBrowserPlan(): boolean { if (!planner) return false; planner.stop(); return true; }

const json = (value: unknown, status = 200): Response =>
  new Response(JSON.stringify(value), { status, headers: { 'Content-Type': 'application/json' } });
const failure = (error: unknown): Response => json({ error: String(error instanceof Error ? error.message : error) }, 400);

export async function browserCutRoom(url: string, init?: RequestInit, deps: CutRoomDeps = defaults): Promise<Response | null> {
  if ((init?.method ?? 'GET').toUpperCase() !== 'POST') return null;
  const route = new URL(url, 'http://beta.local');
  const path = route.pathname, format = route.searchParams.get('format');
  try {
    if (path === '/api/plan') {
      const request = await new Response(init?.body).json() as PlanRequest;
      const plan = await deps.plan(request, partial => {
        if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent(PLAN_PROGRESS_EVENT, { detail: partial }));
      });
      return json(plan);
    }
    if (path === '/api/nest') {
      // Compare re-nests with the browser nester: one thread, same garment rules.
      const { job, options } = await new Response(init?.body).json() as { job: Job; options: NestOptions };
      return json(await deps.nest(job, { time_limit_s: options.time_limit_s, seed: options.seed ?? null,
        early_termination: options.early_termination, fixed: options.fixed, warm_start: options.warm_start }));
    }
    const local = MODULE_ROUTES.has(path) || (path === '/api/import' && format === 'plt')
      || (path === '/api/export' && REPORT_FORMATS.has((format ?? '').toLowerCase()));
    if (!local) return null;
    const body = new Uint8Array(await new Response(init?.body).arrayBuffer());
    const reply = (await deps.core()).request(`${path}${route.search}`, body);
    const headers: Record<string, string> = { 'Content-Type': reply.mime };
    if (reply.extension) headers['X-Beta-Extension'] = reply.extension;
    return new Response(new Uint8Array(reply.body), { status: reply.status, headers });
  } catch (error) {
    return failure(error);
  }
}
