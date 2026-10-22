import { installCutPlanWorker, type CutPlanWorkerPort } from './cutroom';
installCutPlanWorker(globalThis as unknown as CutPlanWorkerPort);
