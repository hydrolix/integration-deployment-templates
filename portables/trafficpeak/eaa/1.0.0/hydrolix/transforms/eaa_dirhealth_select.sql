SELECT
datediff('s', datetime, now64(3)) as hdx_source_latency,
arrayMap(x -> x['agent_infra_type'], connectors) as connectorAgentInfraType,
arrayMap(x -> x['created_at'], connectors) as connectorCreatedAt,
arrayMap(x -> x['name'], connectors) as connectorName,
arrayMap(x -> x['package'], connectors) as connectorPackage,
arrayMap(x -> x['reach'], connectors) as connectorReach,
arrayMap(x -> x['state'], connectors) as connectorState,
arrayMap(x -> x['status'], connectors) as connectorStatus,
arrayMap(x -> x['uuid_url'], connectors) as connectorUuidUrl,
* FROM {STREAM}
