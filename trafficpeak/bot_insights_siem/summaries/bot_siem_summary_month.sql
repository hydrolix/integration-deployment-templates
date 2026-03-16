SELECT
 toStartOfMonth(timestamp)::DateTime AS timestamp,
  bot_type,
  bot_category,
  resource_category,
  attackTypes,
  botScoreRange,
  count() AS cnt_all
FROM
  akamai.siem
GROUP BY
  timestamp,
  bot_type,
  bot_category,
  resource_category,
  attackTypes,
  botScoreRange SETTINGS hdx_primary_key = 'timestamp'