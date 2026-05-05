SELECT
  toStartOfMinute (reqTimeSec) as reqTimeSec,
  statusCode,
  cacheStatus,
  Edge_GeoInfo,
  reqHost,
  country,
  city,
  asn,
  user_agent_category,
  hdx_cdn,
  count() as cnt_all,
  sum(totalBytes) as response_total_bytes,
  avg(timeToFirstByte) as response_ttfb_ms,
  avg(transferTimeMSec) as response_ttlb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (timeToFirstByte) AS quantiles_response_ttfb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (transferTimeMSec) AS quantiles_response_ttlb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (Origin_TurnAroundTime) AS quantiles_origin_ttfb_ms,
  quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (Origin_RequestEndTime) AS quantiles_origin_ttlb_ms
FROM
  __PROJECT_NAME__.logs
GROUP BY
  reqTimeSec,
  statusCode,
  cacheStatus,
  Edge_GeoInfo,
  reqHost,
  country,
  city,
  asn,
  user_agent_category,
  hdx_cdn
  SETTINGS hdx_primary_key = 'reqTimeSec'
