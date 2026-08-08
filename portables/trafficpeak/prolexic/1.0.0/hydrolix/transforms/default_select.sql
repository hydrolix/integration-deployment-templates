SELECT 
    if(eventType = '', 'unknown', eventType) as eventType,
    if(service = '', 'unknown', service) as service,
    if(isOngoing = 'true', 'ONGOING', 'ENDED') as status,
  * EXCEPT (eventType, service, status)
FROM {STREAM}