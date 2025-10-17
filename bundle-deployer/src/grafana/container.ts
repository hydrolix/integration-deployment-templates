// Grafana Docker container management

import { getErrorMessage } from "../utils/error.ts";

export const GRAFANA_LOCATION = "localhost:3000";

export async function kill(): Promise<void> {
  try {
    // List running containers
    const listCmd = new Deno.Command("docker", {
      args: ["ps", "-a", "--format", "{{.ID}}\t{{.Image}}", "--filter", "status=running"],
    });
    
    const { stdout } = await listCmd.output();
    const output = new TextDecoder().decode(stdout);
    const lines = output.trim().split("\n");
    
    for (const line of lines) {
      if (!line) continue;
      
      const [containerId, image] = line.split("\t");
      
      if (image === "javiani/grafana:latest") {
        console.log(`Killing old Grafana container ${image} ${containerId}`);
        
        const killCmd = new Deno.Command("docker", {
          args: ["kill", containerId],
        });
        
        await killCmd.output();
        return;
      }
    }
  } catch (e) {
    throw new Error(`Failed to kill container: ${getErrorMessage(e)}`);
  }
}

export async function start(): Promise<void> {
  try {
    // Start Docker container in detached mode
    const cmd = new Deno.Command("docker", {
      args: ["run", "--rm", "-d", "-p", "3000:3000", "javiani/grafana:latest"],
      stdout: "piped",
      stderr: "piped",
    });
    
    await cmd.output();
    
    // Wait for Grafana to be ready
    await waitForGrafanaReady(GRAFANA_LOCATION, 60);
  } catch (e) {
    throw new Error(`Failed to start Grafana container: ${getErrorMessage(e)}`);
  }
}

async function waitForGrafanaReady(baseUrl: string, maxWaitSecs: number): Promise<void> {
  const healthUrl = `http://${baseUrl}/api/health`;
  const startTime = Date.now();
  
  while (true) {
    const elapsed = (Date.now() - startTime) / 1000;
    
    if (elapsed > maxWaitSecs) {
      throw new Error("Grafana failed to become ready within timeout");
    }
    
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 2000);
      
      const response = await fetch(healthUrl, {
        signal: controller.signal,
      });
      
      clearTimeout(timeoutId);
      
      if (response.ok) {
        console.log("Grafana is ready!");
        return;
      }
    } catch {
      // Ignore errors and retry
    }
    
    console.log("Grafana not ready yet, waiting...");
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
}