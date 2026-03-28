/**
 * Simple HTTP client pre-configured with service URLs from nucleo.
 *
 * Placeholder values ({{auth_url}}, {{api_url}}, etc.) are replaced
 * at scaffold time with the active nucleo configuration.
 */

const AUTH_URL = process.env.AUTH_URL || "{{auth_url}}";
const API_URL = process.env.API_URL || "{{api_url}}";

async function get(url) {
  const resp = await fetch(url);
  return resp.json();
}

async function post(url, data) {
  const resp = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  return resp.json();
}

module.exports = { get, post, AUTH_URL, API_URL };
