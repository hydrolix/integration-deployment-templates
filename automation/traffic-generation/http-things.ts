// geo-data.ts
// Standalone module for create fake http things
// Usage: 
// import {randomUserAgent, randomHttpStatusCode, randomOrganizationName, randomRoutePath }
//    from "https://raw.githubusercontent.com/hydrolix/integration-deployment-templates/refs/heads/main/automation/traffic-generation/http-things.ts";
//

/**
 * Generates a random user agent string
 * @returns {string} Random user agent
 * @example
 * const ua = randomUserAgent();
 * console.log(ua); // "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
 */
export function randomUserAgent(): string {
  const userAgents = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15",
    "VLC/3.0.16 LibVLC/3.0.16",
    "ExoPlayer/2.18.1",
    "Wombat Rusty Chicken OS 14_0 like Mac OS X) AppleWokKit/1.1",
    "Zuplo/6.51.9",
    "Zuplo/6.52.1",
    "Zuplo/6.53.0"
  ];
  return userAgents[Math.floor(Math.random() * userAgents.length)];
}

/**
 * Generates a random HTTP status code
 * @returns {number} Random HTTP status code
 * @example
 * const status = randomHttpStatusCode();
 * console.log(status); // 200
 */
export function randomHttpStatusCode(): number {
  const statusCodes = [200, 201, 203, 204, 301, 302, 304, 400, 401, 403, 404, 429, 500, 502, 503];
  return statusCodes[Math.floor(Math.random() * statusCodes.length)];
}

/**
 * Generates a random HTTP method
 * @returns {string} Random HTTP method
 * @example
 * const method = randomHttpMethod();
 * console.log(method); // "GET"
 */
export function randomHttpMethod(): string {
  const methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
  return methods[Math.floor(Math.random() * methods.length)];
}

/**
 * Generates a random organization name
 * @returns {string} Random organization name
 * @example
 * const org = randomAsOrganization();
 * console.log(org); // "Enron Drilling Solutions"
 */
export function randomOrganizationName(): string {
  const orgs = [
    "Enron Drilling Solutions",
    "Apex Digital Networks Inc",
    "Meridian Cloud Solutions",
    "TechFlow Systems Corporation",
    "Global Connect Holdings",
    "NetVault Infrastructure Ltd",
    "Quantum Fiber Networks",
    "Atlas Computing Services",
    "Pinnacle Data Systems",
    "Horizon Technology Group",
    "Velocity Networks International",
    "StreamLine Digital Corp",
    "CoreBridge Communications",
    "NexGen Infrastructure Solutions",
    "DataSphere Technologies",
    "Zenith Network Services",
    "Fusion Cloud Dynamics",
    "Prism Connectivity Solutions",
    "Summit Digital Infrastructure",
    "Catalyst Network Systems",
    "Elevate Tech Holdings"
  ];
  return orgs[Math.floor(Math.random() * orgs.length)];
}

/**
 * Generates a random route path
 * @returns {string} Random route path
 * @example
 * const path = randomRoutePath();
 * console.log(path); // "/api/vnis/data-pipeline"
 */
export function randomRoutePath(): string {
  const routePaths = [
    "/api/vnis/data-pipeline",
    "/apex-networks/bandwidth-monitor",
    "/meridian/cloud-storage/sync",
    "/techflow/system-metrics",
    "/globalconnect/fiber-status",
    "/netvault/infrastructure/health",
    "/quantum-fiber/network-topology",
    "/atlas/compute-clusters",
    "/pinnacle/data-warehouse/query",
    "/horizon-tech/analytics-engine",
    "/velocity/cdn-edge-nodes",
    "/streamline/media-delivery",
    "/corebridge/communication-hub",
    "/nexgen/smart-routing",
    "/datasphere/ml-inference",
    "/zenith/network-optimization",
    "/fusion-cloud/auto-scaling",
    "/prism/connectivity-mesh",
    "/summit/edge-computing",
    "/catalyst/orchestration-api",
    "/elevate/performance-tuning",
    "/apex/traffic-shaping",
    "/meridian/backup-services",
    "/techflow/log-aggregation",
    "/globalconnect/peering-exchange",
    "/netvault/security-gateway",
    "/quantum/entanglement-protocol",
    "/atlas/container-registry",
    "/pinnacle/stream-processing",
    "/horizon/iot-device-management",
    "/velocity/acceleration-proxy",
    "/streamline/adaptive-bitrate",
    "/corebridge/message-queue",
    "/nexgen/ai-driven-routing",
    "/datasphere/real-time-analytics",
    "/zenith/load-balancer",
    "/fusion/serverless-functions",
    "/prism/mesh-gateway",
    "/summit/distributed-cache",
    "/catalyst/workflow-engine"
  ];
  return routePaths[Math.floor(Math.random() * routePaths.length)];
}
