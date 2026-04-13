/**
 * DownloadProgress - Show download progress and completed tracks
 */

import { useState, useEffect } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TrackInfo, DownloadProgressEvent, DownloadResult } from '../types';

interface DownloadProgressProps {
  selectedTracks: TrackInfo[];
  isDownloading: boolean;
  result: DownloadResult | null;
  onComplete: () => void;
  onBackToSelection: () => void;
}

interface CompletedTrack {
  title: string;
  status: 'completed' | 'error';
  error?: string;
}

export function DownloadProgress({
  selectedTracks,
  isDownloading,
  result,
  onComplete,
  onBackToSelection,
}: DownloadProgressProps) {
  const [progress, setProgress] = useState(0);
  const [currentTrack, setCurrentTrack] = useState<string>('');
  const [completedTracks, setCompletedTracks] = useState<CompletedTrack[]>([]);
  const [unlistenDownloadProgress, setUnlistenDownloadProgress] = useState<UnlistenFn | null>(
    null
  );

  // Setup event listener for download progress
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setupListener = async () => {
      try {
        unlisten = await listen<DownloadProgressEvent>('download-progress', (event) => {
          const { current, total, track_title, status } = event.payload;

          setProgress((current / total) * 100);
          setCurrentTrack(track_title);

          if (status === 'completed') {
            setCompletedTracks((prev) => [
              ...prev,
              { title: track_title, status: 'completed' },
            ]);
          } else if (status === 'error') {
            setCompletedTracks((prev) => [
              ...prev,
              { title: track_title, status: 'error' },
            ]);
          }
        });

        setUnlistenDownloadProgress(unlisten);
      } catch (error) {
        console.error('Error setting up download listener:', error);
      }
    };

    setupListener();

    // Cleanup
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const handleDone = () => {
    if (unlistenDownloadProgress) {
      unlistenDownloadProgress();
    }
    onComplete();
  };

  // Show download completed summary
  if (result && !isDownloading) {
    return (
      <div className="screen download-progress">
        <button className="back-button" onClick={onBackToSelection}>
          ← Retour
        </button>

        <div className="download-summary">
          <div className="summary-icon">
            {result.failed === 0 ? '✨' : '⚠️'}
          </div>
          <h2 className="summary-title">
            {result.failed === 0
              ? 'Téléchargement réussi !'
              : 'Téléchargement terminé'}
          </h2>

          <div className="summary-stats">
            {result.successful > 0 && (
              <div className="summary-stat success">
                <div className="summary-stat-value">{result.successful}</div>
                <div className="summary-stat-label">
                  réussi{result.successful > 1 ? 'es' : 'e'}
                </div>
              </div>
            )}
            {result.failed > 0 && (
              <div className="summary-stat error">
                <div className="summary-stat-value">{result.failed}</div>
                <div className="summary-stat-label">
                  erreur{result.failed > 1 ? 's' : ''}
                </div>
              </div>
            )}
          </div>

          {result.errors.length > 0 && (
            <div className="summary-errors">
              {result.errors.map((error, idx) => (
                <div key={idx} className="summary-error-item">
                  {error}
                </div>
              ))}
            </div>
          )}

          <button
            className="button button-primary"
            onClick={handleDone}
            style={{ alignSelf: 'stretch' }}
          >
            ✓ Fermer
          </button>
        </div>
      </div>
    );
  }

  // Show downloading progress
  return (
    <div className="screen download-progress">
      <div className="progress-header">
        <h2 className="progress-title">Téléchargement en cours...</h2>
        <span className="progress-counter">
          {completedTracks.length + (currentTrack ? 1 : 0)} / {selectedTracks.length}
        </span>
      </div>

      <div className="progress-bar-container">
        <div
          className="progress-bar"
          style={{
            width: `${progress}%`,
          }}
        ></div>
      </div>

      {currentTrack && (
        <div className="progress-current-track">
          <div className="progress-spinner">⚙️</div>
          <div className="progress-track-info">
            <div className="progress-track-label">En cours</div>
            <div className="progress-track-title">{currentTrack}</div>
          </div>
        </div>
      )}

      {completedTracks.length > 0 && (
        <>
          <div style={{ marginTop: 'var(--spacing-lg)' }}>
            <div style={{ fontSize: '13px', fontWeight: '500', marginBottom: 'var(--spacing-md)', color: 'var(--color-text-secondary)' }}>
              Pistes téléchargées
            </div>
            <div className="completed-tracks">
              {completedTracks.map((track, idx) => (
                <div
                  key={idx}
                  className={`completed-track ${track.status}`}
                >
                  <div className="completed-status-icon">
                    {track.status === 'completed' ? '✓' : '✗'}
                  </div>
                  <div className="completed-track-title">{track.title}</div>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
