// One invocation tests one translated raster mask. The minimum feasible index
// means the lowest x, then the lowest y. All shifts are unsigned and bounded.
struct Params {
    stride: u32,
    mask_rows: u32,
    mask_stride: u32,
    y_count: u32,
    candidates: u32,
    first: u32,
    pad0: u32,
    pad1: u32,
}
@group(0) @binding(0) var<storage, read> strip: array<u32>;
@group(0) @binding(1) var<storage, read> mask: array<u32>;
@group(0) @binding(2) var<uniform> p: Params;
@group(0) @binding(3) var<storage, read_write> best: atomic<u32>;

@compute @workgroup_size(64)
fn scan(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x + id.y * 4194240u;
    if index >= p.candidates || index < p.first || index >= atomicLoad(&best) {
        return;
    }
    let x = index / p.y_count;
    let y = index % p.y_count;
    let word = y / 32u;
    let shift = y % 32u;
    // Probe central rows first: crowded positions usually reject immediately.
    for (var k = 0u; k < p.mask_rows; k++) {
        let row = (k + p.mask_rows / 2u) % p.mask_rows;
        let base = (x + row) * p.stride + word;
        for (var w = 0u; w < p.mask_stride; w++) {
            let bits = mask[row * p.mask_stride + w];
            if (strip[base + w] & (bits << shift)) != 0u { return; }
            if shift != 0u && (bits >> (32u - shift)) != 0u {
                if (strip[base + w + 1u] & (bits >> (32u - shift))) != 0u { return; }
            }
        }
    }
    atomicMin(&best, index);
}
