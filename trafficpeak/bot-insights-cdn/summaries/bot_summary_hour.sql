SELECT
  toStartOfHour (timestamp) as timestamp,
  cacheStatus,
  response_status_code,
  request_host,
  client_country_iso_code,
  client_city,
  client_asn,
  edge_pop,
  user_agent_category,
  resource_category,
  hdx_cdn,
  is_bot_traffic,
  count() AS cnt_all,
  sum(response_total_bytes) AS totalBytes
FROM
  __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  cacheStatus,
  response_status_code,
  request_host,
  client_country_iso_code,
  client_city,
  client_asn,
  edge_pop,
  user_agent_category,
  resource_category,
  is_bot_traffic,
  hdx_cdn SETTINGS hdx_primary_key = 'timestamp'
