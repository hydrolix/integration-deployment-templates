-- Summary: Zuplo API Abuse
-- Granularity: 5 minutes
-- Parent table: zuplo_gateway (via __TABLE_NAME__ placeholder)
-- Dimensions retained: client_asn, client_country, api_key_id, route_group,
--   akamai_security_policy, akamai_security_deny_rule, akamai_security_deny_group
-- Serves analyses: suspicious traffic attribution, blocked and throttled activity, edge policy evidence
-- Note: Akamai dimensions are populated on DS2 rows only (null on Zuplo-only rows)

SELECT
  toStartOfInterval(timestamp, INTERVAL 5 MINUTE) AS timestamp,
  ifNull(nullIf(client_asn, ''), 'unknown') AS client_asn,
  ifNull(nullIf(coalesce(client_country, client_country_iso_code), ''), 'unknown') AS client_country,
  ifNull(nullIf(api_key_id, ''), 'unknown') AS api_key_id,
  ifNull(nullIf(route_group, ''), 'unknown') AS route_group,
  ifNull(nullIf(akamai_security_policy, ''), 'none') AS policy_name,
  ifNull(nullIf(akamai_security_deny_rule, ''), 'none') AS akamai_rule_id,
  ifNull(nullIf(akamai_security_deny_group, ''), 'none') AS akamai_edge_action,
  ifNull(nullIf(gateway_outcome, ''), 'unknown') AS gateway_outcome,
  ifNull(nullIf(auth_outcome, ''), 'unknown') AS auth_outcome,
  ifNull(nullIf(rate_limit_outcome, ''), 'unknown') AS rate_limit_outcome,
  akamai_security_denied,
  count() AS request_count,
  uniq(request_id) AS unique_request_count,
  uniq(client_ip) AS unique_client_ip_count,
  uniq(consumer_id) AS unique_consumer_count
FROM __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  client_asn,
  client_country,
  api_key_id,
  route_group,
  policy_name,
  akamai_rule_id,
  akamai_edge_action,
  gateway_outcome,
  auth_outcome,
  rate_limit_outcome,
  akamai_security_denied
SETTINGS hdx_primary_key = 'timestamp'
