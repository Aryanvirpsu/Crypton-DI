/**
 * crypton-admin surface config
 * Set these in services/crypton-admin/.env (or .env.local for local overrides)
 */
export const MAIN_URL = process.env.REACT_APP_MAIN_URL || 'https://cryptonid.tech';
export const DEMO_URL = process.env.REACT_APP_DEMO_URL || 'https://demo.cryptonid.tech';
export const API_BASE = process.env.REACT_APP_API_BASE || 'https://auth.cryptonid.tech';
