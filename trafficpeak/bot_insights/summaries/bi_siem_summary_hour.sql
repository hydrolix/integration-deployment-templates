-- Summary: Bot SIEM Summary Hour
-- Derived from deployed `bi_siem_summary_hour` schema
-- Granularity: 60 minutes
-- Parent table: bot_detection_siem (via __SIEM_TABLE_NAME__ placeholder)
-- Dimensions retained: timestamp, request_host, action_taken, client_asn, policy_id

SELECT
  toStartOfHour(timestamp) AS timestamp,
  request_host,
  action_taken,
  client_asn,
  policy_id,
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
FROM __PROJECT_NAME__.__SIEM_TABLE_NAME__
WHERE
  hdx_transform IN ('akamai-siem', 'akamai-siem-gz')
  AND (length(toString(action_taken)) > 0 OR length(toString(auth_outcome)) > 0)
GROUP BY
  timestamp,
  request_host,
  action_taken,
  client_asn,
  policy_id
SETTINGS hdx_primary_key = 'timestamp'
