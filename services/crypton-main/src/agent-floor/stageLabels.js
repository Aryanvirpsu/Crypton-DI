import { getEntity } from './scenario';

/**
 * Accessible names and short tags for everything on the stage. Shared by the 2D stage
 * and the 3D world, so a robot or chip is announced the same way in either renderer.
 */

export const RESOURCE_GLYPH = {
  'RS-REPORT': 'RPT',
  'RS-RECORDS': 'REC',
  'RS-EXTERNAL': 'EXT',
};

// [id, short word]. The word is dropped where space is tight; the legend carries it.
export const CHIP_TEXT = {
  'GR-001': ['GR-001', 'grant'],
  'DL-001': ['DL-001', 'delegation'],
  'ACT-001': ['ACT-001', 'read'],
  'ACT-002': ['ACT-002', 'delegate'],
  'ACT-003': ['ACT-003', 'requested'],
};

export function inspectLabel(id) {
  const e = getEntity(id);
  if (!e) return `Inspect ${id}`;
  if (e.type === 'principal' && e.kind === 'human') return 'Inspect Manager (you, the human manager)';
  if (e.type === 'principal') return `Inspect ${e.name}, ${e.role.toLowerCase()}`;
  if (e.type === 'resource') return `Inspect resource ${e.id}: ${e.name}`;
  return `Inspect ${e.id}`;
}
