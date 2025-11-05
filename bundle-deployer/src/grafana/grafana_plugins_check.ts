// Validation: Check Grafana plugin requirements by querying deployed dashboards
// Uses Grafana's own API to detect which plugins are actually being used

import { getErrorMessage } from "../utils/error.ts";
import { GRAFANA_LOCATION } from "./container.ts";

interface PluginInfo {
  id: string;
  name: string;
  type: string;
  enabled: boolean;
  version?: string;
}

interface DashboardPluginUsage {
  dashboardUid: string;
  dashboardTitle: string;
  pluginId: string;
  pluginType: string;
  panelCount: number;
}

// Check for strict mode at module level
const STRICT_PLUGINS = Deno.args.includes("--strict-plugins") || 
                       Deno.env.get("STRICT_PLUGIN_VALIDATION") === "true";

/**
 * Check deployed dashboards for plugin usage after deployment
 * This should be called AFTER dashboards are created in Grafana
 */
export async function checkDeployedDashboards(dashboardUids: string[]): Promise<void> {
  console.log("\n🔌 Checking deployed dashboards for plugin usage...");
  
  if (dashboardUids.length === 0) {
    console.log("  ℹ️  No dashboards to check");
    return;
  }
  
  // Get all installed plugins first
  const installedPlugins = await getInstalledPlugins();
  const installedMap = new Map(installedPlugins.map(p => [p.id, p]));
  
  // Track plugin usage across all dashboards
  const pluginUsage = new Map<string, DashboardPluginUsage[]>();
  
  // Query each deployed dashboard
  for (const uid of dashboardUids) {
    try {
      const dashboard = await getDashboardByUid(uid);
      const plugins = extractPluginsFromDashboard(dashboard);
      
      for (const [pluginId, count] of plugins.entries()) {
        if (!pluginUsage.has(pluginId)) {
          pluginUsage.set(pluginId, []);
        }
        
        pluginUsage.get(pluginId)!.push({
          dashboardUid: uid,
          dashboardTitle: dashboard.dashboard?.title || uid,
          pluginId: pluginId,
          pluginType: installedMap.get(pluginId)?.type || "unknown",
          panelCount: count,
        });
      }
    } catch (e) {
      console.warn(`  ⚠️  Could not check dashboard ${uid}: ${getErrorMessage(e)}`);
      // Continue checking other dashboards even if one fails
    }
  }
  
  if (pluginUsage.size === 0) {
    console.log("  ✓ Dashboards only use built-in panels");
    return;
  }
  
  // Categorize plugins
  const installedExternal: string[] = [];
  const missingPlugins: string[] = [];
  
  for (const pluginId of pluginUsage.keys()) {
    const plugin = installedMap.get(pluginId);
    
    if (plugin) {
      if (plugin.type === "panel" && !isBuiltinPlugin(plugin.id)) {
        installedExternal.push(pluginId);
      }
    } else {
      missingPlugins.push(pluginId);
    }
  }
  
  // Report installed external plugins
  if (installedExternal.length > 0) {
    console.log(`\n✓ Using ${installedExternal.length} external plugin(s) (installed):`);
    for (const pluginId of installedExternal) {
      const plugin = installedMap.get(pluginId)!;
      const usage = pluginUsage.get(pluginId)!;
      const totalPanels = usage.reduce((sum, u) => sum + u.panelCount, 0);
      console.log(`  • ${plugin.name} (${pluginId}) - ${totalPanels} panel(s) across ${usage.length} dashboard(s)`);
    }
  }
  
  // Report missing plugins
  if (missingPlugins.length > 0) {
    const level = STRICT_PLUGINS ? "ERROR" : "WARNING";
    const icon = STRICT_PLUGINS ? "❌" : "⚠️";
    const logFn = STRICT_PLUGINS ? console.error : console.log;
    
    logFn(`\n${icon} ${level}: Missing plugins detected!`);
    logFn("\nMissing plugins:");
    
    for (const pluginId of missingPlugins) {
      const usage = pluginUsage.get(pluginId)!;
      const totalPanels = usage.reduce((sum, u) => sum + u.panelCount, 0);
      
      logFn(`  • ${pluginId} - ${totalPanels} panel(s) across ${usage.length} dashboard(s)`);
      logFn(`    Used in:`);
      for (const u of usage) {
        logFn(`      - "${u.dashboardTitle}" (${u.panelCount} panel(s))`);
      }
    }
    
    logFn("\n📋 To fix:");
    logFn("  1. Update grafana/container.ts to install missing plugins");
    logFn("  2. Add to GF_INSTALL_PLUGINS environment variable:");
    logFn(`     "-e", "GF_INSTALL_PLUGINS=${missingPlugins.join(',')}"`);
    logFn("\n  Example:");
    logFn(`     const cmd = new Deno.Command("docker", {`);
    logFn(`       args: [`);
    logFn(`         "run", "--rm", "-d", "-p", "3000:3000",`);
    logFn(`         "-e", "GF_INSTALL_PLUGINS=${missingPlugins.join(',')}",`);
    logFn(`         "javiani/grafana:latest"`);
    logFn(`       ],`);
    logFn(`     });`);
    
    // FAIL HARD in strict mode
    if (STRICT_PLUGINS) {
      throw new Error(
        `Plugin validation failed: ${missingPlugins.length} required plugin(s) missing. ` +
        `Please install: ${missingPlugins.join(', ')}`
      );
    } else {
      logFn("\n⚠️  Dashboards may not display correctly without these plugins");
    }
  }
}

