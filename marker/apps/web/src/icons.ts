/** BETA icon set: original 24 x 24 stroke icons (Apache-2.0, part of this app). No icon font, no CDN.
 * Use icon(name) for decorative icons next to a visible label; give icon-only buttons an aria-label. */
const PATHS: Record<string, string> = {
  // Navigation and workflow
  home: '<path d="M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
  pattern: '<path d="M4 4v16h16"/><path d="M5 19c5-1 6-5 7-9s3-6 8-6"/>',
  'photo-marker': '<path d="M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><path d="M9 11h3v5H9z"/><path d="M13.5 11h2l.5 2-1 3h-1.5z"/>',
  digitize: '<rect x="3" y="7" width="18" height="13" rx="2"/><circle cx="12" cy="13.5" r="3.5"/><path d="M8 7l2-3h4l2 3"/>',
  marker: '<rect x="3" y="5" width="18" height="14" rx="1"/><path d="M9 5v14"/><path d="M15 5c-2 4-2 10 0 14"/>',
  plan: '<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 12.5l9 5 9-5"/><path d="M3 17l9 5 9-5"/>',
  compare: '<path d="M12 4v16"/><path d="M5 8h14"/><path d="M5 8l-3 6h6z"/><path d="M19 8l-3 6h6z"/><path d="M8 20h8"/>',
  rolls: '<circle cx="7" cy="12" r="4"/><circle cx="7" cy="12" r="1"/><path d="M7 8h13v8H7"/>',
  floor: '<path d="M3 21V10l5 3v-3l5 3V7l8 4v10z"/><path d="M7 17h2M12 17h2M17 17h2"/>',
  spread: '<path d="M3 8h14l4 3H7z"/><path d="M3 12.5h14l4 3H7z"/><path d="M3 17h14l4 3H7z"/>',
  cut: '<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M8.5 7.5L20 18M8.5 16.5L20 6"/>',
  bundles: '<path d="M3 12V4h8l10 10-8 8z"/><circle cx="7.5" cy="8" r="1.5"/>',
  sewing: '<path d="M5 19L18 6"/><circle cx="19.5" cy="4.5" r="1.8"/><path d="M4 13c3 0 3 7 7 7"/>',
  reports: '<path d="M6 3h9l4 4v14H6z"/><path d="M14 3v5h5"/><path d="M9 13h6M9 17h6M9 9h2"/>',
  help: '<circle cx="12" cy="12" r="9"/><path d="M9.6 9.4a2.5 2.5 0 1 1 3.4 2.4c-.6.3-1 .9-1 1.6v.6"/><path d="M12 17.3v.2"/>',
  // Files
  open: '<path d="M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"/>',
  new: '<path d="M6 3h8l4 4v14H6z"/><path d="M12 10v6M9 13h6"/>',
  file: '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>',
  save: '<path d="M5 3h11l4 4v13a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z"/><path d="M8 3v5h7V3"/><path d="M7 21v-7h10v7"/>',
  download: '<path d="M12 4v11"/><path d="M7 10l5 5 5-5"/><path d="M5 20h14"/>',
  upload: '<path d="M12 16V5"/><path d="M7 10l5-5 5 5"/><path d="M5 20h14"/>',
  print: '<path d="M7 9V3h10v6"/><rect x="3" y="9" width="18" height="8" rx="1"/><path d="M7 14h10v7H7z"/>',
  demo: '<circle cx="12" cy="12" r="9"/><path d="M10 8.5v7l6-3.5z"/>',
  // App
  menu: '<path d="M4 6h16M4 12h16M4 18h16"/>',
  more: '<circle cx="5" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="19" cy="12" r="1.2"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6"/><path d="M12 7.3v.2"/>',
  key: '<circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M17 6l3 3M14.5 8.5l2 2"/>',
  globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  moon: '<path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"/>',
  settings: '<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>',
  'panel-left': '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>',
  'panel-right': '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>',
  'chevrons-left': '<path d="M11 17l-5-5 5-5"/><path d="M18 17l-5-5 5-5"/>',
  'chevrons-right': '<path d="M13 17l5-5-5-5"/><path d="M6 17l5-5-5-5"/>',
  'chevron-down': '<path d="M6 9l6 6 6-6"/>',
  'chevron-right': '<path d="M9 6l6 6-6 6"/>',
  'arrow-right': '<path d="M5 12h14"/><path d="M13 6l6 6-6 6"/>',
  'arrow-left': '<path d="M19 12H5"/><path d="M11 6l-6 6 6 6"/>',
  'arrow-up': '<path d="M12 19V5"/><path d="M6 11l6-6 6 6"/>',
  'arrow-down': '<path d="M12 5v14"/><path d="M6 13l6 6 6-6"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  'check-circle': '<circle cx="12" cy="12" r="9"/><path d="M8 12.5l3 3 5-6"/>',
  alert: '<path d="M12 3.5L2.5 20h19z"/><path d="M12 10v4.5"/><path d="M12 17.3v.2"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  minus: '<path d="M5 12h14"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>',
  keyboard: '<rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10"/>',
  // Marker tools
  play: '<path d="M7 4.5l12 7.5-12 7.5z"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="1.5"/>',
  undo: '<path d="M9 14L4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/>',
  redo: '<path d="M15 14l5-5-5-5"/><path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13"/>',
  rotate: '<path d="M20 12a8 8 0 1 1-2.3-5.6"/><path d="M20 4v5h-5"/>',
  flip: '<path d="M12 3v18"/><path d="M9 7L3 17h6z"/><path d="M15 7l6 10h-6z"/>',
  lock: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  unlock: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 7.6-1.8"/>',
  trash: '<path d="M4 7h16"/><path d="M10 11v6M14 11v6"/><path d="M6 7l1 13a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1l1-13"/><path d="M9 7V4h6v3"/>',
  move: '<path d="M12 3v18M3 12h18"/><path d="M9 6l3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3"/>',
  select: '<path d="M4 8V5a1 1 0 0 1 1-1h3M12 4h0M16 4h3a1 1 0 0 1 1 1v3M20 12v0M20 16v3a1 1 0 0 1-1 1h-3M12 20h0M8 20H5a1 1 0 0 1-1-1v-3M4 12v0"/>',
  measure: '<path d="M3 17L17 3l4 4L7 21z"/><path d="M7 13l2 2M10 10l2 2M13 7l2 2"/>',
  compact: '<path d="M4 4v16"/><path d="M20 8h-9M14 5l-3 3 3 3"/><path d="M20 16h-9M14 13l-3 3 3 3"/>',
  'close-end': '<path d="M14 4v16"/><path d="M4 12h6M7 9l3 3-3 3"/><path d="M20 9l-3 3 3 3"/>',
  'zoom-in': '<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4M11 8v6M8 11h6"/>',
  'zoom-out': '<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4M8 11h6"/>',
  fit: '<path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/><rect x="8" y="8" width="8" height="8" rx="1"/>',
  swap: '<path d="M7 4L3 8l4 4"/><path d="M3 8h14"/><path d="M17 12l4 4-4 4"/><path d="M21 16H7"/>',
  tilt: '<path d="M4 20h16"/><path d="M5 19L17 7"/><path d="M13 20a7 7 0 0 0-2-5"/>',
  swatches: '<rect x="3" y="3" width="8" height="8" rx="1.5"/><rect x="13" y="3" width="8" height="8" rx="1.5"/><rect x="3" y="13" width="8" height="8" rx="1.5"/><rect x="13" y="13" width="8" height="8" rx="1.5"/>',
  target: '<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="1.5"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3"/>',
  fill: '<rect x="3" y="3" width="18" height="18" rx="1"/><path d="M3 12h9V3M12 12v9"/><path d="M15 15l3 3"/>',
  library: '<path d="M4 4h4v16H4zM10 4h4v16h-4z"/><path d="M16 5l3.5-1L22 19l-3.5 1z"/>',
  layers: '<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 12.5l9 5 9-5"/>',
};

export type IconName = keyof typeof PATHS;

export function icon(name: string, extra = ''): string {
  return `<svg class="icon${extra ? ` ${extra}` : ''}" viewBox="0 0 24 24" aria-hidden="true" focusable="false">${PATHS[name] ?? ''}</svg>`;
}

/** Icon button markup: visible label is optional; the accessible name and tooltip always include the key hint. */
export function iconButton(id: string, name: string, label: string, keyHint = '', extra = ''): string {
  const tip = keyHint ? `${label} · ${keyHint}` : label;
  return `<button type="button" id="${id}" class="icon-btn" aria-label="${label}"${keyHint ? ` aria-keyshortcuts="${keyHint}"` : ''} data-tip="${tip}" ${extra}>${icon(name)}<span class="btn-label">${label}</span></button>`;
}
