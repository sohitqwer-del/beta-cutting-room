import type { QuickPiece, QuickWarning } from '@beta/marker-wasm';
/** Keep real quality problems as warnings. Routine review stays visible as guidance. */
export function scanWarnings(warnings: QuickWarning[], pieces: QuickPiece[]): QuickWarning[] {
  return warnings.filter(w => w.code !== 'review_required' && !(w.code === 'grain_unconfirmed' && pieces.some(p => (p.id === w.pieceId || p.id.endsWith(`:${w.pieceId}`)) && p.suggestions.grain)));
}
