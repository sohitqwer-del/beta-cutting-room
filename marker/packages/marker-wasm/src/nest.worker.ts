import { installNestWorker, type NestWorkerPort } from './nest-worker';
installNestWorker(globalThis as unknown as NestWorkerPort);
