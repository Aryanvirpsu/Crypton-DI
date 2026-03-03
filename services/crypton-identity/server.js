const https = require("https");
const fs = require("fs");
const path = require("path");
const express = require("express");

const app = express();
app.use(express.static(__dirname));

https.createServer(
  {
    key: fs.readFileSync(path.join(__dirname, "crypton.local-key.pem")),
    cert: fs.readFileSync(path.join(__dirname, "crypton.local.pem")),
  },
  app
).listen(3000, "0.0.0.0", () => {
  console.log("HTTPS running: https://crypton.local:3000");
});