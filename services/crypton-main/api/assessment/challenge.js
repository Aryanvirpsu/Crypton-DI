'use strict';
// GET /api/assessment/challenge: a signed, short-lived form token. Stateless; touches no database.
const { createApi } = require('../_lib/handler');

const api = createApi();
module.exports = (req, res) => api.challenge(req, res);
