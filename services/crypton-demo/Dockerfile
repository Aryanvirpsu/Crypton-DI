FROM node:20-slim

WORKDIR /app

# Install only what server.js needs: express + cookie-parser.
# The package.json includes @crypton/sdk (a workspace package) and React build
# deps — neither is available or needed in this Docker context. Install directly
# so we don't need the workspace root or an npm registry entry for @crypton/sdk.
RUN npm install express@^4 cookie-parser@^1

# Copy the Express OAuth server and its static assets
COPY server.js ./
COPY public ./public

EXPOSE 4000

CMD ["node", "server.js"]
