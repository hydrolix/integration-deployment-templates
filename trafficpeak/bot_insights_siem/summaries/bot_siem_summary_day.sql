SELECT
  toStartOfDay (timestamp) AS timestamp,
  bot_type,
  bot_category,
  resource_category,
  attackTypes,
  botScoreRange,
  count() AS cnt_all
FROM
  __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timestamp,
  bot_type,
  bot_category,
  resource_category,
  attackTypes,
  botScoreRange SETTINGS hdx_primary_key = 'timestamp'