export const MOCK_DEVICES = [
  { id: "dev_001", ico: "💻", name: "MacBook Pro", type: "Laptop · macOS 14", status: "active", enrolled: "Mar 1, 2026", last: "2 min ago", fp: "a3:f7:2c:91..." },
  { id: "dev_002", ico: "📱", name: "iPhone 15 Pro", type: "Phone · iOS 17", status: "active", enrolled: "Feb 28, 2026", last: "1 hr ago", fp: "b8:12:aa:5e..." },
  { id: "dev_003", ico: "🖥", name: "Work Desktop", type: "Desktop · Windows 11", status: "inactive", enrolled: "Jan 15, 2026", last: "5 days ago", fp: "c4:9d:0f:77..." },
];

export const MOCK_PASSKEYS = [
  { id: "pk_a3f72c91b8e4", name: "MacBook Pro — Touch ID", attest: "packed", created: "Mar 1, 2026", lastUsed: "2 min ago", device: "MacBook Pro", active: true },
  { id: "pk_b812aa5e3d71", name: "iPhone 15 Pro — Face ID", attest: "apple", created: "Feb 28, 2026", lastUsed: "1h ago", device: "iPhone 15 Pro", active: true },
  { id: "pk_c49d0f77aa12", name: "YubiKey 5 — NFC", attest: "fido-u2f", created: "Jan 10, 2026", lastUsed: "12d ago", device: "Hardware Token", active: false },
];

export const MOCK_AUDIT_LOGS = [
  { id: "evt_001", actor: "aryan@crypton.io", action: "LOGIN", device: "MacBook Pro", ip: "192.168.1.1", loc: "San Francisco, CA", time: "2m ago", type: "success" },
  { id: "evt_002", actor: "aryan@crypton.io", action: "DEVICE_ENROLL", device: "iPhone 15 Pro", ip: "192.168.1.1", loc: "San Francisco, CA", time: "1h ago", type: "info" },
  { id: "evt_003", actor: "admin@crypton.io", action: "ROLE_CHANGE", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "3h ago", type: "warning" },
  { id: "evt_004", actor: "aryan@crypton.io", action: "LOGIN_BLOCKED", device: "Unknown", ip: "185.220.101.4", loc: "Tokyo, JP", time: "6h ago", type: "danger" },
  { id: "evt_005", actor: "admin@crypton.io", action: "POLICY_UPDATE", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "8h ago", type: "info" },
  { id: "evt_006", actor: "aryan@crypton.io", action: "DEVICE_REVOKE", device: "Work Desktop", ip: "192.168.1.1", loc: "San Francisco, CA", time: "1d ago", type: "danger" },
  { id: "evt_007", actor: "sarah@crypton.io", action: "LOGIN", device: "iPad Air", ip: "74.125.24.100", loc: "Austin, TX", time: "1d ago", type: "success" },
  { id: "evt_008", actor: "admin@crypton.io", action: "PASSKEY_REVOKE", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "2d ago", type: "danger" },
  { id: "evt_009", actor: "sarah@crypton.io", action: "LOGIN", device: "iPhone 14", ip: "74.125.24.100", loc: "Austin, TX", time: "2d ago", type: "success" },
  { id: "evt_010", actor: "aryan@crypton.io", action: "LOGIN", device: "MacBook Pro", ip: "192.168.1.1", loc: "San Francisco, CA", time: "3d ago", type: "success" },
];

export const MOCK_RISK_FEED = [
  { id: "feed_001", ico: "🚨", type: "danger", msg: "Geo-velocity alert: 8,400km in 3 hours", time: "2m ago" },
  { id: "feed_002", ico: "⚠", type: "warning", msg: "TOR exit node detected — IP 185.220.101.4", time: "14m ago" },
  { id: "feed_003", ico: "⚠", type: "warning", msg: "3 failed passkey attempts — admin@crypton.io", time: "1h ago" },
  { id: "feed_004", ico: "🛡", type: "info", msg: "Device reputation verified — MacBook Pro", time: "2h ago" },
  { id: "feed_005", ico: "✓", type: "success", msg: "Behavioral baseline updated — sarah@crypton.io", time: "4h ago" },
];

export const MOCK_RISK_USERS = [
  { id: "risk_001", user: "aryan@crypton.io",  score: 12, level: "LOW",    device: "MacBook Pro",  ip: "192.168.1.1",    loc: "San Francisco, CA", time: "2m ago",  reasons: ["Known device", "Normal hours", "Trusted location"] },
  { id: "risk_002", user: "admin@crypton.io",  score: 44, level: "MEDIUM", device: "MacBook Pro",  ip: "10.0.0.5",       loc: "New York, NY",      time: "1h ago",  reasons: ["New IP range", "Off-hours login", "Role: Admin"] },
  { id: "risk_003", user: "sarah@crypton.io",  score: 21, level: "LOW",    device: "iPad Air",     ip: "74.125.24.100",  loc: "Austin, TX",        time: "3h ago",  reasons: ["Known device", "Daytime login"] },
  { id: "risk_004", user: "unknown@extern.io", score: 87, level: "HIGH",   device: "Unknown",      ip: "185.220.101.4",  loc: "Tokyo, JP",         time: "6h ago",  reasons: ["TOR exit node", "Geo-velocity violation", "Unknown device"] },
];

