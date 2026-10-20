// Tiny HTML templating: interpolated values are escaped unless they are Raw.

export class Raw {
  readonly value: string;
  constructor(value: string) { this.value = value; }
  toString(): string { return this.value; }
}

export type Part = string | number | Raw | null | undefined | false | readonly Part[];

/** Marks trusted markup so html`` does not escape it. */
export const raw = (s: string): Raw => new Raw(s);

/** Escapes text for HTML element content and quoted attribute values. */
export function esc(value: unknown): string {
  return String(value).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

function render(part: Part): string {
  if (part === null || part === undefined || part === false) return '';
  if (part instanceof Raw) return part.value;
  if (Array.isArray(part)) return part.map(render).join('');
  return esc(part);
}

/** Tagged template that escapes every interpolation by default. */
export function html(strings: TemplateStringsArray, ...values: Part[]): Raw {
  let out = strings[0] ?? '';
  values.forEach((v, i) => { out += render(v) + (strings[i + 1] ?? ''); });
  return new Raw(out);
}

/** Formats a number with a fixed number of decimals and a real minus sign. */
export function fmt(n: number, digits = 2): string {
  const s = Math.abs(n).toFixed(digits);
  return n < 0 && Number(s) !== 0 ? `−${s}` : s;
}

/** Formats a signed difference: +1.23 or −1.23. */
export function signed(n: number, digits = 2): string {
  return n > 0 ? `+${fmt(n, digits)}` : fmt(n, digits);
}
