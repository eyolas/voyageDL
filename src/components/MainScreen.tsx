/**
 * MainScreen - Main interface for URL analysis and track selection.
 * Supports multiple analyses, pause, and cancel.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { TrackInfo, AppConfig, AnalyzeProgressEvent } from '../types';
import { TrackList } from './TrackList';
import { Alert } from './Alert';

interface MainScreenProps {
  config: AppConfig;
  onSettingsClick: () => void;
  onChangeFolder: (newPath: string) => Promise<void>;
  onAddToQueue: (tracks: TrackInfo[]) => void;
}

type URLType = 'youtube' | 'deezer';

interface AnalyzeResult {
  id: string;
  url: string;
  urlType: URLType;
  tracks: TrackInfo[];
}

let resultCounter = 0;

export function MainScreen({
  config,
  onSettingsClick,
  onChangeFolder,
  onAddToQueue,
}: MainScreenProps) {
  const [urlInput, setUrlInput] = useState('');
  const [activeCount, setActiveCount] = useState(0);
  const [paused, setPaused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [analyzeProgress, setAnalyzeProgress] = useState<AnalyzeProgressEvent | null>(null);
  const [results, setResults] = useState<AnalyzeResult[]>([]);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Listen to analyze-progress events
  useEffect(() => {
    const setup = async () => {
      unlistenRef.current = await listen<AnalyzeProgressEvent>('analyze-progress', (event) => {
        setAnalyzeProgress(event.payload);
        if (event.payload.status === 'paused') {
          setPaused(true);
        } else {
          setPaused(false);
        }
      });
    };
    setup();
    return () => { unlistenRef.current?.(); };
  }, []);

  const detectURLType = (url: string): { type: URLType; url: string } | null => {
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
    const detected = detectURLType(urlInput);
    if (!detected) {
      setError('Veuillez entrer une URL YouTube ou Deezer valide');
      return;
    }

    setError(null);
    setPaused(false);
    setAnalyzeProgress(null);
    setActiveCount((c) => c + 1);

    const analyzedUrl = urlInput;
    const analyzedType = detected.type;
    setUrlInput('');

    // Run analysis in background - don't block the UI
    invoke<TrackInfo[]>(
      analyzedType === 'youtube' ? 'fetch_youtube_info' : 'fetch_deezer_playlist',
      { url: detected.url },
    )
      .then((tracks) => {
        if (tracks.length > 0) {
          resultCounter += 1;
          setResults((prev) => [
            { id: `result-${resultCounter}`, url: analyzedUrl, urlType: analyzedType, tracks },
            ...prev,
          ]);
        }
      })
      .catch((err) => {
        const errorMsg = err instanceof Error ? err.message : String(err);
        setError(errorMsg);
      })
      .finally(() => {
        setActiveCount((c) => c - 1);
        setPaused(false);
        setAnalyzeProgress(null);
      });
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && urlInput.trim()) {
      handleAnalyze();
    }
  };

  const handleCancel = async () => {
    try {
      await invoke('cancel_analyze');
    } catch (err) {
      console.error('Failed to cancel analyze:', err);
    }
  };

  const handleTogglePause = async () => {
    try {
      const isPaused = await invoke<boolean>('toggle_pause_analyze');
      setPaused(isPaused);
    } catch (err) {
      console.error('Failed to toggle pause:', err);
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

  const handleAddToQueue = useCallback((resultId: string, selectedTracks: TrackInfo[]) => {
    onAddToQueue(selectedTracks);
    setResults((prev) => prev.filter((r) => r.id !== resultId));
  }, [onAddToQueue]);

  const handleDismissResult = useCallback((resultId: string) => {
    setResults((prev) => prev.filter((r) => r.id !== resultId));
  }, []);

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
            />
            <button
              className="analyze-button"
              onClick={handleAnalyze}
              disabled={!urlInput.trim()}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              Analyser
            </button>
          </div>
        </div>

        {/* Active analysis */}
        {activeCount > 0 && (
          <div className="analyze-loading">
            <div className="analyze-loading-content">
              {paused ? (
                <svg width="28" height="28" viewBox="0 0 24 24" fill="var(--color-warning)" stroke="none">
                  <rect x="6" y="4" width="4" height="16" rx="1" />
                  <rect x="14" y="4" width="4" height="16" rx="1" />
                </svg>
              ) : (
                <div className="equalizer" style={{ justifyContent: 'center', height: '36px' }}>
                  <div className="equalizer-bar" />
                  <div className="equalizer-bar" />
                  <div className="equalizer-bar" />
                  <div className="equalizer-bar" />
                  <div className="equalizer-bar" />
                </div>
              )}
              <div className="analyze-loading-text">
                {analyzeProgress ? (
                  <>
                    <span className="analyze-loading-title">
                      {paused ? 'En pause' : 'Recherche YouTube'} {analyzeProgress.current}/{analyzeProgress.total}
                    </span>
                    <span className="analyze-loading-detail">
                      {analyzeProgress.artist} — {analyzeProgress.track_title}
                    </span>
                  </>
                ) : (
                  <>
                    <span className="analyze-loading-title">Analyse en cours...</span>
                    <span className="analyze-loading-detail">
                      Recuperation des informations
                    </span>
                  </>
                )}
              </div>
            </div>
            <div className="analyze-loading-actions">
              {analyzeProgress && (
                <button className="analyze-action-btn pause" onClick={handleTogglePause} title={paused ? 'Reprendre' : 'Pause'}>
                  {paused ? (
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="none">
                      <polygon points="5 3 19 12 5 21 5 3" />
                    </svg>
                  ) : (
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="none">
                      <rect x="6" y="4" width="4" height="16" rx="1" />
                      <rect x="14" y="4" width="4" height="16" rx="1" />
                    </svg>
                  )}
                  {paused ? 'Reprendre' : 'Pause'}
                </button>
              )}
              <button className="analyze-action-btn cancel" onClick={handleCancel} title="Annuler">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
                Annuler
              </button>
            </div>
            <div className="analyze-loading-bar">
              <div
                className="analyze-loading-bar-fill"
                style={analyzeProgress ? {
                  animation: paused ? 'none' : undefined,
                  width: `${(analyzeProgress.current / analyzeProgress.total) * 100}%`,
                  transition: 'width 0.3s ease-out',
                } : undefined}
              />
            </div>
          </div>
        )}

        {/* Results list */}
        {results.map((result) => (
          <div key={result.id} className="analyze-result">
            <div className="analyze-result-header">
              <span className="analyze-result-badge">{result.urlType === 'deezer' ? 'Deezer' : 'YouTube'}</span>
              <span className="analyze-result-count">{result.tracks.length} piste{result.tracks.length > 1 ? 's' : ''}</span>
              <button className="analyze-result-dismiss" onClick={() => handleDismissResult(result.id)} title="Fermer">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
            <TrackList
              tracks={result.tracks}
              onAddToQueue={(selectedTracks) => handleAddToQueue(result.id, selectedTracks)}
            />
          </div>
        ))}

        {/* Empty state */}
        {activeCount === 0 && results.length === 0 && (
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