export const MOCK_SESSIONS = [
  { id: "ses_001", user: "aryan@crypton.io", device: "MacBook Pro", browser: "Chrome 122", loc: "San Francisco, CA", ip: "192.168.1.1", started: "Today, 09:14 AM", duration: "4h 32m", active: true },
  { id: "ses_002", user: "aryan@crypton.io", device: "iPhone 15 Pro", browser: "Safari Mobile", loc: "San Francisco, CA", ip: "192.168.1.2", started: "Today, 11:02 AM", duration: "2h 44m", active: true },
  { id: "ses_003", user: "sarah@crypton.io", device: "iPad Air", browser: "Safari", loc: "Austin, TX", ip: "74.125.24.100", started: "Today, 08:30 AM", duration: "5h 16m", active: true },
  { id: "ses_004", user: "admin@crypton.io", device: "MacBook Pro", browser: "Firefox 123", loc: "New York, NY", ip: "10.0.0.5", started: "Yesterday, 11:58 PM", duration: "Idle 8h", active: false },
];

export const MOCK_USERS = [
  { id: "usr_001", name: "Aryan Vir", email: "aryan@crypton.io", role: "Super Admin", devices: 2, lastActive: "2m ago", avatar: "A" },
  { id: "usr_002", name: "Admin User", email: "admin@crypton.io", role: "Admin", devices: 1, lastActive: "1h ago", avatar: "AU" },
  { id: "usr_003", name: "Sarah Kim", email: "sarah@crypton.io", role: "Security Analyst", devices: 2, lastActive: "3h ago", avatar: "S" },
  { id: "usr_004", name: "Dev Read", email: "dev@crypton.io", role: "Viewer", devices: 1, lastActive: "2d ago", avatar: "D" },
];

export const MOCK_DASHBOARD_STATS = {
  activeDevices: 3,
  authEvents24h: 47,
  securityScore: 98,
};

export const MOCK_ACTIVITY = [
  { id: "act_001", ico: "✓", type: "s", title: "Authentication successful", meta: "MacBook Pro · Chrome · San Francisco, CA", time: "2m ago", link: "auditlogs" },
  { id: "act_002", ico: "📱", type: "i", title: "New device enrolled", meta: "iPhone 15 Pro · Passkey created", time: "1h ago", link: "devices" },
  { id: "act_003", ico: "✓", type: "s", title: "Authentication successful", meta: "iPad Air · Safari · New York, NY", time: "3h ago", link: "auditlogs" },
  { id: "act_004", ico: "⚠", type: "w", title: "Unrecognized device blocked", meta: "Unknown · Tokyo, JP · Request denied", time: "6h ago", link: "risk" },
  { id: "act_005", ico: "🔒", type: "i", title: "Security sweep completed", meta: "All 3 devices verified · Zero anomalies", time: "12h ago", link: "sessions" },
];

export const MOCK_ORG = {
  orgName: "Crypton Labs",
  domain: "crypton.io",
  domainVerified: true,
  mfaEnforced: true,
  sessionTimeoutHours: 8,
  allowedCountries: ["US", "CA", "GB", "DE", "AU"],
};

export const MOCK_POLICIES = [
  { id: "geo_block",       label: "Block High-Risk Countries",       desc: "Deny auth from CN, RU, KP, IR and other flagged regions",                          active: true,  cat: "geo" },
  { id: "stepup_risk",     label: "Step-Up Auth if Risk > 70",       desc: "Require additional verification when risk score exceeds threshold",                 active: true,  cat: "risk" },
  { id: "tor_block",       label: "Block TOR / VPN IPs",             desc: "Reject requests from known TOR exit nodes and datacenter IPs",                     active: true,  cat: "network" },
  { id: "geo_velocity",    label: "Geo-Velocity Protection",         desc: "Block impossible travel — flag logins from multiple continents within 4h",          active: false, cat: "geo" },
  { id: "device_trust",    label: "Device Trust Duration — 30 days", desc: "Re-verify device passkey after 30 days of inactivity",                             active: true,  cat: "device" },
  { id: "failed_attempts", label: "Lock After 5 Failed Attempts",    desc: "Temporary 15-minute lockout after 5 consecutive failed auth attempts",              active: true,  cat: "auth" },
  { id: "off_hours",       label: "Notify on Off-Hours Login",       desc: "Send alert when users authenticate outside 06:00–22:00 local time",                 active: false, cat: "auth" },
  { id: "new_device",      label: "Require Approval for New Devices",desc: "Admin must approve new device enrollment via existing trusted device",              active: false, cat: "device" },
];

export const PAGE_LABELS = {
  dashboard: "Dashboard", devices: "Devices",
  auditlogs: "Audit Logs", rbac: "Users & Roles", recovery: "Recovery",
  risk: "Risk Intelligence", sessions: "Sessions", admin: "Admin",
  policy: "Policy Engine", orgsettings: "Org Settings",
  demo: "Demo Actions",
};

export const ROLES = ["Super Admin", "Admin", "Security Analyst", "Viewer"];

export const ROLE_COLORS = {
  "Super Admin": "var(--danger)", "Admin": "var(--accent)",
  "Security Analyst": "var(--warning)", "Viewer": "var(--muted)",
};

export const ROLE_PERMS = {
  "Super Admin": ["Revoke devices", "Change roles", "Update policies", "View audit logs", "Kill sessions", "Emergency lockdown"],
  "Admin":       ["Revoke devices", "Change roles", "Update policies", "View audit logs", "Kill sessions"],
  "Security Analyst": ["View audit logs", "View risk scores", "Flag users"],
  "Viewer":      ["View dashboard", "View devices (read-only)"],
};
