import http from "k6/http";
import { check, sleep } from "k6";

// ---------- Configuración por variables de entorno ----------
const BASE_URL = __ENV.BASE_URL || "http://localhost:3000";
const KEY_PREFIX = __ENV.KEY_PREFIX || "testkey";
const VALUE = (__ENV.VALUE || "hola").trim();
const TTL_MS = Number(__ENV.TTL_MS || 100000);

// Tasas por escenario (requests por segundo)
const RATE_PING = Number(__ENV.RATE_PING || 50);
const RATE_PUTGET = Number(__ENV.RATE_PUTGET || 25);

// Duración total de la prueba
const DURATION = __ENV.DURATION || "2m";

// ---------- Opciones de ejecución ----------
export const options = {
  discardResponseBodies: true,
  thresholds: {
    http_req_failed: ["rate<0.01"],           // <1% errores
    http_req_duration: ["p(95)<500"],         // 95% < 500ms
    "checks{scenario:ping}": ["rate>0.99"],
    "checks{scenario:put_get}": ["rate>0.98"],
  },
  scenarios: {
    ping: {
      executor: "constant-arrival-rate",
      rate: RATE_PING,
      timeUnit: "1s",
      duration: DURATION,
      preAllocatedVUs: Math.max(10, RATE_PING),
      exec: "scenarioPing",
      tags: { scenario: "ping" },
    },
    put_get: {
      executor: "constant-arrival-rate",
      rate: RATE_PUTGET,
      timeUnit: "1s",
      duration: DURATION,
      preAllocatedVUs: Math.max(20, RATE_PUTGET),
      exec: "scenarioPutGet",
      tags: { scenario: "put_get" },
    },
  },
};

// ---------- Helpers ----------
function uniqueKey() {
  // clave única por tiempo + VU + iteración
  return `${KEY_PREFIX}_${__VU}_${__ITER}_${Date.now()}`;
}

// ---------- Escenario: Ping ----------
export function scenarioPing() {
  const res = http.get(`${BASE_URL}/ping`, {
    headers: { Accept: "application/json" },
  });
  check(res, {
    "ping 200": (r) => r.status === 200,
  });
  sleep(0.1);
}

// ---------- Escenario: Put con TTL + Get ----------
export function scenarioPutGet() {
  const key = uniqueKey();

  // PUT con TTL
  const putRes = http.put(
    `${BASE_URL}/kv/${encodeURIComponent(key)}`,
    JSON.stringify({ value: VALUE, ttl: TTL_MS }),
    { headers: { "Content-Type": "application/json" } }
  );

  // Validar PUT
  check(putRes, {
    "put 200/201": (r) => r.status === 200 || r.status === 201,
  });

  // GET del mismo key
  const getRes = http.get(`${BASE_URL}/kv/${encodeURIComponent(key)}`, {
    headers: { Accept: "application/json" },
  });

  // Validar GET
  check(getRes, {
    "get 200": (r) => r.status === 200,
  });

  // pequeñas pausas ayudan a no saturar el cliente
  sleep(0.1);
}
