<p align="center">
  <img src="assets/banner.svg" alt="MCP Odoo Architecture" width="100%"/>
</p>

<h1 align="center">MCP Odoo</h1>

<p align="center">
  <a href="https://github.com/fgribreau/mcp-odoo/actions/workflows/ci.yml"><img src="https://github.com/fgribreau/mcp-odoo/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"/></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-stable-orange.svg" alt="Rust"/></a>
  <a href="https://modelcontextprotocol.io"><img src="https://img.shields.io/badge/MCP-2024--11--05-6366f1.svg" alt="MCP"/></a>
  <a href="https://www.odoo.com"><img src="https://img.shields.io/badge/odoo-16%2B-a855f7.svg" alt="Odoo 16+"/></a>
</p>

<p align="center">
  <strong>A Model Context Protocol (MCP) server that exposes any Odoo instance to Claude and other MCP-compatible AI assistants — the native JSON/2 API on Odoo 19+, the classic JSON-RPC external API on Odoo 16–18, auto-detected.</strong>
</p>

---

## Sponsors

<table>
  <tr>
    <td align="center" width="175">
      <a href="https://france-nuage.fr/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=france-nuage&mtm_content=mcp-odoo">
        <img src="assets/sponsors/france-nuage.svg" height="60" alt="France-Nuage"/><br/>
        <b>France-Nuage</b>
      </a><br/>
      <sub>Host Odoo on sovereign French cloud. Re-internalise when you choose.</sub>
    </td>
    <td align="center" width="175">
      <a href="https://www.hook0.com/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=hook0&mtm_content=mcp-odoo">
        <img src="assets/sponsors/hook0.png" height="60" alt="Hook0"/><br/>
        <b>Hook0</b>
      </a><br/>
      <sub>Send signed Odoo webhooks reliably. Self-hosted, retries handled.</sub>
    </td>
    <td align="center" width="175">
      <a href="https://getnatalia.com/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=natalia&mtm_content=mcp-odoo">
        <img src="assets/sponsors/natalia.svg" height="60" alt="Natalia"/><br/>
        <b>Natalia</b>
      </a><br/>
      <sub>AI voice agent feeds qualified leads straight into your Odoo CRM 24/7.</sub>
    </td>
    <td align="center" width="175">
      <a href="https://netir.fr/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=netir&mtm_content=mcp-odoo">
        <img src="assets/sponsors/netir.svg" height="60" alt="Netir"/><br/>
        <b>Netir</b>
      </a><br/>
      <sub>Hire vetted French freelance Odoo devs via mentored marketplace.</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="233">
      <a href="https://nobullshitconseil.com/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=nbc&mtm_content=mcp-odoo">
        <img src="assets/sponsors/nobullshitconseil.svg" height="60" alt="NoBullshitConseil"/><br/>
        <b>NoBullshitConseil</b>
      </a><br/>
      <sub>No-bullshit tech advisory. ERP &amp; platform strategy for execs.</sub>
    </td>
    <td align="center" width="233">
      <a href="https://qualneo.fr/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=qualneo&mtm_content=mcp-odoo">
        <img src="assets/sponsors/qualneo.svg" height="60" alt="Qualneo"/><br/>
        <b>Qualneo</b>
      </a><br/>
      <sub>Qualiopi LMS that syncs trainees &amp; invoices to your Odoo backend.</sub>
    </td>
    <td align="center" width="233">
      <a href="https://recapro.ai/?mtm_source=github&mtm_medium=sponsor&mtm_campaign=recapro&mtm_content=mcp-odoo">
        <img src="assets/sponsors/recapro.png" height="60" alt="Recapro"/><br/>
        <b>Recapro</b>
      </a><br/>
      <sub>Sovereign AI logs sales calls into Odoo activities automatically.</sub>
    </td>
  </tr>
</table>

> **Interested in sponsoring?** [Get in touch](mailto:rust@fgribreau.com)

## Overview

A Rust MCP server that **dynamically discovers** your Odoo models and exposes them as tools. Claude (or any MCP client) can browse, search, read, create, update, and delete records on your Odoo instance — no plugin to install on the Odoo side, just an API key.

