const { createProxyMiddleware } = require('http-proxy-middleware');
module.exports = function setupProxy(app) {
  app.use(createProxyMiddleware('/api/assessment', { target: 'http://127.0.0.1:8091', changeOrigin: false }));
};
