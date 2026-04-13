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
  album?: string;
  album_cover_url?: string;
  track_number?: number;
  year?: string;
}

export interface AppConfig {
  download_dir: string;
}

export interface DownloadProgressEvent {
  current: number;
  total: number;
  track_title: string;
  track_id: string;
  status: 'downloading' | 'completed' | 'error' | 'cancelled';
}

export interface DownloadResult {
  successful: number;
  failed: number;
  errors: string[];
}

export interface SelectedTrack extends TrackInfo {
  selected: boolean;
}

export interface AnalyzeProgressEvent {
  current: number;
  total: number;
  track_title: string;
  artist: string;
  status: string;
}