It speaks two wire protocols behind one identical tool surface and picks the right one automatically at startup:

- **Odoo 19+** — the native **JSON/2** API (`POST /json/2/{model}/{method}`, Bearer API key).
- **Odoo 16–18** — the classic **JSON-RPC** external API (`POST /jsonrpc`, `execute_kw`), which also needs the user login (`ODOO_LOGIN`).

### Features

- **Odoo 16 → 19** — one binary, JSON/2 on 19+ and classic JSON-RPC on 16–18, auto-detected
- **Dynamic discovery** — automatically lists and introspects all accessible Odoo models
- **Full CRUD** — `search`, `read`, `create`, `write`, `delete` records on any model
- **Arbitrary methods** — call any public ORM method via `call_method`
- **Glob filtering** — include/exclude models with patterns (`sale.*`, `!ir.logging`)
- **Read-only mode** — block all write operations for safe production observability
- **Pagination** — configurable page size with `has_more` / `next_offset` metadata
- **Structured errors** — HTTP 401 / 403 / 404 / 422 / 500 mapped to MCP errors
- **Fast & small** — single Rust binary, no runtime dependencies

## Install

### Homebrew (macOS / Linux)

```bash
brew tap FGRibreau/tap
brew install mcp-server-odoo
```

### Cargo

```bash
cargo install mcp-server-odoo
```

### `cargo binstall` (prebuilt binary, no compile)

```bash
cargo binstall mcp-server-odoo
```

### Prebuilt binaries

