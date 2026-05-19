export interface ServiceDefinition {
  id: string;
  label: string;
  url: string;
  faviconUrl: string;
  isCustom?: boolean;
}

// DuckDuckGo's favicon service returns images directly (no redirect),
// works for all major sites, and doesn't require CORS negotiation.
const fav = (domain: string) =>
  `https://icons.duckduckgo.com/ip3/${domain}.ico`;

export const SERVICES: ServiceDefinition[] = [
  // ── Video streaming ───────────────────────────────────────────────────────
  {
    id: "netflix",
    label: "Netflix",
    url: "https://www.netflix.com",
    faviconUrl: fav("netflix.com"),
  },
  {
    id: "disney-plus",
    label: "Disney+",
    url: "https://www.disneyplus.com",
    faviconUrl: fav("disneyplus.com"),
  },
  {
    id: "prime-video",
    label: "Prime Video",
    url: "https://www.primevideo.com",
    faviconUrl: fav("primevideo.com"),
  },
  {
    id: "hulu",
    label: "Hulu",
    url: "https://www.hulu.com",
    faviconUrl: fav("hulu.com"),
  },
  {
    id: "max",
    label: "Max",
    url: "https://www.max.com",
    faviconUrl: fav("max.com"),
  },
  {
    id: "peacock",
    label: "Peacock",
    url: "https://www.peacocktv.com",
    faviconUrl: fav("peacocktv.com"),
  },
  {
    id: "paramount-plus",
    label: "Paramount+",
    url: "https://www.paramountplus.com",
    faviconUrl: fav("paramountplus.com"),
  },
  {
    id: "apple-tv",
    label: "Apple TV+",
    url: "https://tv.apple.com",
    faviconUrl: fav("tv.apple.com"),
  },
  {
    id: "crunchyroll",
    label: "Crunchyroll",
    url: "https://www.crunchyroll.com",
    faviconUrl: fav("crunchyroll.com"),
  },
  {
    id: "twitch",
    label: "Twitch",
    url: "https://www.twitch.tv",
    faviconUrl: fav("twitch.tv"),
  },
  // ── Music streaming ───────────────────────────────────────────────────────
  {
    id: "spotify",
    label: "Spotify",
    url: "https://open.spotify.com",
    faviconUrl: fav("open.spotify.com"),
  },
  {
    id: "apple-music",
    label: "Apple Music",
    url: "https://music.apple.com",
    faviconUrl: fav("music.apple.com"),
  },
  {
    id: "youtube-music",
    label: "YouTube Music",
    url: "https://music.youtube.com",
    faviconUrl: fav("music.youtube.com"),
  },
  {
    id: "tidal",
    label: "Tidal",
    url: "https://listen.tidal.com",
    faviconUrl: fav("listen.tidal.com"),
  },
  {
    id: "amazon-music",
    label: "Amazon Music",
    url: "https://music.amazon.com",
    faviconUrl: fav("music.amazon.com"),
  },
];
