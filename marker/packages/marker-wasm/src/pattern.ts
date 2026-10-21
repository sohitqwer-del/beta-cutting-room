/** BETA Pattern document contract. Lengths are millimetres; y points up. */
import type { Fabric, Job, Json, MarkKind, Piece, Pt, RotationRule } from './types';
export type PatternValue = number | string;
export type PointOperation = {
    op: 'path_endpoint' | 'intersect_xy' | 'shoulder' | 'triangle' | 'line_axis' | 'curve_axis' | 'circle_intersection' | 'arc_intersection' | 'circle_line' | 'arc_line' | 'circle_tangent' | 'arc_tangent' | 'curve_point' | 'true_dart';
    [field: string]: Json;
} | {
    op: 'absolute';
    x: PatternValue;
    y: PatternValue;
} | {
    op: 'along_line';
    from: string;
    to: string;
    distance: PatternValue;
} | {
    op: 'polar';
    from: string;
    angle: PatternValue;
    distance: PatternValue;
} | {
    op: 'midpoint';
    a: string;
    b: string;
} | {
    op: 'foot';
    point: string;
    a: string;
    b: string;
} | {
    op: 'normal';
    a: string;
    b: string;
    distance: PatternValue;
} | {
    op: 'bisector';
    vertex: string;
    a: string;
    b: string;
    distance: PatternValue;
} | {
    op: 'line_intersection';
    a: [
        string,
        string
    ];
    b: [
        string,
        string
    ];
} | {
    op: 'intersection';
    a: string;
    b: string;
    index?: number;
};
export type DrawingObject = {
    kind: 'offset_path' | 'derived_path' | 'arc_length' | 'elliptical_arc' | 'cubic_path' | 'cut_path' | 'move' | 'rotate' | 'mirror_line' | 'mirror_axis';
    [field: string]: Json;
} | {
    kind: 'point';
    operation: PointOperation;
} | {
    kind: 'line';
    from: string;
    to: string;
} | {
    kind: 'cubic';
    points: [
        string,
        string,
        string,
        string
    ];
} | {
    kind: 'spline';
    points: string[];
    tangents?: [
        PatternValue,
        PatternValue
    ][];
} | {
    kind: 'arc';
    center: string;
    radius: PatternValue;
    start: PatternValue;
    sweep: PatternValue;
};
export interface PatternEdge {
    object: string;
    reversed?: boolean;
    allowance?: PatternValue;
    corner?: 'miter' | 'square' | 'bevel' | 'mirrored';
}
export type Annotation = {
    kind: 'shaped_notch';
    edge: number;
    distance: PatternValue;
    depth: PatternValue;
    width: PatternValue;
    shape: 'slit' | 'v' | 't' | 'castle';
} | {
    kind: 'seam_intersections';
    path: string;
    depth: PatternValue;
    width: PatternValue;
    shape: 'slit' | 'v' | 't' | 'castle';
} | {
    kind: 'points';
    mark: MarkKind;
    points: string[];
    text?: string | null;
} | {
    kind: 'button';
    point: string;
    diameter: PatternValue;
} | {
    kind: 'notch';
    edge: number;
    distance: PatternValue;
    depth: PatternValue;
};
export interface PatternPiece {
    internal_paths?: {
        name: string;
        edges: PatternEdge[];
        closed?: boolean;
    }[];
    anchors?: Record<string, string>;
    grain_angle?: {
        origin: string;
        angle: PatternValue;
        length: PatternValue;
    } | null;
    label_point?: string | null;
    id: string;
    name: string;
    edges: PatternEdge[];
    quantity?: number;
    mirror_quantity?: number;
    material?: string | null;
    grain?: [
        string,
        string
    ] | null;
    fold_line?: [
        string,
        string
    ] | null;
    rotation?: RotationRule;
    buffer?: PatternValue;
    annotations?: Annotation[];
    grade_points?: Record<string, string>;
}
export interface GradeRule {
    mode?: 'relative' | 'incremental';
    deltas: Record<string, Pt>;
}
export interface PatternDocument {
    format: 'beta-pattern.doc/v1';
    name: string;
    label?: string | null;
    sizes: string[];
    base_size: string;
    measurements: Record<string, Record<string, number>>;
    variables: Record<string, PatternValue>;
    drawing: Record<string, DrawingObject>;
    pieces: PatternPiece[];
    rules?: Record<string, GradeRule>;
    groups?: Record<string, {
        name: string;
        objects: string[];
        visible?: boolean;
    }>;
    meta?: Record<string, Json>;
}
export interface GradedPiece {
    /** Actual graded stitch paths in piece edge order, without allowances. */
    seam_edges?: Pt[][];
    piece: Piece;
    seam: Pt[];
    points: Record<string, Pt>;
}
export interface PatternGrade {
    sizes: Record<string, GradedPiece[]>;
}
export interface PatternRequest {
    document: PatternDocument;
    sizes?: string[];
    tolerance_mm?: number;
    fabric?: Fabric;
    ratio?: Record<string, number>;
}
export interface PatternImportRequest {
    pattern: string;
    measurements?: string | null;
    size?: string;
}
export interface PatternImportResponse {
    document: PatternDocument;
    warnings: string[];
}
export interface PatternCore {
    grade(document: PatternDocument): PatternGrade;
    inspect(document: PatternDocument, size: string): PatternInspection;
    import_measurements(document: PatternDocument, source: string, format: string, size: string): PatternDocument;
    export_measurements(document: PatternDocument): string;
    nest(document: PatternDocument, reference: string): {
        sizes: Record<string, GradedPiece[]>;
    };
    move_point(document: PatternDocument, id: string, size: string, target: Pt): PatternDocument;
    piece_action(document: PatternDocument, request: Record<string, Json>): {
        document: PatternDocument;
        message?: string;
    };
    check_formula(document: PatternDocument, size: string, expression: string): number;
    snap(document: PatternDocument, size: string, request: {
        at: Pt;
        radius: number;
        grid: number;
        points: boolean;
        paths: boolean;
        ignore?: string;
    }): {
        point: Pt;
        object: string | null;
    };
    image_scale(a: Pt, b: Pt, distance: number): number;
    import_seamly(pattern: string, measurements: string | undefined, size: string): PatternImportResponse;
    catalog(): PatternToolSpec[];
    apply_tool(document: PatternDocument, request: PatternToolRequest): {
        document: PatternDocument;
        message?: string;
    };
    export_pdf(document: PatternDocument, paper: 'A4' | 'A3'): Uint8Array;
    export_plt(document: PatternDocument): string;
    starter(): PatternDocument;
    to_job(document: PatternDocument, fabric: Fabric, ratio: Record<string, number>): Job;
    export_dxf(document: PatternDocument): string;
    from_job(job: Job): PatternDocument;
}
interface RawPattern {
    default(options: {
        module_or_path: URL;
    }): Promise<unknown>;
    grade(document: string): string;
    inspect(document: string, size: string): string;
    import_measurements(document: string, source: string, format: string, size: string): string;
    export_measurements(document: string): string;
    nest(document: string, reference: string): string;
    move_point(document: string, id: string, size: string, target: string): string;
    piece_action(document: string, request: string): string;
    check_formula(document: string, size: string, expression: string): number;
    snap(document: string, size: string, request: string): string;
    image_scale(a: string, b: string, distance: number): number;
    import_seamly(pattern: string, measurements: string | undefined, size: string): string;
    catalog(): string;
    apply_tool(document: string, request: string): string;
    export_pdf(document: string, paper: string): Uint8Array;
    export_plt(document: string): string;
    starter(): string;
    to_job(document: string, fabric: string, ratio: string): string;
    export_dxf(document: string): string;
    from_job(job: string): string;
}
/** Load local wasm-bindgen assets. This function never contacts an external service. */
export async function loadPatternCore(): Promise<PatternCore> {
    const url = new URL('wasm/beta_pattern_wasm.js', document.baseURI);
    const raw = await import(/* @vite-ignore */ url.href) as RawPattern;
    await raw.default({ module_or_path: new URL('./beta_pattern_wasm_bg.wasm', url) });
    return {
        grade: doc => JSON.parse(raw.grade(JSON.stringify(doc))),
        import_measurements: (doc, source, format, size) => JSON.parse(raw.import_measurements(JSON.stringify(doc), source, format, size)),
        export_measurements: doc => raw.export_measurements(JSON.stringify(doc)),
        nest: (doc, reference) => JSON.parse(raw.nest(JSON.stringify(doc), reference)),
        move_point: (doc, id, size, target) => JSON.parse(raw.move_point(JSON.stringify(doc), id, size, JSON.stringify(target))),
        piece_action: (doc, request) => JSON.parse(raw.piece_action(JSON.stringify(doc), JSON.stringify(request))),
        inspect: (doc, size) => JSON.parse(raw.inspect(JSON.stringify(doc), size)),
        check_formula: (doc, size, expression) => raw.check_formula(JSON.stringify(doc), size, expression),
        snap: (doc, size, request) => JSON.parse(raw.snap(JSON.stringify(doc), size, JSON.stringify(request))),
        image_scale: (a, b, distance) => raw.image_scale(JSON.stringify(a), JSON.stringify(b), distance),
        import_seamly: (pattern, measurements, size) => JSON.parse(raw.import_seamly(pattern, measurements, size)),
        catalog: () => JSON.parse(raw.catalog()),
        apply_tool: (doc, request) => JSON.parse(raw.apply_tool(JSON.stringify(doc), JSON.stringify(request))),
        export_pdf: (doc, paper) => raw.export_pdf(JSON.stringify(doc), paper),
        export_plt: doc => raw.export_plt(JSON.stringify(doc)),
        starter: () => JSON.parse(raw.starter()),
        to_job: (doc, fabric, ratio) => JSON.parse(raw.to_job(JSON.stringify(doc), JSON.stringify(fabric), JSON.stringify(ratio))),
        export_dxf: doc => raw.export_dxf(JSON.stringify(doc)),
        from_job: job => JSON.parse(raw.from_job(JSON.stringify(job))),
    };
}
/** Serializable editor geometry. Every coordinate is calculated by Rust. */
export interface PatternInspection {
    objects: Record<string, {
        kind: 'point' | 'path';
        points: Pt[];
    }>;
    pieces: {
        id: string;
        ok: boolean;
        error?: string;
        graded?: GradedPiece;
        edges?: {
            object: string;
            length_mm: number;
        }[];
    }[];
}
export interface PatternToolParameter {
    name: string;
    label?: string;
    kind: string;
    help?: string;
    default?: Json;
    choices?: string[];
    required?: boolean;
}
export interface PatternToolSpec {
    point: boolean;
    id: string;
    name: string;
    group: string;
    help: string;
    parameters: PatternToolParameter[];
}
export interface PatternToolRequest {
    tool: string;
    id: string;
    size: string;
    parameters: Record<string, Json>;
    selection?: string[];
}
