SELECT
  toStartOfHour (timeStamp) AS timeStamp,
  requestName,
  answer,
  dnssecFlag,
  requestType,
  tcpFlag,
  edns0Flag,
  country,
  state,
  count() AS cnt_all,
  uniq (requestIP) AS uniq_requestIP,
  uniq (requestPort) AS uniq_requestPort,
  array (
    countIf (
      edns0Size >= 000
      and edns0Size <= 499
    ) as "0 - 499",
    countIf (
      edns0Size >= 500
      and edns0Size <= 999
    ) as "500 - 999",
    countIf (
      edns0Size >= 1000
      and edns0Size <= 1499
    ) as "1000 - 1499",
    countIf (
      edns0Size >= 1500
      and edns0Size <= 1999
    ) as "1500 - 1999",
    countIf (
      edns0Size >= 2000
      and edns0Size <= 2999
    ) as "2000 - 2999",
    countIf (
      edns0Size >= 3000
      and edns0Size <= 3999
    ) as "3000 - 3999",
    countIf (
      edns0Size >= 4000
      and edns0Size <= 4999
    ) as "4000 - 4999",
    countIf (
      edns0Size >= 5000
      and edns0Size <= 5999
    ) as "5000 - 5999",
    countIf (
      edns0Size >= 6000
      and edns0Size <= 6999
    ) as "6000 - 6999",
    countIf (
      edns0Size >= 7000
      and edns0Size <= 7999
    ) as "7000 - 7999",
    countIf (
      edns0Size >= 8000
      and edns0Size <= 8999
    ) as "8000 - 8999",
    countIf (
      edns0Size >= 9000
      and edns0Size <= 9999
    ) as "9000 - 9999"
  ) AS edns_size
FROM
  __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
  timeStamp,
  requestName,
  answer,
  dnssecFlag,
  requestType,
  tcpFlag,
  edns0Flag,
  country,
  state SETTINGS hdx_primary_key = 'timeStamp'
