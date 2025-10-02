SELECT
  toStartOfMinute (timestamp) AS minute,
  customer,
  payment_method,
  status,
  currency,
  event_type,
  COUNT(*) as event_count,
  SUM(amount) as total_amount,
  AVG(amount) as avg_amount,
  MIN(timestamp) as first_event_time,
  MAX(timestamp) as last_event_time
FROM
   __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  minute,
  customer,
  payment_method,
  status,
  currency,
  event_type SETTINGS hdx_primary_key = 'minute'