export function displayNameFromBaseUrl(): string {
  const baseUrl = import.meta.env.BASE_URL;
  return baseUrl.replace(/^\/|\/$/g, "");
}
