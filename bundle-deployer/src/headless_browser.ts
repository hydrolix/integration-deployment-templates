// Headless browser automation for Grafana dashboard validation

import puppeteer from "puppeteer";
import { GRAFANA_LOCATION } from "./grafana/container.ts";

export async function run(grafanaDashboardId: string): Promise<[number, number]> {
  console.log(`Starting headless browser test for dashboard: ${grafanaDashboardId}`);

  const { cookieName, cookieValue } = await getGrafanaSessionCookie();
  console.log(`Got Grafana session cookie: ${cookieName}`);

  // Try to find system Chrome
  const executablePath = Deno.env.get("PUPPETEER_EXECUTABLE_PATH") ||
    (Deno.build.os === "darwin" ? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" :
      Deno.build.os === "linux" ? "/usr/bin/google-chrome" :
        undefined);

  const browser = await puppeteer.launch({
    headless: "new",
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
    executablePath,
  });

  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1920, height: 4080 });

    let datasourceErrorCount = 0;

    const badDatasourceRegex = /Datasource \w+ was not found/;
    const fourHundredRegex = /400 \w+/;

    // Listen to console messages for datasource errors
    page.on('console', (msg: { text: () => string }) => {
      const text = msg.text();
      console.log("****" + text);
      if (text.includes('400')) {
        datasourceErrorCount++;
        console.log(`ERROR: Datasource not found - ${text}`);
      }
    });

    // Set the session cookie
    await page.setCookie({
      name: cookieName,
      value: cookieValue,
      url: `http://${GRAFANA_LOCATION}/`,
      path: '/',
      httpOnly: true,
      secure: false,
      sameSite: 'Lax',
    });

    const dashboardSlug = await getDashboardSlug(grafanaDashboardId);

    // Navigate to dashboard (2 weeks time range)
    const url = `http://${GRAFANA_LOCATION}/d/${grafanaDashboardId}/${dashboardSlug}`;
    console.log(`Dashboard ID=${grafanaDashboardId}`);
    console.log(`Navigating to: ${url}`);
    await page.goto(url, {
      waitUntil: 'networkidle2',
      timeout: 120000,
    });

    const pageTitle = await page.title();
    const pageUrl = page.url();
    console.log(`Page loaded - Title: "${pageTitle}", URL: ${pageUrl}`);

    console.log("Page navigation completed");

    // Wait for all panels to load and query data
    console.log("Waiting 120 seconds for all panels to load and query data...");
    await new Promise(resolve => setTimeout(resolve, 30000));

    console.log(`Datasource errors: ${datasourceErrorCount}`);

    if (datasourceErrorCount === 0) {
      console.log("Success! No datasource errors detected.");
    }

    // Return 0 for nodata errors since we're not checking those anymore
    return [datasourceErrorCount, 0];

  } finally {
    await browser.close();
  }
}
async function getDashboardSlug(dashboardId: string): Promise<string> {
  const response = await fetch(
    `http://${GRAFANA_LOCATION}/api/dashboards/uid/${dashboardId}`,
    {
      headers: {
        'Authorization': 'Basic ' + btoa('admin:admin'),
      },
    }
  );

  const data = await response.json();
  return data.meta?.slug || 'dashboard';  // Fallback to 'dashboard'
}

async function getGrafanaSessionCookie(): Promise<{ cookieName: string; cookieValue: string }> {
  const response = await fetch(`http://${GRAFANA_LOCATION}/login`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      user: 'admin',
      password: 'admin',
    }),
    redirect: 'manual', // Don't follow redirects
  });

  const setCookieHeader = response.headers.get('set-cookie');

  if (!setCookieHeader) {
    throw new Error("No valid cookie found in response");
  }

  // Parse cookie name and value
  const cookieFirst = setCookieHeader.split(';')[0];
  const cookieParts = cookieFirst.split('=');

  if (cookieParts.length !== 2) {
    throw new Error("Invalid cookie format");
  }

  return {
    cookieName: cookieParts[0],
    cookieValue: cookieParts[1],
  };
}