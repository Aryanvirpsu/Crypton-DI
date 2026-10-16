import React from 'react';

/**
 * Three original, geometry-only miniature robots. Presentation only: no scenario facts
 * live here. Colours come from CSS custom properties (see agentFloor.css), shapes are
 * deliberately simple so each silhouette reads at ~44px.
 */

function Svg({ children, label }) {
  return (
    <svg viewBox="0 0 100 100" className="af-robot-svg" role="img" aria-label={label} focusable="false">
      {children}
    </svg>
  );
}

/** SCOUT: compact bone-white research robot with a large optical head. */
export function Scout() {
  return (
    <Svg label="Scout, a compact off-white research robot with a large round optical head">
      <ellipse cx="50" cy="94" rx="26" ry="4" className="af-shadow" />
      <rect x="36" y="82" width="10" height="10" rx="4" className="af-limb" />
      <rect x="54" y="82" width="10" height="10" rx="4" className="af-limb" />
      <rect x="27" y="52" width="46" height="32" rx="12" className="af-shell af-shell--bone" />
      <rect x="39" y="62" width="22" height="6" rx="3" className="af-panel" />
      <rect x="44" y="45" width="12" height="9" rx="3" className="af-limb" />
      <circle cx="50" cy="31" r="22" className="af-shell af-shell--bone" />
      <circle cx="50" cy="31" r="14" className="af-lens" />
      <circle cx="50" cy="31" r="7" className="af-led af-led--lime" />
      <circle cx="46" cy="27" r="2.4" className="af-glint" />
      <line x1="66" y1="14" x2="72" y2="5" className="af-stroke" />
      <circle cx="73" cy="4" r="2.6" className="af-led af-led--lime" />
    </Svg>
  );
}

/** RELAY: graphite coordinator with articulated manipulators. */
export function Relay() {
  return (
    <Svg label="Relay, a graphite coordination robot with two articulated arms">
      <ellipse cx="50" cy="94" rx="30" ry="4" className="af-shadow" />
      <rect x="30" y="80" width="40" height="11" rx="4" className="af-limb" />
      <rect x="26" y="40" width="48" height="42" rx="9" className="af-shell af-shell--graphite" />
      <rect x="35" y="49" width="30" height="5" rx="2.5" className="af-led af-led--lime" />
      <rect x="35" y="60" width="30" height="14" rx="3" className="af-panel" />
      <rect x="38" y="20" width="24" height="20" rx="6" className="af-shell af-shell--graphite" />
      <rect x="43" y="26" width="14" height="5" rx="2.5" className="af-led af-led--lime" />
      {/* left arm: shoulder, upper arm, elbow, forearm, gripper */}
      <circle cx="23" cy="48" r="5" className="af-joint" />
      <polyline points="23,48 11,58 15,74" className="af-arm" />
      <circle cx="11" cy="58" r="3.6" className="af-joint" />
      <path d="M15 74 l-5 6 M15 74 l5 6" className="af-arm af-arm--thin" />
      {/* right arm raised: mid-handoff pose */}
      <circle cx="77" cy="48" r="5" className="af-joint" />
      <polyline points="77,48 90,44 88,28" className="af-arm" />
      <circle cx="90" cy="44" r="3.6" className="af-joint" />
      <path d="M88 28 l-5 -5 M88 28 l5 -5" className="af-arm af-arm--thin" />
    </Svg>
  );
}

/** COURIER: low-profile wheeled tool/external-operations robot. */
export function Courier() {
  return (
    <Svg label="Courier, a low wheeled delivery robot with a flat cargo tray">
      <ellipse cx="50" cy="95" rx="38" ry="4" className="af-shadow" />
      <rect x="22" y="44" width="56" height="7" rx="3" className="af-tray" />
      <rect x="12" y="52" width="76" height="26" rx="10" className="af-shell af-shell--machine" />
      <rect x="18" y="58" width="64" height="5" rx="2.5" className="af-stripe" />
      <circle cx="26" cy="70" r="2.4" className="af-led af-led--coral" />
      <circle cx="34" cy="70" r="2.4" className="af-led af-led--lime" />
      <line x1="76" y1="44" x2="76" y2="30" className="af-stroke" />
      <circle cx="76" cy="28" r="3.2" className="af-led af-led--coral" />
      <circle cx="30" cy="82" r="11" className="af-wheel" />
      <circle cx="30" cy="82" r="4.5" className="af-hub" />
      <circle cx="70" cy="82" r="11" className="af-wheel" />
      <circle cx="70" cy="82" r="4.5" className="af-hub" />
    </Svg>
  );
}

export const ROBOT_ART = { 'AG-SCOUT': Scout, 'AG-RELAY': Relay, 'AG-COURIER': Courier };
