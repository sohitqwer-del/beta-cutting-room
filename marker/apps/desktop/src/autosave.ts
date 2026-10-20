/** Keep a previous session's recovery file until the operator chooses a session. */
export function projectAutosave<T>(hooks: {
  read(): T;
  busy(): boolean;
  write(value: T): Promise<void>;
  saved(): void;
}, recoveryPending: boolean) {
  let pending = recoveryPending, writing = false, lastSnapshot = '';
  return {
    /** Explicitly choose the current work instead of the previous autosave. */
    useCurrent(): void { pending = false; },
    async tick(): Promise<void> {
      if (pending || writing || hooks.busy()) return;
      const value = hooks.read(), text = JSON.stringify(value);
      if (text === lastSnapshot) return;
      writing = true;
      try {
        await hooks.write(value);
        lastSnapshot = text;
        if (JSON.stringify(hooks.read()) === text) hooks.saved();
      } finally { writing = false; }
    },
  };
}
