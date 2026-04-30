SELECT
    akamai_geo('country', assumeNotNull(requestIP)) AS country,
    akamai_geo('state', assumeNotNull(requestIP)) AS state,
    akamai_geo('city', assumeNotNull(requestIP)) AS city,
    splitByChar(':', assumeNotNull(answers))[1] as ttl,
    splitByChar(':', assumeNotNull(answers))[2] as answer,
datediff('s', timeStamp, now64(3)) as hdx_source_latency,
* FROM {STREAM}
