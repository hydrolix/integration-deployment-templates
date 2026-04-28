-- Summary: Bot Aggregate Path Minute
-- Derived from deployed `bot_agg_path_minute` schema
-- Granularity: 1 minute
-- Parent table: bot_detection (via __TABLE_NAME__ placeholder)
-- Dimensions retained: timestamp, request_host, request_path_norm, bot_class, asn_type

SELECT
  toStartOfMinute(reqTimeSec) AS timestamp,
  request_host,
  request_path_norm,
  bot_class,
  asn_type,
  count() AS cnt_all,
  countIf(and(greaterOrEquals(toUInt16(response_status_code), 200), less(toUInt16(response_status_code), 300))) AS cnt_2xx, -- hdx-noqa: SS-16
  countIf(and(greaterOrEquals(toUInt16(response_status_code), 400), less(toUInt16(response_status_code), 500))) AS cnt_4xx, -- hdx-noqa: SS-16
  countIf(equals(toUInt16(response_status_code), 429)) AS cnt_429, -- hdx-noqa: SS-16
  countIf(greaterOrEquals(toUInt16(response_status_code), 500)) AS cnt_5xx, -- hdx-noqa: SS-16
  countIf(equals(cache_was_cached, true)) AS cnt_cached, -- hdx-noqa: SS-16
  countIf(equals(cache_was_cached, false)) AS cnt_cache_miss, -- hdx-noqa: SS-16
  avg(time_to_first_byte_ms) AS avg_ttfb,
  avg(origin_time_to_first_byte_ms) AS avg_origin_ttfb,
  quantileTDigest(0.95)(origin_time_to_first_byte_ms) AS p95_origin_ttfb,
  quantileTDigest(0.99)(origin_time_to_first_byte_ms) AS p99_origin_ttfb,
  uniq(client_ip) AS uniq_client_ip,
  uniq(request_query_string) AS uniq_qs,
  avg(hdx_source_latency_sec) AS avg_hdx_latency
FROM akamai.logs
WHERE length(request_path_norm) > 0
GROUP BY
  timestamp,
  request_host,
  request_path_norm,
  bot_class,
  asn_type
SETTINGS hdx_primary_key = 'timestamp'