async function getDashboardByUid(uid: string): Promise<any> {
  const url = `http://${GRAFANA_LOCATION}/api/dashboards/uid/${uid}`;
  
  const response = await fetch(url, {
    headers: {
      'Authorization': 'Basic ' + btoa('admin:admin'),
    },
  });
  
  if (!response.ok) {
    throw new Error(`Failed to fetch dashboard: ${response.statusText}`);
  }
  
  return await response.json();
}

function extractPluginsFromDashboard(dashboardData: any): Map<string, number> {
  const pluginCounts = new Map<string, number>();
  const dashboard = dashboardData.dashboard || dashboardData;
  
  if (!dashboard.panels || !Array.isArray(dashboard.panels)) {
    return pluginCounts;
  }
  
  function processPanel(panel: any): void {
    if (panel.type) {
      const count = pluginCounts.get(panel.type) || 0;
      pluginCounts.set(panel.type, count + 1);
    }
    
    // Handle nested panels (rows)
    if (panel.panels && Array.isArray(panel.panels)) {
      panel.panels.forEach(processPanel);
    }
  }
  
  dashboard.panels.forEach(processPanel);
  
  // Filter out built-in panel types
  const externalPlugins = new Map<string, number>();
  for (const [type, count] of pluginCounts.entries()) {
    if (!isBuiltinPanelType(type)) {
      externalPlugins.set(type, count);
    }
  }
  
  return externalPlugins;
}

function isBuiltinPanelType(type: string): boolean {
  const builtinTypes = [
    "graph", "timeseries", "stat", "gauge", "bargauge", "table", "text",
    "alertlist", "dashlist", "heatmap", "logs", "nodeGraph", "barchart",
    "candlestick", "canvas", "geomap", "histogram", "live", "news",
    "piechart", "state-timeline", "status-history", "table-old", "trace",
    "xychart", "row", "grafana-piechart-panel", "graph-old"
  ];
  
  return builtinTypes.includes(type);
}

function isBuiltinPlugin(pluginId: string): boolean {
  // Grafana built-in plugins have specific prefixes
  return pluginId.startsWith("grafana-") && 
         (pluginId.endsWith("-datasource") || 
          pluginId.includes("builtin") ||
          pluginId === "grafana-clickhouse-datasource");
}

async function getInstalledPlugins(): Promise<PluginInfo[]> {
  const url = `http://${GRAFANA_LOCATION}/api/plugins`;
  
  const response = await fetch(url, {
    headers: {
      'Authorization': 'Basic ' + btoa('admin:admin'),
    },
  });
  
  if (!response.ok) {
    throw new Error(`Failed to fetch plugins: ${response.statusText}`);
  }
  
  const plugins = await response.json();
  
  if (!Array.isArray(plugins)) {
    throw new Error("Unexpected plugins response format");
  }
  
  return plugins.map((p: any) => ({
    id: p.id || p.name,
    name: p.name || p.id,
    type: p.type || "unknown",
    enabled: p.enabled !== false,
    version: p.info?.version,
  }));
}