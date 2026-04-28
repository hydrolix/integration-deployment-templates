-- Summary: Bot SIEM Class Day
-- Classification-oriented SIEM evidence by reporting bucket
-- Granularity: 1440 minutes (1 day)
-- Parent table: bot_detection_siem (via __SIEM_TABLE_NAME__ placeholder)
-- Dimensions retained: timestamp, request_host, client_asn, akamai_canonical_bot_class, bot_category, bot_category_source, bot_type

SELECT
  toStartOfDay(timestamp) AS timestamp,
  request_host,
  client_asn,
  akamai_canonical_bot_class,
  bot_category,
  bot_category_source,
  bot_type,
  count() AS cnt_all,
  avg(bot_score) AS avg_bot_score,
  avg(origin_time_to_first_byte_ms) AS avg_origin_ttfb,
  countIf(equals(cache_was_cached, false)) AS cnt_cache_miss, -- hdx-noqa: SS-16
  uniq(client_ip) AS uniq_client_ip
FROM __PROJECT_NAME__.__SIEM_TABLE_NAME__
WHERE
  hdx_transform IN ('akamai-siem', 'akamai-siem-gz')
  AND notEmpty(toString(client_asn))
GROUP BY
  timestamp,
  request_host,
  client_asn,
  akamai_canonical_bot_class,
  bot_category,
  bot_category_source,
  bot_type
SETTINGS hdx_primary_key = 'timestamp'
