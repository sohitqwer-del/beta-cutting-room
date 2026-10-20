/** Read current live panel state, or migrate the first project layout. */
export function projectPanels(project: { planning?: unknown; storage: Record<string, string> }): unknown {
  const panels = project.planning as Record<string, unknown> | undefined;
  const legacy = project.storage['beta-cutting-room-v1'];
  return { ...panels, cuttingRoom: panels?.cuttingRoom ?? (legacy ? JSON.parse(legacy) as unknown : undefined) };
}
