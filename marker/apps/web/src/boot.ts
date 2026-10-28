/** The loading screen in index.html shows before any script runs (brand, one line, a progress bar).
 * The app moves it to its real steps and removes it when the first screen is ready. Safe when it is absent (tests, desktop). */
type Boot = (percent: number, text?: string) => void;
const boot = (): Boot | undefined => (globalThis as { __boot?: Boot }).__boot;

export function bootStep(percent: number, text?: string): void { boot()?.(percent, text); }
/** The first screen is drawn: fade the loading screen out (at once with reduced motion). */
export function bootDone(): void { boot()?.(100); }
