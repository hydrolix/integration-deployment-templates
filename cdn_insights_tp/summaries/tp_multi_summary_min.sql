SELECT
  toStartOfMinute (reqTimeSec) as reqTimeSec,
  statusCode,
  cacheStatus,
  edge_pop,
  reqHost,
  country,
  city,
  Edge_GeoInfo, -- this is client_asn // asn
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
  reqTimeSec,
  statusCode,
  cacheStatus,
  edge_pop,
  reqHost,
  country,
  city,
  Edge_GeoInfo,
  user_agent_category,
  hdx_cdn
  SETTINGS hdx_primary_key = 'reqTimeSec'