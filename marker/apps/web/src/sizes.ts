/** Garment size order, as on a size chart: XXS, XS, S, M, L, XL, XXL (2XL), 3XL …; then number sizes (28, 30, 32 …);
 * then anything else in A–Z order. Used for size lists (ratio, cut plan, legends); it never changes the data. */
const LETTERS = ['XXXS', 'XXS', 'XS', 'S', 'M', 'L', 'XL', 'XXL', 'XXXL', 'XXXXL', 'XXXXXL'];

function rank(size: string): [number, number, string] {
  const s = size.trim().toUpperCase();
  const plus = /^(\d)XL$/.exec(s);
  const letter = LETTERS.indexOf(plus ? 'X'.repeat(Number(plus[1])) + 'L' : s);
  if (letter >= 0) return [0, letter, s];
  const number = Number(s.replace(',', '.'));
  if (s !== '' && Number.isFinite(number)) return [1, number, s];
  return [2, 0, s];
}
export function bySize(a: string, b: string): number {
  const [ga, va, sa] = rank(a), [gb, vb, sb] = rank(b);
  return ga - gb || va - vb || sa.localeCompare(sb);
}