Grab a tarball/zip for your platform from [Releases](https://github.com/fgribreau/mcp-odoo/releases). Targets shipped:

`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`.

### From source

```bash
git clone https://github.com/fgribreau/mcp-odoo.git
cd mcp-odoo
cargo build --release
```

## Quick Start

### Prerequisites

- An Odoo instance with API access (Odoo 16 or newer)
- An Odoo API key — see below (on Odoo 16–18 the classic API also accepts the user password)
- On Odoo 16–18, the user login/email (`ODOO_LOGIN`)

### 1. Configure

```bash
cp .env.example .env
# Edit .env with your values — every variable is documented inline
```

### 2. Add to Claude Code

```bash
claude mcp add odoo \
  --command mcp-server-odoo \
  --env "ODOO_URL=https://your-odoo-instance.com" \
  --env "ODOO_API_KEY=YOUR_API_KEY" \
  --env "ODOO_DB=your-database"
```

<details>
<summary>How to get your Odoo API key</summary>

1. Log into Odoo as the user you want the MCP server to act as
2. Click your avatar (top-right) → **My Profile** → **Account Security** tab
3. Under **API Keys**, click **New API Key**
4. Set **Name** to `mcp-server`, **Scope** to `rpc`, choose an expiration date
5. Click **Generate Key** — **copy it immediately**, it is shown only once

**Verify it works:**

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST https://your-odoo.com/json/2/res.users/search \
  -H "Authorization: Bearer YOUR_KEY" \
  -H "Content-Type: application/json" \
  -H "X-Odoo-Database: your-db" \
  -d '{"domain": [["id","=",1]]}'
# 200 = valid, 401 = invalid key, 403 = missing permissions
```

> **Security:** for production, create a dedicated Odoo user with minimal access rights, restrict its groups to only the models you need, and rotate keys regularly.

</details>

## Configuration

### Claude Code

The recommended way is via the CLI:

```bash
claude mcp add odoo \
  --command /absolute/path/to/mcp-server-odoo \
  --env "ODOO_URL=https://your-odoo-instance.com" \
  --env "ODOO_API_KEY=YOUR_API_KEY" \
  --env "ODOO_DB=your-database" \
  --env "READ_ONLY=true"
```

Or manually in your MCP settings file:

```json
{
  "mcpServers": {
    "odoo": {
      "command": "/absolute/path/to/mcp-server-odoo",
      "env": {
        "ODOO_URL": "https://your-odoo-instance.com",
        "ODOO_API_KEY": "YOUR_API_KEY",
        "ODOO_DB": "your-database",
        "READ_ONLY": "true"
      }
    }
  }
}
```

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "odoo": {
      "command": "/absolute/path/to/mcp-server-odoo",
      "env": {
        "ODOO_URL": "https://your-odoo-instance.com",
        "ODOO_API_KEY": "YOUR_API_KEY",
        "ODOO_DB": "your-database"
      }
    }
  }
}
```

### Environment variables

All options can be set via env vars **or** CLI flags (`--odoo-url`, `--odoo-api-key`, `--odoo-db`, ...). The `.env.example` file contains exhaustive documentation for every variable, including where to find each value, how to verify it, and security recommendations.

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ODOO_URL` | **yes** | — | Base URL of your Odoo instance (no trailing slash) |
| `ODOO_API_KEY` | **yes** | — | API key. On Odoo 19+ (JSON/2) this must be a Bearer API key; on Odoo 16–18 (JSON-RPC) it may be the API key **or** the user password |
| `ODOO_DB` | **yes** | — | PostgreSQL database name |
| `ODOO_PROTOCOL` | no | `auto` | Wire protocol: `auto` (detect from server version), `json2` (Odoo 19+), or `jsonrpc` (Odoo 16–18) |
| `ODOO_LOGIN` | conditional | — | User login/email. **Required** when the resolved protocol is `jsonrpc` (Odoo 16–18); ignored for `json2` |
| `MODEL_INCLUDE` | no | `*` | Comma-separated glob patterns for models to expose |
| `MODEL_EXCLUDE` | no | *(empty)* | Comma-separated glob patterns for models to hide |
| `READ_ONLY` | no | `false` | Block all write operations |
| `PAGE_SIZE` | no | `80` | Default records per `search` page |

### Odoo versions & protocols

The server talks to Odoo over one of two wire protocols and exposes the exact same tools either way:

| Odoo version | Protocol | Transport | Extra config |
|--------------|----------|-----------|--------------|
| 19+ | `json2` | `POST /json/2/{model}/{method}` with a Bearer API key | — |
| 16–18 | `jsonrpc` | `POST /jsonrpc` → `object.execute_kw` | `ODOO_LOGIN` (user login/email) |

With `ODOO_PROTOCOL=auto` (the default) the server calls the unauthenticated `/web/webclient/version_info` endpoint once at startup and picks `json2` for Odoo ≥ 19, `jsonrpc` otherwise. On the JSON-RPC path it authenticates once at startup (login + key/password) and caches the resulting user id. Set `ODOO_PROTOCOL` explicitly to skip auto-detection (e.g. behind a reverse proxy that masks the version).

### Model filtering

Glob patterns let you control which Odoo models are exposed. **Exclude always takes precedence over include.**

```bash
# Only sales + contacts
MODEL_INCLUDE="sale.*,res.partner"

# Everything except internals and logs
MODEL_EXCLUDE="ir.*,bus.*,base.*"

# Accounting except sequences
MODEL_INCLUDE="account.*"
MODEL_EXCLUDE="account.sequence*"
```

<details>
<summary>Common Odoo model prefixes</summary>

| Prefix | Module | Examples |
|--------|--------|----------|
| `res.*` | Core resources | `res.partner`, `res.users`, `res.company` |
| `sale.*` | Sales | `sale.order`, `sale.order.line` |
| `purchase.*` | Purchasing | `purchase.order`, `purchase.order.line` |
| `account.*` | Accounting | `account.move`, `account.payment`, `account.journal` |
| `stock.*` | Inventory | `stock.picking`, `stock.move`, `stock.warehouse` |
| `hr.*` | HR | `hr.employee`, `hr.contract`, `hr.leave` |
| `project.*` | Projects | `project.project`, `project.task` |
| `crm.*` | CRM | `crm.lead`, `crm.team` |
| `product.*` | Products | `product.product`, `product.template` |
| `ir.*` | Internal/system (usually excluded) | `ir.model`, `ir.cron`, `ir.logging` |
| `bus.*` | Real-time bus (usually excluded) | `bus.bus`, `bus.presence` |

</details>

### Read-only mode

When `READ_ONLY=true`, the server blocks `create`, `write`, `delete`, and most `call_method` invocations. The following methods remain **always allowed**:

| Always allowed | Prefix-based |
|---------------|--------------|
| `name_get`, `name_search`, `read_group` | `get_*` |
| `fields_get`, `search`, `search_read` | `check_*`, `has_*` |
| `search_count`, `default_get` | `is_*`, `can_*` |

> **Recommended:** set `READ_ONLY=true` when connecting to production, exploring data, or first-time setup.

## Usage examples

Once configured, you can ask Claude questions like:

- *"List my 10 most recent sale orders with their total and customer."*
- *"Find all contacts at companies in France that bought from us in the last 90 days."*
- *"Show me the stock levels for products in the warehouse 'WH/Stock'."*
- *"Create a new opportunity for Acme Corp with expected revenue €25,000."*
- *"Mark sale order SO0042 as confirmed."*

Claude will pick the right tool (`search`, `read`, `create`, `write`, `call_method`, ...) and run it on your Odoo instance.

## Available tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `list_models` | *(none)* | List all accessible models (filtered by config) |
| `describe_model` | `model` | Get field definitions for a model |
| `search` | `model`, `domain`, `fields?`, `limit?`, `offset?`, `order?` | Search records with pagination |
| `read` | `model`, `ids`, `fields?` | Read records by ID |
| `create` | `model`, `values` | Create a new record |
| `write` | `model`, `ids`, `values` | Update existing records |
| `delete` | `model`, `ids` | Delete records by ID |
| `call_method` | `model`, `method`, `ids?`, `kwargs?` | Call any public ORM method |

### Examples

**Search for company customers:**
```json
{
  "model": "res.partner",
  "domain": [["is_company", "=", true], ["customer_rank", ">", 0]],
  "fields": ["name", "email", "country_id"],
  "limit": 10
}
```

**Read a specific sale order:**
```json
{
  "model": "sale.order",
  "ids": [42],
  "fields": ["name", "state", "amount_total", "partner_id"]
}
```

**Create a contact:**
```json
{
  "model": "res.partner",
  "values": { "name": "Acme Corp", "is_company": true, "email": "info@acme.com" }
}
```

**Confirm a sale order via `call_method`:**
```json
{ "model": "sale.order", "method": "action_confirm", "ids": [42] }
```

## CLI reference

```
mcp-server-odoo [OPTIONS]

Options:
      --odoo-url <ODOO_URL>          URL of the Odoo instance [env: ODOO_URL]
      --odoo-api-key <ODOO_API_KEY>  API key (or password on the JSON-RPC external API) [env: ODOO_API_KEY]
      --odoo-db <ODOO_DB>            Odoo database name [env: ODOO_DB]
      --odoo-protocol <PROTOCOL>     Wire protocol: auto, json2, jsonrpc [env: ODOO_PROTOCOL] [default: auto]
      --odoo-login <ODOO_LOGIN>      User login/email — required for the JSON-RPC protocol [env: ODOO_LOGIN]
      --model-include <PATTERNS>     Comma-separated glob patterns for inclusion [env: MODEL_INCLUDE] [default: *]
      --model-exclude <PATTERNS>     Comma-separated glob patterns for exclusion [env: MODEL_EXCLUDE]
      --read-only                    Block all write operations [env: READ_ONLY]
      --page-size <PAGE_SIZE>        Default page size for list operations [env: PAGE_SIZE] [default: 80]
  -h, --help                         Print help
  -V, --version                      Print version
```

## Development

```bash
# Build debug version
cargo build

# Run unit tests
cargo test

# Run unit + integration tests (requires a live Odoo instance)
cargo test --features integration

# Run with verbose logging
RUST_LOG=debug ./target/debug/mcp-server-odoo
```

Integration tests are **black-box** (no mocks): they drive the public library API against a **real** Odoo server. The same suite runs unchanged against both protocols; only the environment differs, which is what proves the two transports behave identically.

Env vars: `ODOO_TEST_URL`, `ODOO_TEST_DB`, `ODOO_TEST_API_KEY`, and — for the JSON-RPC path — `ODOO_TEST_LOGIN`. `ODOO_TEST_PROTOCOL` (default `auto`) mirrors `ODOO_PROTOCOL`.

**Odoo 19 (JSON/2).** `docker-compose.test.yml` spins up a disposable Odoo 19 + Postgres. JSON/2 needs a real Bearer API key; generate one with `odoo shell`:

```bash
docker compose -p mcp-odoo-it19 -f docker-compose.test.yml up -d
until curl -sf http://localhost:18069/web/login; do sleep 2; done
KEY=$(docker compose -p mcp-odoo-it19 -f docker-compose.test.yml exec -T odoo \
  odoo shell -d test_odoo --no-http --db_host=db --db_user=odoo --db_password=odoo <<'PY' 2>/dev/null | sed -n 's/^APIKEY=//p'
admin = env.ref('base.user_admin')
print('APIKEY=' + env['res.users.apikeys'].with_user(admin)._generate('rpc', 'mcp-it', False))
env.cr.commit()
PY
)
ODOO_TEST_URL=http://localhost:18069 ODOO_TEST_DB=test_odoo ODOO_TEST_API_KEY=$KEY \
  cargo test --features integration
docker compose -p mcp-odoo-it19 -f docker-compose.test.yml down -v
```

**Odoo 16 (JSON-RPC).** `docker-compose.test16.yml` spins up a disposable Odoo 16 + Postgres. The classic API accepts the admin password directly, so no key wizard is needed:

```bash
docker compose -p mcp-odoo-it16 -f docker-compose.test16.yml up -d
until curl -sf http://localhost:16069/web/login; do sleep 2; done
ODOO_TEST_URL=http://localhost:16069 ODOO_TEST_DB=test_odoo \
  ODOO_TEST_LOGIN=admin ODOO_TEST_API_KEY=admin ODOO_TEST_PROTOCOL=auto \
  cargo test --features integration
docker compose -p mcp-odoo-it16 -f docker-compose.test16.yml down -v
```

## Troubleshooting

### `401 Unauthorized`

1. Verify your API key is correct (`ODOO_API_KEY`).
2. Check that the key has scope `rpc` and has not expired.
3. Make sure the user owning the key has at least *read* permission on the models you query.

### `403 Forbidden`

The key is valid but the underlying user lacks access to a specific model. Adjust the user's groups in Odoo, or add the model to `MODEL_EXCLUDE` if you don't need it.

### `404 Not Found` on `/json/2/...`

The JSON/2 endpoint only exists on **Odoo 19+**. With `ODOO_PROTOCOL=auto` (the default) the server detects this and uses the classic JSON-RPC external API instead, so you should not see this error. If you forced `ODOO_PROTOCOL=json2` against an older server, drop the override (or set `jsonrpc`) and provide `ODOO_LOGIN`.

### `ODOO_LOGIN is required for the JSON-RPC transport`

The server resolved to the classic JSON-RPC protocol (Odoo 16–18) but no user login was given. Set `ODOO_LOGIN` to the login/email of the API user.

### `Could not connect` / timeouts

1. Verify `ODOO_URL` is reachable from your machine (try a `curl`).
2. Check for firewalls / VPN / Cloudflare Access in front of the instance.
3. Make sure the URL includes the protocol (`https://`).

### "No models available"

Your `MODEL_INCLUDE` / `MODEL_EXCLUDE` patterns may be filtering everything out. Run with `RUST_LOG=debug` to see what was discovered and what was filtered.

## Contributing

Contributions are welcome — please open an issue first if you plan a substantial change. Run `cargo fmt && cargo clippy --all-targets -- -D warnings` before submitting a PR.

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- Built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk) — Rust MCP SDK
- Inspired by the [Model Context Protocol](https://modelcontextprotocol.io/) specification
- Designed for the [Odoo JSON/2 API](https://www.odoo.com/documentation/19.0/developer/reference/external_api.html) (Odoo 19+)
