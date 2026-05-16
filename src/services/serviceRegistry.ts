export interface ServiceDefinition {
  id: string;
  label: string;
  url: string;
  /** lucide-react icon name (imported by Sidebar) */
  icon: string;
}

export const SERVICES: ServiceDefinition[] = [
  {
    id: "spotify",
    label: "Spotify",
    url: "https://open.spotify.com",
    icon: "Music",
  },
  {
    id: "youtube-music",
    label: "YouTube Music",
    url: "https://music.youtube.com",
    icon: "Tv2",
  },
  {
    id: "youtube",
    label: "YouTube",
    url: "https://www.youtube.com",
    icon: "Play",
  },
  {
    id: "apple-music",
    label: "Apple Music",
    url: "https://music.apple.com",
    icon: "Headphones",
  },
  {
    id: "soundcloud",
    label: "SoundCloud",
    url: "https://soundcloud.com",
    icon: "Radio",
  },
  {
    id: "tidal",
    label: "Tidal",
    url: "https://listen.tidal.com",
    icon: "Waves",
  },
  {
    id: "deezer",
    label: "Deezer",
    url: "https://www.deezer.com",
    icon: "Disc",
  },
];
