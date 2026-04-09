-- Summary: Zuplo API Overview
-- Granularity: 1 minute
-- Parent table: zuplo_gateway (via __TABLE_NAME__ placeholder)
-- Dimensions retained: route_group, gateway_outcome, auth_outcome, rate_limit_outcome, response_status_code
-- Serves analyses: volume, errors, latency, auth outcomes, throttling outcomes

SELECT
  toStartOfMinute(timestamp) AS timestamp,
  ifNull(nullIf(route_group, ''), 'unknown') AS route_group,
  ifNull(nullIf(gateway_outcome, ''), 'unknown') AS gateway_outcome,
  ifNull(nullIf(auth_outcome, ''), 'unknown') AS auth_outcome,
  ifNull(nullIf(rate_limit_outcome, ''), 'unknown') AS rate_limit_outcome,
  response_status_code,
  count() AS request_count,
  uniq(request_id) AS unique_request_count,
  uniq(consumer_id) AS unique_consumer_count,
  avg(gateway_latency_ms) AS avg_gateway_latency_ms,
  quantileTDigest(0.50)(gateway_latency_ms) AS p50_gateway_latency_ms,
  quantileTDigest(0.95)(gateway_latency_ms) AS p95_gateway_latency_ms,
  quantileTDigest(0.99)(gateway_latency_ms) AS p99_gateway_latency_ms
FROM __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  route_group,
  gateway_outcome,
  auth_outcome,
  rate_limit_outcome,
  response_status_code
SETTINGS hdx_primary_key = 'timestamp'
