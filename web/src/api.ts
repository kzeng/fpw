export type HealthResponse = {
  status: string;
  service: string;
};

export type RecentProject = {
  path: string;
  name: string;
  updatedAtUnixMs: number;
};

export type RecentProjectsResponse = {
  projects: RecentProject[];
};

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(path, { headers: { Accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}`);
  }
  return response.json() as Promise<T>;
}

export function getHealth(): Promise<HealthResponse> {
  return getJson<HealthResponse>("/api/health");
}

export function getRecentProjects(): Promise<RecentProjectsResponse> {
  return getJson<RecentProjectsResponse>("/api/recent-projects");
}

