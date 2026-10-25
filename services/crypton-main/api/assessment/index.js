'use strict';
// POST /api/assessment. The logic lives in ../_lib (underscore folders are not routed by Vercel).
const { createApi } = require('../_lib/handler');

const api = createApi();
module.exports = (req, res) => api.submit(req, res);
