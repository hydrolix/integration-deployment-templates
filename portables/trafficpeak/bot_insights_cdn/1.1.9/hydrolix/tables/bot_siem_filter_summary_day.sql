-- Summary: Bot SIEM Filter Summary Day
-- Filter-aware SIEM summary for dashboard-global trend panels
-- Granularity: 1440 minutes (1 day)
-- Parent table: bot_detection_siem (via __SIEM_TABLE_NAME__ placeholder)
-- Dimensions retained: timestamp, request_host, client_asn, is_bot_traffic, ai_category, resource_category

SELECT
  toStartOfDay(timestamp) AS timestamp,
  request_host,
  client_asn,
  is_bot_traffic,
  ai_category,
  resource_category,
  count() AS cnt_all,
  countIf(or(equals(action_taken, 'deny'), equals(action_taken, 'block'))) AS cnt_blocked, -- hdx-noqa: SS-16
  countIf(equals(auth_outcome, 'fail')) AS cnt_auth_fail, -- hdx-noqa: SS-16
  countIf(like(business_outcome, '%fail%')) AS cnt_biz_fail, -- hdx-noqa: SS-16
  avg(bot_score) AS avg_bot_score,
  countIf(and(greaterOrEquals(toUInt16(response_status_code), 200), less(toUInt16(response_status_code), 300))) AS cnt_2xx, -- hdx-noqa: SS-16
  countIf(and(greaterOrEquals(toUInt16(response_status_code), 400), less(toUInt16(response_status_code), 500))) AS cnt_4xx, -- hdx-noqa: SS-16
  countIf(greaterOrEquals(toUInt16(response_status_code), 500)) AS cnt_5xx, -- hdx-noqa: SS-16
  uniq(client_ip) AS uniq_client_ip,
  countIf(equals(cache_was_cached, false)) AS cnt_cache_miss -- hdx-noqa: SS-16
FROM akamai.siem
WHERE
  hdx_transform IN ('akamai-siem', 'akamai-siem-gz')
  AND (length(toString(action_taken)) > 0 OR length(toString(auth_outcome)) > 0)
GROUP BY
  timestamp,
  request_host,
  client_asn,
  is_bot_traffic,
  ai_category,
  resource_category
SETTINGS hdx_primary_key = 'timestamp'
