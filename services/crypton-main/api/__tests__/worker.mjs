// One independent API instance, as its own OS process. Used by the database tests to prove
// that duplicate and concurrent requests stay correct across instances that share nothing
// but the database. Prints "PORT <n>" when ready.
import { startApiServer } from '../../scripts/dev-api.mjs';

const server = await startApiServer({ env: process.env, port: 0 });
console.log(`PORT ${server.address().port}`);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close(() => process.exit(0)));
