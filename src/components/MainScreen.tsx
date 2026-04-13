/**
 * MainScreen - Main interface for URL analysis and track selection
 */

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { TrackInfo, AppConfig } from '../types';
import { TrackList } from './TrackList';
import { Alert } from './Alert';

interface MainScreenProps {
  config: AppConfig;
  onSettingsClick: () => void;
  onChangeFolder: (newPath: string) => Promise<void>;
  onAddToQueue: (tracks: TrackInfo[]) => void;
}

type URLType = 'youtube' | 'deezer' | null;

interface DetectedURL {
  type: URLType;
  url: string;
}

export function MainScreen({
  config,
  onSettingsClick,
  onChangeFolder,
  onAddToQueue,
}: MainScreenProps) {
  const [urlInput, setUrlInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [tracks, setTracks] = useState<TrackInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Detect URL type (YouTube or Deezer)
  const detectURLType = (url: string): DetectedURL | null => {
    const trimmedUrl = url.trim();

    if (
      trimmedUrl.includes('youtube.com') ||
      trimmedUrl.includes('youtu.be') ||
      trimmedUrl.includes('youtube-nocookie.com')
    ) {
      return { type: 'youtube', url: trimmedUrl };
    }

    if (trimmedUrl.includes('deezer.com')) {
      return { type: 'deezer', url: trimmedUrl };
    }

    return null;
  };

  const handleAnalyze = async () => {
    try {
      setError(null);
      setTracks([]);

      const detected = detectURLType(urlInput);
      if (!detected) {
        setError('Veuillez entrer une URL YouTube ou Deezer valide');
        return;
      }

      setLoading(true);

      if (detected.type === 'youtube') {
        const result = await invoke<TrackInfo[]>('fetch_youtube_info', {
          url: detected.url,
        });
        setTracks(result);
      } else if (detected.type === 'deezer') {
        const result = await invoke<TrackInfo[]>('fetch_deezer_playlist', {
          url: detected.url,
        });
        setTracks(result);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      setTracks([]);
      console.error('Error analyzing URL:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !loading && urlInput.trim()) {
      handleAnalyze();
    }
  };

  const handleChangeFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        title: 'Selectionner le dossier de telechargement',
      });

      if (selected && typeof selected === 'string') {
        await onChangeFolder(selected);
      }
    } catch (error) {
      console.error('Error selecting folder:', error);
    }
  };

  const handleAddToQueue = (selectedTracks: TrackInfo[]) => {
    onAddToQueue(selectedTracks);
    setTracks([]);
    setUrlInput('');
  };

  return (
    <div className="screen main-screen">
      <div className="main-header">
        <h1 className="main-header-title">
          <svg className="main-header-icon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18V5l12-2v13" />
            <circle cx="6" cy="18" r="3" />
            <circle cx="18" cy="16" r="3" />
          </svg>
          Voyage DL
        </h1>
        <div className="main-header-actions">
          <button className="settings-button" onClick={onSettingsClick} title="Parametres">
            ⚙️
          </button>
        </div>
      </div>

      <div className="main-content">
        <div className="download-folder-info" onClick={handleChangeFolder} title="Cliquer pour modifier">
          <span className="folder-icon">📁</span>
          <span>
            {config.download_dir ? `Dossier: ${config.download_dir}` : 'Pas de dossier configure'}
          </span>
        </div>

        {error && (
          <Alert
            type="error"
            title="Erreur"
            message={error}
            onClose={() => setError(null)}
          />
        )}

        <div className="url-input-section">
          <label className="url-input-label">URL YouTube ou Deezer</label>
          <div className="url-input-wrapper">
            <input
              type="text"
              className="url-input"
              placeholder="Colle une URL YouTube ou Deezer..."
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              onKeyPress={handleKeyPress}
              disabled={loading}
            />
            <button
              className={`analyze-button ${loading ? 'loading' : ''}`}
              onClick={handleAnalyze}
              disabled={loading || !urlInput.trim()}
            >
              {loading ? (
                <>
                  <svg
                    className="spinner"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <circle cx="12" cy="12" r="1"></circle>
                    <path d="M12 1v6"></path>
                    <path d="M12 17v6"></path>
                    <path d="M4.22 4.22l4.24 4.24"></path>
                    <path d="M15.54 15.54l4.24 4.24"></path>
                    <path d="M1 12h6"></path>
                    <path d="M17 12h6"></path>
                    <path d="M4.22 19.78l4.24-4.24"></path>
                    <path d="M15.54 8.46l4.24-4.24"></path>
                  </svg>
                  Analyse...
                </>
              ) : (
                <>
                  <span>🔍</span>
                  Analyser
                </>
              )}
            </button>
          </div>
        </div>

        {loading && (
          <div className="analyze-loading">
            <div className="analyze-loading-content">
              <div className="equalizer" style={{ justifyContent: 'center', height: '36px' }}>
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
              </div>
              <div className="analyze-loading-text">
                <span className="analyze-loading-title">Analyse en cours...</span>
                <span className="analyze-loading-detail">
                  {detectURLType(urlInput)?.type === 'deezer'
                    ? 'Recuperation de la playlist Deezer'
                    : 'Recuperation des informations YouTube'}
                </span>
              </div>
            </div>
            <div className="analyze-loading-bar">
              <div className="analyze-loading-bar-fill" />
            </div>
          </div>
        )}

        {tracks.length > 0 && (
          <TrackList
            tracks={tracks}
            onAddToQueue={handleAddToQueue}
          />
        )}

        {!loading && tracks.length === 0 && (
          <div className="main-empty-state">
            <div className="main-empty-state-inner">
              <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-tertiary)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18V5l12-2v13" />
                <circle cx="6" cy="18" r="3" />
                <circle cx="18" cy="16" r="3" />
              </svg>
              <p className="main-empty-state-text">
                Colle une URL YouTube ou Deezer pour commencer
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
