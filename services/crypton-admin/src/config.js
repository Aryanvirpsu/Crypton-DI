/**
 * crypton-admin surface config
 * Set these in services/crypton-admin/.env (or .env.local for local overrides)
 */
export const MAIN_URL = process.env.REACT_APP_MAIN_URL || 'http://localhost:3000';
export const DEMO_URL = process.env.REACT_APP_DEMO_URL || 'http://localhost:3001';
export const API_BASE = process.env.REACT_APP_API_BASE || '';
