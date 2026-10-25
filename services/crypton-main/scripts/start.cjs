// CRA 5 derives allowedHosts from a LAN address, which can be absent on VPN/offline hosts.
// Keep host checks explicit and keep the development server on loopback.
process.env.HOST = '127.0.0.1';
const configPath = require.resolve('react-scripts/config/webpackDevServer.config');
const configure = require(configPath);
require.cache[configPath].exports = (...args) => ({ ...configure(...args), allowedHosts: ['localhost', '127.0.0.1'] });
require('react-scripts/scripts/start');
