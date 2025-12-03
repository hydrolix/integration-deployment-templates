SELECT
  toStartOfMinute (reqTimeSec) as timestamp,
  statusCode as response_status_code,
  cacheStatus as cache_was_cached,
  edge_pop,
  reqHost as request_host,
  country as client_country_iso_code,
  city,
  Edge_GeoInfo as client_asn,
  user_agent_category,
  hdx_cdn,
  count() as cnt_all,
  sum(totalBytes) as response_total_bytes,
  avg(timeToFirstByte) as response_ttfb_ms,
  avg(transferTimeMSec) as response_ttlb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (timeToFirstByte) AS quantiles_response_ttfb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (transferTimeMSec) AS quantiles_response_ttlb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (origin_time_to_first_byte_ms) AS quantiles_origin_ttfb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (origin_time_to_last_byte_ms) AS quantiles_origin_ttlb_ms
FROM
  __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  response_status_code,
  cache_was_cached,
  edge_pop,
  request_host,
  client_country_iso_code,
  city,
  client_asn,
  user_agent_category,
  hdx_cdn
  SETTINGS hdx_primary_key = 'timestamp'