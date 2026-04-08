SELECT
  toStartOfHour (timestamp) AS timestamp,
  status, --Unused
  host, --Unused
  country, --Unused
  city, --Unused
  asn, --Unused
  bot_type,
  bot_category,
  ai_category,
  resource_category,
  is_bot_traffic,
  attackTypes,
  botScoreRange,
  count() AS cnt_all,
  sum(bytes) AS totalBytes --Unused
FROM
  __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  status,
  host,
  country,
  city,
  asn,
  bot_type,
  bot_category,
  ai_category,
  resource_category,
  attackTypes,
  botScoreRange,
  is_bot_traffic SETTINGS hdx_primary_key = 'timestamp'
