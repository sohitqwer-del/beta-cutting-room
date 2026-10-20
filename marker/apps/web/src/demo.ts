import type { Job, MarkerResult, Piece } from '@beta/marker-wasm';
/** Original synthetic shapes, drawn for this app. No factory or vendor data. */
const piece = (id: string, name: string, size: string, outline: Piece['outline']): Piece => ({
  id, name, size, outline, quantity: 1, mirror_quantity: 1, rotation: 'inherit', buffer: 0,
  grain: [[30, 30], [200, 30]], marks: [],
});
export const demoJob: Job = {
  format: 'beta-marker.job/v1', name: 'Everyday shirt · demo',
  fabric: { width: 1500, nap: 'none', tilt_deg: 0, allow_flip: false },
  spacing: { piece_gap: 6, edge_gap: 10 }, order: { sizes: { M: 1, L: 1 } }, meta: { source: 'Original synthetic demo' },
  pieces: [
    piece('front-m', 'Front', 'M', [[0, 0], [680, 0], [680, 220], [570, 300], [480, 400], [0, 400]]),
    piece('back-l', 'Back', 'L', [[0, 0], [720, 0], [720, 310], [600, 420], [0, 420]]),
    piece('sleeve-m', 'Sleeve', 'M', [[0, 0], [410, 0], [500, 140], [410, 280], [0, 280]]),
    piece('cuff-l', 'Cuff', 'L', [[0, 0], [250, 0], [250, 100], [0, 100]]),
  ],
};
export const demoResult: MarkerResult = {
  format: 'beta-marker.result/v1', job_name: demoJob.name, width: 1500, length: 1470,
  piece_area: 1400600, efficiency: 1400600 / (1500 * 1470), engine: 'synthetic demo', seed: 42, runtime_s: 0,
  placements: [
    { piece_id: 'front-m', copy: 0, mirrored: false, rotation_deg: 0, translate: [10, 10] },
    { piece_id: 'front-m', copy: 0, mirrored: true, rotation_deg: 180, translate: [10, 830] },
    { piece_id: 'back-l', copy: 0, mirrored: false, rotation_deg: 0, translate: [750, 10] },
    { piece_id: 'back-l', copy: 0, mirrored: true, rotation_deg: 180, translate: [750, 870] },
    { piece_id: 'sleeve-m', copy: 0, mirrored: false, rotation_deg: 0, translate: [10, 900] },
    { piece_id: 'sleeve-m', copy: 0, mirrored: true, rotation_deg: 180, translate: [530, 1190] },
    { piece_id: 'cuff-l', copy: 0, mirrored: false, rotation_deg: 0, translate: [1060, 910] },
    { piece_id: 'cuff-l', copy: 0, mirrored: true, rotation_deg: 180, translate: [1060, 1140] },
  ], warnings: [],
};
