SELECT
  toStartOfHour (reqTimeSec) AS timestamp,
  cacheStatus,
  statusCode,
  reqHost,
  country,
  city,
  asn,
  user_agent_category,
  ai_category,
  resource_category,
  is_bot_traffic,
  count() AS cnt_all,
  sum(totalBytes) AS totalBytes
FROM
  __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  cacheStatus,
  statusCode,
  reqHost,
  country,
  city,
  asn,
  user_agent_category,
  ai_category,
  resource_category,
  is_bot_traffic SETTINGS hdx_primary_key = 'timestamp'