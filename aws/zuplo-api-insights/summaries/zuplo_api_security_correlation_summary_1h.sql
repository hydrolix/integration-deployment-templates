-- Summary: Zuplo API Security Correlation
-- Granularity: 1 hour
-- Parent table: zuplo_gateway (via __TABLE_NAME__ placeholder)
-- Dimensions retained: route_group, akamai_security_policy, akamai_security_deny_rule,
--   akamai_security_deny_group, gateway_outcome
-- Serves analyses: edge-blocked versus gateway-served comparisons and false-positive candidate review
-- Note: Akamai dimensions are populated on DS2 rows only (null on Zuplo-only rows)

SELECT
  toStartOfHour(timestamp) AS timestamp,
  ifNull(nullIf(route_group, ''), 'unknown') AS route_group,
  ifNull(nullIf(akamai_security_policy, ''), 'none') AS policy_name,
  ifNull(nullIf(akamai_security_deny_rule, ''), 'none') AS akamai_rule_id,
  ifNull(nullIf(akamai_security_deny_group, ''), 'none') AS akamai_edge_action,
  ifNull(nullIf(gateway_outcome, ''), 'unknown') AS gateway_outcome,
  akamai_security_denied,
  count() AS request_count,
  uniq(request_id) AS unique_request_count,
  uniq(client_ip) AS unique_client_ip_count,
  uniq(client_asn) AS unique_client_asn_count,
  -- gateway_latency_ms is sourced from Zuplo plugin telemetry only; DS2 rows
  -- have null gateway_latency_ms because Akamai does not measure gateway latency.
  -- quantileTDigest naturally excludes null values, so this metric reflects
  -- Zuplo-sourced measurements only. This is intentional and correct.
  quantileTDigest(0.95)(gateway_latency_ms) AS p95_gateway_latency_ms
FROM __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  route_group,
  policy_name,
  akamai_rule_id,
  akamai_edge_action,
  gateway_outcome,
  akamai_security_denied
SETTINGS hdx_primary_key = 'timestamp'
