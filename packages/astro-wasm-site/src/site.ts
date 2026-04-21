export function displayNameFromBaseUrl(baseUrl: string): string {
  return baseUrl.replace(/^\/|\/$/g, "");
}

export function withBaseUrl(baseUrl: string, href: string): string {
  const normalizedBaseUrl = baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
  const normalizedHref = href.startsWith("/") ? href : `/${href}`;
  return `${normalizedBaseUrl}${normalizedHref}`;
}
