import { createContext, useContext } from 'react';

/**
 * Per-canvas state shared by world components. Scenario facts are NOT here: they arrive
 * as props from the reducer and the authority graph. This only carries the world's own
 * clock and where the robots currently are, so the camera can follow them.
 *
 * time       scaled world seconds (stops when the frame is held)
 * sceneTime  real seconds since the current scene began
 * timeScale  1 while the world runs, easing to 0 when the frame is held
 * live       real seconds, never scaled or held: the clock for things that carry on while the
 *            main floor is held (the far floors)
 * robots     Map of robot id -> THREE.Object3D (live position)
 * hover      Set of robot ids hovered or focused (pointer, or the robot's name tag)
 */
export const WorldContext = createContext(null);

export function useWorld() {
  const ctx = useContext(WorldContext);
  if (!ctx) throw new Error('useWorld must be used inside <World3D>');
  return ctx;
}
