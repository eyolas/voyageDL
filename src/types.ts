/**
 * Type definitions for Voyage DL
 */

export interface TrackInfo {
  id: string;
  title: string;
  artist: string;
  url: string;
  thumbnail_url: string;
  duration_seconds: number;
}

export interface AppConfig {
  download_dir: string;
}

export interface DownloadProgressEvent {
  current: number;
  total: number;
  track_title: string;
  status: 'downloading' | 'completed' | 'error';
}

export interface DownloadResult {
  successful: number;
  failed: number;
  errors: string[];
}

export interface SelectedTrack extends TrackInfo {
  selected: boolean;
}
