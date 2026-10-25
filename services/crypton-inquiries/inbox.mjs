import { openInbox, settings } from './store.mjs';
const inbox = openInbox(settings());
try { console.log(JSON.stringify(inbox.list(), null, 2)); } finally { inbox.close(); }
